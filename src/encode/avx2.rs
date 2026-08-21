#![expect(clippy::wildcard_imports)]

#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

use pyo3::{prelude::*, types::PyBytes};

use super::python;

const INPUT_BYTES: usize = 24;
const OUTPUT_BYTES: usize = 32;

#[inline(always)]
unsafe fn reshuffle(input: __m256i) -> __m256i {
    let input = unsafe {
        _mm256_shuffle_epi8(
            input,
            _mm256_setr_epi8(
                5, 4, 6, 5, 8, 7, 9, 8, 11, 10, 12, 11, 14, 13, 15, 14, 1, 0, 2, 1, 4, 3, 5, 4, 7,
                6, 8, 7, 10, 9, 11, 10,
            ),
        )
    };
    let hi = unsafe {
        _mm256_mulhi_epu16(
            _mm256_and_si256(input, _mm256_set1_epi32(0x0fc0_fc00)),
            _mm256_set1_epi32(0x0400_0040),
        )
    };
    let lo = unsafe {
        _mm256_mullo_epi16(
            _mm256_and_si256(input, _mm256_set1_epi32(0x003f_03f0)),
            _mm256_set1_epi32(0x0100_0010),
        )
    };
    unsafe { _mm256_or_si256(hi, lo) }
}

#[inline(always)]
unsafe fn translate(input: __m256i) -> __m256i {
    let offsets = unsafe {
        _mm256_setr_epi8(
            65, 71, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -19, -16, 0, 0, 65, 71, -4, -4, -4, -4,
            -4, -4, -4, -4, -4, -4, -19, -16, 0, 0,
        )
    };
    let indices = unsafe {
        _mm256_sub_epi8(
            _mm256_subs_epu8(input, _mm256_set1_epi8(51)),
            _mm256_cmpgt_epi8(input, _mm256_set1_epi8(25)),
        )
    };
    unsafe { _mm256_add_epi8(input, _mm256_shuffle_epi8(offsets, indices)) }
}

#[inline(always)]
unsafe fn translate_with_altchars(input: __m256i, plus: __m256i, slash: __m256i) -> __m256i {
    let encoded = unsafe { translate(input) };
    let is_62 = unsafe { _mm256_cmpeq_epi8(input, _mm256_set1_epi8(62)) };
    let is_63 = unsafe { _mm256_cmpeq_epi8(input, _mm256_set1_epi8(63)) };
    let others = unsafe { _mm256_andnot_si256(_mm256_or_si256(is_62, is_63), encoded) };
    unsafe {
        _mm256_or_si256(
            others,
            _mm256_or_si256(
                _mm256_and_si256(is_62, plus),
                _mm256_and_si256(is_63, slash),
            ),
        )
    }
}

#[inline(always)]
unsafe fn encode_block(input: *const u8, output: *mut u8) {
    let source = unsafe {
        _mm256_permutevar8x32_epi32(
            _mm256_loadu_si256(input.cast()),
            _mm256_setr_epi32(0, 0, 1, 2, 3, 4, 5, 6),
        )
    };
    unsafe { _mm256_storeu_si256(output.cast(), translate(reshuffle(source))) };
}

#[inline(always)]
unsafe fn encode_block_with_altchars(
    input: *const u8,
    output: *mut u8,
    plus: __m256i,
    slash: __m256i,
) {
    let source = unsafe {
        _mm256_permutevar8x32_epi32(
            _mm256_loadu_si256(input.cast()),
            _mm256_setr_epi32(0, 0, 1, 2, 3, 4, 5, 6),
        )
    };
    unsafe {
        _mm256_storeu_si256(
            output.cast(),
            translate_with_altchars(reshuffle(source), plus, slash),
        );
    };
}

#[target_feature(enable = "avx2")]
pub unsafe fn encode_simd_prefix(
    mut input: *const u8,
    len: usize,
    mut output: *mut u8,
    alphabet: *const u8,
) -> usize {
    // One 24-byte block plus eight bytes retained for its 32-byte load.
    if len < 32 {
        return 0;
    }

    let mut blocks = (len - 8) / INPUT_BYTES;
    let consumed = blocks * INPUT_BYTES;

    let plus = unsafe { alphabet.add(62).read() };
    let slash = unsafe { alphabet.add(63).read() };
    if plus == b'+' && slash == b'/' {
        while blocks != 0 {
            unsafe { encode_block(input, output) };
            input = unsafe { input.add(INPUT_BYTES) };
            output = unsafe { output.add(OUTPUT_BYTES) };
            blocks -= 1;
        }
    } else {
        let plus = _mm256_set1_epi8(plus.cast_signed());
        let slash = _mm256_set1_epi8(slash.cast_signed());
        while blocks != 0 {
            unsafe { encode_block_with_altchars(input, output, plus, slash) };
            input = unsafe { input.add(INPUT_BYTES) };
            output = unsafe { output.add(OUTPUT_BYTES) };
            blocks -= 1;
        }
    }

    consumed
}

/// # Safety
///
/// The running CPU must support AVX2.
pub unsafe fn encode<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    altchars: Option<&Bound<'py, PyAny>>,
    padded: bool,
    wrapcol: isize,
) -> PyResult<Bound<'py, PyBytes>> {
    python::encode(py, s, altchars, padded, wrapcol, encode_simd_prefix)
}
