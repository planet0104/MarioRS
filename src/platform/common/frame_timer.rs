//! 公共帧率控制
//!
//! 提供帧率限制和帧时间统计功能

use std::time::{Duration, Instant};

/// 帧率控制器
pub struct FrameTimer {
    /// 目标帧间隔
    frame_duration: Duration,
    /// 下一帧的预期时间点
    next_frame: Instant,
}

impl FrameTimer {
    /// 创建帧率控制器
    ///
    /// # Arguments
    /// * `target_fps` - 目标帧率 (如 60.0)
    pub fn new(target_fps: f64) -> Self {
        Self {
            frame_duration: Duration::from_secs_f64(1.0 / target_fps),
            next_frame: Instant::now(),
        }
    }

    /// 检查是否应该渲染下一帧
    ///
    /// 返回 true 表示可以渲染，返回 false 表示应该等待
    pub fn should_render(&self) -> bool {
        Instant::now() >= self.next_frame
    }

    /// 推进到下一帧
    ///
    /// 调用此方法后，should_render() 将返回 false 直到下一帧时间
    pub fn advance(&mut self) {
        let now = Instant::now();
        self.next_frame = now + self.frame_duration;
    }

    /// 等待直到下一帧时间 (使用 sleep)
    ///
    /// 如果当前时间已经超过下一帧时间，立即返回
    pub fn wait_if_needed(&self) {
        let now = Instant::now();
        if now < self.next_frame {
            let sleep_time = self.next_frame - now;
            // 留出 1ms 余量，避免过度睡眠
            if sleep_time > Duration::from_millis(1) {
                std::thread::sleep(sleep_time - Duration::from_millis(1));
            }
        }
    }

    /// 获取目标帧间隔
    pub fn frame_duration(&self) -> Duration {
        self.frame_duration
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new(60.0)
    }
}

/// 帧率统计器
pub struct FpsCounter {
    /// 帧计数
    frame_count: u32,
    /// 帧时间累加器 (毫秒)
    frame_time_accumulator: f32,
    /// 上次更新时间
    last_update: Instant,
    /// 当前显示的 FPS
    fps_display: u32,
    /// 当前显示的平均帧时间 (毫秒)
    frame_time_display: f32,
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            frame_time_accumulator: 0.0,
            last_update: Instant::now(),
            fps_display: 0,
            frame_time_display: 0.0,
        }
    }

    /// 记录一帧的渲染时间
    ///
    /// # Arguments
    /// * `frame_time_ms` - 本帧渲染耗时 (毫秒)
    pub fn record_frame(&mut self, frame_time_ms: f32) {
        self.frame_count += 1;
        self.frame_time_accumulator += frame_time_ms;

        let elapsed = self.last_update.elapsed();
        if elapsed >= Duration::from_secs(1) {
            self.fps_display = (self.frame_count as f32 / elapsed.as_secs_f32()) as u32;
            self.frame_time_display = self.frame_time_accumulator / self.frame_count as f32;
            self.frame_count = 0;
            self.frame_time_accumulator = 0.0;
            self.last_update = Instant::now();
        }
    }

    /// 获取当前 FPS (每秒更新一次)
    pub fn fps(&self) -> u32 {
        self.fps_display
    }

    /// 获取平均帧时间 (毫秒，每秒更新一次)
    pub fn frame_time_ms(&self) -> f32 {
        self.frame_time_display
    }
}

impl Default for FpsCounter {
    fn default() -> Self {
        Self::new()
    }
}
