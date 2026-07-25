//! 公共随机数后端
//!
//! 使用 rand::SmallRng 实现，种子获取方式根据平台不同:
//! - Windows GDI模式: 使用 RtlGenRandom (Win32 API)
//! - WASM: 使用 js_sys::Math::random() 或 getrandom
//! - 其他平台: 使用 SystemTime

use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use std::cell::RefCell;

use crate::platform::RandomBackend;

// ============================================================================
// 种子获取 - 根据平台选择不同实现
// ============================================================================

/// 使用系统时间生成种子 (通用方案，非 WASM)
#[cfg(not(target_arch = "wasm32"))]
fn system_time_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// WASM 平台使用 js_sys::Math::random() 生成种子
#[cfg(target_arch = "wasm32")]
fn wasm_random_seed() -> u64 {
    // 使用 Math.random() 生成随机数作为种子
    let r1 = js_sys::Math::random();
    let r2 = js_sys::Math::random();
    // 将两个 f64 随机数组合成一个 u64 种子
    let high = (r1 * u32::MAX as f64) as u64;
    let low = (r2 * u32::MAX as f64) as u64;
    (high << 32) | low
}

/// Windows 平台使用 RtlGenRandom 生成更高质量的种子
#[cfg(all(
    target_os = "windows",
    feature = "gdi-backend",
    not(feature = "wgpu-backend")
))]
mod win32_seed {
    #[link(name = "Advapi32")]
    unsafe extern "system" {
        #[link_name = "SystemFunction036"]
        fn RtlGenRandom(buffer: *mut u8, length: u32) -> u8;
    }

    pub fn get_seed() -> u64 {
        let mut seed = [0u8; 8];
        unsafe {
            if RtlGenRandom(seed.as_mut_ptr(), 8) == 0 {
                return super::system_time_seed();
            }
        }
        u64::from_le_bytes(seed)
    }
}

/// 获取随机种子
fn get_seed() -> u64 {
    // Windows GDI 模式使用 RtlGenRandom
    #[cfg(all(
        target_os = "windows",
        feature = "gdi-backend",
        not(feature = "wgpu-backend")
    ))]
    {
        win32_seed::get_seed()
    }
    // WASM 平台使用 js_sys::Math::random()
    #[cfg(target_arch = "wasm32")]
    {
        wasm_random_seed()
    }
    // 其他平台使用 SystemTime
    #[cfg(all(
        not(all(
            target_os = "windows",
            feature = "gdi-backend",
            not(feature = "wgpu-backend")
        )),
        not(target_arch = "wasm32")
    ))]
    {
        system_time_seed()
    }
}

// ============================================================================
// 公共随机数后端
// ============================================================================

/// 公共随机数后端实现
pub struct CommonRandom {
    rng: SmallRng,
}

impl CommonRandom {
    pub fn new() -> Self {
        Self {
            rng: SmallRng::seed_from_u64(get_seed()),
        }
    }
}

impl Default for CommonRandom {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomBackend for CommonRandom {
    fn random_range(&mut self, max: i32) -> i32 {
        if max <= 0 {
            return 0;
        }
        self.rng.gen_range(0..max)
    }

    fn random_range_f32(&mut self, max: f32) -> f32 {
        if max <= 0.0 {
            return 0.0;
        }
        self.rng.gen_range(0.0..max)
    }

    fn random_f32(&mut self) -> f32 {
        self.rng.gen_range(0.0..1.0)
    }
}

// ============================================================================
// 全局便捷函数
// ============================================================================

thread_local! {
    static RANDOM: RefCell<CommonRandom> = RefCell::new(CommonRandom::new());
}

/// 重新设置全局随机种子（用于 AI 训练的可复现性）
pub fn reseed(seed: u64) {
    RANDOM.with(|r| {
        *r.borrow_mut() = CommonRandom {
            rng: SmallRng::seed_from_u64(seed),
        };
    });
}

/// 生成 [0, max) 范围内的随机 i32
pub fn random_i32(max: i32) -> i32 {
    RANDOM.with(|r| r.borrow_mut().random_range(max))
}

/// 生成 [0, max) 范围内的随机 usize
pub fn random_usize(max: usize) -> usize {
    random_i32(max as i32) as usize
}

/// 生成 [0, max) 范围内的随机 u32
pub fn random_u32(max: u32) -> u32 {
    random_i32(max as i32) as u32
}

/// 生成 [0, max) 范围内的随机 u8
pub fn random_u8(max: u8) -> u8 {
    random_i32(max as i32) as u8
}

/// 生成 [0, max) 范围内的随机 f32
pub fn random_f32(max: f32) -> f32 {
    RANDOM.with(|r| r.borrow_mut().random_range_f32(max))
}
