# Windows 7/XP 兼容版本编译脚本
# 使用 YY-Thunks 提供 API 兼容层，解决 api-ms-win-core-synch-l1-2-0.dll 等依赖问题
# 禁用 dark-theme feature 以移除 dwmapi.dll 依赖
#
# 适用系统：
# - Windows XP SP3 (x86)
# - Windows XP x64 Edition / Server 2003 (x64)
# - Windows Vista / 7 / 8 / 8.1 / 10 / 11
#
# 功能：
# 1. 链接 YY-Thunks 提供旧版 Windows 缺失的 API 实现
# 2. 移除暗黑主题适配功能（需要 Win10 20H1+ dwmapi.dll 新 API）
# 3. 设置正确的子系统版本
#
# 注意：默认版本（不使用此脚本）仅支持 Windows 10+

param(
    [ValidateSet("x86", "x64")]
    [string]$Arch = "x64"
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " MarioRS Windows 7/XP Build Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# YY-Thunks 配置
$yyThunksVersion = "1.1.9"
$yyThunksUrl = "https://github.com/Chuyu-Team/YY-Thunks/releases/download/v$yyThunksVersion/YY-Thunks-Objs.zip"
$yyThunksZip = "vendor\yy-thunks.zip"
$yyThunksDir = "vendor\yy-thunks"
$yyThunksObjDir = "$yyThunksDir\objs"

# 检查 YY-Thunks 是否存在，不存在则自动下载
if (-not (Test-Path $yyThunksObjDir)) {
    Write-Host "YY-Thunks not found, downloading v$yyThunksVersion..." -ForegroundColor Yellow
    
    # 创建 vendor 目录
    if (-not (Test-Path "vendor")) {
        New-Item -ItemType Directory -Path "vendor" -Force | Out-Null
    }
    
    try {
        # 下载
        Write-Host "  Downloading from GitHub..." -ForegroundColor Gray
        Invoke-WebRequest -Uri $yyThunksUrl -OutFile $yyThunksZip -UseBasicParsing
        
        # 解压
        Write-Host "  Extracting..." -ForegroundColor Gray
        Expand-Archive -Path $yyThunksZip -DestinationPath $yyThunksDir -Force
        
        # 删除 zip 文件
        Remove-Item $yyThunksZip -Force -ErrorAction SilentlyContinue
        
        Write-Host "  YY-Thunks downloaded successfully!" -ForegroundColor Green
        Write-Host ""
    }
    catch {
        Write-Host "[ERROR] Failed to download YY-Thunks: $_" -ForegroundColor Red
        Write-Host "Please download manually:" -ForegroundColor Yellow
        Write-Host "  Invoke-WebRequest -Uri `"$yyThunksUrl`" -OutFile `"$yyThunksZip`"" -ForegroundColor Gray
        Write-Host "  Expand-Archive -Path `"$yyThunksZip`" -DestinationPath `"$yyThunksDir`" -Force" -ForegroundColor Gray
        exit 1
    }
}

# 根据架构选择目标和输出目录
if ($Arch -eq "x86") {
    $target = "i686-pc-windows-msvc"
    $archDir = "x86"
    $subsystem = "5.01"  # Windows XP (x86)
} else {
    $target = "x86_64-pc-windows-msvc"
    $archDir = "x64"
    $subsystem = "5.02"  # Windows XP x64 / Server 2003
}

$objPath = "$yyThunksObjDir\$archDir\YY_Thunks_for_WinXP.obj"
if (-not (Test-Path $objPath)) {
    Write-Host "[ERROR] YY-Thunks obj not found: $objPath" -ForegroundColor Red
    exit 1
}

Write-Host "Target: $target" -ForegroundColor Yellow
Write-Host "YY-Thunks: $objPath" -ForegroundColor Yellow
Write-Host "Features: gdi-backend (dark-theme disabled)" -ForegroundColor Yellow
Write-Host "Compatible: Windows XP SP3 / 7 / 8 / 10 / 11" -ForegroundColor Yellow
Write-Host ""

# 设置环境变量启用 YY-Thunks 链接
$env:MARIO_XP_COMPAT = "1"

# 编译
Write-Host "Building..." -ForegroundColor Cyan
cargo build --release --target $target --no-default-features --features gdi-backend

if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "[ERROR] Build failed!" -ForegroundColor Red
    exit 1
}

