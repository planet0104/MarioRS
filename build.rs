use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let assets_dir = manifest_dir.join("assets");

    // Windows: 嵌入应用程序图标
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let rc_path = assets_dir.join("mario.rc");
        if rc_path.exists() {
            let _ = embed_resource::compile(&rc_path, embed_resource::NONE);
        }

        // Windows XP 兼容性
        if env::var("MARIO_XP_COMPAT").is_ok() {
            let yy_thunks_dir = manifest_dir.join("vendor").join("yy-thunks").join("objs");
            let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
            let (arch_dir, subsystem_version) = match target_arch.as_str() {
                "x86" => ("x86", "5.01"),
                "x86_64" => ("x64", "5.02"),
                _ => ("x64", "5.02"),
            };

            let obj_path = yy_thunks_dir.join(arch_dir).join("YY_Thunks_for_WinXP.obj");
            if obj_path.exists() {
                println!("cargo:rustc-link-arg={}", obj_path.display());
                println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS,{}", subsystem_version);
                println!("cargo:warning=YY-Thunks enabled for Windows XP compatibility ({})", arch_dir);
            }
        }
    }
}
