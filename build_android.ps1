<#
MarioRS Android 构建脚本

功能:
  1. 检查并安装必要工具 (cargo-ndk, Rust Android targets)
  2. 编译 .so 动态库 (默认仅 arm64-v8a，可选全架构)
  3. 复制 .so 文件到 android/app/src/main/jniLibs/
  4. 调用 Gradle 生成 APK

渲染模式:
  自动模式 (GPU + CPU fallback):
    - 优先使用 Vulkan GPU 加速渲染
    - GPU 不可用时自动切换到 CPU 软件渲染
    - 兼容所有 Android 5.0+ 设备

使用方法:
    .\build_android.ps1                       # Debug APK (仅 arm64)
    .\build_android.ps1 -Release              # Release APK (仅 arm64)
    .\build_android.ps1 -Release -AllArch     # Release APK (全架构)
    .\build_android.ps1 -Release -SeparateApks  # 构建三个独立 APK (每个架构一个)
    .\build_android.ps1 -SkipRust             # 仅构建 APK (跳过 Rust 编译)
    .\build_android.ps1 -Help                 # 显示帮助信息

输出文件:
  dist\android\app-release-arm64.apk        # arm64 版本
  dist\android\app-release-universal.apk    # 全架构版本

环境要求:
  - Rust 工具链
  - Android SDK (设置 ANDROID_HOME 或 ANDROID_SDK_ROOT)
  - Android NDK (设置 ANDROID_NDK_HOME 或在 SDK 中安装)
  - JDK 17+
#>

