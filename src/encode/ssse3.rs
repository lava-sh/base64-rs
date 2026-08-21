#![allow(clippy::wildcard_imports)]

#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

use pyo3::{prelude::*, types::PyBytes};

use super::python;

const BYTES_CONSUMED_PER_ROUND: usize = 12;
const BYTES_WRITTEN_PER_ROUND: usize = 16;

#[inline(always)]
unsafe fn mm_mulhi_epu16(mut a: __m128i, b: __m128i) -> __m128i {
    // LLVM (via rustc's codegen) lowers `_mm_mulhi_epu16` into a
    // `punpcklwd`/`punpckhwd`/`psrld`/`packssdw` sequence instead of emitting
    // a single `pmulhuw`
    unsafe {
        core::arch::asm!(
        "pmulhuw {a}, {b}",
        a = inout(xmm_reg) a,
        b = in(xmm_reg) b,
        options(pure, nomem, nostack),
        );
    };
    a
}

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/ssse3/enc_reshuffle.c
#[target_feature(enable = "ssse3")]
fn reshuffle(src: __m128i) -> __m128i {
    // Input, bytes MSB to LSB:
    // 0 0 0 0 l k j i h g f e d c b a
    #[rustfmt::skip]
    let src = _mm_shuffle_epi8(
        src,
        _mm_setr_epi8(
            1, 0, 2, 1,
            4, 3, 5, 4,
            7, 6, 8, 7,
            10, 9, 11, 10
        ),
    );

    // in, bytes MSB to LSB:
    // k l j k
    // h i g h
    // e f d e
    // b c a b

    let t0 = _mm_and_si128(src, _mm_set1_epi32(0x0fc0_fc00));
    // bits, upper case are most significant bits, lower case are least significant bits
    // 0000kkkk LL000000 JJJJJJ00 00000000
    // 0000hhhh II000000 GGGGGG00 00000000
    // 0000eeee FF000000 DDDDDD00 00000000
    // 0000bbbb CC000000 AAAAAA00 00000000

    let t1 = unsafe { mm_mulhi_epu16(t0, _mm_set1_epi32(0x0400_0040)) };
    // 00000000 00kkkkLL 00000000 00JJJJJJ
    // 00000000 00hhhhII 00000000 00GGGGGG
    // 00000000 00eeeeFF 00000000 00DDDDDD
    // 00000000 00bbbbCC 00000000 00AAAAAA

    let t2 = _mm_and_si128(src, _mm_set1_epi32(0x003f_03f0));
    // 00000000 00llllll 000000jj KKKK0000
    // 00000000 00iiiiii 000000gg HHHH0000
    // 00000000 00ffffff 000000dd EEEE0000
    // 00000000 00cccccc 000000aa BBBB0000

    let t3 = _mm_mullo_epi16(t2, _mm_set1_epi32(0x0100_0010));
    // 00llllll 00000000 00jjKKKK 00000000
    // 00iiiiii 00000000 00ggHHHH 00000000
    // 00ffffff 00000000 00ddEEEE 00000000
    // 00cccccc 00000000 00aaBBBB 00000000

    _mm_or_si128(t1, t3)
    // 00llllll 00kkkkLL 00jjKKKK 00JJJJJJ
    // 00iiiiii 00hhhhII 00ggHHHH 00GGGGGG
    // 00ffffff 00eeeeFF 00ddEEEE 00DDDDDD
    // 00cccccc 00bbbbCC 00aaBBBB 00AAAAAA
}

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/ssse3/enc_translate.c
#[target_feature(enable = "ssse3")]
fn translate(src: __m128i) -> __m128i {
    // A lookup table containing the absolute offsets for all ranges:
    #[rustfmt::skip]
    let lut  = _mm_setr_epi8(
            65, 71, -4, -4,
            -4, -4, -4, -4,
            -4, -4, -4, -4,
            -19, -16, 0, 0,
    );

    // Translate values 0..63 to the Base64 alphabet. There are five sets:
    // #  From      To         Abs    Index  Characters
    // 0  [0..25]   [65..90]   +65        0  ABCDEFGHIJKLMNOPQRSTUVWXYZ
    // 1  [26..51]  [97..122]  +71        1  abcdefghijklmnopqrstuvwxyz
    // 2  [52..61]  [48..57]    -4  [2..11]  0123456789
    // 3  [62]      [43]       -19       12  +
    // 4  [63]      [47]       -16       13  /

    // Create LUT indices from the input. The index for range #0 is right,
    // others are 1 less than expected:
    let indices = _mm_subs_epu8(src, _mm_set1_epi8(51));

    // mask is 0xFF (-1) for range #[1..4] and 0x00 for range #0:
    let mask = _mm_cmpgt_epi8(src, _mm_set1_epi8(25));

    // Subtract -1, so add 1 to indices for range #[1..4]. All indices are
    // now correct:
    let indices = _mm_sub_epi8(indices, mask);

    // Add offsets to input values:
    _mm_add_epi8(src, _mm_shuffle_epi8(lut, indices))
}

