//! 公共时间后端
//!
//! 使用 std::time::Instant 实现，适用于所有支持标准库的平台

use crate::platform::TimeBackend;
use std::time::Instant;

/// 公共时间后端实现
pub struct CommonTime {
    start: Instant,
}

impl CommonTime {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Default for CommonTime {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeBackend for CommonTime {
    fn now_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

// 线程局部存储，用于全局便捷函数
thread_local! {
    static TIME: CommonTime = CommonTime::new();
}

/// 获取程序启动以来的毫秒数
pub fn now_ms() -> f64 {
    TIME.with(|t| t.now_ms())
}
