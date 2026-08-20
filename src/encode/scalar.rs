use pyo3::{prelude::*, types::PyBytes};

use super::{
    alphabet::{ALPHABET, URLSAFE_ALPHABET},
    python,
};

const fn encode_pairs(alphabet: &[u8; 64]) -> [u16; 4096] {
    let mut buf = [0; 4096];
    let mut index = 0;
    while index < buf.len() {
        let pair = [alphabet[index >> 6], alphabet[index & 0x3f]];
        buf[index] = u16::from_ne_bytes(pair);
        index += 1;
    }
    buf
}

pub const STANDARD_PAIRS: [u16; 4096] = encode_pairs(&ALPHABET.0);
pub const URL_SAFE_PAIRS: [u16; 4096] = encode_pairs(&URLSAFE_ALPHABET.0);

#[inline(always)]
unsafe fn encode_triplet(input: *const u8, output: *mut u8, alphabet: *const u8) {
    unsafe {
        let bits = (u32::from(input.read())) << 16
            | (u32::from(input.add(1).read())) << 8
            | u32::from(input.add(2).read());

        output.write(alphabet.add((bits >> 18) as usize & 0x3f).read());
        output
            .add(1)
            .write(alphabet.add((bits >> 12) as usize & 0x3f).read());
        output
            .add(2)
            .write(alphabet.add((bits >> 6) as usize & 0x3f).read());
        output
            .add(3)
            .write(alphabet.add(bits as usize & 0x3f).read());
    }
}

#[inline(always)]
unsafe fn encode_triplet_pairs(input: *const u8, output: *mut u8, pairs: *const u16) {
    unsafe {
        let bits = (u32::from(input.read())) << 16
            | (u32::from(input.add(1).read())) << 8
            | u32::from(input.add(2).read());

        output
            .cast::<u16>()
            .write_unaligned(pairs.add((bits >> 12) as usize).read());
        output
            .add(2)
            .cast::<u16>()
            .write_unaligned(pairs.add((bits & 0x0fff) as usize).read());
    }
}

#[inline(always)]
unsafe fn begin_wrapped_quad(
    output: *mut u8,
    written: &mut usize,
    column: &mut usize,
    wrapcol: usize,
) -> *mut u8 {
    unsafe {
        if *column == wrapcol {
            output.add(*written).write(b'\n');
            *written += 1;
            *column = 0;
        }
        output.add(*written)
    }
}

#[inline(always)]
unsafe fn encode_tail(input: *const u8, output: *mut u8, alphabet: *const u8, padded: u8) -> usize {
    unsafe {
        let bits = (u32::from(input.read())) << 16;
        output.write(alphabet.add((bits >> 18) as usize & 0x3f).read());
        output
            .add(1)
            .write(alphabet.add((bits >> 12) as usize & 0x3f).read());
        if padded != 0 {
            output.add(2).write(b'=');
            output.add(3).write(b'=');
            4
        } else {
            2
        }
    }
}

#[inline(always)]
unsafe fn encode_tail2(
    input: *const u8,
    output: *mut u8,
    alphabet: *const u8,
    padded: u8,
) -> usize {
    unsafe {
        let bits = (u32::from(input.read())) << 16 | (u32::from(input.add(1).read())) << 8;
        output.write(alphabet.add((bits >> 18) as usize & 0x3f).read());
        output
            .add(1)
            .write(alphabet.add((bits >> 12) as usize & 0x3f).read());
        output
            .add(2)
            .write(alphabet.add((bits >> 6) as usize & 0x3f).read());
        if padded != 0 {
            output.add(3).write(b'=');
            4
        } else {
            3
        }
    }
}

