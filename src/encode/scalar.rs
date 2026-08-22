// Based on:
//
// * https://github.com/aklomp/base64
//   Last commit full SHA: bf058e571ac5002b75b03fed38e33ed4e8d45eff
//                         ^^^^^^^^
//   First 8:              bf058e57
use pyo3::{prelude::*, types::PyBytes};

use super::{
    python,
    tables::{STANDARD_PAIRS, URLSAFE_PAIRS},
};

#[must_use]
pub const fn encode_pairs(alphabet: &[u8; 64]) -> [u16; 4096] {
    let mut buf = [0; 4096];
    let mut index = 0;
    while index < buf.len() {
        buf[index] = u16::from_ne_bytes([alphabet[index >> 6], alphabet[index & 0x3f]]);
        index += 1;
    }
    buf
}

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/generic/64/enc_loop.c#L2-L28
#[inline(always)]
unsafe fn enc_loop_generic_64_inner(s: &mut *const u8, o: &mut *mut u8, table: *const u16) {
    // Load input:
    let mut src = unsafe { s.cast::<u64>().read_unaligned() };

    // Reorder to 64-bit big-endian, if not already in that format. The
    // workset must be in big-endian, otherwise the shifted bits do not
    // carry over properly among adjacent bytes:
    src = src.swap_bytes();

    // Four indices for the 12-bit lookup table:
    let index0 = ((src >> 52) & 0x0fff) as usize;
    let index1 = ((src >> 40) & 0x0fff) as usize;
    let index2 = ((src >> 28) & 0x0fff) as usize;
    let index3 = ((src >> 16) & 0x0fff) as usize;

    // Table lookup and store:
    unsafe {
        o.cast::<u16>().write_unaligned(table.add(index0).read());
        o.add(2)
            .cast::<u16>()
            .write_unaligned(table.add(index1).read());
        o.add(4)
            .cast::<u16>()
            .write_unaligned(table.add(index2).read());
        o.add(6)
            .cast::<u16>()
            .write_unaligned(table.add(index3).read());

        *s = s.add(6);
        *o = o.add(8);
    }
}

// https://github.com/aklomp/base64/blob/bf058e57/lib/arch/generic/64/enc_loop.c#L30-L77
#[inline(always)]
pub unsafe fn encode_simd_prefix(
    mut s: *const u8,
    len: usize,
    mut o: *mut u8,
    alphabet: *const u8,
) -> usize {
    let suffix = unsafe { [alphabet.add(62).read(), alphabet.add(63).read()] };

    if !matches!(suffix, [b'+', b'/'] | [b'-', b'_']) && len < 32 * 1024 {
        return 0;
    }

    let runtime_table = match suffix {
        [b'+', b'/'] | [b'-', b'_'] => None,
        _ => Some(encode_pairs(unsafe { &*alphabet.cast::<[u8; 64]>() })),
    };

    let table = runtime_table.as_ref().map_or_else(
        || match suffix {
            [b'+', b'/'] => STANDARD_PAIRS.as_ptr(),
            [b'-', b'_'] => URLSAFE_PAIRS.as_ptr(),
            _ => unreachable!(),
        },
        |table| table.as_ptr(),
    );

    if len < 8 {
        return 0;
    }

    // Process blocks of 6 bytes at a time. Because blocks are loaded 8
    // bytes at a time, ensure that there will be at least 2 remaining
    // bytes after the last round, so that the final read will not pass
    // beyond the bounds of the input buffer:
    let mut rounds = (len - 2) / 6;
    let consumed = rounds * 6;

    while rounds > 0 {
        if rounds >= 8 {
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            rounds -= 8;
            continue;
        }
        if rounds >= 4 {
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            rounds -= 4;
            continue;
        }
        if rounds >= 2 {
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
            rounds -= 2;
            continue;
        }
        unsafe { enc_loop_generic_64_inner(&mut s, &mut o, table) };
        break;
    }

    consumed
}

