use core::sync::atomic::{AtomicUsize, Ordering};

// SIMD Instruction Set Architecture
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum SimdIsa {
    Scalar = 0,
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    Sse2 = 1,
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    Sse41 = 2,
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    Avx2 = 3,
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    Avx512 = 4,
    #[cfg(target_arch = "aarch64")]
    Neon = 5,
}

impl SimdIsa {
    #[cold]
    fn detect() -> Self {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            // https://www.intel.com/content/www/us/en/docs/intrinsics-guide/index.html#avx512techs=AVX512_VBMI
            if is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bw") {
                return SimdIsa::Avx512;
            }
            // https://www.intel.com/content/www/us/en/docs/intrinsics-guide/index.html#avxnewtechs=AVX2
            if is_x86_feature_detected!("avx2") {
                return SimdIsa::Avx2;
            }
            // https://www.intel.com/content/www/us/en/docs/intrinsics-guide/index.html#ssetechs=SSE4_1
            if is_x86_feature_detected!("sse4.1") {
                return SimdIsa::Sse41;
            }
            // https://www.intel.com/content/www/us/en/docs/intrinsics-guide/index.html#ssetechs=SSE2
            if is_x86_feature_detected!("sse2") {
                return SimdIsa::Sse2;
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("neon") {
                return SimdIsa::Neon;
            }
        }

        SimdIsa::Scalar
    }

    #[inline(always)]
    pub fn detected() -> Self {
        static CACHED: AtomicUsize = AtomicUsize::new(usize::MAX);

        let raw = CACHED.load(Ordering::Relaxed);
        let raw = if raw == usize::MAX {
            let detect = Self::detect() as usize;
            CACHED.store(detect, Ordering::Relaxed);
            detect
        } else {
            raw
        };

        // SAFETY: `SimdIsa` is `#[repr(usize)]`, any valid discriminant is
        // a valid bit pattern for the enum, so transmute cannot produce UB.
        unsafe { core::mem::transmute::<usize, SimdIsa>(raw) }
    }
}
