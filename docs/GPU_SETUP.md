# GPU Acceleration Setup

GigaWhisper supports GPU acceleration for faster transcription using whisper.cpp.

## Supported GPU Backends

| Backend | GPUs Supported | Performance | Windows Support |
|---------|---------------|-------------|-----------------|
| **Vulkan** | AMD, Intel, NVIDIA | Good | Yes |
| **CUDA** | NVIDIA only | Best | Yes |
| **CPU** | All | Baseline | Yes |

## Recommended Configuration

- **AMD GPUs (RX 6000/7000)**: Use Vulkan backend
- **NVIDIA GPUs (GTX/RTX)**: Use CUDA backend for best performance
- **Intel GPUs**: Use Vulkan backend
- **No GPU / Integrated**: Use CPU backend

## Build Requirements

### For Vulkan (AMD/Intel/NVIDIA)

1. **Install Vulkan SDK**:
   - Download from: https://vulkan.lunarg.com/sdk/home#windows
   - Run installer and select all components
   - Ensure `VULKAN_SDK` environment variable is set automatically

2. **Verify installation**:
   ```powershell
   echo $env:VULKAN_SDK
   # Should show path like: C:\VulkanSDK\1.3.xxx.x
   ```

3. **Build with Vulkan**:
   ```powershell
   cd src-tauri
   cargo build --release --features gpu-vulkan
   ```

### For CUDA (NVIDIA only)

1. **Install CUDA Toolkit**:
   - Download from: https://developer.nvidia.com/cuda-downloads
   - Select Windows > x86_64 > Version 12.x (latest)
   - Install with default options

2. **Install cuDNN** (optional, improves performance):
   - Download from: https://developer.nvidia.com/cudnn
   - Extract to CUDA installation directory

3. **Verify installation**:
   ```powershell
   nvcc --version
   # Should show CUDA version
   ```

4. **Build with CUDA**:
   ```powershell
   cd src-tauri
   cargo build --release --features gpu-cuda
   ```

## Building Installers

Use the provided build script to create installers:

```powershell
# Build all versions (CPU, Vulkan, CUDA)
.\scripts\build-gpu.ps1 -Backend all

# Build only Vulkan version (for AMD GPUs)
.\scripts\build-gpu.ps1 -Backend vulkan

# Build only CUDA version (for NVIDIA GPUs)
.\scripts\build-gpu.ps1 -Backend cuda

# Build CPU-only version
.\scripts\build-gpu.ps1 -Backend cpu
```

## Enabling GPU in the App

1. Open GigaWhisper settings
2. Go to **Transcription** section
3. Enable **GPU Acceleration**
4. Select the appropriate backend (Vulkan for AMD, CUDA for NVIDIA)
5. Restart the app if prompted

## Troubleshooting

### "Vulkan SDK not found"
- Ensure VULKAN_SDK environment variable is set
- Restart your terminal/IDE after SDK installation
- Try running: `set VULKAN_SDK=C:\VulkanSDK\1.3.xxx.x` before build

### "CUDA not found"
- Ensure CUDA is in PATH
- Check that `nvcc --version` works
- Reinstall CUDA Toolkit if needed

### GPU not detected at runtime
- Update GPU drivers to latest version
- Ensure the correct build version is installed (Vulkan for AMD, CUDA for NVIDIA)
- Check Windows GPU settings in Display Settings

### Poor GPU performance
- Ensure you're using a dedicated GPU, not integrated
- Close other GPU-intensive applications
- Try a larger Whisper model (medium or large) to see GPU benefit

## Performance Comparison

Typical transcription times for 10 seconds of audio:

| Configuration | tiny | base | small | medium | large |
|--------------|------|------|-------|--------|-------|
| CPU (8 threads) | ~0.5s | ~1s | ~3s | ~8s | ~15s |
| Vulkan (RX 6700) | ~0.2s | ~0.4s | ~1s | ~3s | ~6s |
| CUDA (RTX 3080) | ~0.1s | ~0.2s | ~0.5s | ~1.5s | ~3s |

*Times are approximate and vary based on audio content and system configuration.*
