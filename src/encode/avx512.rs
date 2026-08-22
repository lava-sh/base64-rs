// Based on:
//
// * https://github.com/simdutf/simdutf
//   Last commit full SHA: d3bac9a2307949f3212746e8f10cad43fe7362c9
//                         ^^^^^^^^
//   First 8:              d3bac9a2
//
// * https://github.com/aklomp/base64
//   Last commit full SHA: bf058e571ac5002b75b03fed38e33ed4e8d45eff
//                         ^^^^^^^^
//   First 8:              bf058e57
#![allow(clippy::wildcard_imports)]

#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

use pyo3::{prelude::*, types::PyBytes};

use super::python;

const BYTES_CONSUMED_PER_ROUND: usize = 48;
const BYTES_PRODUCED_PER_ROUND: usize = 64;

// https://github.com/simdutf/simdutf/blob/d3bac9a2/src/icelake/icelake_base64.inl.cpp#L96-L225
// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/avx512/enc_reshuffle_translate.c
#[target_feature(enable = "avx512f,avx512bw,avx512vbmi")]
pub unsafe fn encode_simd_prefix(
    mut src: *const u8,
    len: usize,
    mut dst: *mut u8,
    alphabet: *const u8,
) -> usize {
    // Translate immediately after reshuffled.
    let lookup = unsafe { _mm512_loadu_si512(alphabet.cast()) };

    // 32-bit input
    // [ 0  0  0  0  0  0  0  0|c1 c0 d5 d4 d3 d2 d1 d0|
    //  b3 b2 b1 b0 c5 c4 c3 c2|a5 a4 a3 a2 a1 a0 b5 b4]
    // output order  [1, 2, 0, 1]
    // [b3 b2 b1 b0 c5 c4 c3 c2|c1 c0 d5 d4 d3 d2 d1 d0|
    //  a5 a4 a3 a2 a1 a0 b5 b4|b3 b2 b1 b0 c3 c2 c1 c0]
    let shuffle_input = _mm512_setr_epi32(
        0x0102_0001_u32.cast_signed(),
        0x0405_0304_u32.cast_signed(),
        0x0708_0607_u32.cast_signed(),
        0x0a0b_090a_u32.cast_signed(),
        0x0d0e_0c0d_u32.cast_signed(),
        0x1011_0f10_u32.cast_signed(),
        0x1314_1213_u32.cast_signed(),
        0x1617_1516_u32.cast_signed(),
        0x191a_1819_u32.cast_signed(),
        0x1c1d_1b1c_u32.cast_signed(),
        0x1f20_1e1f_u32.cast_signed(),
        0x2223_2122_u32.cast_signed(),
        0x2526_2425_u32.cast_signed(),
        0x2829_2728_u32.cast_signed(),
        0x2b2c_2a2b_u32.cast_signed(),
        0x2e2f_2d2e_u32.cast_signed(),
    );

    // After multishift a single 32-bit lane has following layout
    // [c1 c0 d5 d4 d3 d2 d1 d0|b1 b0 c5 c4 c3 c2 c1 c0|
    //  a1 a0 b5 b4 b3 b2 b1 b0|d1 d0 a5 a4 a3 a2 a1 a0]
    // (a = [10:17], b = [4:11], c = [22:27], d = [16:21])

    // 48, 54, 36, 42, 16, 22, 4, 10
    let multi_shifts = _mm512_set1_epi64(0x3036_242a_1016_040a_u64.cast_signed());

    // (1 << 48) - 1
    let input_mask = (1_u64 << BYTES_CONSUMED_PER_ROUND) - 1;

    let rounds = len / BYTES_CONSUMED_PER_ROUND;
    for _ in 0..rounds {
        // Load input.
        let v = unsafe { _mm512_maskz_loadu_epi8(input_mask, src.cast()) };

        // Reorder bytes.
        let in_ = _mm512_permutexvar_epi8(shuffle_input, v);

        // Divide bits of three input bytes over four output bytes.
        let indices = _mm512_multishift_epi64_epi8(multi_shifts, in_);

        // Translation 6-bit values to ASCII.
        let result = _mm512_permutexvar_epi8(indices, lookup);

        // Store.
        unsafe { _mm512_storeu_si512(dst.cast(), result) };
        src = unsafe { src.add(BYTES_CONSUMED_PER_ROUND) };
        dst = unsafe { dst.add(BYTES_PRODUCED_PER_ROUND) };
    }

    rounds * BYTES_CONSUMED_PER_ROUND
}

/// # Safety
///
/// The running CPU must support AVX512F, AVX512BW and AVX512VBMI.
pub unsafe fn encode<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    altchars: Option<&Bound<'py, PyAny>>,
    padded: bool,
    wrapcol: isize,
) -> PyResult<Bound<'py, PyBytes>> {
    python::encode(py, s, altchars, padded, wrapcol, encode_simd_prefix)
}
