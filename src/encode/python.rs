use core::mem::MaybeUninit;

use pyo3::{
    exceptions::{PyMemoryError, PyValueError},
    ffi,
    prelude::*,
    types::PyBytes,
};

use super::{alphabet::Alphabet, scalar};

const PAIR_TABLE_MIN_INPUT: usize = 32 * 1024;

pub type Fn = unsafe fn(*const u8, usize, *mut u8, *const u8) -> usize;

struct Buffer(ffi::Py_buffer);

impl Drop for Buffer {
    fn drop(&mut self) {
        // SAFETY: `get_buffer` returned successfully, so `self.0` is
        // a valid `Py_buffer` obtained from `PyObject_GetBuffer`.
        unsafe { ffi::PyBuffer_Release(&raw mut self.0) };
    }
}

fn get_buffer(py: Python<'_>, object: &Bound<'_, PyAny>) -> PyResult<Buffer> {
    let mut buf = MaybeUninit::<ffi::Py_buffer>::uninit();

    if unsafe { ffi::PyObject_GetBuffer(object.as_ptr(), buf.as_mut_ptr(), ffi::PyBUF_SIMPLE) } != 0
    {
        return Err(PyErr::fetch(py));
    }
    // SAFETY: `PyObject_GetBuffer` returned 0, so it has
    // successfully initialized `buffer` in place.
    Ok(Buffer(unsafe { buf.assume_init() }))
}

fn encoded_len(input_len: usize, padded: bool, wrapcol: usize) -> Option<usize> {
    let groups = input_len.div_ceil(3);
    let raw_len = groups.checked_mul(4)?;
    let encoded_len = if padded {
        raw_len
    } else {
        raw_len.checked_sub(match input_len % 3 {
            1 => 2,
            2 => 1,
            _ => 0,
        })?
    };
    if wrapcol == 0 || encoded_len == 0 {
        return Some(encoded_len);
    }
    let wrapcol = (wrapcol / 4).max(1) * 4;
    encoded_len.checked_add((encoded_len - 1) / wrapcol)
}

pub fn encode<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    altchars: Option<&Bound<'py, PyAny>>,
    padded: bool,
    wrapcol: isize,
    encode_impl: Fn,
) -> PyResult<Bound<'py, PyBytes>> {
    if wrapcol < 0 {
        return Err(PyValueError::new_err("wrapcol must be >= 0"));
    }

    let altchars = if let Some(altchars) = altchars {
        let buf = get_buffer(py, altchars)?;
        if buf.0.len != 2 {
            return Err(PyValueError::new_err(format!(
                "invalid altchars: {altchars:?}"
            )));
        }
        // buffer is C-contiguous and has exactly two bytes.
        let altchars = buf.0.buf.cast::<u8>();
        Some(unsafe { [altchars.read(), altchars.add(1).read()] })
    } else {
        None
    };

    encode_with_altchars(
        py,
        s,
        altchars,
        padded,
        wrapcol.cast_unsigned(),
        encode_impl,
    )
}

fn encode_with_altchars<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    altchars: Option<[u8; 2]>,
    padded: bool,
    wrapcol: usize,
    encode_impl: Fn,
) -> PyResult<Bound<'py, PyBytes>> {
    let input = get_buffer(py, s)?;
    let mut alphabet = Alphabet::Standard.table().0;
    let kind = altchars.map_or(Some(Alphabet::Standard), |suffix| {
        alphabet[62..].copy_from_slice(&suffix);
        Alphabet::from_altchars(suffix)
    });

    let input_len = input.0.len.cast_unsigned();
    let output_len = encoded_len(input_len, padded, wrapcol)
        .ok_or_else(|| PyMemoryError::new_err("encoded data is too large"))?;

    if output_len > ffi::PY_SSIZE_T_MAX as usize {
        return Err(PyMemoryError::new_err("encoded data is too large"));
    }

    let pairs = kind.map(Alphabet::pairs);

    let result =
        unsafe { ffi::PyBytes_FromStringAndSize(core::ptr::null(), output_len.cast_signed()) };
    if result.is_null() {
        return Err(PyErr::fetch(py));
    }
    let output = unsafe { ffi::PyBytes_AS_STRING(result).cast_mut().cast::<u8>() };
    let input = input.0.buf.cast::<u8>();
    py.detach(move || {
        let consumed = if wrapcol == 0 {
            unsafe { encode_impl(input, input_len, output, alphabet.as_ptr()) }
        } else {
            0
        };
        let input = unsafe { input.add(consumed) };
        let output = unsafe { output.add(consumed / 3 * 4) };
        let input_len = input_len - consumed;
        let custom_pairs = (pairs.is_none() && input_len >= PAIR_TABLE_MIN_INPUT)
            .then(|| scalar::encode_pairs(&alphabet));
        let pairs = custom_pairs.as_ref().map_or_else(
            || pairs.unwrap_or(core::ptr::null()),
            |pairs| pairs.as_ptr(),
        );
        unsafe {
            scalar::encode_into(
                input,
                input_len,
                output,
                alphabet.as_ptr(),
                u8::from(padded),
                wrapcol,
                pairs,
            )
        };
    });
    Ok(unsafe { Bound::from_owned_ptr(py, result).cast_into_unchecked() })
}

pub fn encode_urlsafe<'py>(
    py: Python<'py>,
    s: &Bound<'py, PyAny>,
    padded: bool,
    encode_impl: Fn,
) -> PyResult<Bound<'py, PyBytes>> {
    encode_with_altchars(py, s, Some(*b"-_"), padded, 0, encode_impl)
}
