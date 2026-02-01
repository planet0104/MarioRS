# Windows 平台构建指南

## 🪟 Windows 构建脚本说明

为了方便 Windows 用户，我们提供了两种构建脚本：

### 1. PowerShell 脚本 (推荐)
**文件**: `build-web.ps1`

**特点**:
- ✅ 功能完整，彩色输出
- ✅ 更好的错误处理
- ✅ 支持更多参数
- ✅ 现代化的 Windows 脚本

**使用方法**:
```powershell
# Release 模式构建
.\build-web.ps1

# Debug 模式构建
.\build-web.ps1 -Debug

# 构建并启动服务器
.\build-web.ps1 -Serve

# Debug 模式，不优化
.\build-web.ps1 -Debug -NoOptimize

# 清理构建文件
.\build-web.ps1 -Clean

# 查看帮助
.\build-web.ps1 -Help
```

### 2. 批处理脚本 (兼容性好)
**文件**: `build-web.bat`

**特点**:
- ✅ 兼容所有 Windows 版本
- ✅ 无需 PowerShell
- ✅ 简单易用

**使用方法**:
```cmd
REM Release 模式构建
build-web.bat

REM Debug 模式构建
build-web.bat --debug

REM 构建并启动服务器
build-web.bat --serve

REM Debug 模式，不优化
build-web.bat --debug --no-optimize

REM 清理构建文件
build-web.bat --clean

REM 查看帮助
build-web.bat --help
```

## 📋 前置要求

### 1. 安装 Rust

