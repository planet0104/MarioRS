# Windows 7/XP 兼容版本编译指南

## 概述

使用 `build_win7xp.ps1` 脚本可以编译出兼容 Windows 7 和 Windows XP 的可执行文件。

### 为什么需要这个脚本？

默认编译的版本依赖以下 Windows 8+ API：
- `api-ms-win-core-synch-l1-2-0.dll` - Windows 8+ 同步原语
- `bcryptprimitives.dll` - 加密原语

这些在 Windows 7 和 XP 上不存在或不完全兼容，会导致程序无法启动（错误 0xc0000005）。

### 该脚本的作用

1. **链接 YY-Thunks** - 提供旧版 Windows 缺失的 API 实现
2. **禁用 dark-theme** - 移除暗黑主题适配（需要 Win10 20H1+）
3. **设置子系统版本** - 指定最低支持的 Windows 版本

## 支持的系统

| 系统 | 32位 (x86) | 64位 (x64) |
|------|------------|------------|
| Windows XP SP3 | ✅ | - |
| Windows XP x64 / Server 2003 | - | ✅ |
| Windows Vista | ✅ | ✅ |
| Windows 7 | ✅ | ✅ |
| Windows 8 / 8.1 | ✅ | ✅ |
| Windows 10 / 11 | ✅ | ✅ |

## 前提条件

### 1. 安装 Rust 工具链

确保已安装 Rust 和对应的 MSVC 目标：

```powershell
# x64 目标（默认已安装）
rustup target add x86_64-pc-windows-msvc

# x86 目标（用于 32 位系统）
rustup target add i686-pc-windows-msvc
```

### 2. YY-Thunks（自动下载）

脚本会自动检测 `vendor/yy-thunks/` 目录，如果不存在会自动从 GitHub 下载。

**注意**：`vendor/` 目录已添加到 `.gitignore`，不会被提交到 Git 仓库。

如需手动下载（例如网络问题时）：

```powershell
# 下载 YY-Thunks
Invoke-WebRequest -Uri "https://github.com/Chuyu-Team/YY-Thunks/releases/download/v1.1.9/YY-Thunks-Objs.zip" -OutFile "vendor/yy-thunks.zip"

# 解压
Expand-Archive -Path "vendor/yy-thunks.zip" -DestinationPath "vendor/yy-thunks" -Force
```

## 使用方法

### 编译 64 位版本（默认）

适用于 Windows 7/8/10/11 (x64) 和 Windows XP x64：

```powershell
.\build_win7xp.ps1
```

### 编译 32 位版本

适用于 Windows XP SP3 / Vista / 7 (x86)：

```powershell
.\build_win7xp.ps1 -Arch x86
```

### 手动编译

也可以手动设置环境变量进行编译：

```powershell
# 设置环境变量启用 YY-Thunks
$env:MARIO_XP_COMPAT = "1"

# x64 编译
cargo build --release --target x86_64-pc-windows-msvc --no-default-features --features gdi-backend

# x86 编译
cargo build --release --target i686-pc-windows-msvc --no-default-features --features gdi-backend
```

## 输出文件

编译成功后，文件输出到：

```
dist/win7xp-compat/
  ├── mario.exe    # 可执行文件
  └── mario.cfg    # 配置文件
```

## DLL 依赖

编译后的 exe 仅依赖以下系统原生 DLL：

| DLL | 说明 | XP | Win7 |
|-----|------|----|----|
| kernel32.dll | Windows 核心 API | ✅ | ✅ |
| user32.dll | 用户界面 API | ✅ | ✅ |
| gdi32.dll | 图形设备接口 | ✅ | ✅ |
| advapi32.dll | 高级 API | ✅ | ✅ |
| winmm.dll | 多媒体 API | ✅ | ✅ |
| ntdll.dll | NT 内核接口 | ✅ | ✅ |
| imm32.dll | 输入法管理器 | ✅ | ✅ |

## 与默认版本的区别

| 特性 | 默认版本 | Win7/XP 兼容版本 |
|------|----------|------------------|
| 最低支持 | Windows 10 | Windows XP SP3 |
| 暗黑主题 | ✅ 自动适配 | ❌ 不支持 |
| 文件大小 | ~730 KB | ~665 KB |
| YY-Thunks | 不需要 | 需要 |

## 注意事项

1. **推荐 32 位**：大多数 XP/Win7 系统是 32 位的，建议使用 `-Arch x86`
2. **暗黑主题**：此版本不支持标题栏暗黑主题适配
3. **图标格式**：使用 BMP 格式图标（非 PNG），兼容 XP
4. **YY-Thunks 版本**：当前使用 v1.1.9

## 技术原理

### YY-Thunks

YY-Thunks 是一个 API 兼容层，为旧版 Windows 提供新版 API 的实现：

- `api-ms-win-core-synch-l1-2-0.dll` 中的同步原语
- `GetTickCount64`、`InitializeCriticalSectionEx` 等
- `bcryptprimitives.dll` 中的加密函数

### 子系统版本

- x86: `/SUBSYSTEM:WINDOWS,5.01` (Windows XP)
- x64: `/SUBSYSTEM:WINDOWS,5.02` (Windows XP x64 / Server 2003)

### 暗黑主题检测

默认版本使用 `DwmSetWindowAttribute` + `DWMWA_USE_IMMERSIVE_DARK_MODE` 实现暗黑主题适配，但该 API 仅在 Windows 10 Build 19041 (20H1) 及更高版本可用。兼容版本禁用此功能以避免兼容性问题。