param(
    [switch]$Release,
    [switch]$AllArch,
    [switch]$SeparateApks,
    [switch]$SkipRust,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

# 帮助信息
if ($Help) {
    Get-Help $MyInvocation.MyCommand.Path -Detailed
    exit 0
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  MarioRS Android Build Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Render Mode: GPU + CPU auto-fallback" -ForegroundColor Green

# ============================================================================
# 环境检查
# ============================================================================

Write-Host "`n[1/5] Checking build environment..." -ForegroundColor Yellow

# 检查 Rust
$rustc = Get-Command rustc -ErrorAction SilentlyContinue
if (-not $rustc) {
    Write-Host "  Error: Rust not found. Please install Rust from https://rustup.rs" -ForegroundColor Red
    exit 1
}
Write-Host "  Rust: $($rustc.Path)" -ForegroundColor Gray

# 检查 Android SDK
$androidHome = $env:ANDROID_HOME
if (-not $androidHome) { $androidHome = $env:ANDROID_SDK_ROOT }
if (-not $androidHome) {
    # 尝试常见路径
    $commonPaths = @(
        "$env:LOCALAPPDATA\Android\Sdk",
        "$env:USERPROFILE\AppData\Local\Android\Sdk",
        "C:\Android\Sdk"
    )
    foreach ($path in $commonPaths) {
        if (Test-Path $path) {
            $androidHome = $path
            break
        }
    }
}
if (-not $androidHome -or -not (Test-Path $androidHome)) {
    Write-Host "  Error: Android SDK not found." -ForegroundColor Red
    Write-Host "  Please install Android Studio or set ANDROID_HOME environment variable." -ForegroundColor Red
    exit 1
}
$env:ANDROID_HOME = $androidHome
Write-Host "  Android SDK: $androidHome" -ForegroundColor Gray

# 检查 NDK
$ndkHome = $env:ANDROID_NDK_HOME
if (-not $ndkHome) {
    # 在 SDK 中查找 NDK
    $ndkDir = Join-Path $androidHome "ndk"
    if (Test-Path $ndkDir) {
        $ndkVersions = Get-ChildItem $ndkDir -Directory | Sort-Object Name -Descending
        if ($ndkVersions.Count -gt 0) {
            $ndkHome = $ndkVersions[0].FullName
        }
    }
    # 尝试 ndk-bundle 目录
    if (-not $ndkHome) {
        $ndkBundle = Join-Path $androidHome "ndk-bundle"
        if (Test-Path $ndkBundle) {
            $ndkHome = $ndkBundle
        }
    }
}
if (-not $ndkHome -or -not (Test-Path $ndkHome)) {
    Write-Host "  Error: Android NDK not found." -ForegroundColor Red
    Write-Host "  Please install NDK via Android Studio SDK Manager or set ANDROID_NDK_HOME." -ForegroundColor Red
    exit 1
}
$env:ANDROID_NDK_HOME = $ndkHome
Write-Host "  Android NDK: $ndkHome" -ForegroundColor Gray

# 检查 JDK
$javaHome = $env:JAVA_HOME
if (-not $javaHome -or -not (Test-Path $javaHome)) {
    Write-Host "  Warning: JAVA_HOME not set. Gradle may fail." -ForegroundColor Yellow
}
else {
    Write-Host "  Java: $javaHome" -ForegroundColor Gray
}

# ============================================================================
# 安装 cargo-ndk 和 Rust targets
# ============================================================================

Write-Host "`n[2/5] Checking cargo-ndk and Rust targets..." -ForegroundColor Yellow

# 检查 cargo-ndk
$cargoNdk = Get-Command cargo-ndk -ErrorAction SilentlyContinue
if (-not $cargoNdk) {
    Write-Host "  Installing cargo-ndk..." -ForegroundColor Gray
    cargo install cargo-ndk
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  Error: Failed to install cargo-ndk" -ForegroundColor Red
        exit 1
    }
}
else {
    Write-Host "  cargo-ndk: installed" -ForegroundColor Gray
}

# 添加 Android targets (根据参数决定)
if ($AllArch -or $SeparateApks) {
    $targets = @(
        "aarch64-linux-android",      # arm64-v8a
        "armv7-linux-androideabi",    # armeabi-v7a
        "x86_64-linux-android"        # x86_64
    )
    if ($SeparateApks) {
        Write-Host "  Building separate APKs for each architecture" -ForegroundColor Gray
    } else {
        Write-Host "  Building all architectures (arm64, armv7, x86_64)" -ForegroundColor Gray
    }
} else {
    $targets = @(
        "aarch64-linux-android"       # arm64-v8a only
    )
    Write-Host "  Building arm64 only (use -AllArch or -SeparateApks for all)" -ForegroundColor Gray
}

foreach ($target in $targets) {
    $installed = rustup target list --installed | Select-String $target
    if (-not $installed) {
        Write-Host "  Adding target: $target" -ForegroundColor Gray
        rustup target add $target
    }
    else {
        Write-Host "  Target $target : installed" -ForegroundColor Gray
    }
}

# ============================================================================
# 公共变量和函数
# ============================================================================

$jniLibsDir = Join-Path $PSScriptRoot "android\app\src\main\jniLibs"
$androidDir = Join-Path $PSScriptRoot "android"
$buildType = if ($Release) { "--release" } else { "" }
$gradleTask = if ($Release) { "assembleRelease" } else { "assembleDebug" }

# 架构映射
$allArchitectures = @("arm64-v8a", "armeabi-v7a", "x86_64")
$libcppMappings = @{
    "arm64-v8a" = "aarch64-linux-android"
    "armeabi-v7a" = "arm-linux-androideabi"
    "x86_64" = "x86_64-linux-android"
}
$sysroot = "$ndkHome\toolchains\llvm\prebuilt\windows-x86_64\sysroot\usr\lib"

# 编译单个架构的 Rust 代码
function Build-RustArch {
    param([string]$arch)
    
    Write-Host "  Compiling for $arch (feature: android)..." -ForegroundColor Gray
    cargo ndk -t $arch -o $jniLibsDir build --no-default-features --features android $buildType
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  Error: $arch build failed" -ForegroundColor Red
        return $false
    }
    
    # 复制 libc++_shared.so
    if ($libcppMappings.ContainsKey($arch)) {
        $srcLib = "$sysroot\$($libcppMappings[$arch])\libc++_shared.so"
        $dstDir = "$jniLibsDir\$arch"
        if (Test-Path $srcLib) {
            Copy-Item $srcLib $dstDir -Force
        }
    }
    return $true
}

