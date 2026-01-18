//! Build information module
//!
//! Contains compile-time constants about the build variant.

/// The GPU variant this build was compiled with.
/// Used to determine which update endpoint to use.
#[cfg(feature = "gpu-cuda")]
pub const BUILD_VARIANT: &str = "cuda";

#[cfg(feature = "gpu-vulkan")]
pub const BUILD_VARIANT: &str = "vulkan";

#[cfg(not(any(feature = "gpu-cuda", feature = "gpu-vulkan")))]
pub const BUILD_VARIANT: &str = "cpu";

/// Human-readable name for the build variant
#[cfg(feature = "gpu-cuda")]
pub const BUILD_VARIANT_DISPLAY: &str = "CUDA (NVIDIA GPU)";

#[cfg(feature = "gpu-vulkan")]
pub const BUILD_VARIANT_DISPLAY: &str = "Vulkan (GPU)";

#[cfg(not(any(feature = "gpu-cuda", feature = "gpu-vulkan")))]
pub const BUILD_VARIANT_DISPLAY: &str = "CPU";
