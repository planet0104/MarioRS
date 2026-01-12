# MarioRS

A Rust port of Mike Wiering's Turbo Pascal Mario clone game.

![Mario Game Screenshot](capture/mario.jpg)

> **Original Website**: [Wiering Software - Mario](https://wieringsoftware.nl/mario/) | [Source Code](https://wieringsoftware.nl/mario/source.html)

## Features

- Complete port of all 6 original levels
- Two-player alternating mode (Mario and Luigi)
- Cross-platform support: Windows / Linux / macOS
- Native Windows GDI rendering (small binary, ~700KB)
- Windows 7/XP compatible version
- Window scaling and fullscreen support
- Dark theme auto-adaptation (Windows 10+)

## Controls

### Game Keys

| Key | Action |
|-----|--------|
| `←` `→` | Move left/right |
| `↑` | Enter pipe (when above a pipe) |
| `↓` | Crouch / Exit pipe |
| `Alt` / `Space` | Jump |
| `Ctrl` | Shoot fireball (Fire Mario only) |

### Shortcuts

| Key | Action |
|-----|--------|
| `P` | Pause game |
| `S` | Toggle status bar |
| `F11` | Toggle fullscreen/windowed mode |
| `ESC` | Exit fullscreen / Quit game / Back to menu |

### Menu Structure

```
Main Menu (MENU)
├── START (Start Game)
│   ├── NO SAVE      - Start new game without saving
│   ├── GAME SELECT  - Select save slot (1/2/3)
│   └── ERASE        - Delete save
├── OPTIONS
│   ├── SOUND ON/OFF     - Toggle sound
│   └── STATUSLINE ON/OFF - Toggle status bar
└── END (Quit Game)
```

## Building

### Requirements

- Rust 1.85+ (Edition 2024)
- Windows: Visual Studio Build Tools (MSVC)
- Linux/macOS: Standard development toolchain

### Windows Build

#### Default Version (Windows 10+, Recommended)

```powershell
cargo build --release
# Or use the build script
.\build_release.ps1
```

#### Windows 7/XP Compatible Version

```powershell
.\build_win7xp.ps1           # 64-bit
.\build_win7xp.ps1 -Arch x86 # 32-bit
```

See [build_win7xp.md](build_win7xp.md) for details.

### Linux/macOS Build

```bash
cargo build --release --features wgpu-backend
```

### Running

```bash
# Windows (GDI backend, default)
cargo run --release

# Linux/macOS (wgpu backend)
cargo run --release --features wgpu-backend
```

## Project Structure

```
MarioRS/
├── src/
│   ├── main.rs          # Entry point
│   ├── lib.rs           # Library entry
│   ├── mario.rs         # Game state machine
│   ├── play.rs          # Main game logic
│   ├── players.rs       # Player behavior (Mario/Luigi)
│   ├── enemies.rs       # Enemy system
│   ├── figures.rs       # Game object behavior
│   ├── vga256.rs        # VGA rendering abstraction
│   ├── renderer.rs      # Renderer
│   ├── backgr.rs        # Background drawing
│   ├── sprites.rs       # Sprite data
│   ├── palettes.rs      # Palette management
│   ├── keyboard.rs      # Keyboard input
│   ├── music.rs         # Sound system
│   ├── txt.rs           # Text rendering
│   ├── config.rs        # Configuration management
│   ├── persist.rs       # Persistence utilities
│   ├── worlds/          # Level data
│   │   ├── intro.rs     # Opening animation
│   │   └── level_*.rs   # Levels 1-6
│   └── platform/        # Platform abstraction layer
│       ├── mod.rs       # Platform trait definitions
│       ├── windows.rs   # Windows GDI backend
│       ├── desktop.rs   # Cross-platform wgpu backend
│       └── audio/       # Audio backends
│           ├── waveout.rs    # Windows WaveOut
│           ├── cpal_audio.rs # Cross-platform cpal
│           └── web_audio.rs  # Web Audio (placeholder)
├── assets/
│   ├── sprites/         # Sprite data files
│   ├── *.BK             # Background data
│   └── mario.ico        # Application icon
├── examples/
│   ├── create_icon.rs   # Icon generation tool
│   └── export_sprites.rs # Sprite export tool
└── build.rs             # Build script
```

## Build Options

| Feature | Description | Platform |
|---------|-------------|----------|
| `gdi-backend` | Native Windows GDI rendering | Windows |
| `wgpu-backend` | Cross-platform GPU rendering | All |
| `dark-theme` | Dark theme adaptation | Windows 10+ |

Default: `gdi-backend` + `dark-theme`

## Platform Support

| Platform | Backend | Minimum Version |
|----------|---------|-----------------|
| Windows 10/11 | GDI | Default support |
| Windows 7/8 | GDI + YY-Thunks | Use compatible version |
| Windows XP | GDI + YY-Thunks | Use compatible version |
| Linux | wgpu + cpal | Requires wgpu-backend |
| macOS | wgpu + cpal | Requires wgpu-backend |

## Levels

The game contains 6 levels. After completing all levels, Turbo mode is unlocked (faster enemies):

1. **Level 1** - Classic overworld (grassland)
2. **Level 2** - Underground sewer
3. **Level 3** - Sky level (clouds)
4. **Level 4** - Castle level (lava)
5. **Level 5** - Snow level
6. **Level 6** - Final level

## Cheat Codes

Press `P` to pause the game, then press `Tab` to enter cheat code mode:

| Cheat Code | Effect |
|------------|--------|
| `1UP` | Spawn 1UP mushroom |
| `F1F2` | Get mushroom (grow big) |
| `FFB5` | Get fire flower |
| `9C32` | Get invincibility star |
| `03E8` | Add one life |
| `2305` | Complete current level |
| `D235` | Toggle Turbo mode |
| `MONO` | Black and white mode |
| `VGAMODE` | Restore normal colors |

## Acknowledgments

- Original Pascal game author: **Mike Wiering** (1994-95)
- YY-Thunks: [Chuyu-Team](https://github.com/Chuyu-Team/YY-Thunks)
- Reference article: [Programming Nostalgia: revisiting Mike Wiering's Mario](https://www.codeproject.com/Articles/5360383/Programming-Nostalgia-revisiting-Mike-Wiering-s-Ma)

## License

See [LICENSE](LICENSE) file.
