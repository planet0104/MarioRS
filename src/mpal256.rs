//! MPAL256 调色板模块
//!
//! 存储 6bit RenderState 调色板值 (0-63)，共 256 * 3 个数值
//! 调色板在 build.rs 中预先解析为静态常量

use crate::backgr::get_generated_asset_mpal256;
use crate::palettes::PalType;

/// 获取 MPAL256 调色板（编译期生成的静态常量）
pub fn mpal256_palette() -> &'static PalType {
    get_generated_asset_mpal256()
}