# 调用 Gradle 构建 APK
function Build-Apk {
    Push-Location $androidDir
    try {
        $gradlew = Join-Path $androidDir "gradlew.bat"
        $wrapperJar = Join-Path $androidDir "gradle\wrapper\gradle-wrapper.jar"
        
        if (-not (Test-Path $wrapperJar)) {
            $gradle = Get-Command gradle -ErrorAction SilentlyContinue
            if ($gradle) {
                & gradle $gradleTask
            } else {
                Write-Host "  Error: Neither gradle wrapper nor system gradle found." -ForegroundColor Red
                return $false
            }
        } else {
            & $gradlew $gradleTask
        }
        
        return ($LASTEXITCODE -eq 0)
    }
    finally {
        Pop-Location
    }
}

# 清理 jniLibs 目录
function Clear-JniLibs {
    if (Test-Path $jniLibsDir) {
        Get-ChildItem -Path $jniLibsDir -Directory | ForEach-Object {
            Remove-Item $_.FullName -Recurse -Force
        }
    }
}

# ============================================================================
# 构建逻辑
# ============================================================================

# 输出目录 (与 build_win7xp.ps1 保持一致)
$distDir = Join-Path $PSScriptRoot "dist\android"
if (-not (Test-Path $distDir)) {
    New-Item -ItemType Directory -Path $distDir -Force | Out-Null
}

# Gradle APK 输出目录
$gradleApkDir = if ($Release) {
    Join-Path $androidDir "app\build\outputs\apk\release"
} else {
    Join-Path $androidDir "app\build\outputs\apk\debug"
}
$apkBaseName = if ($Release) { "app-release" } else { "app-debug" }

