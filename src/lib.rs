#![cfg_attr(target_arch = "arm", feature(stdarch_arm_neon_intrinsics))]

pub mod decode;
pub mod encode;
pub mod simd_dispatch;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[pyo3::pymodule(name = "_base64_rs")]
mod base64_rs {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    use pyo3::exceptions::PyRuntimeError;
    use pyo3::{prelude::*, types::PyBytes};

    use crate::simd_dispatch::SimdIsa;

    #[pymodule_export]
    #[allow(non_upper_case_globals)]
    const __version__: &str = env!("CARGO_PKG_VERSION");

    #[pyfunction(
        name = "_b64encode",
        signature = (s, altchars = None, *, padded = true, wrapcol = 0)
    )]
    fn b64encode<'py>(
        py: Python<'py>,
        s: &Bound<'py, PyAny>,
        altchars: Option<&Bound<'py, PyAny>>,
        padded: bool,
        wrapcol: isize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        match SimdIsa::detected() {
            #[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
            SimdIsa::Neon64 => unsafe {
                crate::encode::neon64::encode(py, s, altchars, padded, wrapcol)
            },
            #[cfg(target_arch = "arm")]
            SimdIsa::Neon32 => unsafe {
                crate::encode::neon32::encode(py, s, altchars, padded, wrapcol)
            },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Avx512 => unsafe {
                crate::encode::avx512::encode(py, s, altchars, padded, wrapcol)
            },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Avx2 => unsafe {
                crate::encode::avx2::encode(py, s, altchars, padded, wrapcol)
            },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Avx => unsafe { crate::encode::avx::encode(py, s, altchars, padded, wrapcol) },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Ssse3 => unsafe {
                crate::encode::ssse3::encode(py, s, altchars, padded, wrapcol)
            },
            _ => crate::encode::scalar::encode(py, s, altchars, padded, wrapcol),
        }
    }

    #[rustfmt::skip]
    macro_rules! b64encode_simd_fn {
        ($fn_name:ident, $py_name:literal, $cfg:meta, $isa:path, $err_msg:literal, $encode:path) => {
            #[allow(dead_code)]
            #[cfg($cfg)]
            #[pyfunction(
                name = $py_name,
                signature = (s, altchars = None, *, padded = true, wrapcol = 0)
            )]
            fn $fn_name<'py>(
                py: Python<'py>,
                s: &Bound<'py, PyAny>,
                altchars: Option<&Bound<'py, PyAny>>,
                padded: bool,
                wrapcol: isize,
            ) -> PyResult<Bound<'py, PyBytes>> {
                if !matches!(SimdIsa::detected(), $isa) {
                    return Err(PyRuntimeError::new_err($err_msg));
                }

                // SAFETY: the cached runtime ISA check above.
                unsafe { $encode(py, s, altchars, padded, wrapcol) }
            }
        };
    }

    b64encode_simd_fn!(
        b64encode_avx512,
        "_b64encode_avx512",
        any(target_arch = "x86", target_arch = "x86_64"),
        SimdIsa::Avx512,
        "AVX-512F and AVX-512VBMI are not supported by this CPU",
        crate::encode::avx512::encode
    );

    b64encode_simd_fn!(
        b64encode_avx2,
        "_b64encode_avx2",
        any(target_arch = "x86", target_arch = "x86_64"),
        SimdIsa::Avx2,
        "AVX2 is not supported by this CPU",
        crate::encode::avx2::encode
    );

    b64encode_simd_fn!(
        b64encode_avx,
        "_b64encode_avx",
        any(target_arch = "x86", target_arch = "x86_64"),
        SimdIsa::Avx,
        "AVX is not supported by this CPU",
        crate::encode::avx::encode
    );

    b64encode_simd_fn!(
        b64encode_ssse3,
        "_b64encode_ssse3",
        any(target_arch = "x86", target_arch = "x86_64"),
        SimdIsa::Ssse3,
        "SSSE3 is not supported by this CPU",
        crate::encode::ssse3::encode
    );

    b64encode_simd_fn!(
        b64encode_neon64,
        "_b64encode_neon64",
        any(target_arch = "aarch64", target_arch = "arm64ec"),
        SimdIsa::Neon64,
        "NEON is not supported by this CPU",
        crate::encode::neon64::encode
    );

    b64encode_simd_fn!(
        b64encode_neon32,
        "_b64encode_neon32",
        target_arch = "arm",
        SimdIsa::Neon32,
        "NEON is not supported by this CPU",
        crate::encode::neon32::encode
    );

    #[pyfunction(
        name = "_b64encode_scalar",
        signature = (s, altchars = None, *, padded = true, wrapcol = 0)
    )]
    fn b64encode_scalar<'py>(
        py: Python<'py>,
        s: &Bound<'py, PyAny>,
        altchars: Option<&Bound<'py, PyAny>>,
        padded: bool,
        wrapcol: isize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        crate::encode::scalar::encode(py, s, altchars, padded, wrapcol)
    }

    #[pyfunction(name = "_standard_b64encode", signature = (s))]
    fn standard_b64encode<'py>(
        py: Python<'py>,
        s: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        match SimdIsa::detected() {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Avx512 => unsafe { crate::encode::avx512::encode(py, s, None, true, 0) },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Avx2 => unsafe { crate::encode::avx2::encode(py, s, None, true, 0) },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Avx => unsafe { crate::encode::avx::encode(py, s, None, true, 0) },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Ssse3 => unsafe { crate::encode::ssse3::encode(py, s, None, true, 0) },
            _ => crate::encode::scalar::encode(py, s, None, true, 0),
        }
    }

    #[pyfunction(name = "_urlsafe_b64encode", signature = (s, *, padded = true))]
    fn urlsafe_b64encode<'py>(
        py: Python<'py>,
        s: &Bound<'py, PyAny>,
        padded: bool,
    ) -> PyResult<Bound<'py, PyBytes>> {
        match SimdIsa::detected() {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Avx512 => crate::encode::python::encode_urlsafe(
                py,
                s,
                padded,
                crate::encode::avx512::encode_simd_prefix,
            ),
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Avx2 => crate::encode::python::encode_urlsafe(
                py,
                s,
                padded,
                crate::encode::avx2::encode_simd_prefix,
            ),
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Avx => crate::encode::python::encode_urlsafe(
                py,
                s,
                padded,
                crate::encode::avx::encode_simd_prefix,
            ),
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Ssse3 => crate::encode::python::encode_urlsafe(
                py,
                s,
                padded,
                crate::encode::ssse3::encode_simd_prefix,
            ),
            _ => {
                crate::encode::python::encode_urlsafe(py, s, padded, crate::encode::scalar::no_simd)
            }
        }
    }
}
