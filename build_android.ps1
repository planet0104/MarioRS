<#
MarioRS Android 构建脚本

功能:
  1. 检查并安装必要工具 (cargo-ndk, Rust Android targets)
  2. 编译多架构 .so 动态库 (arm64-v8a, armeabi-v7a, x86_64)
  3. 复制 .so 文件到 android/app/src/main/jniLibs/
  4. 调用 Gradle 生成 APK

使用方法:
  .\build_android.ps1           # 构建 Debug APK
  .\build_android.ps1 -Release  # 构建 Release APK
  .\build_android.ps1 -SkipRust # 仅构建 APK (跳过 Rust 编译)

环境要求:
  - Rust 工具链
  - Android SDK (设置 ANDROID_HOME 或 ANDROID_SDK_ROOT)
  - Android NDK (设置 ANDROID_NDK_HOME 或在 SDK 中安装)
  - JDK 17+
#>

param(
    [switch]$Release,
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

# 添加 Android targets
$targets = @(
    "aarch64-linux-android",      # arm64-v8a
    "armv7-linux-androideabi",    # armeabi-v7a
    "x86_64-linux-android"        # x86_64
)

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
# 编译 Rust 代码
# ============================================================================

if (-not $SkipRust) {
    Write-Host "`n[3/5] Building Rust libraries with cargo-ndk..." -ForegroundColor Yellow

    $buildType = if ($Release) { "--release" } else { "" }
    $targetDir = if ($Release) { "release" } else { "debug" }

    # 使用 cargo-ndk 编译
    # -t 指定目标架构，-o 指定输出目录
    $jniLibsDir = Join-Path $PSScriptRoot "android\app\src\main\jniLibs"

    Write-Host "  Compiling for arm64-v8a..." -ForegroundColor Gray
    cargo ndk -t arm64-v8a -o $jniLibsDir build --no-default-features --features android $buildType
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  Error: arm64-v8a build failed" -ForegroundColor Red
        exit 1
    }

    Write-Host "  Compiling for armeabi-v7a..." -ForegroundColor Gray
    cargo ndk -t armeabi-v7a -o $jniLibsDir build --no-default-features --features android $buildType
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  Error: armeabi-v7a build failed" -ForegroundColor Red
        exit 1
    }

    Write-Host "  Compiling for x86_64..." -ForegroundColor Gray
    cargo ndk -t x86_64 -o $jniLibsDir build --no-default-features --features android $buildType
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  Error: x86_64 build failed" -ForegroundColor Red
        exit 1
    }

    Write-Host "  Rust libraries built successfully!" -ForegroundColor Green

    # 复制 libc++_shared.so (cpal/oboe C++ 运行时依赖)
    Write-Host "`n  Copying libc++_shared.so from NDK..." -ForegroundColor Gray
    $sysroot = "$ndkHome\toolchains\llvm\prebuilt\windows-x86_64\sysroot\usr\lib"
    
    $libcppMappings = @{
        "arm64-v8a" = "aarch64-linux-android"
        "armeabi-v7a" = "arm-linux-androideabi"
        "x86_64" = "x86_64-linux-android"
    }
    
    foreach ($arch in $libcppMappings.Keys) {
        $srcLib = "$sysroot\$($libcppMappings[$arch])\libc++_shared.so"
        $dstDir = "$jniLibsDir\$arch"
        if (Test-Path $srcLib) {
            Copy-Item $srcLib $dstDir -Force
            Write-Host "    Copied libc++_shared.so to $arch" -ForegroundColor Gray
        }
    }

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

$androidDir = Join-Path $PSScriptRoot "android"
$gradleTask = if ($Release) { "assembleRelease" } else { "assembleDebug" }

Push-Location $androidDir
try {
    # 使用 gradlew 或直接调用 gradle
    $gradlew = Join-Path $androidDir "gradlew.bat"
    
    # 如果没有 gradle-wrapper.jar，尝试使用系统 gradle
    $wrapperJar = Join-Path $androidDir "gradle\wrapper\gradle-wrapper.jar"
    if (-not (Test-Path $wrapperJar)) {
        Write-Host "  Note: gradle-wrapper.jar not found." -ForegroundColor Yellow
        Write-Host "  Please run 'gradle wrapper' in android/ directory first," -ForegroundColor Yellow
        Write-Host "  or open the project in Android Studio to generate it." -ForegroundColor Yellow
        
        # 尝试使用系统 gradle
        $gradle = Get-Command gradle -ErrorAction SilentlyContinue
        if ($gradle) {
            Write-Host "  Using system gradle: $($gradle.Path)" -ForegroundColor Gray
            & gradle $gradleTask
        }
        else {
            Write-Host "  Error: Neither gradle wrapper nor system gradle found." -ForegroundColor Red
            Write-Host "  Please install Gradle or open the project in Android Studio." -ForegroundColor Red
            exit 1
        }
    }
    else {
        & $gradlew $gradleTask
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Host "  Error: Gradle build failed" -ForegroundColor Red
        exit 1
    }
}
finally {
    Pop-Location
}

# ============================================================================
# 输出结果
# ============================================================================

Write-Host "`n[5/5] Build completed!" -ForegroundColor Green

$apkDir = if ($Release) {
    Join-Path $androidDir "app\build\outputs\apk\release"
} else {
    Join-Path $androidDir "app\build\outputs\apk\debug"
}

$apkFile = if ($Release) { "app-release.apk" } else { "app-debug.apk" }
$apkPath = Join-Path $apkDir $apkFile

if (Test-Path $apkPath) {
    $fileInfo = Get-Item $apkPath
    $sizeMB = [math]::Round($fileInfo.Length / 1MB, 2)
    
    Write-Host "`n========================================" -ForegroundColor Green
    Write-Host "  APK built successfully!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "  Output: $apkPath"
    Write-Host "  Size: $sizeMB MB"
    Write-Host ""
    Write-Host "  To install on device:" -ForegroundColor Cyan
    Write-Host "    adb install -r `"$apkPath`"" -ForegroundColor Gray
}
else {
    Write-Host "  Warning: APK file not found at expected location" -ForegroundColor Yellow
    Write-Host "  Check android/app/build/outputs/apk/ for APK files" -ForegroundColor Yellow
}
