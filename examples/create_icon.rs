//! ICO 图标生成工具 - 从 Mario 精灵生成游戏图标
//!
//! 运行方式: cargo run --example create_icon
//!
//! 输出: assets/mario.ico
//!
//! 注意：此工具生成 XP 兼容的 ICO 文件（使用 BMP 格式而非 PNG 压缩）

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use mario::sprites::{PALETTE, SpriteDataManager};

fn main() {
    // 创建输出目录
    let output_dir = Path::new("assets");
    fs::create_dir_all(output_dir).expect("Failed to create assets directory");

    // 加载精灵管理器
    let sprites = SpriteDataManager::new();

    // 使用大马里奥行走精灵作为图标（最经典形象）
    let icon_pixels = &sprites.LWMAR_000;

    // 生成多尺寸 ICO
    // XP 兼容：使用 BMP 格式，限制在 48x48 及以下
    // Vista+：可以使用更大尺寸和 PNG 压缩
    let sizes = [16, 32, 48];
    let mut images: Vec<(u32, Vec<u8>)> = Vec::new();

    for &size in &sizes {
        let rgba = scale_sprite_to_rgba(icon_pixels, 20, 28, size);
        images.push((size, rgba));
    }

    // 写入 XP 兼容的 ICO 文件（BMP 格式）
    let ico_path = output_dir.join("mario.ico");
    write_ico_bmp(&ico_path, &images).expect("Failed to write ICO file");

    println!("图标生成完成: {}", ico_path.display());
    println!("\n图标尺寸: {:?}", sizes);
    println!("格式: BMP (Windows XP 兼容)");

    // 同时导出一个大尺寸 PNG 供预览
    let preview_path = output_dir.join("mario_icon_preview.png");
    let preview_rgba = scale_sprite_to_rgba(icon_pixels, 20, 28, 256);
    let img =
        image::RgbaImage::from_raw(256, 256, preview_rgba).expect("Failed to create preview image");
    img.save(&preview_path).expect("Failed to save preview PNG");
    println!("\n预览图片: {}", preview_path.display());
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
        [0, 0, 0, 0]
    }
}

/// 缩放精灵到指定尺寸（最近邻插值，保持像素风格）
fn scale_sprite_to_rgba(
    pixels: &[[u8; 20]; 28],
    src_w: u32,
    src_h: u32,
    target_size: u32,
) -> Vec<u8> {
    let mut rgba = vec![0u8; (target_size * target_size * 4) as usize];

    // 计算偏移以居中（正方形画布）
    let scale = (target_size as f32 / src_h.max(src_w) as f32).floor() as u32;
    let scale = scale.max(1);

    let scaled_w = src_w * scale;
    let scaled_h = src_h * scale;

    let offset_x = (target_size.saturating_sub(scaled_w)) / 2;
    let offset_y = (target_size.saturating_sub(scaled_h)) / 2;

    for y in 0..target_size {
        for x in 0..target_size {
            let color = if x >= offset_x
                && x < offset_x + scaled_w
                && y >= offset_y
                && y < offset_y + scaled_h
            {
                // 在精灵区域内
                let src_x = ((x - offset_x) / scale) as usize;
                let src_y = ((y - offset_y) / scale) as usize;
                if src_x < src_w as usize && src_y < src_h as usize {
                    palette_to_rgba(pixels[src_y][src_x])
                } else {
                    [0, 0, 0, 0]
                }
            } else {
                // 透明背景
                [0, 0, 0, 0]
            };

            let idx = ((y * target_size + x) * 4) as usize;
            rgba[idx..idx + 4].copy_from_slice(&color);
        }
    }

    rgba
}

