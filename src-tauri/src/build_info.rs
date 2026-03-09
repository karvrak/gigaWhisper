//! Build information module
//!
//! Single-build configuration with Vulkan GPU support compiled in.
//! CPU fallback is always available at runtime via the gpu_enabled setting.

/// Build identifier used for update manifests.
/// With the single-installer approach, there is only one build variant.
pub const BUILD_VARIANT: &str = "vulkan";

/// Human-readable name for the build
pub const BUILD_VARIANT_DISPLAY: &str = "Vulkan (GPU)";
