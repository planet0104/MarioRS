# MarioRS

将 Mike Wiering 的 Turbo Pascal 马里奥克隆游戏移植到 Rust。

![Mario Game Screenshot](capture/mario.jpg)

> **原版官网**: [Wiering Software - Mario](https://wieringsoftware.nl/mario/) | [源码下载](https://wieringsoftware.nl/mario/source.html)

## 特性

- 完整移植原版 6 个关卡
- 双人轮流模式（Mario 和 Luigi）
- 多平台支持：Windows / Linux / macOS
- Windows 原生 GDI 渲染（体积小，约 700KB）
- Windows 7/XP 兼容版本
- 窗口缩放和全屏支持
- 暗黑主题自动适配（Windows 10+）

## 游戏控制

### 游戏按键

| 按键 | 功能 |
|------|------|
| `←` `→` | 左右移动 |
| `↑` | 进入管道（在管道上方时） |
| `↓` | 蹲下 / 从管道出来 |
| `Alt` / `空格` | 跳跃 |
| `Ctrl` | 发射火球（火焰马里奥状态） |

### 快捷键

| 按键 | 功能 |
|------|------|
| `P` | 暂停游戏 |
| `S` | 切换状态栏显示 |
| `F11` | 切换全屏/窗口模式 |
| `ESC` | 退出全屏 / 退出游戏 / 返回上级菜单 |

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
│       └── audio/       # 音频后端
│           ├── waveout.rs    # Windows WaveOut
│           ├── cpal_audio.rs # 跨平台 cpal
│           └── web_audio.rs  # Web Audio (占位)
├── assets/
│   ├── sprites/         # 精灵数据文件
│   ├── *.BK             # 背景数据
│   └── mario.ico        # 应用图标
├── examples/
│   ├── create_icon.rs   # 图标生成工具
│   └── export_sprites.rs # 精灵导出工具
└── build.rs             # 构建脚本
```

## 构建选项

| Feature | 说明 | 平台 |
|---------|------|------|
| `gdi-backend` | Windows 原生 GDI 渲染 | Windows |
| `wgpu-backend` | 跨平台 GPU 渲染 | 全平台 |
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
