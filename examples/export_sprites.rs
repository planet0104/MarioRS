//! 精灵导出工具 - 将游戏精灵导出为 PNG 图片
//!
//! 运行方式: cargo run --example export_sprites
//!
//! 输出: output/ 目录下的 PNG 文件

use std::fs;
use std::path::Path;

// 引入游戏模块
use mario::sprites::{PALETTE, SpriteDataManager};

fn main() {
    // 创建输出目录
    let output_dir = Path::new("output/sprites");
    fs::create_dir_all(output_dir).expect("Failed to create output directory");

    // 加载精灵管理器
    let sprites = SpriteDataManager::new();

    // 导出 Mario 精灵（20x28）
    export_sprite_20x28(&sprites.SWMAR_000, "SWMAR_000", output_dir);
    export_sprite_20x28(&sprites.SWMAR_001, "SWMAR_001", output_dir);
    export_sprite_20x28(&sprites.SJMAR_000, "SJMAR_000", output_dir);
    export_sprite_20x28(&sprites.SJMAR_001, "SJMAR_001", output_dir);
    export_sprite_20x28(&sprites.LWMAR_000, "LWMAR_000", output_dir);
    export_sprite_20x28(&sprites.LWMAR_001, "LWMAR_001", output_dir);
    export_sprite_20x28(&sprites.LJMAR_000, "LJMAR_000", output_dir);
    export_sprite_20x28(&sprites.LJMAR_001, "LJMAR_001", output_dir);
    export_sprite_20x28(&sprites.FWMAR_000, "FWMAR_000", output_dir);
    export_sprite_20x28(&sprites.FWMAR_001, "FWMAR_001", output_dir);
    export_sprite_20x28(&sprites.FJMAR_000, "FJMAR_000", output_dir);
    export_sprite_20x28(&sprites.FJMAR_001, "FJMAR_001", output_dir);

    // 导出 Luigi 精灵（20x28）
    export_sprite_20x28(&sprites.LWLUI_000, "LWLUI_000", output_dir);
    export_sprite_20x28(&sprites.LWLUI_001, "LWLUI_001", output_dir);
    export_sprite_20x28(&sprites.LJLUI_000, "LJLUI_000", output_dir);
    export_sprite_20x28(&sprites.LJLUI_001, "LJLUI_001", output_dir);
    export_sprite_20x28(&sprites.SWLUI_000, "SWLUI_000", output_dir);
    export_sprite_20x28(&sprites.SWLUI_001, "SWLUI_001", output_dir);
    export_sprite_20x28(&sprites.SJLUI_000, "SJLUI_000", output_dir);
    export_sprite_20x28(&sprites.SJLUI_001, "SJLUI_001", output_dir);
    export_sprite_20x28(&sprites.FWLUI_000, "FWLUI_000", output_dir);
    export_sprite_20x28(&sprites.FWLUI_001, "FWLUI_001", output_dir);
    export_sprite_20x28(&sprites.FJLUI_000, "FJLUI_000", output_dir);
    export_sprite_20x28(&sprites.FJLUI_001, "FJLUI_001", output_dir);

    // 导出一些常用精灵（20x14）
    export_sprite_20x14(&sprites.COIN_000, "COIN_000", output_dir);
    export_sprite_20x14(&sprites.QUEST_000, "QUEST_000", output_dir);
    export_sprite_20x14(&sprites.QUEST_001, "QUEST_001", output_dir);
    export_sprite_20x14(&sprites.STAR_000, "STAR_000", output_dir);
    export_sprite_20x14(&sprites.FLOWER_000, "FLOWER_000", output_dir);
    export_sprite_20x14(&sprites.CHAMP_000, "CHAMP_000", output_dir);
    export_sprite_20x14(&sprites.LIFE_000, "LIFE_000", output_dir);

    // 导出敌人精灵
    export_sprite_20x14(&sprites.CHIBIBO_000, "CHIBIBO_000", output_dir);
    export_sprite_20x14(&sprites.CHIBIBO_001, "CHIBIBO_001", output_dir);
    export_sprite_20x24(&sprites.GRKOOPA_000, "GRKOOPA_000", output_dir);
    export_sprite_20x24(&sprites.GRKOOPA_001, "GRKOOPA_001", output_dir);

    println!("精灵导出完成！文件保存在 output/sprites/ 目录");
    println!("\n推荐用于图标的精灵：");
    println!("  - LWMAR_000.png (大马里奥行走 - 经典形象)");
    println!("  - LJMAR_000.png (大马里奥跳跃 - 动感形象)");
    println!("  - FWMAR_000.png (火球马里奥 - 高级形象)");
}

