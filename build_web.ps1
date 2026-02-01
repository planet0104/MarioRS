# Mario RS - Web 平台自动化构建脚本 (Windows PowerShell)
# 用途: 编译 Rust -> WASM，生成 JS 绑定，优化输出，准备部署

# 严格模式

param(
    [switch]$Debug,
    [switch]$Serve,
    [switch]$NoOptimize,
    [switch]$Clean,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

# 颜色输出函数
function Write-Info {
    param([string]$Message)
    Write-Host "ℹ️  $Message" -ForegroundColor Blue
}

function Write-Success {
    param([string]$Message)
    Write-Host "✅ $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "⚠️  $Message" -ForegroundColor Yellow
}

function Write-Error-Custom {
    param([string]$Message)
    Write-Host "❌ $Message" -ForegroundColor Red
}

# 检查必要的工具
function Test-Tools {
    Write-Info "Checking necessary tools..."
    
    # 检查 cargo
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error-Custom "cargo is not installed. Please install Rust first."
        Write-Host "Visit: https://rustup.rs/"
        exit 1
    }
    
    # 检查 wasm-bindgen
    if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) {
        Write-Error-Custom "wasm-bindgen is not installed"
        Write-Host "Please run: cargo install wasm-bindgen-cli"
        exit 1
    }
    
    # 检查 WASM 目标
    $targets = rustup target list --installed
    if ($targets -notcontains "wasm32-unknown-unknown") {
        Write-Warning "WASM target is not installed, installing..."
        rustup target add wasm32-unknown-unknown
    }
    
    Write-Success "All necessary tools are ready"
}

# 清理之前的构建
function Clear-Build {
    Write-Info "Cleaning previous builds..."
    
    if (Test-Path "pkg") {
        Remove-Item -Recurse -Force "pkg"
    }
    
    if (Test-Path "dist\web") {
        Remove-Item -Recurse -Force "dist\web"
    }
    
    # 清理 WASM 目标构建
    try {
        cargo clean --target wasm32-unknown-unknown 2>$null
    } catch {
        # 忽略错误
    }
    
    Write-Success "Cleaning completed"
}

# 构建 WASM
function Build-Wasm {
    param([string]$Mode)
    
    Write-Info "Building WASM ($Mode mode)..."
    
    # 设置 RUSTFLAGS 启用 WebGPU 不稳定 API
    # WebGPU 在 web-sys 中是不稳定的，需要此标志
    $env:RUSTFLAGS = "--cfg=web_sys_unstable_apis"
    
    if ($Mode -eq "release") {
        cargo build --release --target wasm32-unknown-unknown --features wgpu-backend --lib
        $script:WasmPath = "target\wasm32-unknown-unknown\release\mario.wasm"
    } else {
        cargo build --target wasm32-unknown-unknown --features wgpu-backend --lib
        $script:WasmPath = "target\wasm32-unknown-unknown\debug\mario.wasm"
    }
    
    # 清除 RUSTFLAGS
    $env:RUSTFLAGS = $null
    
    # 检查 WASM 文件是否存在
    if (-not (Test-Path $script:WasmPath)) {
        Write-Error-Custom "WASM build failed: $script:WasmPath does not exist"
        exit 1
    }
    
    # 显示文件大小
    $size = (Get-Item $script:WasmPath).Length
    $sizeMB = [math]::Round($size / 1MB, 2)
    Write-Success "WASM build completed (size: $sizeMB MB)"
}

# 生成 JS 绑定
function New-Bindings {
    param([string]$Mode)
    
    Write-Info "Generating JavaScript bindings..."
    
    $debugFlag = ""
    if ($Mode -eq "debug") {
        $debugFlag = "--debug"
    }
    
    # 构建参数
    $args = @(
        $debugFlag,
        "--target", "web",
        "--out-dir", "pkg",
        "--out-name", "mario_rs",
        $script:WasmPath
    ) | Where-Object { $_ -ne "" }
    
    & wasm-bindgen @args
    
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Custom "Generating JS bindings failed"
        exit 1
    }
    
    Write-Success "JS bindings generated successfully"
}

