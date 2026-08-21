#![allow(clippy::wildcard_imports)]

#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

use pyo3::{prelude::*, types::PyBytes};

use super::python;

const BYTES_CONSUMED_PER_ROUND: usize = 24;
const BYTES_PRODUCED_PER_ROUND: usize = 32;

#[inline]
#[target_feature(enable = "avx2")]
unsafe fn mm256_mulhi_epu16(mut a: __m256i, b: __m256i) -> __m256i {
    // LLVM (via rustc's codegen) lowers `_mm256_mulhi_epu16` into a
    // `punpcklwd`/`punpckhwd`/`psrld`/`packssdw` sequence instead of emitting
    // a single `vpmulhuw`
    unsafe {
        core::arch::asm!(
        "vpmulhuw {a}, {a}, {b}",
        a = inout(ymm_reg) a,
        b = in(ymm_reg) b,
        options(pure, nomem, nostack),
        );
    }
    a
}

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/avx2/enc_reshuffle.c
#[target_feature(enable = "avx2")]
fn reshuffle(src: __m256i) -> __m256i {
    // Translation of the SSSE3 reshuffling algorithm to AVX2. This one
    // works with shifted (4 bytes) input in order to be able to work
    // efficiently in the two 128-bit lanes.

    // Input, bytes MSB to LSB:
    // 0 0 0 0 x w v u t s r q p o n m
    // l k j i h g f e d c b a 0 0 0 0
    #[rustfmt::skip]
    let src = _mm256_shuffle_epi8(src, _mm256_set_epi8(
        10, 11,  9, 10,
         7,  8,  6,  7,
         4,  5,  3,  4,
         1,  2,  0,  1,

        14, 15, 13, 14,
        11, 12, 10, 11,
         8,  9,  7,  8,
         5,  6,  4,  5));
    // Input, bytes MSB to LSB:
    // w x v w
    // t u s t
    // q r p q
    // n o m n
    // k l j k
    // h i g h
    // e f d e
    // b c a b

    let t0 = _mm256_and_si256(src, _mm256_set1_epi32(0x0FC0FC00));
    // bits, upper case are most significant bits, lower case are least
    // significant bits.
    // 0000wwww XX000000 VVVVVV00 00000000
    // 0000tttt UU000000 SSSSSS00 00000000
    // 0000qqqq RR000000 PPPPPP00 00000000
    // 0000nnnn OO000000 MMMMMM00 00000000
    // 0000kkkk LL000000 JJJJJJ00 00000000
    // 0000hhhh II000000 GGGGGG00 00000000
    // 0000eeee FF000000 DDDDDD00 00000000
    // 0000bbbb CC000000 AAAAAA00 00000000

    let t1 = unsafe { mm256_mulhi_epu16(t0, _mm256_set1_epi32(0x04000040)) };
    // 00000000 00wwwwXX 00000000 00VVVVVV
    // 00000000 00ttttUU 00000000 00SSSSSS
    // 00000000 00qqqqRR 00000000 00PPPPPP
    // 00000000 00nnnnOO 00000000 00MMMMMM
    // 00000000 00kkkkLL 00000000 00JJJJJJ
    // 00000000 00hhhhII 00000000 00GGGGGG
    // 00000000 00eeeeFF 00000000 00DDDDDD
    // 00000000 00bbbbCC 00000000 00AAAAAA

    let t2 = _mm256_and_si256(src, _mm256_set1_epi32(0x003F03F0));
    // 00000000 00xxxxxx 000000vv WWWW0000
    // 00000000 00uuuuuu 000000ss TTTT0000
    // 00000000 00rrrrrr 000000pp QQQQ0000
    // 00000000 00oooooo 000000mm NNNN0000
    // 00000000 00llllll 000000jj KKKK0000
    // 00000000 00iiiiii 000000gg HHHH0000
    // 00000000 00ffffff 000000dd EEEE0000
    // 00000000 00cccccc 000000aa BBBB0000

    let t3 = _mm256_mullo_epi16(t2, _mm256_set1_epi32(0x01000010));
    // 00xxxxxx 00000000 00vvWWWW 00000000
    // 00uuuuuu 00000000 00ssTTTT 00000000
    // 00rrrrrr 00000000 00ppQQQQ 00000000
    // 00oooooo 00000000 00mmNNNN 00000000
    // 00llllll 00000000 00jjKKKK 00000000
    // 00iiiiii 00000000 00ggHHHH 00000000
    // 00ffffff 00000000 00ddEEEE 00000000
    // 00cccccc 00000000 00aaBBBB 00000000

    _mm256_or_si256(t1, t3)
    // 00xxxxxx 00wwwwXX 00vvWWWW 00VVVVVV
    // 00uuuuuu 00ttttUU 00ssTTTT 00SSSSSS
    // 00rrrrrr 00qqqqRR 00ppQQQQ 00PPPPPP
    // 00oooooo 00nnnnOO 00mmNNNN 00MMMMMM
    // 00llllll 00kkkkLL 00jjKKKK 00JJJJJJ
    // 00iiiiii 00hhhhII 00ggHHHH 00GGGGGG
    // 00ffffff 00eeeeFF 00ddEEEE 00DDDDDD
    // 00cccccc 00bbbbCC 00aaBBBB 00AAAAAA
}

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/avx2/enc_translate.c
#[target_feature(enable = "avx2")]
fn translate(src: __m256i) -> __m256i {
    // A lookup table containing the absolute offsets for all ranges:
    #[rustfmt::skip]
    let lut = _mm256_setr_epi8(
        65, 71, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -19, -16, 0, 0,
        65, 71, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -19, -16, 0, 0,
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
    let indices = _mm256_subs_epu8(src, _mm256_set1_epi8(51));

    // mask is 0xFF (-1) for range #[1..4] and 0x00 for range #0:
    let mask = _mm256_cmpgt_epi8(src, _mm256_set1_epi8(25));

    // Subtract -1, so add 1 to indices for range #[1..4]. All indices are
    // now correct:
    let indices = _mm256_sub_epi8(indices, mask);

    // Add offsets to input values:
    _mm256_add_epi8(src, _mm256_shuffle_epi8(lut, indices))
}

#[target_feature(enable = "avx2")]
fn translate_with_altchars(src: __m256i, plus: __m256i, slash: __m256i) -> __m256i {
    let encoded = translate(src);

    // Values 62 and 63 map to '+' and '/' in the standard alphabet, but
    // for URL-safe / custom alphabets these two characters are supplied
    // by the caller instead. Build masks identifying which lanes held
    // 62 ('+') or 63 ('/') before translation:
    let is_62 = _mm256_cmpeq_epi8(src, _mm256_set1_epi8(62));
    let is_63 = _mm256_cmpeq_epi8(src, _mm256_set1_epi8(63));

    // Keep the standard-alphabet result for every lane except 62/63:
    let others = _mm256_andnot_si256(_mm256_or_si256(is_62, is_63), encoded);

    // Substitute the caller-supplied alt characters into the 62/63 lanes:
    _mm256_or_si256(
        others,
        _mm256_or_si256(
            _mm256_and_si256(is_62, plus),
            _mm256_and_si256(is_63, slash),
        ),
    )
}

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/avx2/enc_loop.c
#[inline]
#[target_feature(enable = "avx2")]
unsafe fn encode_block(src: *const u8, dst: *mut u8) {
    // First load is done at s - 0 to not get a segfault:
    let src = unsafe { _mm256_loadu_si256(src.cast()) };

    // Shift by 4 bytes, as required by enc_reshuffle:
    let src = _mm256_permutevar8x32_epi32(src, _mm256_setr_epi32(0, 0, 1, 2, 3, 4, 5, 6));

    // Reshuffle, translate, store:
    let src = reshuffle(src);
    let encoded = translate(src);
    unsafe { _mm256_storeu_si256(dst.cast(), encoded) };
}

#[inline]
#[target_feature(enable = "avx2")]
unsafe fn encode_block_with_altchars(src: *const u8, dst: *mut u8, plus: __m256i, slash: __m256i) {
    // First load is done at s - 0 to not get a segfault:
    let src = unsafe { _mm256_loadu_si256(src.cast()) };

    // Shift by 4 bytes, as required by enc_reshuffle:
    let src = _mm256_permutevar8x32_epi32(src, _mm256_setr_epi32(0, 0, 1, 2, 3, 4, 5, 6));

    // Reshuffle, translate (with alt chars), store:
    let src = reshuffle(src);
    let encoded = translate_with_altchars(src, plus, slash);
    unsafe { _mm256_storeu_si256(dst.cast(), encoded) };
}

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/avx2/enc_loop.c
#[target_feature(enable = "avx2")]
pub unsafe fn encode_simd_prefix(
    mut src: *const u8,
    len: usize,
    mut dst: *mut u8,
    alphabet: *const u8,
) -> usize {
    if len < 32 {
        return 0;
    }

    // Process blocks of 24 bytes at a time. Because blocks are loaded 32
    // bytes at a time an offset of -4, ensure that there will be at least
    // 4 remaining bytes after the last round, so that the final read will
    // not pass beyond the bounds of the input buffer:
    let mut rounds = (len - 4) / BYTES_CONSUMED_PER_ROUND;
    let consumed = rounds * BYTES_CONSUMED_PER_ROUND;

    let plus = unsafe { alphabet.add(62).read() };
    let slash = unsafe { alphabet.add(63).read() };

    if plus == b'+' && slash == b'/' {
        while rounds != 0 {
            unsafe { encode_block(src, dst) };
            src = unsafe { src.add(BYTES_CONSUMED_PER_ROUND) };
            dst = unsafe { dst.add(BYTES_PRODUCED_PER_ROUND) };
            rounds -= 1;
        }
    } else {
        let plus = _mm256_set1_epi8(plus.cast_signed());
        let slash = _mm256_set1_epi8(slash.cast_signed());
        while rounds != 0 {
            unsafe { encode_block_with_altchars(src, dst, plus, slash) };
            src = unsafe { src.add(BYTES_CONSUMED_PER_ROUND) };
            dst = unsafe { dst.add(BYTES_PRODUCED_PER_ROUND) };
            rounds -= 1;
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
