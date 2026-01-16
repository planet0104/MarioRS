# MarioRS

A Rust port of Mike Wiering's Turbo Pascal Mario clone game.

![Mario Game Screenshot](capture/mario.jpg)

> **Original Website**: [Wiering Software - Mario](https://wieringsoftware.nl/mario/) | [Source Code](https://wieringsoftware.nl/mario/source.html)

## Features

- Complete port of all 6 original levels
- Two-player alternating mode (Mario and Luigi)
- Cross-platform support: Windows / Linux / macOS / Android
- Native Windows GDI rendering (small binary, ~700KB)
- Windows 7/XP compatible version
- Android native support (touch screen virtual buttons)
- Window scaling and fullscreen support
- Dark theme auto-adaptation (Windows 10+)

## Controls

### Game Keys

| Key | Action |
|-----|--------|
| `←` `→` | Move left/right |
| `↑` | Enter pipe (when above a pipe) |
| `↓` | Exit pipe |
| `Alt` | Jump (hold for higher jump) |
| `Ctrl` | Run/Accelerate |
| `Space` | Shoot fireball (Fire Mario only) |

### Shortcuts

| Key | Action |
|-----|--------|
| `P` | Pause game |
| `S` | Toggle status bar |
| `F11` | Toggle fullscreen/windowed mode |
| `ESC` | Exit fullscreen / Quit game / Back to menu |

### Android Touch Controls

The Android version provides a virtual button interface:

| Control | Action | Maps to |
|---------|--------|---------|
| D-Pad (left side) | Direction control (up/down/left/right); Left to go back in menu | Arrow keys |
| A Button | Jump (hold for higher jump); Menu confirm | Alt |
| B Button | Run/Accelerate | Ctrl |
| X Button | Shoot fireball (Fire Mario only) | Space |
| Y Button | Alternate function | Shift |
| E Button (top right) | Enter/Exit layout edit mode | - |
| P Button (below E) | Pause/Resume game | P |
| R Button (edit mode) | Reset button layout to default | - |

**Menu Controls**:
- **Confirm selection**: Tap A button
- **Go back**: Tap D-Pad left
- **Pause game**: Tap P button

**Layout Edit Mode**: Tap the E button in the top right corner to enter edit mode. You can drag virtual buttons to adjust their positions. Tap the E button again to exit and save the layout.

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

### Android Build

#### Requirements

- Rust toolchain
- Android SDK (set `ANDROID_HOME` or `ANDROID_SDK_ROOT` environment variable)
- Android NDK (set `ANDROID_NDK_HOME` or install via SDK Manager)
- JDK 17+
- cargo-ndk (automatically installed by the script)

#### Using the Build Script

```powershell
# Build Debug APK (arm64 architecture only)
.\build_android.ps1

# Build Release APK (arm64 architecture only)
.\build_android.ps1 -Release

# Build all architectures merged into one APK (arm64, armv7, x86_64)
.\build_android.ps1 -AllArch

# Build three separate APKs (one per architecture, smaller size)
.\build_android.ps1 -SeparateApks

# Build Release + three separate APKs
.\build_android.ps1 -Release -SeparateApks

# Build APK only (skip Rust compilation, for rebuilding after Java/resource changes)
.\build_android.ps1 -SkipRust

# Show help information
.\build_android.ps1 -Help
```

#### Script Parameters

| Parameter | Description |
|-----------|-------------|
| `-Release` | Build Release version (default is Debug) |
| `-AllArch` | Compile all architectures merged into one APK: arm64-v8a, armeabi-v7a, x86_64 |
| `-SeparateApks` | Generate separate APK for each architecture (smaller size, easier distribution) |
| `-SkipRust` | Skip Rust compilation, only execute Gradle APK build |
| `-Help` | Show detailed help information |

#### Build Process

1. **Environment Check**: Verify Rust, Android SDK, NDK, and JDK are properly installed
2. **Tool Installation**: Automatically install cargo-ndk and add Android Rust targets
3. **Compile Rust**: Use cargo-ndk to compile and generate .so dynamic libraries
4. **Copy Dependencies**: Copy libc++_shared.so to jniLibs directory
5. **Build APK**: Call Gradle to generate the final APK file

#### Output Files

All APKs are output to the `dist/android/` directory:

**Single APK Mode** (default or `-AllArch`):

| File | Description |
|------|-------------|
| `dist/android/app-debug-arm64.apk` | Debug version (ARM64 only) |
| `dist/android/app-release-arm64.apk` | Release version (ARM64 only) |
| `dist/android/app-release-universal.apk` | Release version (all architectures) |

**Separate APKs Mode** (`-SeparateApks`):

| File | Architecture | Target Devices |
|------|--------------|----------------|
| `dist/android/app-release-arm64-v8a.apk` | ARM 64-bit | Modern phones (recommended) |
| `dist/android/app-release-armeabi-v7a.apk` | ARM 32-bit | Older phones |
| `dist/android/app-release-x86_64.apk` | x86 64-bit | Emulators/Chromebooks |

#### Install to Device

```bash
# Install ARM64 version (recommended)
adb install -r dist/android/app-release-arm64.apk

# Install separate APK (choose based on device architecture)
adb install -r dist/android/app-release-arm64-v8a.apk
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
│       ├── android.rs   # Android native backend
│       ├── touch_panel.rs # Touch screen virtual buttons
│       └── audio/       # Audio backends
│           ├── waveout.rs    # Windows WaveOut
│           ├── cpal_audio.rs # Cross-platform cpal
│           ├── oboe_audio.rs # Android Oboe
│           └── web_audio.rs  # Web Audio (placeholder)
├── android/             # Android project
│   ├── app/
│   │   ├── src/main/
│   │   │   ├── java/        # Java/Kotlin code
│   │   │   ├── jniLibs/     # Compiled .so files
│   │   │   └── AndroidManifest.xml
│   │   └── build.gradle.kts
│   └── build.gradle.kts
├── assets/
│   ├── sprites/         # Sprite data files
│   ├── onscreen_controls/ # Touch button image assets
│   ├── *.BK             # Background data
│   └── mario.ico        # Application icon
├── examples/
│   ├── create_icon.rs   # Icon generation tool
│   └── export_sprites.rs # Sprite export tool
├── build_android.ps1    # Android build script
└── build.rs             # Build script
```

## Build Options

| Feature | Description | Platform |
|---------|-------------|----------|
| `gdi-backend` | Native Windows GDI rendering | Windows |
| `wgpu-backend` | Cross-platform GPU rendering | Windows/Linux/macOS |
| `android` | Android native rendering | Android |
| `touch-panel` | Touch screen virtual buttons | Android |
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
| Android | ANativeWindow + Oboe | Requires android feature |

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
