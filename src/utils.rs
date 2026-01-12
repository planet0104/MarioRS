// 工具函数模块
//
// 使用平台抽象层的随机数实现，保持原有 API 兼容

use crate::platform;

/// 返回 [0, n) 之间的随机整数，n>0
pub fn random_usize(n: usize) -> usize {
    platform::random_usize(n)
}

pub fn random_i32(n: i32) -> i32 {
    platform::random_i32(n)
}

pub fn random_f32(n: f32) -> f32 {
    platform::random_f32(n)
}

pub fn random_u32(n: u32) -> u32 {
    platform::random_u32(n)
}

pub fn random_u8(n: u8) -> u8 {
    platform::random_u8(n)
}

pub trait InRange {
    fn is_in_range(&self, start: i32, end: i32) -> bool;
}

impl InRange for i32 {
    fn is_in_range(&self, start: i32, end: i32) -> bool {
        *self >= start && *self <= end
    }
}
