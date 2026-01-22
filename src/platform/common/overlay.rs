//! 公共叠加层绘制
//!
//! 提供 FPS 显示等叠加层绘制功能

use crate::txt::SWISS_FONT_GLYPHS;

/// 在 RGBA 叠加层上绘制 FPS 信息
///
/// # Arguments
/// * `overlay` - RGBA 格式的叠加层缓冲区 (width * height * 4 字节)
/// * `width` - 叠加层宽度
/// * `height` - 叠加层高度
/// * `fps` - 当前帧率
/// * `frame_time_ms` - 平均帧时间 (毫秒)
pub fn draw_fps_to_overlay_rgba(
    overlay: &mut [u8],
    width: u32,
    height: u32,
    fps: u32,
    frame_time_ms: f32,
) {
    if width == 0 || height == 0 {
        return;
    }
    let w = width as usize;
    let h = height as usize;
    if overlay.len() < w.saturating_mul(h).saturating_mul(4) {
        return;
    }

    let text = format!("FPS:{} MS:{:.1}", fps, frame_time_ms);
    let mut x_pos = 10usize;
    let y_pos = 10usize;
    let scale = 1usize;

    // 内部绘制字形函数
    let draw_glyph = |overlay: &mut [u8],
                      x_pos: usize,
                      y_pos: usize,
                      glyph_w: usize,
                      glyph_h: usize,
                      bitmap: &[u8],
                      color: [u8; 4],
                      dx: usize,
                      dy: usize| {
        for row in 0..glyph_h {
            for col in 0..glyph_w {
                let bit_index = row * glyph_w + col;
                let byte_index = bit_index / 8;
                let bit_offset = bit_index % 8;
                if byte_index >= bitmap.len() {
                    continue;
                }
                let byte = bitmap[byte_index];
                let bit = (byte >> bit_offset) & 1;
                if bit != 1 {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = x_pos + col * scale + sx + dx;
                        let py = y_pos + row * scale + sy + dy;
                        if px >= w || py >= h {
                            continue;
                        }
                        let idx = (py * w + px) * 4;
                        overlay[idx] = color[0];
                        overlay[idx + 1] = color[1];
                        overlay[idx + 2] = color[2];
                        overlay[idx + 3] = color[3];
                    }
                }
            }
        }
    };

    let shadow = [0u8, 0u8, 0u8, 255u8];
    let white = [255u8, 255u8, 255u8, 255u8];

    for ch in text.chars() {
        let ch_code = ch as usize;
        if ch_code < 32 || ch_code > 129 {
            x_pos += 8;
            continue;
        }
        let glyph_idx = ch_code - 32;
        if glyph_idx >= SWISS_FONT_GLYPHS.len() {
            x_pos += 8;
            continue;
        }
        let glyph = &SWISS_FONT_GLYPHS[glyph_idx];
        let glyph_w = glyph.width() as usize;
        let glyph_h = glyph.height() as usize;
        let bitmap = glyph.bitmap();

        // 绘制阴影
        draw_glyph(overlay, x_pos, y_pos, glyph_w, glyph_h, bitmap, shadow, 1, 1);
        // 绘制白色文字
        draw_glyph(overlay, x_pos, y_pos, glyph_w, glyph_h, bitmap, white, 0, 0);

        x_pos += glyph_w * scale + 2;
    }
}