访问 [https://rustup.rs/](https://rustup.rs/) 下载并安装 Rust

或使用以下命令（PowerShell）:
```powershell
# 下载并运行 Rust 安装程序
Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile "rustup-init.exe"
.\rustup-init.exe
```

### 2. 添加 WASM 目标

```cmd
rustup target add wasm32-unknown-unknown
```

### 3. 安装 wasm-bindgen

```cmd
cargo install wasm-bindgen-cli
```

### 4. 安装 wasm-opt (可选，用于优化)

**方法 1: 使用 Chocolatey**
```powershell
# 如果还没有安装 Chocolatey
Set-ExecutionPolicy Bypass -Scope Process -Force
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))

# 安装 binaryen
choco install binaryen
```

**方法 2: 手动下载**
1. 访问 [https://github.com/WebAssembly/binaryen/releases](https://github.com/WebAssembly/binaryen/releases)
2. 下载最新版本的 Windows 压缩包
3. 解压到任意目录（如 `C:\binaryen`）
4. 将 `bin` 目录添加到系统 PATH 环境变量

**添加到 PATH**:
```powershell
# PowerShell (以管理员身份运行)
$env:Path += ";C:\binaryen\bin"
[Environment]::SetEnvironmentVariable("Path", $env:Path, [System.EnvironmentVariableTarget]::Machine)
```

或者手动添加：
1. 右键"此电脑" → "属性"
2. "高级系统设置" → "环境变量"
3. 在"系统变量"中找到 `Path`，点击"编辑"
4. 添加 `C:\binaryen\bin`

### 5. 安装本地 Web 服务器 (可选)

**方法 1: Python (推荐)**
```powershell
# 检查是否已安装
python --version

# 如果没有安装，从 https://www.python.org/downloads/ 下载安装
```

**方法 2: basic-http-server (Rust)**
```cmd
cargo install basic-http-server
```

**方法 3: http-server (Node.js)**
```cmd
# 需要先安装 Node.js: https://nodejs.org/
npm install -g http-server
```

## 🚀 快速开始

### PowerShell 执行策略

如果 PowerShell 脚本无法运行，可能需要修改执行策略：

```powershell
# 查看当前执行策略
Get-ExecutionPolicy

# 临时允许（仅当前会话）
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass

# 永久允许（需要管理员权限）
Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
```

### 基本构建流程

**使用 PowerShell**:
```powershell
# 1. 打开 PowerShell
# 2. 进入项目目录
cd C:\path\to\mario-rs

# 3. 运行构建脚本
.\build-web.ps1

# 4. 进入 web 目录
cd web

# 5. 启动服务器
python -m http.server 8080

# 6. 浏览器访问
# http://localhost:8080
```

**使用批处理**:
```cmd
REM 1. 打开命令提示符
REM 2. 进入项目目录
cd C:\path\to\mario-rs

REM 3. 运行构建脚本
build-web.bat

REM 4. 进入 web 目录
cd web

REM 5. 启动服务器
python -m http.server 8080

REM 6. 浏览器访问
REM http://localhost:8080
```

### 一键构建并运行

**PowerShell**:
```powershell
.\build-web.ps1 -Serve
```

**批处理**:
```cmd
build-web.bat --serve
```

## 🐛 常见问题

### 1. PowerShell 脚本无法运行

**错误**: "无法加载文件，因为在此系统上禁止运行脚本"

**解决方案**:
```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
```

### 2. cargo 命令未找到

**错误**: "'cargo' 不是内部或外部命令"

**解决方案**:
1. 重新打开命令行窗口
2. 或手动添加到 PATH: `%USERPROFILE%\.cargo\bin`

### 3. wasm-bindgen 版本不匹配

**错误**: "wasm-bindgen version mismatch"

**解决方案**:
```cmd
# 卸载旧版本
cargo uninstall wasm-bindgen-cli

# 安装匹配的版本（与 Cargo.toml 中的版本一致）
cargo install wasm-bindgen-cli --version 0.2.x
```

### 4. WASM 文件过大

**解决方案**:
1. 确保使用 release 模式
2. 安装并启用 wasm-opt
3. 检查 Cargo.toml 优化配置

### 5. 浏览器无法访问本地服务器

**问题**: http://localhost:8080 无法访问

**解决方案**:
1. 检查端口是否被占用
2. 尝试其他端口: `python -m http.server 8000`
3. 检查防火墙设置
4. 使用 `http://127.0.0.1:8080` 代替 localhost

### 6. WebGPU 不可用

**问题**: 浏览器显示 "WebGPU not supported"

**解决方案**:
1. 使用 Chrome 113+ 或 Edge 113+
2. 访问 `chrome://flags`
3. 搜索 "WebGPU"
4. 启用 "Unsafe WebGPU"
5. 重启浏览器

## 📊 性能优化建议

### 1. 使用 Release 模式
```powershell
.\build-web.ps1  # 默认 release 模式
```

### 2. 启用 wasm-opt
确保已安装 binaryen，构建脚本会自动优化

### 3. 检查构建输出大小
```powershell
# 查看 WASM 文件大小
Get-ChildItem web\pkg\mario_rs_bg.wasm | Select-Object Name, Length
```

### 4. 使用压缩传输
在生产环境中启用 gzip/brotli 压缩

## 🔧 高级用法

### 自定义构建配置

**修改 Cargo.toml**:
```toml
[profile.release]
opt-level = "z"      # 体积优化
lto = true           # 链接时优化
codegen-units = 1    # 更好的优化
strip = true         # 移除调试符号
panic = "abort"      # 减小体积
```

### 环境变量配置

```powershell
# 设置 Rust 编译器标志
$env:RUSTFLAGS = "-C target-feature=+atomics,+bulk-memory"

# 启用详细输出
$env:RUST_LOG = "debug"

# 然后构建
.\build-web.ps1
```

### 批量构建（Debug + Release）

创建 `build-all.ps1`:
```powershell
Write-Host "构建 Debug 版本..." -ForegroundColor Cyan
.\build-web.ps1 -Debug -NoOptimize
Rename-Item "web" "web-debug"

Write-Host "构建 Release 版本..." -ForegroundColor Cyan
.\build-web.ps1
Rename-Item "web" "web-release"

Write-Host "构建完成！" -ForegroundColor Green
```

## 🌐 部署到生产环境

### 1. 构建优化版本
```powershell
.\build-web.ps1  # Release 模式自动优化
```

### 2. 检查输出
```powershell
tree /F web
```

### 3. 压缩文件（可选）
```powershell
# 创建 ZIP 压缩包
Compress-Archive -Path web\* -DestinationPath mario-rs-web.zip
```

### 4. 上传到服务器
使用 FTP、SCP 或 Git 上传 `web` 目录

### 5. 配置 Web 服务器
确保服务器正确设置 MIME 类型：
- `.wasm` → `application/wasm`
- `.js` → `application/javascript`

## 📚 参考资源

- [Rust 官方文档](https://www.rust-lang.org/zh-CN/)
- [wasm-bindgen 文档](https://rustwasm.github.io/docs/wasm-bindgen/)
- [WebGPU 规范](https://www.w3.org/TR/webgpu/)
- [Binaryen (wasm-opt)](https://github.com/WebAssembly/binaryen)

## 💡 提示

- 推荐使用 **Windows Terminal** 获得更好的命令行体验
- 使用 **VSCode** 编辑代码，安装 rust-analyzer 扩展
- 定期更新 Rust 工具链: `rustup update`
- 使用 **Git Bash** 可以运行 Linux 风格的 shell 脚本

## 🆘 获取帮助

如遇到问题：
1. 查看脚本输出的错误信息
2. 检查 Windows 事件查看器
3. 查阅 Rust 官方文档
4. 搜索相关错误信息

---

**祝您构建顺利！🎮✨**