# Optimize WASM
function Optimize-Wasm {
    Write-Info "Optimizing WASM..."
    
    # Check if wasm-opt is available
    if (Get-Command wasm-opt -ErrorAction SilentlyContinue) {
        $originalSize = (Get-Item "pkg\mario_rs_bg.wasm").Length
        $originalSizeMB = [math]::Round($originalSize / 1MB, 2)
        
        wasm-opt -Oz `
            --enable-bulk-memory `
            --enable-mutable-globals `
            --enable-reference-types `
            --enable-sign-ext `
            -o "pkg\mario_rs_bg_opt.wasm" `
            "pkg\mario_rs_bg.wasm"
        
        if ($LASTEXITCODE -eq 0) {
            Move-Item -Force "pkg\mario_rs_bg_opt.wasm" "pkg\mario_rs_bg.wasm"
            
            $optimizedSize = (Get-Item "pkg\mario_rs_bg.wasm").Length
            $optimizedSizeMB = [math]::Round($optimizedSize / 1MB, 2)
            Write-Success "WASM optimization completed ($originalSizeMB MB -> $optimizedSizeMB MB)"
        } else {
            Write-Warning "WASM optimization failed, continuing with unoptimized version"
        }
    } else {
        Write-Warning "wasm-opt is not installed, skipping optimization"
        Write-Host "  Installation methods:"
        Write-Host "    Windows: choco install binaryen"
        Write-Host "    Or download from https://github.com/WebAssembly/binaryen/releases"
    }
}

# 准备部署文件
function New-Deployment {
    Write-Info "Preparing deployment files..."
    
    # 创建 dist/web 目录
    New-Item -ItemType Directory -Force -Path "dist\web" | Out-Null
    
    # 复制 pkg 文件夹到 dist/web
    Copy-Item -Recurse -Force "pkg" "dist\web\"
    
    # 复制 HTML 文件到 dist/web
    if (Test-Path "index.html") {
        Copy-Item "index.html" "dist\web\"
    } else {
        Write-Warning "index.html does not exist, skipping copy"
    }
    
    # 复制 assets/icon（如果存在）
    if (Test-Path "assets\icon.png") {
        Write-Info "Copying icon..."
        New-Item -ItemType Directory -Force -Path "dist\web\assets" | Out-Null
        Copy-Item "assets\icon.png" "dist\web\assets\"
    } elseif (Test-Path "assets") {
        Write-Info "Copying icon files..."
        New-Item -ItemType Directory -Force -Path "dist\web\assets" | Out-Null
        # Only copy icon-related files
        Get-ChildItem "assets" -File | Where-Object { $_.Name -like "*icon*" } | ForEach-Object {
            Copy-Item $_.FullName "dist\web\assets\"
        }
    }
    
    # 创建 .gitignore（如果不存在）
    if (-not (Test-Path "dist\web\.gitignore")) {
        Write-Info "Creating dist\web\.gitignore"
        @"
# 构建输出被 git 忽略，但在部署时需要
!pkg/
!*.wasm
!*.js
"@ | Out-File -FilePath "dist\web\.gitignore" -Encoding UTF8
    }
    
    Write-Success "Deployment files prepared (located in dist\web)"
}

