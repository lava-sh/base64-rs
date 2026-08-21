#![allow(clippy::wildcard_imports)]

#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

use pyo3::{prelude::*, types::PyBytes};

use super::python;

/// Loads 512 bits (64 bytes) of integer data from memory into a `__m512i`.
///
/// # Safety
///
/// `src` must be readable for 64 bytes.
#[inline(always)]
unsafe fn load_unaligned(src: *const u8) -> __m512i {
    // SAFETY: `_mm512_loadu_si512` accepts unaligned addresses.
    unsafe { _mm512_loadu_si512(src.cast::<__m512i>()) }
}

/// Stores one unaligned 64-byte vector to a byte pointer.
///
/// # Safety
///
/// `dst` must be writable for 64 bytes.
#[inline(always)]
unsafe fn store_unaligned(dst: *mut u8, value: __m512i) {
    // SAFETY: `_mm512_storeu_si512` accepts unaligned addresses.
    unsafe { _mm512_storeu_si512(dst.cast::<__m512i>(), value) };
}

const BYTES_CONSUMED_PER_VECTOR: usize = 48;
const BYTES_WRITTEN_PER_VECTOR: usize = 64;

/// Broadcasts one 8-byte shift pattern across all eight 64-bit lanes of a 512-bit vector,
/// so `_mm512_multishift_epi64_epi8` applies the same per-lane shift schedule to every group.
const fn repeat_u64_lane_controls(group: [u8; 8]) -> [u8; 64] {
    let mut buf = [0; 64];
    let mut index = 0;
    while index < buf.len() {
        buf[index] = group[index % group.len()];
        index += 1;
    }
    buf
}

#[rustfmt::skip]
const fn make_input_shuffle_indices() -> [u8; 64] {
    let mut buf = [0; 64];
    let mut group = 0;
    while group < 16 {
        let input = group * 3;
        let output = group * 4;
        buf[output] = (input + 1) as u8;     // B
        buf[output + 1] = input as u8;       // A
        buf[output + 2] = (input + 2) as u8; // C
        buf[output + 3] = (input + 1) as u8; // B (duplicated)
        group += 1;
    }
    buf
}

const INPUT_SHUFFLE_INDICES: [u8; 64] = make_input_shuffle_indices();

// Per-byte bit offsets fed to `_mm512_multishift_epi64_epi8`. Within each
// duplicated [B A C B] 32-bit group, these extract the four 6-bit fields:
//   byte0 (A>>2)      -> shift 10
//   byte1 (A<<4|B>>4) -> shift 4
//   byte2 (B<<2|C>>6) -> shift 22
//   byte3 (C)         -> shift 16
// repeated every 4 bytes across the 64-bit lane, hence the 8-value pattern.
const EXTRACTION_SHIFTS: [u8; 64] = repeat_u64_lane_controls([10, 4, 22, 16, 42, 36, 54, 48]);

#[target_feature(enable = "avx512f,avx512vbmi")]
pub unsafe fn encode_simd_prefix(
    mut input: *const u8,
    len: usize,
    mut output: *mut u8,
    alphabet: *const u8,
) -> usize {
    // One 48-byte SIMD block plus 24 bytes left for the scalar tail,
    // so the 64-byte vector load remains in bounds.
    if len < 72 {
        return 0;
    }

    let mut blocks = (len - 24) / BYTES_CONSUMED_PER_VECTOR;
    let consumed = blocks * BYTES_CONSUMED_PER_VECTOR;

    // SAFETY: `INPUT_SHUFFLE_INDICES` are 64 readable bytes.
    let permutation = unsafe { load_unaligned(INPUT_SHUFFLE_INDICES.as_ptr()) };
    // SAFETY: `EXTRACTION_SHIFTS` are 64 readable bytes.
    let shifts = unsafe { load_unaligned(EXTRACTION_SHIFTS.as_ptr()) };
    // SAFETY: AVX-512's `vpermb` masks indexes to its low six bits.
    let alphabet = unsafe { load_unaligned(alphabet) };

    macro_rules! block {
        () => {{
            // load: 64 raw bytes cover 16 input groups (48 consumed + 16 slack
            // that overlaps the *next* block's read window, never written out).
            let src = unsafe { load_unaligned(input) };
            // reorder bytes: `vpermb` duplicates each [A B C] group into
            // [B A C B], per `INPUT_SHUFFLE_INDICES`.
            let duplicated = _mm512_permutexvar_epi8(permutation, src);
            let values = _mm512_multishift_epi64_epi8(shifts, duplicated);
            // translate: 6-bit index -> ASCII character via the 64-byte alphabet table.
            let encoded = _mm512_permutexvar_epi8(values, alphabet);
            // store 64 encoded ASCII bytes, advance by one block.
            unsafe { store_unaligned(output, encoded) };
            input = unsafe { input.add(BYTES_CONSUMED_PER_VECTOR) };
            output = unsafe { output.add(BYTES_WRITTEN_PER_VECTOR) };
        }};
    }

    // Eight independent blocks let the CPU overlap permutes, multishifts and stores.
    while blocks >= 8 {
        block!();
        block!();
        block!();
        block!();
        block!();
        block!();
        block!();
        block!();
        blocks -= 8;
    }
    while blocks != 0 {
        block!();
        blocks -= 1;
    }

    consumed
}

/// # Safety
///
/// The running CPU must support `avx512f` and `avx512vbmi`.
pub unsafe fn encode<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    altchars: Option<&Bound<'py, PyAny>>,
    padded: bool,
    wrapcol: isize,
) -> PyResult<Bound<'py, PyBytes>> {
    python::encode(py, s, altchars, padded, wrapcol, encode_simd_prefix)
}
