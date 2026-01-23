// Mario RS - Rust重制版马里奥游戏

// Windows Release 模式：隐藏控制台窗口，使用纯窗口模式
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

// 架构说明：
// - main.rs: 程序入口，只调用平台层
// - mario.rs: 游戏核心状态机（平台无关）
// - platform.rs: 平台抽象接口定义
// - platform_desktop.rs: wgpu 渲染后端（跨平台，体积大）
// - platform_windows.rs: Win32 GDI 后端（仅 Windows，体积小）
//
// 渲染后端选择（通过 Cargo features）：
// - wgpu-backend: 使用 pixels + wgpu（默认，跨平台）
// - gdi-backend:  使用 Win32 GDI（仅 Windows，体积最小）

// ============== 入口点 ==============

fn main() {
    let result = run_platform();

    if let Err(e) = result {
        eprintln!("游戏错误: {}", e);
    }
}

/// 根据编译 feature 选择平台实现
/// wgpu-backend 优先，如果同时启用则使用 wgpu
/// 注意：Android 有自己的入口点（android_main），不使用此函数
#[cfg(all(feature = "wgpu-backend", not(target_os = "android")))]
fn run_platform() -> Result<(), Box<dyn std::error::Error>> {
    mario::platform::run_game()
}

/// Windows GDI 后端（仅在未启用 wgpu-backend 时使用）
#[cfg(all(
    target_os = "windows",
    feature = "gdi-backend",
    not(feature = "wgpu-backend")
))]
fn run_platform() -> Result<(), Box<dyn std::error::Error>> {
    mario::platform::run_game()
}

/// Android 平台（main.rs 不使用，入口在 lib.rs 的 android_main）
#[cfg(target_os = "android")]
fn run_platform() -> Result<(), Box<dyn std::error::Error>> {
    // Android 使用 NativeActivity，入口点是 lib.rs 中的 android_main
    // 这个函数不会被调用，仅用于编译通过
    Ok(())
}
