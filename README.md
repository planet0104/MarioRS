# MarioRS

将 Mike Wiering 的 Turbo Pascal 马里奥克隆游戏移植到 Rust。

![Mario Game Screenshot](capture/mario.jpg)

> **原版官网**: [Wiering Software - Mario](https://wieringsoftware.nl/mario/) | [源码下载](https://wieringsoftware.nl/mario/source.html)

## 特性

- 完整移植原版 6 个关卡
- 双人轮流模式（Mario 和 Luigi）
- 多平台支持：Windows / Linux / macOS / Android
- Windows 原生 GDI 渲染（体积小，约 700KB）
- Windows 7/XP 兼容版本
- Android 原生支持（触摸屏虚拟按键）
- 窗口缩放和全屏支持
- 暗黑主题自动适配（Windows 10+）

## 游戏控制

### 游戏按键

| 按键 | 功能 |
|------|------|
| `←` `→` | 左右移动 |
| `↑` | 进入管道（在管道上方时） |
| `↓` | 从管道出来 |
| `Alt` | 跳跃（按住可控制跳跃高度） |
| `Ctrl` | 加速/跑步 |
| `空格` | 发射火球（火焰马里奥状态） |

### 快捷键

| 按键 | 功能 |
|------|------|
| `P` | 暂停游戏 |
| `S` | 切换状态栏显示 |
| `F11` | 切换全屏/窗口模式 |
| `ESC` | 退出全屏 / 退出游戏 / 返回上级菜单 |

### Android 触摸控制

Android 版本提供虚拟按键界面：

| 控件 | 功能 | 对应按键 |
|------|------|----------|
| D-Pad (左侧) | 方向控制 (上下左右)；左键可返回上级菜单 | 方向键 |
| A 按钮 | 跳跃 (按住可控制高度)；菜单确认 | Alt |
| B 按钮 | 加速/跑步 | Ctrl |
| X 按钮 | 发射火球 (火焰马里奥状态) | Space |
| Y 按钮 | 备用功能 | Shift |
| E 按钮 (右上角) | 进入/退出布局编辑模式 | - |
| P 按钮 (E下方) | 暂停/继续游戏 | P |
| R 按钮 (编辑模式) | 重置按钮布局为默认位置 | - |

**菜单操作**：
- **确认选择**: 点击 A 按钮
- **返回上级**: 点击 D-Pad 左键
- **暂停游戏**: 点击 P 按钮

**布局编辑模式**：点击右上角 E 按钮进入编辑模式，可拖拽调整虚拟按键位置。调整完成后再次点击 E 按钮退出并保存布局。

### 菜单结构

```
主菜单 (MENU)
├── START (开始游戏)
│   ├── NO SAVE      - 不保存进度开始新游戏
│   ├── GAME SELECT  - 选择存档槽位 (1/2/3)
│   └── ERASE        - 删除存档
├── OPTIONS (选项)
│   ├── SOUND ON/OFF     - 音效开关
│   └── STATUSLINE ON/OFF - 状态栏开关
└── END (退出游戏)
```

## 编译运行

### 环境要求

- Rust 1.85+ (Edition 2024)
- Windows: Visual Studio Build Tools (MSVC)
- Linux/macOS: 标准开发工具链

### Windows 编译

#### 默认版本（Windows 10+，推荐）

```powershell
cargo build --release
# 或使用脚本
.\build_release.ps1
```

#### Windows 7/XP 兼容版本

```powershell
.\build_win7xp.ps1           # 64位
.\build_win7xp.ps1 -Arch x86 # 32位
```

详见 [build_win7xp.md](build_win7xp.md)

### Linux/macOS 编译

```bash
cargo build --release --features wgpu-backend
```

### Android 编译

#### 环境要求

- Rust 工具链
- Android SDK（设置 `ANDROID_HOME` 或 `ANDROID_SDK_ROOT` 环境变量）
- Android NDK（设置 `ANDROID_NDK_HOME` 或通过 SDK Manager 安装）
- JDK 17+
- cargo-ndk（脚本会自动安装）

#### 使用构建脚本

```powershell
# 构建 Debug APK (仅 arm64 架构)
.\build_android.ps1

# 构建 Release APK (仅 arm64 架构)
.\build_android.ps1 -Release

# 构建所有架构合并的 APK (arm64, armv7, x86_64)
.\build_android.ps1 -AllArch

# 构建三个独立 APK (每个架构一个，体积更小)
.\build_android.ps1 -SeparateApks

# 构建 Release + 三个独立 APK
.\build_android.ps1 -Release -SeparateApks

# 仅构建 APK (跳过 Rust 编译，用于仅修改 Java/资源后重新打包)
.\build_android.ps1 -SkipRust

# 显示帮助信息
.\build_android.ps1 -Help
```

#### 脚本参数说明

| 参数 | 说明 |
|------|------|
| `-Release` | 构建 Release 版本（默认 Debug） |
| `-AllArch` | 编译所有架构合并到一个 APK：arm64-v8a, armeabi-v7a, x86_64 |
| `-SeparateApks` | 为每个架构生成独立的 APK（体积更小，便于分发） |
| `-SkipRust` | 跳过 Rust 编译，仅执行 Gradle 构建 APK |
| `-Help` | 显示详细帮助信息 |

#### 构建流程

1. **环境检查**：验证 Rust、Android SDK、NDK、JDK 是否正确安装
2. **工具安装**：自动安装 cargo-ndk 和添加 Android Rust targets
3. **编译 Rust**：使用 cargo-ndk 编译生成 .so 动态库
4. **复制依赖**：复制 libc++_shared.so 到 jniLibs 目录
5. **构建 APK**：调用 Gradle 生成最终 APK 文件

#### 输出文件

所有 APK 输出到 `dist/android/` 目录：

**单一 APK 模式**（默认或 `-AllArch`）：

| 文件 | 说明 |
|------|------|
| `dist/android/app-debug-arm64.apk` | Debug 版本 (仅 ARM64) |
| `dist/android/app-release-arm64.apk` | Release 版本 (仅 ARM64) |
| `dist/android/app-release-universal.apk` | Release 版本 (包含所有架构) |

**独立 APK 模式**（`-SeparateApks`）：

| 文件 | 架构 | 适用设备 |
|------|------|----------|
| `dist/android/app-release-arm64-v8a.apk` | ARM 64位 | 现代手机（推荐） |
| `dist/android/app-release-armeabi-v7a.apk` | ARM 32位 | 老旧手机 |
| `dist/android/app-release-x86_64.apk` | x86 64位 | 模拟器/Chromebook |

#### 安装到设备

```bash
# 安装 ARM64 版本 (推荐)
adb install -r dist/android/app-release-arm64.apk

# 安装独立 APK (根据设备架构选择)
adb install -r dist/android/app-release-arm64-v8a.apk
```

### 运行

```bash
# Windows (GDI 后端，默认)
cargo run --release

# Linux/macOS (wgpu 后端)
cargo run --release --features wgpu-backend
```

## 项目结构

```
MarioRS/
├── src/
│   ├── main.rs          # 程序入口
│   ├── lib.rs           # 库入口
│   ├── mario.rs         # 游戏状态机
│   ├── play.rs          # 主游戏逻辑
│   ├── players.rs       # 玩家行为 (Mario/Luigi)
│   ├── enemies.rs       # 敌人系统
│   ├── figures.rs       # 游戏物体行为
│   ├── vga256.rs        # VGA 渲染抽象
│   ├── renderer.rs      # 渲染器
│   ├── backgr.rs        # 背景绘制
│   ├── sprites.rs       # 精灵数据
│   ├── palettes.rs      # 调色板管理
│   ├── keyboard.rs      # 键盘输入
│   ├── music.rs         # 音效系统
│   ├── txt.rs           # 文本渲染
│   ├── config.rs        # 配置管理
│   ├── persist.rs       # 持久化工具
│   ├── worlds/          # 关卡数据
│   │   ├── intro.rs     # 开场动画
│   │   └── level_*.rs   # 关卡 1-6
│   └── platform/        # 平台抽象层
│       ├── mod.rs       # 平台 trait 定义
│       ├── windows.rs   # Windows GDI 后端
│       ├── desktop.rs   # 跨平台 wgpu 后端
│       ├── android.rs   # Android 原生后端
│       ├── touch_panel.rs # 触摸屏虚拟按键
│       └── audio/       # 音频后端
│           ├── waveout.rs    # Windows WaveOut
│           ├── cpal_audio.rs # 跨平台 cpal
│           ├── oboe_audio.rs # Android Oboe
│           └── web_audio.rs  # Web Audio (占位)
├── android/             # Android 项目
│   ├── app/
│   │   ├── src/main/
│   │   │   ├── java/        # Java/Kotlin 代码
│   │   │   ├── jniLibs/     # 编译生成的 .so 文件
│   │   │   └── AndroidManifest.xml
│   │   └── build.gradle.kts
│   └── build.gradle.kts
├── assets/
│   ├── sprites/         # 精灵数据文件
│   ├── onscreen_controls/ # 触摸按键图片资源
│   ├── *.BK             # 背景数据
│   └── mario.ico        # 应用图标
├── examples/
│   ├── create_icon.rs   # 图标生成工具
│   └── export_sprites.rs # 精灵导出工具
├── build_android.ps1    # Android 构建脚本
└── build.rs             # 构建脚本
```

## 构建选项

| Feature | 说明 | 平台 |
|---------|------|------|
| `gdi-backend` | Windows 原生 GDI 渲染 | Windows |
| `wgpu-backend` | 跨平台 GPU 渲染 | Windows/Linux/macOS |
| `android` | Android 原生渲染 | Android |
| `touch-panel` | 触摸屏虚拟按键 | Android |
| `dark-theme` | 暗黑主题适配 | Windows 10+ |

默认: `gdi-backend` + `dark-theme`

## 平台支持

| 平台 | 后端 | 最低版本 |
|------|------|----------|
| Windows 10/11 | GDI | 默认支持 |
| Windows 7/8 | GDI + YY-Thunks | 需使用兼容版本 |
| Windows XP | GDI + YY-Thunks | 需使用兼容版本 |
| Linux | wgpu + cpal | 需 wgpu-backend |
| macOS | wgpu + cpal | 需 wgpu-backend |
| Android | ANativeWindow + Oboe | 需 android feature |

## 关卡

游戏包含 6 个关卡，通关后解锁 Turbo 模式（敌人速度加快）：

1. **Level 1** - 经典地上关卡（草地）
2. **Level 2** - 地下水道关卡
3. **Level 3** - 高空关卡（云层）
4. **Level 4** - 城堡关卡（熔岩）
5. **Level 5** - 雪地关卡
6. **Level 6** - 最终关卡

## 作弊码

在游戏中按 `P` 暂停，然后按 `Tab` 进入作弊码输入模式：

| 作弊码 | 效果 |
|--------|------|
| `1UP` | 生成 1UP 蘑菇 |
| `F1F2` | 获得蘑菇（变大） |
| `FFB5` | 获得火焰花 |
| `9C32` | 获得无敌星星 |
| `03E8` | 增加一条生命 |
| `2305` | 直接通关当前关卡 |
| `D235` | 切换 Turbo 模式 |
| `MONO` | 黑白模式 |
| `VGAMODE` | 恢复正常颜色 |

## 致谢

- 原版 Pascal 游戏作者: **Mike Wiering** (1994-95)
- YY-Thunks: [Chuyu-Team](https://github.com/Chuyu-Team/YY-Thunks)
- 参考文章: [Programming Nostalgia: revisiting Mike Wiering's Mario](https://www.codeproject.com/Articles/5360383/Programming-Nostalgia-revisiting-Mike-Wiering-s-Ma)

## 许可证

参见 [LICENSE](LICENSE) 文件。