#[inline(always)]
unsafe fn encode_into(
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
    let wrapcol = if wrapcol == 0 {
        0
    } else {
        (wrapcol / 4).max(1) * 4
    };
    let output_len = if padded != 0 && !input_len.is_multiple_of(3) {
        raw_len
    } else {
        unpadded_len
    };

    if wrapcol == 0 {
        let mut input = input;
        let mut output = output;
        let end = unsafe { input.add(input_len - input_len % 3) };

        if pair_table.is_null() {
            while input < end {
                unsafe {
                    encode_triplet(input, output, alphabet);
                    input = input.add(3);
                    output = output.add(4);
                }
            }
        } else {
            while input < end {
                unsafe {
                    encode_triplet_pairs(input, output, pair_table);
                    input = input.add(3);
                    output = output.add(4);
                }
            }
        }

        match input_len % 3 {
            1 => {
                let bits = (u32::from(unsafe { input.read() })) << 16;
                unsafe {
                    output.write(alphabet.add((bits >> 18) as usize & 0x3f).read());
                    output
                        .add(1)
                        .write(alphabet.add((bits >> 12) as usize & 0x3f).read());
                }
                if padded != 0 {
                    unsafe {
                        output.add(2).write(b'=');
                        output.add(3).write(b'=');
                    }
                }
            }
            2 => {
                let bits = (u32::from(unsafe { input.read() })) << 16
                    | (u32::from(unsafe { input.add(1).read() })) << 8;
                unsafe {
                    output.write(alphabet.add((bits >> 18) as usize & 0x3f).read());
                    output
                        .add(1)
                        .write(alphabet.add((bits >> 12) as usize & 0x3f).read());
                    output
                        .add(2)
                        .write(alphabet.add((bits >> 6) as usize & 0x3f).read());
                    if padded != 0 {
                        output.add(3).write(b'=');
                    }
                }
            }
            _ => {}
        }
        return output_len;
    }

    let mut input = input;
    let mut written = 0;
    let mut column = 0;
    let end = unsafe { input.add(input_len - input_len % 3) };

    if pair_table.is_null() {
        while input < end {
            let output = unsafe { begin_wrapped_quad(output, &mut written, &mut column, wrapcol) };
            unsafe { encode_triplet(input, output, alphabet) };
            input = unsafe { input.add(3) };
            written += 4;
            column += 4;
        }
    } else {
        while input < end {
            let output = unsafe { begin_wrapped_quad(output, &mut written, &mut column, wrapcol) };
            unsafe { encode_triplet_pairs(input, output, pair_table) };
            input = unsafe { input.add(3) };
            written += 4;
            column += 4;
        }
    }

    let tail_len = match input_len % 3 {
        1 => {
            let output = unsafe { begin_wrapped_quad(output, &mut written, &mut column, wrapcol) };
            unsafe { encode_tail(input, output, alphabet, padded) }
        }
        2 => {
            let output = unsafe { begin_wrapped_quad(output, &mut written, &mut column, wrapcol) };
            unsafe { encode_tail2(input, output, alphabet, padded) }
        }
        _ => 0,
    };
    written += tail_len;

    debug_assert_eq!(
        written,
        output_len + (output_len.saturating_sub(1) / wrapcol)
    );
    written
}

pub unsafe fn encode_raw(
    input: *const u8,
    input_len: usize,
    output: *mut u8,
    alphabet: *const u8,
    padded: bool,
    wrapcol: usize,
    pairs: *const u16,
) -> usize {
    unsafe {
        encode_into(
            input,
            input_len,
            output,
            alphabet,
            u8::from(padded),
            wrapcol,
            pairs,
        )
    }
}

pub unsafe fn encode_with_pairs(
    input: *const u8,
    input_len: usize,
    output: *mut u8,
    alphabet: &[u8; 64],
    padded: bool,
    wrapcol: usize,
) -> usize {
    let pairs = encode_pairs(alphabet);
    unsafe {
        encode_raw(
            input,
            input_len,
            output,
            alphabet.as_ptr(),
            padded,
            wrapcol,
            pairs.as_ptr(),
        )
    }
}

const unsafe fn no_simd(_: *const u8, _: usize, _: *mut u8, _: *const u8) -> usize {
    0
}

pub fn encode<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    altchars: Option<&Bound<'py, PyAny>>,
    padded: bool,
    wrapcol: isize,
) -> PyResult<Bound<'py, PyBytes>> {
    python::encode(py, s, altchars, padded, wrapcol, no_simd)
}
