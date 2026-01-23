<#
MarioRS Release build script using cargo zbuild
Purpose: minimal binary size + static CRT (no vcruntime DLLs)

Usage:
  .\build_release_zbuild.ps1           # use stable toolchain
  .\build_release_zbuild.ps1 -Nightly  # use nightly + build-std (smaller)

This script will try to install `cargo-zbuild` if it's not already available.
#>

param(
    [switch]$Nightly,
    [switch]$ForceStable
)

$ErrorActionPreference = "Stop"

# Default behavior: prefer nightly for best size unless user explicitly forces stable
if (-not $PSBoundParameters.ContainsKey('Nightly') -and -not $PSBoundParameters.ContainsKey('ForceStable')) {
    $Nightly = $true
    Write-Host "Defaulting to nightly toolchain for smaller binary (use -ForceStable to force stable)." -ForegroundColor Yellow
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  MarioRS Release Build (cargo zbuild)" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# 注意: RUSTFLAGS 已在 .cargo\config.toml 中配置，包括:
# - 静态链接 CRT (target-feature=+crt-static)
# - MSVC 链接器优化 (/OPT:REF, /OPT:ICF, /INCREMENTAL:NO)
# 此处不设置 $env:RUSTFLAGS，避免覆盖配置文件

# force Windows GDI backend with wgpu rendering and dark-theme support
# gdi-backend: 使用 GDI 创建窗口（比 winit 体积小）
# wgpu-backend: 使用 wgpu GPU 渲染
# dark-theme: 暗黑主题适配（Win10+）
$featureArgs = @("--no-default-features", "--features", "gdi-backend,wgpu-backend,dark-theme")

# ensure cargo-zbuild is available (cargo subcommand executable is cargo-zbuild)
$cz = Get-Command cargo-zbuild -ErrorAction SilentlyContinue
if (-not $cz) {
    Write-Host "cargo-zbuild not found. Installing cargo-zbuild..." -ForegroundColor Yellow
    Write-Host "This requires network access and cargo in PATH." -ForegroundColor Gray
    cargo install cargo-zbuild
}

if ($Nightly) {
    Write-Host "`n[1/3] Checking nightly toolchain..." -ForegroundColor Yellow
    $nightlyInstalled = rustup toolchain list | Select-String "nightly"
    if (-not $nightlyInstalled) {
        Write-Host "  Installing nightly toolchain..." -ForegroundColor Gray
        rustup toolchain install nightly
    }

    Write-Host "[2/3] Using nightly + cargo zbuild (build-std)..." -ForegroundColor Yellow
    Write-Host "  RUSTFLAGS: from .cargo/config.toml (CRT static + MSVC optimizations)" -ForegroundColor Gray
    Write-Host "  Features: --no-default-features --features gdi-backend,wgpu-backend,dark-theme" -ForegroundColor Gray

    # cargo zbuild 自带 build-std 优化，产生最小体积
    # 使用 --bin mario 只编译可执行文件，避免生成 cdylib (mario.dll)
    cargo +nightly zbuild @featureArgs `
        -Z build-std=std,panic_abort `
        --target x86_64-pc-windows-msvc `
        --bin mario

    $exePath = "target\x86_64-pc-windows-msvc\release\mario.exe"
} else {
    Write-Host "`n[1/3] Building with stable toolchain via cargo zbuild..." -ForegroundColor Yellow
    Write-Host "  RUSTFLAGS: from .cargo/config.toml (CRT static + MSVC optimizations)" -ForegroundColor Gray
    Write-Host "  Features: --no-default-features --features gdi-backend,wgpu-backend,dark-theme" -ForegroundColor Gray

    # 使用 --bin mario 只编译可执行文件，避免生成 cdylib (mario.dll)
    cargo zbuild @featureArgs --target x86_64-pc-windows-msvc --bin mario

    $exePath = "target\release\mario.exe"
}

if ($LASTEXITCODE -ne 0) {
    Write-Host "`nBuild failed!" -ForegroundColor Red
    exit 1
}

# check output
if (Test-Path $exePath) {
    $fileInfo = Get-Item $exePath
    $sizeMB = [math]::Round($fileInfo.Length / 1MB, 2)
    $sizeKB = [math]::Round($fileInfo.Length / 1KB, 0)

    Write-Host "`n========================================" -ForegroundColor Green
    Write-Host "  Build succeeded!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "  Output file: $exePath"
    Write-Host "  File size: $sizeMB MB ($sizeKB KB)"

    # llvm-strip 后处理: 进一步精简二进制文件
    Write-Host "`n[Post-processing with llvm-strip]" -ForegroundColor Yellow
    $llvmStrip = Get-Command llvm-strip -ErrorAction SilentlyContinue
    if ($llvmStrip) {
        $beforeStripSize = (Get-Item $exePath).Length
        Write-Host "  Using llvm-strip to remove unnecessary sections..." -ForegroundColor Gray
        & llvm-strip --strip-unneeded $exePath
        if ($LASTEXITCODE -eq 0) {
            $afterStripSize = (Get-Item $exePath).Length
            $beforeStripKB = [math]::Round($beforeStripSize / 1KB, 0)
            $afterStripKB = [math]::Round($afterStripSize / 1KB, 0)
            Write-Host "  llvm-strip completed: $beforeStripKB KB -> $afterStripKB KB" -ForegroundColor Green
        } else {
            Write-Host "  llvm-strip failed (exit $LASTEXITCODE), skipping..." -ForegroundColor Yellow
        }
    } else {
        Write-Host "  llvm-strip not found on PATH. Skipping..." -ForegroundColor Gray
        Write-Host "  (Install LLVM or use 'strip' from mingw/msys2 for further size reduction)" -ForegroundColor Gray
    }

    Write-Host "`n[Checking DLL dependencies]" -ForegroundColor Yellow
    $dumpbin = Get-Command dumpbin -ErrorAction SilentlyContinue
    if ($dumpbin) {
        $deps = dumpbin /dependents $exePath 2>$null | Select-String "\.dll" | ForEach-Object { $_.ToString().Trim() }
        if ($deps) {
            Write-Host "  Dependent DLLs:" -ForegroundColor Gray
            $deps | ForEach-Object { Write-Host "    $_" -ForegroundColor Gray }
        }
    } else {
        Write-Host "  (dumpbin not available, skipping dependency check)" -ForegroundColor Gray
        Write-Host "  CRT static linking enabled; should not depend on vcruntime*.dll" -ForegroundColor Gray
    }

    Write-Host "`nTip: You can further compress with UPX: upx --best $exePath" -ForegroundColor Cyan
} else {
    Write-Host "Output file not found: $exePath" -ForegroundColor Red
    exit 1
}

# Optional automatic UPX compression: if upx is available, create a backup and compress
Write-Host "`n[Optional] Checking for UPX to optionally compress binary..." -ForegroundColor Yellow
$upx = Get-Command upx -ErrorAction SilentlyContinue
if ($upx) {
    Write-Host "  UPX found: $($upx.Path)" -ForegroundColor Gray
    $backupPath = "$exePath.orig"
    if (-not (Test-Path $backupPath)) {
        Copy-Item -Path $exePath -Destination $backupPath -Force
        Write-Host "  Backup created: $backupPath" -ForegroundColor Gray
    } else {
        Write-Host "  Backup already exists: $backupPath" -ForegroundColor Gray
    }

    $beforeSize = (Get-Item $exePath).Length
    Write-Host "  Compressing with UPX (--best --lzma)..." -ForegroundColor Gray
    # Run UPX and capture exit
    $upxArgs = "--best --lzma `"$exePath`""
    $psi = Start-Process -FilePath $upx.Path -ArgumentList $upxArgs -NoNewWindow -PassThru -Wait
    if ($psi.ExitCode -eq 0) {
        $afterSize = (Get-Item $exePath).Length
        $beforeKB = [math]::Round($beforeSize / 1KB, 0)
        $afterKB = [math]::Round($afterSize / 1KB, 0)
        Write-Host "  UPX compression completed: $beforeKB KB -> $afterKB KB" -ForegroundColor Green
        Write-Host "  (backup at $backupPath)" -ForegroundColor Gray
    } else {
        Write-Host "  UPX compression failed (exit $($psi.ExitCode)). Restoring backup..." -ForegroundColor Red
        Copy-Item -Path $backupPath -Destination $exePath -Force
        exit 1
    }
} else {
    Write-Host "  UPX not found on PATH. Skipping automatic compression." -ForegroundColor Gray
}
