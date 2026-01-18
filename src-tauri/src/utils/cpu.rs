//! CPU Detection
//!
//! Detect CPU capabilities for optimal whisper.cpp configuration.

/// CPU information for optimization
#[derive(Debug, Clone, serde::Serialize)]
pub struct CpuInfo {
    /// Number of physical CPU cores
    pub physical_cores: usize,
    /// Number of logical CPU cores (including hyperthreading)
    pub logical_cores: usize,
    /// Recommended number of threads for whisper.cpp
    pub recommended_threads: usize,
    /// Whether the CPU supports AVX2
    pub has_avx2: bool,
    /// Whether the CPU supports AVX512
    pub has_avx512: bool,
}

impl CpuInfo {
    /// Detect CPU information
    pub fn detect() -> Self {
        let logical_cores = num_cpus::get();
        let physical_cores = num_cpus::get_physical();

        // Recommended threads for whisper.cpp:
        // - Use physical cores for best performance (avoid hyperthreading overhead)
        // - But leave 1-2 cores free for system responsiveness
        // - Minimum 1 thread, maximum 8 (beyond 8, diminishing returns)
        let recommended_threads = calculate_optimal_threads(physical_cores);

        // Detect SIMD capabilities
        let (has_avx2, has_avx512) = detect_simd_support();

        Self {
            physical_cores,
            logical_cores,
            recommended_threads,
            has_avx2,
            has_avx512,
        }
    }
}

/// Calculate optimal thread count for whisper.cpp
fn calculate_optimal_threads(physical_cores: usize) -> usize {
    if physical_cores <= 2 {
        // On dual-core, use all physical cores
        physical_cores
    } else if physical_cores <= 4 {
        // On quad-core, leave 1 core free
        physical_cores - 1
    } else if physical_cores <= 8 {
        // On 6-8 cores, use physical cores minus 2
        physical_cores - 2
    } else {
        // On high-core-count CPUs, cap at 8 threads
        // Beyond 8 threads, whisper.cpp shows diminishing returns
        8
    }
}

/// Detect SIMD instruction support
#[cfg(target_arch = "x86_64")]
fn detect_simd_support() -> (bool, bool) {
    let has_avx2 = is_x86_feature_detected!("avx2");
    let has_avx512f = is_x86_feature_detected!("avx512f");

    (has_avx2, has_avx512f)
}

#[cfg(not(target_arch = "x86_64"))]
fn detect_simd_support() -> (bool, bool) {
    (false, false)
}

/// Get optimal number of threads for transcription
///
/// If `configured_threads` is 0, returns auto-detected optimal value.
/// Otherwise, returns the configured value capped at logical cores.
pub fn get_optimal_threads(configured_threads: usize) -> usize {
    if configured_threads == 0 {
        CpuInfo::detect().recommended_threads
    } else {
        // Cap at logical cores to avoid over-subscription
        let max_threads = num_cpus::get();
        configured_threads.min(max_threads)
    }
}

/// Get a human-readable description of the CPU optimization status
pub fn get_cpu_optimization_summary() -> String {
    let info = CpuInfo::detect();

    let simd_status = if info.has_avx512 {
        "AVX-512 enabled"
    } else if info.has_avx2 {
        "AVX2 enabled"
    } else {
        "Basic SIMD"
    };

    format!(
        "{} cores ({} physical), {} threads recommended, {}",
        info.logical_cores,
        info.physical_cores,
        info.recommended_threads,
        simd_status
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_detection() {
        let info = CpuInfo::detect();

        assert!(info.physical_cores > 0);
        assert!(info.logical_cores > 0);
        assert!(info.logical_cores >= info.physical_cores);
        assert!(info.recommended_threads > 0);
        assert!(info.recommended_threads <= info.physical_cores);
    }

    #[test]
    fn test_optimal_threads_calculation() {
        // Test various core counts
        assert_eq!(calculate_optimal_threads(1), 1);
        assert_eq!(calculate_optimal_threads(2), 2);
        assert_eq!(calculate_optimal_threads(4), 3);
        assert_eq!(calculate_optimal_threads(6), 4);
        assert_eq!(calculate_optimal_threads(8), 6);
        assert_eq!(calculate_optimal_threads(16), 8);
        assert_eq!(calculate_optimal_threads(32), 8);
    }

    #[test]
    fn test_get_optimal_threads() {
        // Test auto-detection (0 = auto)
        let auto = get_optimal_threads(0);
        assert!(auto > 0);

        // Test manual override
        let manual = get_optimal_threads(4);
        assert!(manual <= num_cpus::get());
    }
}
