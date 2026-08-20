pub mod decode;
pub mod encode;
pub mod simd_dispatch;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[pyo3::pymodule(name = "_base64_rs")]
mod base64_rs {
    use pyo3::{exceptions::PyRuntimeError, prelude::*, types::PyBytes};

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
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Avx512 => unsafe {
                crate::encode::avx512::encode(py, s, altchars, padded, wrapcol)
            },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Avx2 => unsafe {
                crate::encode::avx2::encode(py, s, altchars, padded, wrapcol)
            },
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Ssse3 => unsafe {
                crate::encode::ssse3::encode(py, s, altchars, padded, wrapcol)
            },
            _ => crate::encode::scalar::encode(py, s, altchars, padded, wrapcol),
        }
    }

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

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[pyfunction(
        name = "_b64encode_avx2",
        signature = (s, altchars = None, *, padded = true, wrapcol = 0)
    )]
    fn b64encode_avx2<'py>(
        py: Python<'py>,
        s: &Bound<'py, PyAny>,
        altchars: Option<&Bound<'py, PyAny>>,
        padded: bool,
        wrapcol: isize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        if !matches!(SimdIsa::detected(), SimdIsa::Avx2 | SimdIsa::Avx512) {
            return Err(PyRuntimeError::new_err("AVX2 is not supported by this CPU"));
        }
        // SAFETY: the cached runtime ISA check above.
        unsafe { crate::encode::avx2::encode(py, s, altchars, padded, wrapcol) }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[pyfunction(
        name = "_b64encode_ssse3",
        signature = (s, altchars = None, *, padded = true, wrapcol = 0)
    )]
    fn b64encode_ssse3<'py>(
        py: Python<'py>,
        s: &Bound<'py, PyAny>,
        altchars: Option<&Bound<'py, PyAny>>,
        padded: bool,
        wrapcol: isize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        if !matches!(
            SimdIsa::detected(),
            SimdIsa::Ssse3 | SimdIsa::Avx2 | SimdIsa::Avx512
        ) {
            return Err(PyRuntimeError::new_err(
                "SSSE3 is not supported by this CPU",
            ));
        }
        // SAFETY: the cached runtime ISA check above.
        unsafe { crate::encode::ssse3::encode(py, s, altchars, padded, wrapcol) }
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[pyfunction(
        name = "_b64encode_avx512",
        signature = (s, altchars = None, *, padded = true, wrapcol = 0)
    )]
    fn b64encode_avx512<'py>(
        py: Python<'py>,
        s: &Bound<'py, PyAny>,
        altchars: Option<&Bound<'py, PyAny>>,
        padded: bool,
        wrapcol: isize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        if SimdIsa::detected() != SimdIsa::Avx512 {
            return Err(PyRuntimeError::new_err(
                "AVX-512F and AVX-512VBMI are not supported by this CPU",
            ));
        }
        // SAFETY: the cached runtime ISA check above.
        unsafe { crate::encode::avx512::encode(py, s, altchars, padded, wrapcol) }
    }
}