/// 写入 XP 兼容的 ICO 文件（使用 BMP 格式而非 PNG）
///
/// ICO 中的 BMP 格式：
/// - 不包含 BITMAPFILEHEADER（14 字节）
/// - 只包含 BITMAPINFOHEADER（40 字节）+ 像素数据 + AND mask
/// - 高度字段是实际高度的 2 倍（包含 XOR 和 AND mask）
/// - 像素数据从下到上存储（bottom-up DIB）
fn write_ico_bmp(path: &Path, images: &[(u32, Vec<u8>)]) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    // 预编码所有 BMP 数据
    let mut bmp_data: Vec<Vec<u8>> = Vec::new();
    for (size, rgba) in images {
        let bmp = create_ico_bmp(*size, rgba);
        bmp_data.push(bmp);
    }

    let num_images = images.len() as u16;

    // ICO 文件头（6 字节）
    writer.write_all(&[0, 0])?; // 保留，必须为 0
    writer.write_all(&1u16.to_le_bytes())?; // 类型：1 = ICO
    writer.write_all(&num_images.to_le_bytes())?; // 图像数量

    // 计算数据偏移（文件头6字节 + 每个图像目录项16字节）
    let header_size = 6 + (num_images as u32 * 16);
    let mut current_offset = header_size;

    // 写入图像目录项（每个 16 字节）
    for (i, (size, _rgba)) in images.iter().enumerate() {
        let w = if *size >= 256 { 0u8 } else { *size as u8 };
        let h = w;
        let bmp_size = bmp_data[i].len() as u32;

        writer.write_all(&[w])?; // 宽度（0 = 256）
        writer.write_all(&[h])?; // 高度（0 = 256）
        writer.write_all(&[0])?; // 调色板颜色数（0 = 无调色板/真彩色）
        writer.write_all(&[0])?; // 保留
        writer.write_all(&1u16.to_le_bytes())?; // 颜色平面数
        writer.write_all(&32u16.to_le_bytes())?; // 位深度（32 = RGBA）
        writer.write_all(&bmp_size.to_le_bytes())?; // BMP 数据大小
        writer.write_all(&current_offset.to_le_bytes())?; // 数据偏移

        current_offset += bmp_size;
    }

    // 写入 BMP 数据
    for data in &bmp_data {
        writer.write_all(data)?;
    }

    writer.flush()?;
    Ok(())
}

/// 创建 ICO 格式的 BMP 数据（不含 BITMAPFILEHEADER）
fn create_ico_bmp(size: u32, rgba: &[u8]) -> Vec<u8> {
    let mut bmp = Vec::new();

    // BITMAPINFOHEADER (40 bytes)
    let bi_size: u32 = 40;
    let bi_width: i32 = size as i32;
    let bi_height: i32 = (size * 2) as i32; // ICO 中高度是 2 倍（XOR + AND mask）
    let bi_planes: u16 = 1;
    let bi_bit_count: u16 = 32; // 32-bit RGBA
    let bi_compression: u32 = 0; // BI_RGB
    let bi_size_image: u32 = size * size * 4; // 像素数据大小（不含 mask）
    let bi_x_pels_per_meter: i32 = 0;
    let bi_y_pels_per_meter: i32 = 0;
    let bi_clr_used: u32 = 0;
    let bi_clr_important: u32 = 0;

    bmp.extend_from_slice(&bi_size.to_le_bytes());
    bmp.extend_from_slice(&bi_width.to_le_bytes());
    bmp.extend_from_slice(&bi_height.to_le_bytes());
    bmp.extend_from_slice(&bi_planes.to_le_bytes());
    bmp.extend_from_slice(&bi_bit_count.to_le_bytes());
    bmp.extend_from_slice(&bi_compression.to_le_bytes());
    bmp.extend_from_slice(&bi_size_image.to_le_bytes());
    bmp.extend_from_slice(&bi_x_pels_per_meter.to_le_bytes());
    bmp.extend_from_slice(&bi_y_pels_per_meter.to_le_bytes());
    bmp.extend_from_slice(&bi_clr_used.to_le_bytes());
    bmp.extend_from_slice(&bi_clr_important.to_le_bytes());

    // XOR mask (像素数据) - BGRA 格式，从下到上存储
    for y in (0..size).rev() {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let r = rgba[idx];
            let g = rgba[idx + 1];
            let b = rgba[idx + 2];
            let a = rgba[idx + 3];
            // BMP 使用 BGRA 顺序
            bmp.push(b);
            bmp.push(g);
            bmp.push(r);
            bmp.push(a);
        }
    }

    // AND mask（透明度掩码）
    // 每行需要 4 字节对齐
    let mask_row_bytes = ((size + 31) / 32) * 4;
    for y in (0..size).rev() {
        let mut row = vec![0u8; mask_row_bytes as usize];
        for x in 0..size {
            let idx = ((y * size + x) * 4 + 3) as usize;
            let alpha = rgba[idx];
            if alpha < 128 {
                // 透明像素：AND mask 位设为 1
                let byte_idx = (x / 8) as usize;
                let bit_idx = 7 - (x % 8);
                row[byte_idx] |= 1 << bit_idx;
            }
        }
        bmp.extend_from_slice(&row);
    }

    bmp
}
