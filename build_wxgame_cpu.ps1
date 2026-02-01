# ============================================================================
# WeChat Mini Game Build Script - CPU Software Rendering
# ============================================================================

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Mario RS - WeChat Mini Game Build" -ForegroundColor Cyan
Write-Host "  CPU Software Rendering Version" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# Check tools
Write-Host ""
Write-Host "[1/6] Checking build tools..." -ForegroundColor Yellow

if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) {
    Write-Host "Installing wasm-bindgen-cli..." -ForegroundColor Yellow
    cargo install wasm-bindgen-cli
}

Write-Host "Tools check complete" -ForegroundColor Green

# Clean old build
Write-Host ""
Write-Host "[2/6] Cleaning old build..." -ForegroundColor Yellow
$outDir = "dist\wxgame_cpu"
if (Test-Path $outDir) {
    Remove-Item -Recurse -Force $outDir
}
New-Item -ItemType Directory -Path $outDir | Out-Null
Write-Host "Clean complete" -ForegroundColor Green

# Build WASM
Write-Host ""
Write-Host "[3/6] Building WASM (wxgame-cpu-backend + SIMD)..." -ForegroundColor Yellow

# 启用 WASM SIMD 和 bulk-memory 特性
$env:RUSTFLAGS = "--cfg=web_sys_unstable_apis -C target-feature=+simd128,+bulk-memory"
cargo build --target wasm32-unknown-unknown --release --lib --features "wxgame-cpu-backend"

if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

Write-Host "Build complete (SIMD enabled)" -ForegroundColor Green

# Generate JS bindings
Write-Host ""
Write-Host "[4/6] Generating JS bindings (no-modules) ..." -ForegroundColor Yellow

wasm-bindgen target/wasm32-unknown-unknown/release/mario.wasm --out-dir $outDir --target no-modules --no-typescript

if ($LASTEXITCODE -ne 0) {
    Write-Host "wasm-bindgen failed!" -ForegroundColor Red
    exit 1
}

# Rename files
if (Test-Path "$outDir/mario.js") {
    Move-Item "$outDir/mario.js" "$outDir/mario_wxgame_cpu.js" -Force
}
if (Test-Path "$outDir/mario_bg.wasm") {
    Move-Item "$outDir/mario_bg.wasm" "$outDir/mario_wxgame_cpu_bg.wasm" -Force
}

Write-Host "JS bindings generated" -ForegroundColor Green

# Post-process JS file
Write-Host ""
Write-Host "[5/6] Post-processing JS file (WeChat adaptation)..." -ForegroundColor Yellow

$jsFile = "$outDir/mario_wxgame_cpu.js"
$jsContent = Get-Content $jsFile -Raw -Encoding UTF8

# Read polyfill template
$polyfill = Get-Content "build_wxgame_cpu/polyfill.js" -Raw -Encoding UTF8

# FIX 1: Remove document-related code
$jsContent = $jsContent -replace "let script_src;\s*if \(typeof document[^}]+\{[^}]+\}", 'let script_src = "mario_wxgame_cpu.js"'

# FIX 2: Fix detached ArrayBuffer check for WeChat Mini Game
$jsContent = $jsContent -replace `
    'cachedDataViewMemory0 === null \|\| cachedDataViewMemory0\.buffer\.detached === true \|\| \(cachedDataViewMemory0\.buffer\.detached === undefined && cachedDataViewMemory0\.buffer !== wasm\.memory\.buffer\)', `
    'cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer !== wasm.memory.buffer'

$jsContent = $jsContent -replace `
    'cachedUint8ArrayMemory0 === null \|\| cachedUint8ArrayMemory0\.byteLength === 0', `
    'cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.buffer !== wasm.memory.buffer'

$jsContent = $jsContent -replace `
    'cachedFloat32ArrayMemory0 === null \|\| cachedFloat32ArrayMemory0\.byteLength === 0', `
    'cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.buffer !== wasm.memory.buffer'

# FIX 3: Export wasm_bindgen to global scope
$globalExport = @"

// Export wasm_bindgen to global scope for WeChat Mini Game
if (typeof GameGlobal !== 'undefined') {
    GameGlobal.wasm_bindgen = wasm_bindgen;
}
"@

# Combine: polyfill + patched content + global export
$finalContent = $polyfill + $jsContent + $globalExport
$finalContent | Set-Content $jsFile -Encoding UTF8

Write-Host "JS post-processing complete" -ForegroundColor Green

# Create entry files
Write-Host ""
Write-Host "[6/6] Creating entry files..." -ForegroundColor Yellow

Copy-Item "build_wxgame_cpu/game.js" "$outDir/game.js" -Force
Copy-Item "build_wxgame_cpu/game.json" "$outDir/game.json" -Force
Copy-Item "build_wxgame_cpu/project.config.json" "$outDir/project.config.json" -Force

Write-Host "Entry files created" -ForegroundColor Green

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "  Build Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Output directory: $outDir" -ForegroundColor Cyan
Write-Host ""
Write-Host "Files:" -ForegroundColor Cyan
Get-ChildItem $outDir -File | ForEach-Object { Write-Host "  - $($_.Name)" }
Write-Host ""
Write-Host "Usage:" -ForegroundColor Yellow
Write-Host "  1. Import $outDir directory in WeChat DevTools" -ForegroundColor White
Write-Host "  2. Update appid in project.config.json" -ForegroundColor White
Write-Host "  3. Compile and run" -ForegroundColor White
