#![allow(clippy::wildcard_imports)]

#[cfg(target_arch = "arm")]
use core::arch::arm::*;

use pyo3::{prelude::*, types::PyBytes};

use super::python;

const BYTES_CONSUMED_PER_ROUND: usize = 48;
const BYTES_PRODUCED_PER_ROUND: usize = 64;

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/neon32/enc_reshuffle.c
#[inline]
#[target_feature(enable = "neon")]
fn reshuffle(
    src: (uint8x16_t, uint8x16_t, uint8x16_t),
) -> (uint8x16_t, uint8x16_t, uint8x16_t, uint8x16_t) {
    // Input:
    // in[0]  = a7 a6 a5 a4 a3 a2 a1 a0
    // in[1]  = b7 b6 b5 b4 b3 b2 b1 b0
    // in[2]  = c7 c6 c5 c4 c3 c2 c1 c0

    // Output:
    // out[0] = 00 00 a7 a6 a5 a4 a3 a2
    // out[1] = 00 00 a1 a0 b7 b6 b5 b4
    // out[2] = 00 00 b3 b2 b1 b0 c7 c6
    // out[3] = 00 00 c5 c4 c3 c2 c1 c0

    // Move the input bits to where they need to be in the outputs. Except
    // for the first output, the high two bits are not cleared.
    let out0 = vshrq_n_u8(src.0, 2);
    let out1 = vshrq_n_u8(src.1, 4);
    let out2 = vshrq_n_u8(src.2, 6);
    let out1 = vsliq_n_u8(out1, src.0, 4);
    let out2 = vsliq_n_u8(out2, src.1, 2);

    // Clear the high two bits in the second, third and fourth output.
    let out1 = vandq_u8(out1, vdupq_n_u8(0x3F));
    let out2 = vandq_u8(out2, vdupq_n_u8(0x3F));
    let out3 = vandq_u8(src.2, vdupq_n_u8(0x3F));

    (out0, out1, out2, out3)
}

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/neon32/codec.c
#[inline(always)]
#[target_feature(enable = "neon")]
fn vqtbl1q_u8(lut: uint8x16_t, indices: uint8x16_t) -> uint8x16_t {
    // NEON32 only supports 64-bit wide lookups in 128-bit tables. Emulate
    // the NEON64 `vqtbl1q_u8` intrinsic to do 128-bit wide lookups.
    let lut2 = uint8x8x2_t(vget_low_u8(lut), vget_high_u8(lut));
    vcombine_u8(
        vtbl2_u8(lut2, vget_low_u8(indices)),
        vtbl2_u8(lut2, vget_high_u8(indices)),
    )
}

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/neon32/enc_translate.c
#[inline]
#[target_feature(enable = "neon")]
unsafe fn translate(
    src: (uint8x16_t, uint8x16_t, uint8x16_t, uint8x16_t),
) -> (uint8x16_t, uint8x16_t, uint8x16_t, uint8x16_t) {
    #[rustfmt::skip]
    let lut: uint8x16_t =  unsafe { vld1q_u8([
         65,  71, 252, 252,
        252, 252, 252, 252,
        252, 252, 252, 252,
        237, 240,   0,   0,
    ].as_ptr()) };

    let offset = vdupq_n_u8(51);

    let (src0, src1, src2, src3) = src;

    // Create LUT indices from input:
    // the index for range #0 is right, others are 1 less than expected:
    let indices0 = vqsubq_u8(src0, offset);
    let indices1 = vqsubq_u8(src1, offset);
    let indices2 = vqsubq_u8(src2, offset);
    let indices3 = vqsubq_u8(src3, offset);

    // mask is 0xFF (-1) for range #[1..4] and 0x00 for range #0:
    let mask0 = vcgtq_u8(src0, vdupq_n_u8(25));
    let mask1 = vcgtq_u8(src1, vdupq_n_u8(25));
    let mask2 = vcgtq_u8(src2, vdupq_n_u8(25));
    let mask3 = vcgtq_u8(src3, vdupq_n_u8(25));

    // Subtract -1, so add 1 to indices for range #[1..4], All indices are
    // now correct:
    let indices0 = vsubq_u8(indices0, mask0);
    let indices1 = vsubq_u8(indices1, mask1);
    let indices2 = vsubq_u8(indices2, mask2);
    let indices3 = vsubq_u8(indices3, mask3);

    // Lookup delta values:
    let delta0 = vqtbl1q_u8(lut, indices0);
    let delta1 = vqtbl1q_u8(lut, indices1);
    let delta2 = vqtbl1q_u8(lut, indices2);
    let delta3 = vqtbl1q_u8(lut, indices3);

    // Add delta values:
    let out0 = vaddq_u8(src0, delta0);
    let out1 = vaddq_u8(src1, delta1);
    let out2 = vaddq_u8(src2, delta2);
    let out3 = vaddq_u8(src3, delta3);

    (out0, out1, out2, out3)
}

#[inline]
#[target_feature(enable = "neon")]
unsafe fn encode_block(src: *const u8, dst: *mut u8) {
    // Load 48 bytes and deinterleave:
    let src = unsafe { vld3q_u8(src) };

    // Reshuffle:
    let mut out = reshuffle((src.0, src.1, src.2));

    // Translate reshuffled bytes to the Base64 alphabet:
    out = unsafe { translate(out) };

    // Interleave and store output:
    unsafe { vst4q_u8(dst, uint8x16x4_t(out.0, out.1, out.2, out.3)) };
}

#[target_feature(enable = "neon")]
pub unsafe fn encode_simd_prefix(
    mut src: *const u8,
    len: usize,
    mut dst: *mut u8,
    _alphabet: *const u8,
) -> usize {
    if len < 48 {
        return 0;
    }

    let mut rounds = len / BYTES_CONSUMED_PER_ROUND;
    let consumed = rounds * BYTES_CONSUMED_PER_ROUND;

    while rounds != 0 {
        unsafe { encode_block(src, dst) };
        src = unsafe { src.add(BYTES_CONSUMED_PER_ROUND) };
        dst = unsafe { dst.add(BYTES_PRODUCED_PER_ROUND) };
        rounds -= 1;
    }

    consumed
}

/// # Safety
///
/// The running CPU must support NEON.
pub unsafe fn encode<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    altchars: Option<&Bound<'py, PyAny>>,
    padded: bool,
    wrapcol: isize,
) -> PyResult<Bound<'py, PyBytes>> {
    python::encode(py, s, altchars, padded, wrapcol, encode_simd_prefix)
}