# 创建输出目录
$outDir = "dist\win7xp-compat"
if (-not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
}

# 复制文件
$exeSrc = "target\$target\release\mario.exe"
$exeDst = "$outDir\mario.exe"
Copy-Item $exeSrc $exeDst -Force
Copy-Item "mario.cfg" "$outDir\mario.cfg" -Force -ErrorAction SilentlyContinue

# 显示文件大小
$size = (Get-Item $exeDst).Length / 1KB
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host " Build Successful!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host "Output: $exeDst ($([math]::Round($size, 1)) KB)" -ForegroundColor Green
Write-Host ""

# 使用 dumpbin 检查 DLL 依赖（如果可用）
Write-Host "Checking DLL dependencies..." -ForegroundColor Cyan

# 尝试找到 dumpbin
$dumpbinPaths = @(
    "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe",
    "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe",
    "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe",
    "C:\Program Files (x86)\Microsoft Visual Studio\2019\*\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe"
)

$dumpbin = $null
foreach ($pattern in $dumpbinPaths) {
    $found = Get-ChildItem -Path $pattern -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found) {
        $dumpbin = $found.FullName
        break
    }
}

if ($dumpbin) {
    $deps = & $dumpbin /DEPENDENTS $exeDst 2>$null | 
            Select-String -Pattern '^\s+\S+\.dll$' |
            ForEach-Object { $_.ToString().Trim().ToLower() } |
            Sort-Object -Unique
    
    # Win7/XP 兼容的系统 DLL 列表
    $compatDlls = @(
        "kernel32.dll",
        "user32.dll", 
        "gdi32.dll",
        "advapi32.dll",
        "winmm.dll",
        "ntdll.dll",
        "imm32.dll",
        "shell32.dll",
        "ole32.dll",
        "oleaut32.dll",
        "ws2_32.dll"
    )
    
    Write-Host ""
    Write-Host "DLL Dependencies:" -ForegroundColor Yellow
    foreach ($dll in $deps) {
        if ($compatDlls -contains $dll) {
            Write-Host "  [OK] $dll" -ForegroundColor Green
        } elseif ($dll -match "^api-ms-win") {
            Write-Host "  [!]  $dll (handled by YY-Thunks)" -ForegroundColor Yellow
        } elseif ($dll -eq "dwmapi.dll") {
            Write-Host "  [X]  $dll (dark-theme should be disabled!)" -ForegroundColor Red
        } else {
            Write-Host "  [?]  $dll (unknown)" -ForegroundColor Gray
        }
    }
} else {
    # 回退到简单的字符串搜索
    Write-Host "(dumpbin not found, using simple string search)" -ForegroundColor Gray
    $bytes = [System.IO.File]::ReadAllBytes($exeDst)
    $text = [System.Text.Encoding]::ASCII.GetString($bytes)
    $dlls = [regex]::Matches($text, '(?i)[a-z0-9_-]+\.dll') | 
            ForEach-Object { $_.Value.ToLower() } | 
            Sort-Object -Unique |
            Where-Object { $_ -match '^(kernel32|user32|gdi32|advapi32|winmm|dwmapi|ntdll|api-ms-win|imm32)' }
    
    foreach ($dll in $dlls) {
        if ($dll -match 'dwmapi|api-ms-win') {
            Write-Host "  [!] $dll" -ForegroundColor Yellow
        } else {
            Write-Host "  [OK] $dll" -ForegroundColor Green
        }
    }
}

Write-Host ""
Write-Host "Supported: Windows XP SP3 / Vista / 7 / 8 / 8.1 / 10 / 11" -ForegroundColor Cyan

# 清理环境变量
Remove-Item Env:\MARIO_XP_COMPAT -ErrorAction SilentlyContinue
