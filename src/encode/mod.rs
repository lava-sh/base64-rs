pub mod alphabet;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod avx512;
pub mod python;
pub mod scalar;
