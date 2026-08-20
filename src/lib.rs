pub mod decode;
pub mod encode;
pub mod simd_dispatch;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[pyo3::pymodule(name = "_base64_rs")]
mod base64_rs {
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
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            SimdIsa::Avx512 => unsafe {
                crate::encode::avx512::encode(py, s, altchars, padded, wrapcol)
            },
            _ => crate::encode::scalar::encode(py, s, altchars, padded, wrapcol),
        }
    }
}
