#![allow(clippy::wildcard_imports)]

#[cfg(target_arch = "aarch64")]
use core::arch::aarch64::*;

use pyo3::{prelude::*, types::PyBytes};

use super::python;

const BYTES_CONSUMED_PER_ROUND: usize = 48;
const BYTES_PRODUCED_PER_ROUND: usize = 64;

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/neon64/enc_reshuffle.c
#[inline]
#[target_feature(enable = "neon")]
fn reshuffle(src: uint8x16x3_t) -> uint8x16x4_t {
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
    let mut out1 = vshrq_n_u8(src.1, 4);
    let mut out2 = vshrq_n_u8(src.2, 6);
    out1 = vsliq_n_u8(out1, src.0, 4);
    out2 = vsliq_n_u8(out2, src.1, 2);

    // Clear the high two bits in the second, third and fourth output.
    out1 = vandq_u8(out1, vdupq_n_u8(0x3F));
    out2 = vandq_u8(out2, vdupq_n_u8(0x3F));
    let out3 = vandq_u8(src.2, vdupq_n_u8(0x3F));

    uint8x16x4_t(out0, out1, out2, out3)
}

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/neon64/enc_loop.c
#[inline]
#[target_feature(enable = "neon")]
unsafe fn encode_block(src: *const u8, dst: *mut u8, tbl_enc: uint8x16x4_t) {
    // Load 48 bytes and deinterleave:
    let src = unsafe { vld3q_u8(src) };

    // Divide bits of three input bytes over four output bytes:
    let mut out = reshuffle(src);

    // The bits have now been shifted to the right locations;
    // translate their values 0..63 to the Base64 alphabet.
    // Use a 64-byte table lookup:
    out.0 = vqtbl4q_u8(tbl_enc, out.0);
    out.1 = vqtbl4q_u8(tbl_enc, out.1);
    out.2 = vqtbl4q_u8(tbl_enc, out.2);
    out.3 = vqtbl4q_u8(tbl_enc, out.3);

    // Interleave and store output:
    unsafe { vst4q_u8(dst, out) };
}

#[target_feature(enable = "neon")]
pub unsafe fn encode_simd_prefix(
    mut src: *const u8,
    len: usize,
    mut dst: *mut u8,
    alphabet: *const u8,
) -> usize {
    if len < 48 {
        return 0;
    }

    let mut rounds = len / BYTES_CONSUMED_PER_ROUND;
    let consumed = rounds * BYTES_CONSUMED_PER_ROUND;

    // Build a 64-byte encoding table from the caller-supplied alphabet,
    // laid out contiguously as required by vqtbl4q_u8:
    let tbl_enc = unsafe {
        uint8x16x4_t(
            vld1q_u8(alphabet),
            vld1q_u8(alphabet.add(16)),
            vld1q_u8(alphabet.add(32)),
            vld1q_u8(alphabet.add(48)),
        )
    };

    while rounds != 0 {
        unsafe { encode_block(src, dst, tbl_enc) };
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
