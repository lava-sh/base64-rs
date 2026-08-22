pub mod alphabet;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod avx;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod avx2;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod avx512;
#[cfg(target_arch = "arm")]
pub mod neon32;
#[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
pub mod neon64;
pub mod python;
pub mod scalar;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod ssse3;
pub mod tables;