#[inline]
#[target_feature(enable = "ssse3")]
fn translate_with_altchars(src: __m128i, plus: __m128i, slash: __m128i) -> __m128i {
    let encoded = translate(src);

    // Values 62 and 63 map to '+' and '/' in the standard alphabet, but
    // for URL-safe / custom alphabets these two characters are supplied
    // by the caller instead. Build masks identifying which lanes held
    // 62 ('+') or 63 ('/') before translation:
    let is_62 = _mm_cmpeq_epi8(src, _mm_set1_epi8(62));
    let is_63 = _mm_cmpeq_epi8(src, _mm_set1_epi8(63));

    // Keep the standard-translated bytes everywhere except at the 62/63
    // lanes, which will be overwritten below:
    let others = _mm_andnot_si128(_mm_or_si128(is_62, is_63), encoded);

    // Blend in the caller-supplied alt characters at the 62/63 lanes:
    _mm_or_si128(
        others,
        _mm_or_si128(_mm_and_si128(is_62, plus), _mm_and_si128(is_63, slash)),
    )
}

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/ssse3/enc_loop.c
#[inline]
#[target_feature(enable = "ssse3")]
unsafe fn encode_block(src: *const u8, dst: *mut u8) {
    // Load input:
    let src = unsafe { _mm_loadu_si128(src.cast()) };

    // Reshuffle:
    let src = reshuffle(src);

    // Translate reshuffled bytes to the Base64 alphabet:
    let encoded = translate(src);

    // Store:
    unsafe { _mm_storeu_si128(dst.cast(), encoded) };
}

#[inline]
#[target_feature(enable = "ssse3")]
unsafe fn encode_block_with_altchars(src: *const u8, dst: *mut u8, plus: __m128i, slash: __m128i) {
    // Load input:
    let src = unsafe { _mm_loadu_si128(src.cast()) };

    // Reshuffle:
    let src = reshuffle(src);

    // Translate reshuffled bytes to the Base64 alphabet, substituting
    // the caller-supplied alt characters for '+' and '/':
    let encoded = translate_with_altchars(src, plus, slash);

    // Store:
    unsafe { _mm_storeu_si128(dst.cast(), encoded) };
}

#[target_feature(enable = "ssse3")]
pub unsafe fn encode_simd_prefix(
    mut src: *const u8,
    len: usize,
    mut dst: *mut u8,
    alphabet: *const u8,
) -> usize {
    // A 12-byte group uses one 16-byte read. Leave four bytes for scalar
    // finalization so the final vector read remains in bounds.
    if len < 16 {
        return 0;
    }

    let mut blocks = (len - 4) / BYTES_CONSUMED_PER_ROUND;
    let consumed = blocks * BYTES_CONSUMED_PER_ROUND;
    let plus = unsafe { alphabet.add(62).read() };
    let slash = unsafe { alphabet.add(63).read() };

    if plus == b'+' && slash == b'/' {
        while blocks != 0 {
            unsafe { encode_block(src, dst) };
            src = unsafe { src.add(BYTES_CONSUMED_PER_ROUND) };
            dst = unsafe { dst.add(BYTES_WRITTEN_PER_ROUND) };
            blocks -= 1;
        }
    } else {
        let plus = _mm_set1_epi8(plus.cast_signed());
        let slash = _mm_set1_epi8(slash.cast_signed());
        while blocks != 0 {
            unsafe { encode_block_with_altchars(src, dst, plus, slash) };
            src = unsafe { src.add(BYTES_CONSUMED_PER_ROUND) };
            dst = unsafe { dst.add(BYTES_WRITTEN_PER_ROUND) };
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
