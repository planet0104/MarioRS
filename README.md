# MarioRS

将 Mike Wiering 的 Turbo Pascal 马里奥克隆游戏移植到 Rust。

Porting Mike Wiering's Turbo Pascal Mario clone game to Rust.

![Mario Game Screenshot](capture/mario.jpg)

> **原版官网 / Original Website**: [Wiering Software - Mario](https://wieringsoftware.nl/mario/) | [源码下载 / Source Download](https://wieringsoftware.nl/mario/source.html)

> **学习文档 / Learning Guide**: [wgpu游戏开发学习指南](docs/WGPU_LEARNING_GUIDE.md) - 从零开始学习本项目的GPU渲染实现

## 特性 / Features

- 完整移植原版 6 个关卡 / Full port of original 6 levels
- 双人轮流模式（Mario 和 Luigi）/ Two-player turn-based mode (Mario and Luigi)
- 多平台支持 / Cross-platform support：Windows / Linux / macOS / Android
- **双渲染后端架构 / Dual Rendering Backend Architecture**：
  - **wgpu GPU 渲染 / GPU Rendering**（默认 / Default）：跨平台硬件加速 / Cross-platform hardware acceleration，支持 Vulkan/Metal/DirectX 12
  - **CPU 软件渲染 / CPU Software Rendering**：纯软件渲染，兼容 Windows XP 等老旧系统 / Pure software rendering, compatible with legacy systems like Windows XP
- **手柄支持 / Gamepad Support**：Windows 和 Android 平台支持 USB/蓝牙手柄 / USB/Bluetooth gamepad support on Windows and Android
- **Android TV 遥控器支持 / Android TV Remote Support**：支持使用 Android TV 遥控器控制游戏 / Control game with Android TV remote
- **Android GPU 自动降级 / Android GPU Auto-Fallback**：GPU 初始化失败时自动切换到 CPU 渲染 / Automatic fallback to CPU rendering when GPU initialization fails
- Windows 7/XP 兼容版本（使用 CPU 后端 + YY-Thunks）/ Windows 7/XP compatible version (CPU backend + YY-Thunks)
- Android 原生支持（触摸屏虚拟按键）/ Android native support (touch screen virtual buttons)
- 窗口缩放和全屏支持 / Window scaling and fullscreen support
- 暗黑主题自动适配（Windows 10+）/ Dark theme auto-adaptation (Windows 10+)

## 游戏控制 / Game Controls

### 键盘控制 / Keyboard Controls

| 按键 / Key | 功能 / Function |
|------|------|
| `←` `→` | 左右移动 / Move left/right |
| `↑` | 进入管道 / Enter pipe |
| `↓` | 从管道出来 / Exit pipe |
| `Alt` | 跳跃（按住可控制跳跃高度）/ Jump (hold for higher jump) |
| `Ctrl` | 加速/跑步 / Sprint/Run |
| `空格 / Space` | 发射火球（火焰马里奥状态）/ Fire (Fire Mario) |

### 快捷键 / Hotkeys

| 按键 / Key | 功能 / Function |
|------|------|
| `P` | 暂停游戏 / Pause game |
| `S` | 切换状态栏显示 / Toggle status bar |
| `F11` | 切换全屏/窗口模式 / Toggle fullscreen |
| `ESC` | 退出全屏/退出游戏/返回上级菜单 / Exit fullscreen/game/back to menu |

### 手柄控制 / Gamepad Controls

支持 Windows 和 Android 平台的 USB/蓝牙手柄。手柄连接后自动检测。

Supports USB/Bluetooth gamepads on Windows and Android. Gamepad is auto-detected when connected.

| 按键 / Button | 功能 / Function |
|------|------|
| 左摇杆/D-Pad | 方向移动 / Direction movement |
| A / B | 跳跃 / Jump |
| LB / RB | 加速/跑步 / Sprint/Run |
| X | 发射火球 / Fire |
| START | 开始/暂停 / Start/Pause |
| SELECT | 返回 / Back |

**手柄按钮映射 / Gamepad Button Mapping:**
- **跳跃 / Jump**: A 或 B 按钮 / A or B button
- **加速 / Sprint**: LB 或 RB 肩键 / LB or RB shoulder button
- **发射 / Fire**: X 按钮 / X button

### Android TV 遥控器 / Android TV Remote

支持使用 Android TV 遥控器控制游戏，适合在电视上游玩。

Supports Android TV remote control for playing on TV.

| 按键 / Key | 功能 / Function |
|------|------|
| 上 / Up | 菜单向上/跳跃 / Menu up/Jump |
| 下 / Down | 菜单向下/进入管道/发射 / Menu down/Enter pipe/Fire |
| 左 / Left | 左移动 / Move left |
| 右 / Right | 右移动 / Move right |
| OK (确认) | 菜单确认/跳跃 / Menu confirm/Jump |
| 返回 / Back | 返回上级菜单 / Back to previous menu |

**遥控器特性 / Remote Features:**
- 自动加速模式：检测到遥控器时自动开启加速 / Auto-sprint mode when remote is detected
- 空中慢动作：跳跃后空中停留时间延长，便于单键操作 / Extended air time for easier single-button control

### Android 触摸控制 / Android Touch Controls

Android 版本使用原生 Android 按钮实现虚拟控制界面，解决了多点触摸延迟问题。

Android version uses native Android buttons for virtual controls, solving multi-touch delay issues.

**游戏控制按钮（屏幕下方）/ Game Control Buttons (Bottom of Screen)**：

| 控件 / Control | 功能 / Function | 对应按键 / Mapped Key |
|------|------|----------|
| D-Pad (左下角 / Bottom Left) | 方向控制 / Direction control；左键可返回上级菜单 / Left = Back | Arrow Keys |
| A 按钮 / A Button (右下角 / Bottom Right) | 跳跃 / Jump；菜单确认 / Menu confirm | Alt |
| B 按钮 / B Button | 加速/跑步 / Sprint/Run | Ctrl |
| X 按钮 / X Button | 发射火球 / Fire | Space |
| Y 按钮 / Y Button | 备用功能 / Reserved | Shift |

**功能按钮（右上角）/ Function Buttons (Top Right)**：

| 按钮 / Button | 功能 / Function |
|------|------|
| H | 隐藏/显示游戏控制按钮 / Hide/Show game control buttons |
| E | 进入/退出布局编辑模式 / Enter/Exit layout edit mode |
| KB | 打开/关闭虚拟键盘（输入作弊码）/ Open/Close virtual keyboard (for cheat codes) |

**菜单操作 / Menu Operations**：
- **确认选择 / Confirm**: 点击 A 按钮 / Tap A button
- **返回上级 / Back**: 点击 D-Pad 左键 / Tap D-Pad left

**布局编辑模式 / Layout Edit Mode**：点击右上角 E 按钮进入编辑模式，可拖拽调整虚拟按键位置。调整完成后再次点击 E 按钮退出并保存布局。

Tap E button to enter edit mode, drag to adjust virtual button positions. Tap E again to exit and save layout.

**虚拟键盘 / Virtual Keyboard**：点击 KB 按钮打开虚拟键盘面板，用于输入作弊码。键盘面板可拖动调整位置。

Tap KB button to open virtual keyboard panel for entering cheat codes. Panel can be dragged to adjust position.

**输入设备自动切换 / Input Device Auto-Switch**：
- 检测到手柄或遥控器输入时自动隐藏虚拟按钮 / Auto-hide virtual buttons when gamepad/remote input detected
- 手柄和遥控器可与触摸控制同时使用 / Gamepad and remote can be used together with touch controls

### 菜单结构 / Menu Structure

```
主菜单 / Main Menu (MENU)
├── START (开始游戏 / Start Game)
│   ├── NO SAVE      - 不保存进度开始新游戏 / Start without saving
│   ├── GAME SELECT  - 选择存档槽位 (1/2/3) / Select save slot
│   └── ERASE        - 删除存档 / Delete save
├── OPTIONS (选项 / Options)
│   ├── SOUND ON/OFF     - 音效开关 / Sound toggle
│   └── STATUSLINE ON/OFF - 状态栏开关 / Status bar toggle
└── END (退出游戏 / Exit Game)
```

## 编译运行 / Build & Run

### 环境要求 / Requirements

- Rust 1.85+ (Edition 2024)
- Windows: Visual Studio Build Tools (MSVC)
- Linux/macOS: 标准开发工具链 / Standard build toolchain

### Windows 编译 / Windows Build

#### 默认版本（wgpu GPU 渲染，Windows 10+ 推荐）/ Default (wgpu GPU, Windows 10+ Recommended)

```powershell
cargo build --release
# 或使用脚本 / Or use script
.\build_release.ps1
```

默认使用 wgpu GPU 渲染后端，需要支持 Vulkan/DirectX 12 的显卡。

Uses wgpu GPU rendering by default, requires Vulkan/DirectX 12 compatible GPU.

#### Windows 7/XP 兼容版本（CPU 软件渲染）/ Windows 7/XP Compatible (CPU Software Rendering)

```powershell
.\build_win7xp.ps1           # 64位 / 64-bit
.\build_win7xp.ps1 -Arch x86 # 32位 / 32-bit
```

此版本使用 CPU 软件渲染后端，通过 GDI StretchDIBits 显示，无需 GPU 支持，兼容 Windows XP SP3 及以上系统。

This version uses CPU software rendering via GDI StretchDIBits, no GPU required, compatible with Windows XP SP3+.

详见 / See [build_win7xp.md](build_win7xp.md)

### Linux/macOS 编译 / Linux/macOS Build

```bash
cargo build --release
```

Linux/macOS 默认使用 wgpu 后端进行 GPU 渲染。

Linux/macOS uses wgpu GPU rendering by default.

### Android 编译 / Android Build

#### 环境要求 / Requirements

- Rust 工具链 / Rust toolchain
- Android SDK（设置 / Set `ANDROID_HOME` 或 / or `ANDROID_SDK_ROOT` 环境变量 / environment variable）
- Android NDK（设置 / Set `ANDROID_NDK_HOME` 或通过 / or install via SDK Manager）
- JDK 17+
- cargo-ndk（脚本会自动安装 / Auto-installed by script）

#### 使用构建脚本 / Using Build Script

```powershell
# 构建 Debug APK (仅 arm64 架构) / Build Debug APK (arm64 only)
.\build_android.ps1

# 构建 Release APK (仅 arm64 架构) / Build Release APK (arm64 only)
.\build_android.ps1 -Release

# 构建所有架构合并的 APK / Build universal APK (arm64, armv7, x86_64)
.\build_android.ps1 -AllArch

# 构建三个独立 APK (每个架构一个) / Build separate APKs (one per architecture)
.\build_android.ps1 -SeparateApks

# 构建 Release + 三个独立 APK / Build Release + separate APKs
.\build_android.ps1 -Release -SeparateApks

# 仅构建 APK (跳过 Rust 编译) / APK only (skip Rust compilation)
.\build_android.ps1 -SkipRust

# 显示帮助信息 / Show help
.\build_android.ps1 -Help
```

#### 脚本参数说明 / Script Parameters

| 参数 / Param | 说明 / Description |
|------|------|
| `-Release` | 构建 Release 版本（默认 Debug）/ Build Release (default Debug) |
| `-AllArch` | 编译所有架构合并到一个 APK / Build universal APK with all architectures |
| `-SeparateApks` | 为每个架构生成独立的 APK / Build separate APK for each architecture |
| `-SkipRust` | 跳过 Rust 编译，仅执行 Gradle 构建 / Skip Rust compilation, Gradle only |
| `-Help` | 显示详细帮助信息 / Show detailed help |

#### 构建流程 / Build Process

1. **环境检查 / Environment Check**：验证 Rust、Android SDK、NDK、JDK 是否正确安装 / Verify installations
2. **工具安装 / Tool Installation**：自动安装 cargo-ndk 和添加 Android Rust targets / Auto-install cargo-ndk and add Android targets
3. **编译 Rust / Compile Rust**：使用 cargo-ndk 编译生成 .so 动态库 / Generate .so libraries via cargo-ndk
4. **复制依赖 / Copy Dependencies**：复制 libc++_shared.so 到 jniLibs 目录 / Copy libc++_shared.so to jniLibs
5. **构建 APK / Build APK**：调用 Gradle 生成最终 APK 文件 / Generate final APK via Gradle

#### 输出文件 / Output Files

所有 APK 输出到 `dist/android/` 目录 / All APKs output to `dist/android/`:

**单一 APK 模式 / Single APK Mode**（默认或 / Default or `-AllArch`）：

| 文件 / File | 说明 / Description |
|------|------|
| `dist/android/app-debug-arm64.apk` | Debug 版本 / Debug version (ARM64 only) |
| `dist/android/app-release-arm64.apk` | Release 版本 / Release version (ARM64 only) |
| `dist/android/app-release-universal.apk` | Release 版本 / Release (all architectures) |

**独立 APK 模式 / Separate APK Mode**（`-SeparateApks`）：

| 文件 / File | 架构 / Arch | 适用设备 / Target Devices |
|------|------|----------|
| `dist/android/app-release-arm64-v8a.apk` | ARM 64位 / ARM 64-bit | 现代手机（推荐）/ Modern phones (Recommended) |
| `dist/android/app-release-armeabi-v7a.apk` | ARM 32位 / ARM 32-bit | 老旧手机 / Legacy phones |
| `dist/android/app-release-x86_64.apk` | x86 64位 / x86 64-bit | 模拟器/Chromebook / Emulators/Chromebook |

#### 安装到设备 / Install to Device

```bash
# 安装 ARM64 版本 (推荐) / Install ARM64 version (Recommended)
adb install -r dist/android/app-release-arm64.apk

# 安装独立 APK (根据设备架构选择) / Install separate APK (choose by device architecture)
adb install -r dist/android/app-release-arm64-v8a.apk
```

### 运行 / Run

```bash
# Windows/Linux/macOS (wgpu GPU 渲染，默认 / GPU rendering, default)
cargo run --release

# Windows XP 兼容模式 / Windows XP compatible (CPU 软件渲染 / CPU software rendering)
cargo run --release --features cpu-backend --no-default-features
```

## 项目结构 / Project Structure

```
MarioRS/
├── src/
│   ├── main.rs           # 程序入口 / Program entry
│   ├── lib.rs            # 库入口 / Library entry
│   ├── mario.rs          # 游戏状态机 / Game state machine
│   ├── game_runner.rs    # 游戏主运行器 / Main game runner
│   ├── context.rs        # 游戏上下文 / Game context
│   ├── play.rs           # 主游戏逻辑 / Main game logic
│   ├── players.rs        # 玩家行为 (Mario/Luigi) / Player behavior
│   ├── enemies.rs        # 敌人系统 / Enemy system
│   ├── figures.rs        # 游戏物体行为 / Game object behavior
│   ├── joystick.rs       # 手柄状态管理器 / Joystick state manager
│   ├── renderer.rs       # 统一渲染管线 / Unified render pipeline
│   ├── render_state.rs   # 渲染状态管理 / Render state management
│   ├── backgr.rs         # 背景绘制 / Background drawing
│   ├── sprites.rs        # 精灵数据 / Sprite data
│   ├── sprite_assets.rs  # 精灵资源管理 / Sprite asset management
│   ├── palettes.rs       # 调色板管理 / Palette management
│   ├── keyboard.rs       # 键盘输入 / Keyboard input
│   ├── music.rs          # 音效系统 / Sound system
│   ├── txt.rs            # 文本渲染 / Text rendering
│   ├── config.rs         # 配置管理 / Configuration
│   ├── persist.rs        # 持久化工具 / Persistence utilities
│   │
│   ├── gpu/              # GPU 渲染模块 (wgpu) / GPU rendering module
│   │   ├── mod.rs        # 模块入口 / Module entry
│   │   ├── renderer.rs   # GPU 渲染器核心 / GPU renderer core
│   │   ├── pipeline.rs   # 渲染管线创建 / Render pipeline creation
│   │   ├── buffer_pool.rs # GPU 缓冲区池 / GPU buffer pool
│   │   ├── sprite_batch.rs # 精灵批处理 / Sprite batching
│   │   ├── texture_atlas.rs # 纹理图集 / Texture atlas
│   │   ├── tilemap.rs    # 地图块渲染 / Tilemap rendering
│   │   ├── palette.rs    # 调色板管理 / Palette management
│   │   ├── types.rs      # 渲染数据类型 / Render data types
│   │   └── shaders/      # WGSL 着色器 / WGSL shaders
│   │       ├── sprite.wgsl  # 精灵着色器 / Sprite shader
│   │       ├── fill.wgsl    # 填充着色器 / Fill shader
│   │       ├── scale.wgsl   # 缩放着色器 / Scale shader
│   │       └── overlay.wgsl # 叠加层着色器 / Overlay shader
│   │
│   ├── cpu/              # CPU 软件渲染模块 / CPU software rendering module
│   │   ├── mod.rs        # 模块入口 / Module entry
│   │   └── renderer.rs   # CPU 软件渲染器 / CPU software renderer
│   │
│   ├── worlds/           # 关卡数据 / Level data
│   │   ├── intro.rs      # 开场动画 / Intro animation
│   │   └── level_*.rs    # 关卡 1-6 / Levels 1-6
│   │
│   └── platform/         # 平台抽象层 / Platform abstraction layer
│       ├── mod.rs        # 平台 trait 定义 / Platform trait definitions
│       ├── windows.rs    # Windows wgpu + GDI 后端 / Windows wgpu + GDI backend
│       ├── windows_cpu.rs # Windows CPU 软件渲染后端 (XP 兼容) / Windows CPU backend (XP compatible)
│       ├── desktop.rs    # 跨平台 wgpu 后端 (Linux/macOS) / Cross-platform wgpu backend
│       ├── android.rs    # Android 原生后端 (JNI + GPU/CPU 自动切换) / Android native backend (JNI + GPU/CPU auto-fallback)
│       ├── joystick_win.rs     # Windows 手柄后端 (winmm.dll) / Windows gamepad backend
│       ├── joystick_android.rs # Android 手柄后端 (USB/蓝牙) / Android gamepad backend (USB/Bluetooth)
│       ├── joystick_android_tv.rs # Android TV 遥控器后端 / Android TV remote backend
│       ├── common/       # 公共平台实现 / Common platform implementations
│       │   ├── frame_timer.rs # 帧率控制 / Frame rate control
│       │   ├── input.rs      # 输入处理 / Input handling
│       │   ├── random.rs     # 随机数生成 / Random number generation
│       │   ├── storage.rs    # 持久化存储 / Persistent storage
│       │   └── time.rs       # 时间管理 / Time management
│       └── audio/        # 音频后端 / Audio backends
│           ├── waveout.rs    # Windows WaveOut
│           ├── cpal_audio.rs # 跨平台 cpal / Cross-platform cpal
│           └── web_audio.rs  # Web Audio (占位 / placeholder)
│
├── android/              # Android 项目 / Android project
│   ├── app/
│   │   ├── src/main/
│   │   │   ├── java/com/mariogame/mario/
│   │   │   │   ├── MainActivity.java     # 主活动 + JNI 接口 / Main activity + JNI interface
│   │   │   │   ├── GamepadController.java # 手柄控制器 / Gamepad controller
│   │   │   │   ├── RemoteController.java  # 遥控器控制器 / Remote controller
│   │   │   │   └── VirtualController.java # 虚拟按钮控制器 / Virtual button controller
│   │   │   ├── res/
│   │   │   │   ├── layout/       # 按钮布局 XML / Button layout XML
│   │   │   │   ├── drawable/     # 按钮背景资源 / Button drawable resources
│   │   │   │   └── values/       # 样式和尺寸 / Styles and dimensions
│   │   │   ├── jniLibs/      # 编译生成的 .so 文件 / Compiled .so files
│   │   │   └── AndroidManifest.xml
│   │   └── build.gradle.kts
│   └── build.gradle.kts
├── assets/
│   ├── sprites/          # 精灵数据文件 / Sprite data files
│   ├── *.BK              # 背景数据 / Background data
│   └── mario.ico         # 应用图标 / App icon
├── examples/
│   ├── create_icon.rs    # 图标生成工具 / Icon generation tool
│   └── export_sprites.rs # 精灵导出工具 / Sprite export tool
├── build_android.ps1     # Android 构建脚本 / Android build script
└── build.rs              # 构建脚本 / Build script
```

## 渲染架构 / Rendering Architecture

MarioRS 采用双渲染后端架构，支持现代 GPU 加速渲染和传统 CPU 软件渲染。

MarioRS uses a dual rendering backend architecture, supporting both modern GPU-accelerated rendering and traditional CPU software rendering.

### GPU 渲染后端 (wgpu) / GPU Rendering Backend

- 使用 wgpu 进行跨平台 GPU 硬件加速渲染 / Cross-platform GPU hardware acceleration using wgpu
- 支持 Vulkan (Linux/Windows/Android)、Metal (macOS)、DirectX 12 (Windows)
- 精灵批处理、纹理图集、WGSL 着色器 / Sprite batching, texture atlas, WGSL shaders
- 渲染管线 / Render pipeline：Sprite -> Fill -> Scale -> Overlay

### CPU 软件渲染后端 / CPU Software Rendering Backend

- 纯 CPU 软件渲染，无 GPU 依赖 / Pure CPU rendering, no GPU dependency
- Windows: 通过 GDI StretchDIBits 显示帧缓冲 / Display framebuffer via GDI StretchDIBits
- Android: 通过 ANativeWindow API 直接写入帧缓冲 / Direct framebuffer write via ANativeWindow API
- 兼容老旧系统 / Compatible with legacy systems (Windows XP, Android devices without Vulkan)
- 支持索引色精灵、调色板、翻转、透明等效果 / Supports indexed color sprites, palettes, flipping, transparency

### Android GPU 自动降级 / Android GPU Auto-Fallback

Android 平台采用智能渲染策略：

- 优先使用 Vulkan GPU 渲染 / Prioritize Vulkan GPU rendering
- GPU 初始化失败时自动切换到 CPU 软件渲染 / Auto-fallback to CPU software rendering on GPU init failure
- 确保在各种 Android 设备上都能运行 / Ensures compatibility across various Android devices
- 支持整数缩放保持像素清晰 / Integer scaling for pixel-perfect display

## 构建选项 / Build Options

| Feature | 说明 / Description | 平台 / Platform |
|---------|------|------|
| `wgpu-backend` | wgpu GPU 硬件加速渲染 / GPU hardware acceleration | Windows/Linux/macOS/Android |
| `cpu-backend` | CPU 软件渲染（XP 兼容）/ CPU software rendering (XP compatible) | Windows |
| `gdi-backend` | Windows GDI 窗口创建 / Windows GDI window creation | Windows |
| `android` | Android 原生渲染（GPU + CPU 自动降级）/ Android native (GPU + CPU auto-fallback) | Android |
| `touch-panel` | 触摸控制面板 / Touch control panel | Android |
| `dark-theme` | 暗黑主题适配 / Dark theme adaptation | Windows 10+ |

**默认 / Default**: `wgpu-backend` + `dark-theme`

### Feature 组合说明 / Feature Combinations

| 场景 / Scenario | Features | 说明 / Description |
|------|----------|------|
| Windows 现代版（推荐）/ Windows Modern (Recommended) | `wgpu-backend`, `gdi-backend`, `dark-theme` | GPU 渲染 + GDI 窗口 / GPU rendering + GDI window |
| Windows XP 兼容 / Windows XP Compatible | `cpu-backend` | CPU 软件渲染 + GDI 窗口 / CPU rendering + GDI window |
| Linux/macOS | `wgpu-backend` | GPU 渲染 + winit 窗口 / GPU rendering + winit window |
| Android | `android` | GPU 渲染 + CPU 自动降级 + 手柄/遥控器支持 / GPU + CPU fallback + Gamepad/Remote support |

## 平台支持 / Platform Support

| 平台 / Platform | 渲染后端 / Rendering | 音频 / Audio | 输入设备 / Input Devices | 最低版本 / Min Version |
|------|----------|----------|----------|----------|
| Windows 10/11 | wgpu (GPU) | WaveOut | 键盘 + 手柄 / Keyboard + Gamepad | 默认支持 / Default |
| Windows 7/8 | CPU 软件渲染 + YY-Thunks | WaveOut | 键盘 + 手柄 / Keyboard + Gamepad | 需使用兼容版本 / Compatible build |
| Windows XP | CPU 软件渲染 + YY-Thunks | WaveOut | 键盘 + 手柄 / Keyboard + Gamepad | 需使用兼容版本 / Compatible build |
| Linux | wgpu (GPU) | cpal | 键盘 / Keyboard | 默认支持 / Default |
| macOS | wgpu (GPU) | cpal | 键盘 / Keyboard | 默认支持 / Default |
| Android | wgpu (GPU) + CPU Fallback | cpal | 触摸 + 手柄 + 遥控器 / Touch + Gamepad + Remote | API 24+ |
| Android TV | wgpu (GPU) + CPU Fallback | cpal | 遥控器 + 手柄 / Remote + Gamepad | API 24+ |

### 手柄支持详情 / Gamepad Support Details

| 平台 / Platform | 后端 / Backend | 支持的手柄 / Supported Gamepads |
|------|----------|----------|
| Windows | winmm.dll (Multimedia API) | 任何在 joy.cpl 中显示的 USB/蓝牙手柄 / Any USB/Bluetooth gamepad visible in joy.cpl |
| Android | Java InputDevice API | USB/蓝牙手柄 / USB/Bluetooth gamepads |

### Android TV 遥控器支持详情 / Android TV Remote Support Details

- 独立的遥控器处理模块，与手柄逻辑完全分离 / Separate remote handling module, isolated from gamepad logic
- 通过 DPAD_CENTER (OK键) 检测真正的TV遥控器 / Detect real TV remote via DPAD_CENTER (OK key)
- 自动加速模式，便于单手操作 / Auto-sprint mode for single-hand operation
- 支持空中慢动作，便于精确控制 / Extended air time for precise control

## 关卡 / Levels

游戏包含 6 个关卡，通关后解锁 Turbo 模式（敌人速度加快）。

The game contains 6 levels. Completing them unlocks Turbo mode (faster enemies).

1. **Level 1** - 经典地上关卡（草地）/ Classic overworld (Grass)
2. **Level 2** - 地下水道关卡 / Underground sewer
3. **Level 3** - 高空关卡（云层）/ Sky level (Clouds)
4. **Level 4** - 城堡关卡（熔岩）/ Castle level (Lava)
5. **Level 5** - 雪地关卡 / Snow level
6. **Level 6** - 最终关卡 / Final level

## 作弊码 / Cheat Codes

在游戏中按 `P` 暂停，然后按 `Tab` 进入作弊码输入模式。

Press `P` to pause, then `Tab` to enter cheat code mode.

| 作弊码 / Code | 效果 / Effect |
|--------|------|
| `1UP` | 生成 1UP 蘑菇 / Spawn 1UP mushroom |
| `F1F2` | 获得蘑菇（变大）/ Get mushroom (grow) |
| `FFB5` | 获得火焰花 / Get fire flower |
| `9C32` | 获得无敌星星 / Get invincibility star |
| `03E8` | 增加一条生命 / Add one life |
| `2305` | 直接通关当前关卡 / Complete current level |
| `D235` | 切换 Turbo 模式 / Toggle Turbo mode |
| `MONO` | 黑白模式 / Monochrome mode |
| `VGAMODE` | 恢复正常颜色 / Restore normal colors |

## 致谢 / Acknowledgments

- 原版 Pascal 游戏作者 / Original Pascal game author: **Mike Wiering** (1994-95)
- YY-Thunks: [Chuyu-Team](https://github.com/Chuyu-Team/YY-Thunks)
- 参考文章 / Reference article: [Programming Nostalgia: revisiting Mike Wiering's Mario](https://www.codeproject.com/Articles/5360383/Programming-Nostalgia-revisiting-Mike-Wiering-s-Ma)

## 许可证 / License

参见 / See [LICENSE](LICENSE) 文件 / file.