#[inline(always)]
pub unsafe fn encode_into(
    input: *const u8,
    input_len: usize,
    output: *mut u8,
    alphabet: *const u8,
    padded: u8,
    wrapcol: usize,
    pair_table: *const u16,
) -> usize {
    let raw_len = input_len.div_ceil(3) * 4;
    let unpadded_len = match input_len % 3 {
        1 => raw_len - 2,
        2 => raw_len - 1,
        _ => raw_len,
    };
    let output_len = if padded != 0 && !input_len.is_multiple_of(3) {
        raw_len
    } else {
        unpadded_len
    };
    let wrapcol = if wrapcol == 0 {
        usize::MAX
    } else {
        (wrapcol / 4).max(1) * 4
    };

    let mut cursor = input;
    let mut written = 0;
    let mut column = 0;
    let end = unsafe { input.add(input_len - input_len % 3) };

    while cursor < end {
        if column == wrapcol {
            unsafe { output.add(written).write(b'\n') };
            written += 1;
            column = 0;
        }

        let out = unsafe { output.add(written) };
        let first = unsafe { cursor.read() };
        let second = unsafe { cursor.add(1).read() };
        let third = unsafe { cursor.add(2).read() };
        let bits = u32::from(first) << 16 | u32::from(second) << 8 | u32::from(third);

        if pair_table.is_null() {
            let first = unsafe { alphabet.add((bits >> 18) as usize & 0x3f).read() };
            let second = unsafe { alphabet.add((bits >> 12) as usize & 0x3f).read() };
            let third = unsafe { alphabet.add((bits >> 6) as usize & 0x3f).read() };
            let fourth = unsafe { alphabet.add(bits as usize & 0x3f).read() };
            unsafe { out.write(first) };
            unsafe { out.add(1).write(second) };
            unsafe { out.add(2).write(third) };
            unsafe { out.add(3).write(fourth) };
        } else {
            let first = unsafe { pair_table.add((bits >> 12) as usize).read() };
            let second = unsafe { pair_table.add((bits & 0x0fff) as usize).read() };
            unsafe { out.cast::<u16>().write_unaligned(first) };
            unsafe { out.add(2).cast::<u16>().write_unaligned(second) };
        }

        cursor = unsafe { cursor.add(3) };
        written += 4;
        column += 4;
    }

    let tail_len = match input_len % 3 {
        0 => 0,
        remaining => {
            if column == wrapcol {
                unsafe { output.add(written).write(b'\n') };
                written += 1;
            }

            let out = unsafe { output.add(written) };
            let first = unsafe { cursor.read() };
            let first_encoded = unsafe { alphabet.add((first >> 2) as usize).read() };
            unsafe { out.write(first_encoded) };

            if remaining == 1 {
                let second_encoded = unsafe { alphabet.add(((first & 3) << 4) as usize).read() };
                unsafe { out.add(1).write(second_encoded) };
                if padded != 0 {
                    unsafe { out.add(2).write(b'=') };
                    unsafe { out.add(3).write(b'=') };
                    4
                } else {
                    2
                }
            } else {
                let second = unsafe { cursor.add(1).read() };
                let second_encoded = unsafe {
                    alphabet
                        .add((((first & 3) << 4) | (second >> 4)) as usize)
                        .read()
                };
                let third_encoded = unsafe { alphabet.add(((second & 15) << 2) as usize).read() };
                unsafe { out.add(1).write(second_encoded) };
                unsafe { out.add(2).write(third_encoded) };
                if padded != 0 {
                    unsafe { out.add(3).write(b'=') };
                    4
                } else {
                    3
                }
            }
        }
    };
    written += tail_len;

    debug_assert_eq!(
        written,
        if wrapcol == usize::MAX {
            output_len
        } else {
            output_len + (output_len.saturating_sub(1) / wrapcol)
        }
    );

    written
}

#[inline]
pub fn encode<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    altchars: Option<&Bound<'py, PyAny>>,
    padded: bool,
    wrapcol: isize,
) -> PyResult<Bound<'py, PyBytes>> {
    python::encode(py, s, altchars, padded, wrapcol, encode_simd_prefix)
}
