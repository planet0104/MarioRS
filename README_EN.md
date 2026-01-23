# MarioRS

A Rust port of Mike Wiering's Turbo Pascal Mario clone game.

![Mario Game Screenshot](capture/mario.jpg)

> **Original Website**: [Wiering Software - Mario](https://wieringsoftware.nl/mario/) | [Source Code](https://wieringsoftware.nl/mario/source.html)

> **Learning Guide**: [wgpu Game Development Guide](docs/WGPU_LEARNING_GUIDE.md) - Learn GPU rendering implementation from scratch (Chinese)

## Features

- Complete port of all 6 original levels
- Two-player alternating mode (Mario and Luigi)
- Cross-platform support: Windows / Linux / macOS / Android
- **Dual rendering backend architecture**:
  - **wgpu GPU rendering** (default): Cross-platform hardware acceleration with Vulkan/Metal/DirectX 12
  - **CPU software rendering**: Pure software rendering for legacy systems like Windows XP
- Windows 7/XP compatible version (using CPU backend + YY-Thunks)
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

#### Default Version (wgpu GPU Rendering, Windows 10+ Recommended)

```powershell
cargo build --release
# Or use the build script
.\build_release.ps1
```

Uses wgpu GPU rendering backend by default, requires a GPU with Vulkan/DirectX 12 support.

#### Windows 7/XP Compatible Version (CPU Software Rendering)

```powershell
.\build_win7xp.ps1           # 64-bit
.\build_win7xp.ps1 -Arch x86 # 32-bit
```

This version uses the CPU software rendering backend, displays via GDI StretchDIBits, requires no GPU support, compatible with Windows XP SP3 and above.

See [build_win7xp.md](build_win7xp.md) for details.

### Linux/macOS Build

```bash
cargo build --release
```

Linux/macOS uses the wgpu backend for GPU rendering by default.

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
# Windows/Linux/macOS (wgpu GPU rendering, default)
cargo run --release

# Windows XP compatible mode (CPU software rendering)
cargo run --release --features cpu-backend --no-default-features
```

## Project Structure

```
MarioRS/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library entry
│   ├── mario.rs          # Game state machine
│   ├── game_runner.rs    # Game main runner
│   ├── context.rs        # Game context
│   ├── play.rs           # Main game logic
│   ├── players.rs        # Player behavior (Mario/Luigi)
│   ├── enemies.rs        # Enemy system
│   ├── figures.rs        # Game object behavior
│   ├── renderer.rs       # Unified rendering pipeline
│   ├── render_state.rs   # Render state management
│   ├── backgr.rs         # Background drawing
│   ├── sprites.rs        # Sprite data
│   ├── sprite_assets.rs  # Sprite asset management
│   ├── palettes.rs       # Palette management
│   ├── keyboard.rs       # Keyboard input
│   ├── music.rs          # Sound system
│   ├── txt.rs            # Text rendering
│   ├── config.rs         # Configuration management
│   ├── persist.rs        # Persistence utilities
│   │
│   ├── gpu/              # GPU rendering module (wgpu)
│   │   ├── mod.rs        # Module entry
│   │   ├── renderer.rs   # GPU renderer core
│   │   ├── pipeline.rs   # Render pipeline creation
│   │   ├── buffer_pool.rs # GPU buffer pool
│   │   ├── sprite_batch.rs # Sprite batching
│   │   ├── texture_atlas.rs # Texture atlas
│   │   ├── tilemap.rs    # Tilemap rendering
│   │   ├── palette.rs    # Palette management
│   │   ├── types.rs      # Render data types
│   │   └── shaders/      # WGSL shaders
│   │       ├── sprite.wgsl  # Sprite shader
│   │       ├── fill.wgsl    # Fill shader
│   │       ├── scale.wgsl   # Scale shader
│   │       └── overlay.wgsl # Overlay shader
│   │
│   ├── cpu/              # CPU software rendering module
│   │   ├── mod.rs        # Module entry
│   │   └── renderer.rs   # CPU software renderer
│   │
│   ├── worlds/           # Level data
│   │   ├── intro.rs      # Opening animation
│   │   └── level_*.rs    # Levels 1-6
│   │
│   └── platform/         # Platform abstraction layer
│       ├── mod.rs        # Platform trait definitions
│       ├── windows.rs    # Windows wgpu + GDI backend
│       ├── windows_cpu.rs # Windows CPU software rendering backend (XP compatible)
│       ├── desktop.rs    # Cross-platform wgpu backend (Linux/macOS)
│       ├── android.rs    # Android native backend
│       ├── touch_panel.rs # Touch screen virtual buttons
│       ├── common/       # Common platform implementations
│       │   ├── frame_timer.rs # Frame rate control
│       │   ├── input.rs      # Input handling
│       │   ├── random.rs     # Random number generation
│       │   ├── storage.rs    # Persistent storage
│       │   └── time.rs       # Time management
│       └── audio/        # Audio backends
│           ├── waveout.rs    # Windows WaveOut
│           ├── cpal_audio.rs # Cross-platform cpal
│           └── web_audio.rs  # Web Audio (placeholder)
│
├── android/              # Android project
│   ├── app/
│   │   ├── src/main/
│   │   │   ├── java/         # Java/Kotlin code
│   │   │   ├── jniLibs/      # Compiled .so files
│   │   │   └── AndroidManifest.xml
│   │   └── build.gradle.kts
│   └── build.gradle.kts
├── assets/
│   ├── sprites/          # Sprite data files
│   ├── onscreen_controls/ # Touch button image assets
│   ├── *.BK              # Background data
│   └── mario.ico         # Application icon
├── examples/
│   ├── create_icon.rs    # Icon generation tool
│   └── export_sprites.rs # Sprite export tool
├── build_android.ps1     # Android build script
└── build.rs              # Build script
```

## Rendering Architecture

MarioRS uses a dual rendering backend architecture, supporting both modern GPU-accelerated rendering and traditional CPU software rendering:

### GPU Rendering Backend (wgpu)

- Cross-platform GPU hardware-accelerated rendering using wgpu
- Supports Vulkan (Linux/Windows/Android), Metal (macOS), DirectX 12 (Windows)
- Sprite batching, texture atlas, WGSL shaders
- Render pipeline: Sprites -> Fill -> Scale -> Overlay

### CPU Software Rendering Backend

- Pure CPU software rendering with no GPU dependencies
- Displays framebuffer via Windows GDI StretchDIBits
- Compatible with legacy systems like Windows XP that don't support modern graphics APIs
- Supports indexed color sprites, palettes, flipping, transparency, and other effects

## Build Options

| Feature | Description | Platform |
|---------|-------------|----------|
| `wgpu-backend` | wgpu GPU hardware-accelerated rendering | Windows/Linux/macOS/Android |
| `cpu-backend` | CPU software rendering (XP compatible) | Windows |
| `gdi-backend` | Windows GDI window creation | Windows |
| `android` | Android native rendering | Android |
| `touch-panel` | Touch screen virtual buttons | Android |
| `dark-theme` | Dark theme adaptation | Windows 10+ |

**Default**: `wgpu-backend` + `dark-theme`

### Feature Combinations

| Scenario | Features | Description |
|----------|----------|-------------|
| Windows Modern (Recommended) | `wgpu-backend`, `gdi-backend`, `dark-theme` | GPU rendering + GDI window |
| Windows XP Compatible | `cpu-backend` | CPU software rendering + GDI window |
| Linux/macOS | `wgpu-backend` | GPU rendering + winit window |
| Android | `android` | Auto-enables `wgpu-backend` + `touch-panel` |

## Platform Support

| Platform | Rendering Backend | Audio Backend | Minimum Version |
|----------|-------------------|---------------|-----------------|
| Windows 10/11 | wgpu (GPU) | WaveOut | Default support |
| Windows 7/8 | CPU software rendering + YY-Thunks | WaveOut | Use compatible version |
| Windows XP | CPU software rendering + YY-Thunks | WaveOut | Use compatible version |
| Linux | wgpu (GPU) | cpal | Default support |
| macOS | wgpu (GPU) | cpal | Default support |
| Android | wgpu (GPU) | cpal | Requires android feature |

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