# Generate deployment report
function Show-Report {
    Write-Info "Generating build report..."
    
    Write-Host ""
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
    Write-Host "📊 Build Report" -ForegroundColor Cyan
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
    
    # WASM file size
    if (Test-Path "dist\web\pkg\mario_rs_bg.wasm") {
        $wasmSize = (Get-Item "dist\web\pkg\mario_rs_bg.wasm").Length
        $wasmSizeMB = [math]::Round($wasmSize / 1MB, 2)
        Write-Host "WASM size:     $wasmSizeMB MB"
    }
    
    # JS file size
    if (Test-Path "dist\web\pkg\mario_rs.js") {
        $jsSize = (Get-Item "dist\web\pkg\mario_rs.js").Length
        $jsSizeKB = [math]::Round($jsSize / 1KB, 2)
        Write-Host "JS size:       $jsSizeKB KB"
    }
    
    # Total size
    $totalSize = (Get-ChildItem -Recurse "dist\web" | Measure-Object -Property Length -Sum).Sum
    $totalSizeMB = [math]::Round($totalSize / 1MB, 2)
    Write-Host "Total size:    $totalSizeMB MB"
    Write-Host ""
    
    # File list
    Write-Host "📁 Output files:" -ForegroundColor Cyan
    Get-ChildItem -Recurse "dist\web" | Where-Object { -not $_.PSIsContainer } | ForEach-Object {
        $relativePath = $_.FullName.Replace((Get-Location).Path + "\dist\web\", "")
        Write-Host "  $relativePath"
    }
    
    Write-Host ""
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
    Write-Success "Build succeeded!"
    Write-Host ""
    Write-Host "🚀 Local testing:" -ForegroundColor Yellow
    Write-Host "   Method 1 (Python):"
    Write-Host "     cd dist\web"
    Write-Host "     python -m http.server 8080"
    Write-Host ""
    Write-Host "   Method 2 (Node.js):"
    Write-Host "     npm install -g http-server"
    Write-Host "     cd dist\web"
    Write-Host "     http-server"
    Write-Host ""
    Write-Host "   Method 3 (basic-http-server):"
    Write-Host "     cargo install basic-http-server"
    Write-Host "     cd dist\web"
    Write-Host "     basic-http-server"
    Write-Host ""
    Write-Host "🌐 Browser access:" -ForegroundColor Yellow
    Write-Host "   http://localhost:8080"
    Write-Host ""
}

# 启动本地服务器（可选）
function Start-LocalServer {
    Write-Info "Starting local Web server..."
    
    Set-Location "dist\web"
    
    # 尝试 basic-http-server
    if (Get-Command basic-http-server -ErrorAction SilentlyContinue) {
        Write-Host ""
        Write-Success "Server running at http://localhost:4000"
        Write-Host "Press Ctrl+C to stop the server"
        Write-Host ""
        basic-http-server
    }
    # 尝试 Python
    elseif (Get-Command python -ErrorAction SilentlyContinue) {
        Write-Host ""
        Write-Success "Server running at http://localhost:8080"
        Write-Host "Press Ctrl+C to stop the server"
        Write-Host ""
        python -m http.server 8080
    }
    # 尝试 http-server (Node.js)
    elseif (Get-Command http-server -ErrorAction SilentlyContinue) {
        Write-Host ""
        Write-Success "Server running at http://localhost:8080"
        Write-Host "Press Ctrl+C to stop the server"
        Write-Host ""
        http-server
    }
    else {
        Write-Warning "Web server not found"
        Write-Host "Please start the server manually or install:"
        Write-Host "  cargo install basic-http-server"
        Write-Host "  or"
        Write-Host "  npm install -g http-server"
    }
}

# 显示帮助信息
function Show-Help {
    Write-Host "Usage: .\build-web.ps1 [options]"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Debug          Build in debug mode"
    Write-Host "  -Serve          Start local server after build"
    Write-Host "  -NoOptimize     Skip WASM optimization"
    Write-Host "  -Clean          Clean build files only"
    Write-Host "  -Help           Show this help information"
    Write-Host ""
    Write-Host "Examples:"
    Write-Host "  .\build-web.ps1                    # Build in release mode"
    Write-Host "  .\build-web.ps1 -Debug             # Build in debug mode"
    Write-Host "  .\build-web.ps1 -Serve             # Build and start server"
    Write-Host "  .\build-web.ps1 -Debug -NoOptimize # Build in debug mode, skip optimization"
    Write-Host "  .\build-web.ps1 -Clean             # Clean build files"
}

# 主函数
function Main {
    param(
        [switch]$Debug,
        [switch]$Serve,
        [switch]$NoOptimize,
        [switch]$Clean,
        [switch]$Help
    )
    
    # 显示帮助
    if ($Help) {
        Show-Help
        return
    }
    
    # 只清理
    if ($Clean) {
        Clear-Build
        return
    }
    
    # 确定构建模式
    $mode = if ($Debug) { "debug" } else { "release" }
    
    Write-Host ""
    Write-Host "🍄 Mario RS - Web Build Script (Windows) 🍄" -ForegroundColor Magenta
    Write-Host ""
    
    # Execute build steps
    try {
        Test-Tools
        Build-Wasm -Mode $mode
        New-Bindings -Mode $mode
        
        if (($mode -eq "release") -and (-not $NoOptimize)) {
            Optimize-Wasm
        }
        
        New-Deployment
        Show-Report
        
        # 如果指定了 -Serve，启动服务器
        if ($Serve) {
            Start-LocalServer
        }
    }
    catch {
        Write-Error-Custom "Build failed: $_"
        exit 1
    }
}

# Parse command-line arguments and run main function
Main -Debug:$Debug -Serve:$Serve -NoOptimize:$NoOptimize -Clean:$Clean -Help:$Help