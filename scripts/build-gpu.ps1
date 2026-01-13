# GigaWhisper GPU Build Script
# Usage: .\scripts\build-gpu.ps1 -Backend <vulkan|cuda|cpu|all>

param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("vulkan", "cuda", "cpu", "all")]
    [string]$Backend
)

$ErrorActionPreference = "Stop"

function Build-Version {
    param([string]$Name, [string]$Features)

    Write-Host "`n========================================" -ForegroundColor Cyan
    Write-Host "Building GigaWhisper - $Name" -ForegroundColor Cyan
    Write-Host "========================================`n" -ForegroundColor Cyan

    $env:TAURI_BUILD_TARGET = $Name

    if ($Features) {
        Write-Host "Features: $Features" -ForegroundColor Yellow
        cargo tauri build --features $Features
    } else {
        Write-Host "Features: (none - CPU only)" -ForegroundColor Yellow
        cargo tauri build
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Host "Build failed for $Name" -ForegroundColor Red
        exit 1
    }

    # Rename the output
    $buildDir = "src-tauri\target\release\bundle\msi"
    if (Test-Path $buildDir) {
        Get-ChildItem $buildDir -Filter "*.msi" | ForEach-Object {
            $newName = $_.Name -replace "gigawhisper", "gigawhisper-$Name"
            $destPath = Join-Path $buildDir $newName
            if (Test-Path $destPath) { Remove-Item $destPath }
            Copy-Item $_.FullName $destPath
            Write-Host "Created: $newName" -ForegroundColor Green
        }
    }
}

switch ($Backend) {
    "vulkan" {
        Build-Version -Name "vulkan" -Features "gpu-vulkan"
    }
    "cuda" {
        Build-Version -Name "cuda" -Features "gpu-cuda"
    }
    "cpu" {
        Build-Version -Name "cpu" -Features ""
    }
    "all" {
        Build-Version -Name "cpu" -Features ""
        Build-Version -Name "vulkan" -Features "gpu-vulkan"
        Build-Version -Name "cuda" -Features "gpu-cuda"

        Write-Host "`n========================================" -ForegroundColor Green
        Write-Host "All builds completed!" -ForegroundColor Green
        Write-Host "========================================" -ForegroundColor Green
        Write-Host "`nAvailable installers:"
        Write-Host "  - gigawhisper-cpu.msi    (No GPU acceleration)"
        Write-Host "  - gigawhisper-vulkan.msi (AMD/Intel/NVIDIA via Vulkan)"
        Write-Host "  - gigawhisper-cuda.msi   (NVIDIA via CUDA - best perf)"
    }
}

Write-Host "`nDone!" -ForegroundColor Green