/// 将索引颜色转换为 RGBA
fn palette_to_rgba(index: u8) -> [u8; 4] {
    if index == 0 {
        // 索引 0 为透明色
        [0, 0, 0, 0]
    } else if (index as usize) < PALETTE.len() {
        let (r, g, b, a) = PALETTE[index as usize];
        [r, g, b, a]
    } else {
        // 超出调色板范围，返回透明
        [0, 0, 0, 0]
    }
}

/// 导出 20x28 精灵（Mario/Luigi）
fn export_sprite_20x28(pixels: &[[u8; 20]; 28], name: &str, output_dir: &Path) {
    let width = 20u32;
    let height = 28u32;
    let scale = 4u32; // 放大 4 倍以便查看

    let scaled_width = width * scale;
    let scaled_height = height * scale;

    let mut rgba_data = vec![0u8; (scaled_width * scaled_height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let color = palette_to_rgba(pixels[y as usize][x as usize]);
            // 放大像素
            for sy in 0..scale {
                for sx in 0..scale {
                    let px = x * scale + sx;
                    let py = y * scale + sy;
                    let idx = ((py * scaled_width + px) * 4) as usize;
                    rgba_data[idx..idx + 4].copy_from_slice(&color);
                }
            }
        }
    }

    save_png(output_dir, name, scaled_width, scaled_height, &rgba_data);
}

/// 导出 20x14 精灵（常规道具）
fn export_sprite_20x14(pixels: &[[u8; 20]; 14], name: &str, output_dir: &Path) {
    let width = 20u32;
    let height = 14u32;
    let scale = 4u32;

    let scaled_width = width * scale;
    let scaled_height = height * scale;

    let mut rgba_data = vec![0u8; (scaled_width * scaled_height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let color = palette_to_rgba(pixels[y as usize][x as usize]);
            for sy in 0..scale {
                for sx in 0..scale {
                    let px = x * scale + sx;
                    let py = y * scale + sy;
                    let idx = ((py * scaled_width + px) * 4) as usize;
                    rgba_data[idx..idx + 4].copy_from_slice(&color);
                }
            }
        }
    }

    save_png(output_dir, name, scaled_width, scaled_height, &rgba_data);
}

/// 导出 20x24 精灵（Koopa 敌人）
fn export_sprite_20x24(pixels: &[[u8; 20]; 24], name: &str, output_dir: &Path) {
    let width = 20u32;
    let height = 24u32;
    let scale = 4u32;

    let scaled_width = width * scale;
    let scaled_height = height * scale;

    let mut rgba_data = vec![0u8; (scaled_width * scaled_height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let color = palette_to_rgba(pixels[y as usize][x as usize]);
            for sy in 0..scale {
                for sx in 0..scale {
                    let px = x * scale + sx;
                    let py = y * scale + sy;
                    let idx = ((py * scaled_width + px) * 4) as usize;
                    rgba_data[idx..idx + 4].copy_from_slice(&color);
                }
            }
        }
    }

    save_png(output_dir, name, scaled_width, scaled_height, &rgba_data);
}

/// 保存 PNG 文件（使用简单的 PNG 编码）
fn save_png(output_dir: &Path, name: &str, width: u32, height: u32, rgba_data: &[u8]) {
    use std::io::Write;

    let path = output_dir.join(format!("{}.png", name));

    // 使用 image crate 保存 PNG
    let img = image::RgbaImage::from_raw(width, height, rgba_data.to_vec())
        .expect("Failed to create image");
    img.save(&path).expect("Failed to save PNG");

    println!("导出: {}", path.display());
}
