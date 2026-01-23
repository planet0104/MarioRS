@echo off
REM ============================================================================
REM  MarioRS 精灵转换脚本
REM  将 Pascal 汇编格式的精灵数据转换为索引 PNG 格式
REM ============================================================================
REM
REM  使用方法:
REM    1. 双击运行此脚本
REM    2. 或在命令行执行: convert_sprites.cmd
REM
REM  此脚本会:
REM    - 读取 assets/sprites_pascal/ 目录中的 Pascal DB 格式文件
REM    - 执行 Mode X 去平面化转换
REM    - 输出到 assets/sprites_indexed/ 目录
REM
REM  转换后的文件:
REM    - 精灵: BROWN_000.png, PIPE_001.png 等 (灰度 PNG，像素值=调色板索引)
REM    - 背景: BOGEN.png, MOUNT.png 等 (灰度 PNG，像素值=高度值)
REM    - 调色板: MPAL256.png (256x1 RGB PNG)
REM
REM ============================================================================

echo.
echo ========================================
echo   MarioRS 精灵转换工具
echo ========================================
echo.

REM 检查是否在项目根目录
if not exist "Cargo.toml" (
    echo 错误: 请在项目根目录运行此脚本
    echo 当前目录: %CD%
    pause
    exit /b 1
)

REM 运行转换工具
echo 正在编译并运行转换工具...
echo.

cargo run --example convert_to_indexed_png

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo 错误: 转换失败!
    pause
    exit /b 1
)

echo.
echo ========================================
echo   转换完成!
echo ========================================
echo.
echo 现在可以运行: cargo build
echo.

pause
