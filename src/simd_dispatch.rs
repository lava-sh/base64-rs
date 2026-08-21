use core::sync::atomic::{AtomicUsize, Ordering};

// SIMD Instruction Set Architecture
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum SimdIsa {
    Scalar = 0,
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    Ssse3 = 1,
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    Avx = 2,
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    Avx2 = 3,
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    Avx512 = 4,
    #[cfg(target_arch = "arm")]
    Neon32 = 5,
    #[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
    Neon64 = 6,
}

impl SimdIsa {
    #[cold]
    fn detect() -> Self {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            // https://www.intel.com/content/www/us/en/docs/intrinsics-guide/index.html#avx512techs=AVX512_VBMI
            if is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512vbmi") {
                return Self::Avx512;
            }

            // https://www.intel.com/content/www/us/en/docs/intrinsics-guide/index.html#avxnewtechs=AVX2
            if is_x86_feature_detected!("avx2") {
                return Self::Avx2;
            }

            // https://www.intel.com/content/www/us/en/docs/intrinsics-guide/index.html#avxnewtechs=AVX
            if is_x86_feature_detected!("avx") {
                return Self::Avx;
            }

            // `SSE*` instruction set hierarchy:
            // SSE2
            //  └─ SSE3
            //      └─ SSSE3
            //          └─ SSE4.1
            //              └─ SSE4.2
            //
            // Processors with SSE4.1/SSE4.2 support always include SSSE3, so they pass this check.
            if is_x86_feature_detected!("ssse3") {
                return Self::Ssse3;
            }
        }
        #[cfg(target_arch = "arm")]
        {
            // https://developer.arm.com/architectures/instruction-sets/intrinsics
            if std::arch::is_arm_feature_detected!("neon") {
                return Self::Neon32;
            }
        }
        #[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
        {
            // // https://developer.arm.com/architectures/instruction-sets/intrinsics
            if std::arch::is_aarch64_feature_detected!("neon") {
                return Self::Neon64;
            }
        }

        Self::Scalar
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
        unsafe { core::mem::transmute::<usize, Self>(raw) }
    }
}