if ($SeparateApks) {
    # 模式: 为每个架构生成独立的 APK
    Write-Host "`n[3/5] Building separate APKs for each architecture..." -ForegroundColor Yellow
    
    $outputApks = @()
    
    foreach ($arch in $allArchitectures) {
        Write-Host "`n  === Building $arch ===" -ForegroundColor Cyan
        
        if (-not $SkipRust) {
            # 清理所有架构，只保留当前架构
            Clear-JniLibs
            
            # 编译当前架构
            if (-not (Build-RustArch $arch)) {
                Write-Host "  Error: Failed to build $arch" -ForegroundColor Red
                exit 1
            }
        }
        
        # 构建 APK
        Write-Host "  Building APK for $arch..." -ForegroundColor Gray
        if (-not (Build-Apk)) {
            Write-Host "  Error: Gradle build failed for $arch" -ForegroundColor Red
            exit 1
        }
        
        # 立即复制 APK 到 dist 目录 (避免被后续构建覆盖)
        $srcApk = Join-Path $gradleApkDir "$apkBaseName.apk"
        $dstApk = Join-Path $distDir "$apkBaseName-$arch.apk"
        if (Test-Path $srcApk) {
            Copy-Item $srcApk $dstApk -Force
            $outputApks += $dstApk
            $sizeMB = [math]::Round((Get-Item $dstApk).Length / 1MB, 2)
            Write-Host "  Created: $apkBaseName-$arch.apk ($sizeMB MB)" -ForegroundColor Green
        }
    }
    
    # 输出结果
    Write-Host "`n[4/5] All builds completed!" -ForegroundColor Green
    Write-Host "`n========================================" -ForegroundColor Green
    Write-Host "  Separate APKs built successfully!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "  Output directory: $distDir" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  Output files:" -ForegroundColor Cyan
    foreach ($apk in $outputApks) {
        $sizeMB = [math]::Round((Get-Item $apk).Length / 1MB, 2)
        Write-Host "    $(Split-Path $apk -Leaf) ($sizeMB MB)" -ForegroundColor Gray
    }
    Write-Host ""
    Write-Host "  Install commands:" -ForegroundColor Cyan
    Write-Host "    ARM64:  adb install -r `"$distDir\$apkBaseName-arm64-v8a.apk`"" -ForegroundColor Gray
    Write-Host "    ARM32:  adb install -r `"$distDir\$apkBaseName-armeabi-v7a.apk`"" -ForegroundColor Gray
    Write-Host "    x86_64: adb install -r `"$distDir\$apkBaseName-x86_64.apk`"" -ForegroundColor Gray
    
} else {
    # 模式: 生成单个 APK (可能包含多个架构)
    
    if (-not $SkipRust) {
        Write-Host "`n[3/5] Building Rust libraries with cargo-ndk..." -ForegroundColor Yellow

        # 编译目标架构列表
        $archList = @("arm64-v8a")
        if ($AllArch) {
            $archList = @("arm64-v8a", "armeabi-v7a", "x86_64")
        }
        
        foreach ($arch in $archList) {
            if (-not (Build-RustArch $arch)) {
                exit 1
            }
        }

        Write-Host "  Rust libraries built successfully!" -ForegroundColor Green

        # 显示生成的文件
        Write-Host "`n  Generated .so files:" -ForegroundColor Gray
        Get-ChildItem -Path $jniLibsDir -Recurse -Filter "*.so" | ForEach-Object {
            $size = [math]::Round($_.Length / 1KB, 0)
            Write-Host "    $($_.FullName.Replace($jniLibsDir, '')) ($size KB)" -ForegroundColor Gray
        }
    }
    else {
        Write-Host "`n[3/5] Skipping Rust compilation (--SkipRust)" -ForegroundColor Yellow
    }

    # ============================================================================
    # 构建 APK
    # ============================================================================

    Write-Host "`n[4/5] Building APK with Gradle..." -ForegroundColor Yellow

    if (-not (Build-Apk)) {
        Write-Host "  Error: Gradle build failed" -ForegroundColor Red
        exit 1
    }

    # ============================================================================
    # 复制到 dist 目录
    # ============================================================================

    Write-Host "`n[5/5] Build completed!" -ForegroundColor Green

    $srcApk = Join-Path $gradleApkDir "$apkBaseName.apk"
    
    # 确定输出文件名
    if ($AllArch) {
        $dstApkName = "$apkBaseName-universal.apk"
    } else {
        $dstApkName = "$apkBaseName-arm64.apk"
    }
    $dstApk = Join-Path $distDir $dstApkName

    if (Test-Path $srcApk) {
        Copy-Item $srcApk $dstApk -Force
        $sizeMB = [math]::Round((Get-Item $dstApk).Length / 1MB, 2)
        
        Write-Host "`n========================================" -ForegroundColor Green
        Write-Host "  APK built successfully!" -ForegroundColor Green
        Write-Host "========================================" -ForegroundColor Green
        Write-Host "  Output: $dstApk"
        Write-Host "  Size: $sizeMB MB"
        Write-Host ""
        Write-Host "  Render mode: GPU (Vulkan) with CPU auto-fallback" -ForegroundColor Cyan
        Write-Host "  - Uses Vulkan GPU acceleration when available"
        Write-Host "  - Automatically falls back to CPU rendering if GPU fails"
        Write-Host ""
        Write-Host "  To install on device:" -ForegroundColor Cyan
        Write-Host "    adb install -r `"$dstApk`"" -ForegroundColor Gray
    }
    else {
        Write-Host "  Warning: APK file not found at expected location" -ForegroundColor Yellow
        Write-Host "  Check android/app/build/outputs/apk/ for APK files" -ForegroundColor Yellow
    }
}
