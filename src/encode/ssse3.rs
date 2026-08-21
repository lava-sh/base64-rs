#![expect(clippy::wildcard_imports)]

use core::arch::asm;
#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

use pyo3::{prelude::*, types::PyBytes};

use super::python;

const INPUT_BYTES: usize = 12;
const OUTPUT_BYTES: usize = 16;

#[inline(always)]
unsafe fn reshuffle(input: __m128i) -> __m128i {
    let input = unsafe {
        _mm_shuffle_epi8(
            input,
            _mm_setr_epi8(1, 0, 2, 1, 4, 3, 5, 4, 7, 6, 8, 7, 10, 9, 11, 10),
        )
    };
    let high = unsafe { _mm_and_si128(input, _mm_set1_epi32(0x0fc0_fc00)) };
    let high = unsafe { multiply_high_u16(high, _mm_set1_epi32(0x0400_0040)) };
    let low = unsafe { _mm_and_si128(input, _mm_set1_epi32(0x003f_03f0)) };
    let low = unsafe { _mm_mullo_epi16(low, _mm_set1_epi32(0x0100_0010)) };
    unsafe { _mm_or_si128(high, low) }
}

#[inline(always)]
unsafe fn multiply_high_u16(mut left: __m128i, right: __m128i) -> __m128i {
    unsafe {
        asm!(
            "pmulhuw {left}, {right}",
            left = inout(xmm_reg) left,
            right = in(xmm_reg) right,
            options(pure, nomem, nostack),
        );
    };
    left
}

#[inline(always)]
unsafe fn translate(input: __m128i) -> __m128i {
    let offsets = unsafe {
        _mm_setr_epi8(
            65, 71, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -19, -16, 0, 0,
        )
    };
    let indices = unsafe { _mm_subs_epu8(input, _mm_set1_epi8(51)) };
    let mask = unsafe { _mm_cmpgt_epi8(input, _mm_set1_epi8(25)) };
    let indices = unsafe { _mm_sub_epi8(indices, mask) };
    unsafe { _mm_add_epi8(input, _mm_shuffle_epi8(offsets, indices)) }
}

#[inline(always)]
unsafe fn translate_with_altchars(input: __m128i, plus: __m128i, slash: __m128i) -> __m128i {
    let encoded = unsafe { translate(input) };
    let is_62 = unsafe { _mm_cmpeq_epi8(input, _mm_set1_epi8(62)) };
    let is_63 = unsafe { _mm_cmpeq_epi8(input, _mm_set1_epi8(63)) };
    let others = unsafe { _mm_andnot_si128(_mm_or_si128(is_62, is_63), encoded) };
    unsafe {
        _mm_or_si128(
            others,
            _mm_or_si128(_mm_and_si128(is_62, plus), _mm_and_si128(is_63, slash)),
        )
    }
}

#[inline(always)]
unsafe fn encode_block(input: *const u8, output: *mut u8) {
    let input = unsafe { _mm_loadu_si128(input.cast()) };
    let encoded = unsafe { translate(reshuffle(input)) };
    unsafe { _mm_storeu_si128(output.cast(), encoded) };
}

#[inline(always)]
unsafe fn encode_block_with_altchars(
    input: *const u8,
    output: *mut u8,
    plus: __m128i,
    slash: __m128i,
) {
    let input = unsafe { _mm_loadu_si128(input.cast()) };
    let encoded = unsafe { translate_with_altchars(reshuffle(input), plus, slash) };
    unsafe { _mm_storeu_si128(output.cast(), encoded) };
}

#[target_feature(enable = "ssse3")]
pub unsafe fn encode_simd_prefix(
    mut input: *const u8,
    len: usize,
    mut output: *mut u8,
    alphabet: *const u8,
) -> usize {
    // A 12-byte group uses one 16-byte read. Leave four bytes for scalar
    // finalization so the final vector read remains in bounds.
    if len < 16 {
        return 0;
    }

    let mut blocks = (len - 4) / INPUT_BYTES;
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
        let plus = _mm_set1_epi8(plus.cast_signed());
        let slash = _mm_set1_epi8(slash.cast_signed());
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
/// The running CPU must support SSSE3.
pub unsafe fn encode<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    altchars: Option<&Bound<'py, PyAny>>,
    padded: bool,
    wrapcol: isize,
) -> PyResult<Bound<'py, PyBytes>> {
    python::encode(py, s, altchars, padded, wrapcol, encode_simd_prefix)
}
