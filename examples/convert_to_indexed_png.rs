//! 精灵转换工具 - 将 Pascal DB 格式精灵转换为 8-bit 索引 PNG
//!
//! 运行方式: cargo run --example convert_to_indexed_png
//!
//! 输出: assets/sprites_indexed/ 目录下的索引 PNG 文件
//!
//! 索引 PNG 保留调色板索引值，可与游戏的 recolor/init_wall 变色逻辑兼容

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

// 游戏调色板 (从 sprites.rs 复制)
static PALETTE: [(u8, u8, u8, u8); 160] = [
    (0, 0, 0, 255),       // 索引 0 - 透明色
    (0, 40, 167, 255),    // 索引 1
    (48, 112, 72, 255),   // 索引 2
    (8, 175, 88, 255),    // 索引 3
    (143, 79, 39, 255),   // 索引 4
    (232, 0, 0, 255),     // 索引 5
    (231, 143, 48, 255),  // 索引 6
    (175, 175, 191, 255), // 索引 7
    (96, 96, 112, 255),   // 索引 8
    (80, 96, 240, 255),   // 索引 9
    (40, 240, 63, 255),   // 索引 10
    (127, 191, 255, 255), // 索引 11
    (248, 224, 80, 255),  // 索引 12
    (240, 224, 88, 255),  // 索引 13
    (255, 247, 144, 255), // 索引 14
    (255, 255, 255, 255), // 索引 15
    (0, 40, 167, 255),    // 索引 16
    (23, 23, 23, 255),    // 索引 17
    (32, 32, 32, 255),    // 索引 18
    (47, 47, 47, 255),    // 索引 19
    (56, 56, 56, 255),    // 索引 20
    (71, 71, 71, 255),    // 索引 21
    (80, 80, 80, 255),    // 索引 22
    (96, 96, 96, 255),    // 索引 23
    (112, 112, 112, 255), // 索引 24
    (128, 128, 128, 255), // 索引 25
    (144, 144, 144, 255), // 索引 26
    (160, 160, 160, 255), // 索引 27
    (183, 183, 183, 255), // 索引 28
    (200, 200, 200, 255), // 索引 29
    (224, 224, 224, 255), // 索引 30
    (255, 255, 255, 255), // 索引 31
    (55, 48, 63, 255),    // 索引 32
    (63, 64, 88, 255),    // 索引 33
    (71, 80, 119, 255),   // 索引 34
    (79, 96, 144, 255),   // 索引 35
    (87, 112, 175, 255),  // 索引 36
    (95, 127, 200, 255),  // 索引 37
    (103, 136, 231, 255), // 索引 38
    (104, 168, 255, 255), // 索引 39
    (0, 63, 8, 255),      // 索引 40
    (7, 88, 16, 255),     // 索引 41
    (8, 119, 24, 255),    // 索引 42
    (15, 144, 32, 255),   // 索引 43
    (16, 175, 40, 255),   // 索引 44
    (23, 200, 48, 255),   // 索引 45
    (24, 231, 56, 255),   // 索引 46
    (80, 255, 160, 255),  // 索引 47
    (72, 24, 31, 255),    // 索引 48
    (103, 48, 47, 255),   // 索引 49
    (135, 71, 56, 255),   // 索引 50
    (160, 95, 72, 255),   // 索引 51
    (192, 112, 87, 255),  // 索引 52
    (223, 136, 103, 255), // 索引 53
    (248, 159, 111, 255), // 索引 54
    (255, 192, 144, 255), // 索引 55
    (63, 15, 8, 255),     // 索引 56
    (88, 24, 16, 255),    // 索引 57
    (119, 39, 24, 255),   // 索引 58
    (144, 48, 32, 255),   // 索引 59
    (175, 63, 40, 255),   // 索引 60
    (200, 72, 48, 255),   // 索引 61
    (231, 87, 56, 255),   // 索引 62
    (255, 112, 80, 255),  // 索引 63
    (63, 0, 0, 255),      // 索引 64
    (88, 31, 0, 255),     // 索引 65
    (119, 63, 0, 255),    // 索引 66
    (144, 95, 0, 255),    // 索引 67
    (175, 127, 0, 255),   // 索引 68
    (200, 159, 0, 255),   // 索引 69
    (231, 191, 0, 255),   // 索引 70
    (255, 223, 0, 255),   // 索引 71
    (63, 23, 15, 255),    // 索引 72
    (88, 47, 31, 255),    // 索引 73
    (119, 71, 47, 255),   // 索引 74
    (144, 95, 63, 255),   // 索引 75
    (175, 119, 79, 255),  // 索引 76
    (200, 143, 95, 255),  // 索引 77
    (231, 167, 111, 255), // 索引 78
    (255, 215, 136, 255), // 索引 79
    (112, 56, 48, 255),   // 索引 80
    (135, 80, 56, 255),   // 索引 81
    (152, 104, 64, 255),  // 索引 82
    (175, 128, 72, 255),  // 索引 83
    (192, 152, 80, 255),  // 索引 84
    (215, 176, 88, 255),  // 索引 85
    (232, 200, 96, 255),  // 索引 86
    (255, 224, 120, 255), // 索引 87
    (23, 23, 24, 255),    // 索引 88
    (40, 40, 55, 255),    // 索引 89
    (63, 63, 80, 255),    // 索引 90
    (80, 80, 111, 255),   // 索引 91
    (103, 103, 136, 255), // 索引 92
    (120, 120, 167, 255), // 索引 93
    (143, 143, 192, 255), // 索引 94
    (176, 176, 255, 255), // 索引 95
    (15, 24, 23, 255),    // 索引 96
    (23, 47, 39, 255),    // 索引 97
    (31, 64, 55, 255),    // 索引 98
    (39, 87, 71, 255),    // 索引 99
    (47, 104, 87, 255),   // 索引 100
    (55, 127, 103, 255),  // 索引 101
    (63, 144, 119, 255),  // 索引 102
    (80, 192, 152, 255),  // 索引 103
    (24, 24, 31, 255),    // 索引 104
    (55, 55, 63, 255),    // 索引 105
    (80, 80, 95, 255),    // 索引 106
    (111, 111, 127, 255), // 索引 107
    (136, 136, 159, 255), // 索引 108
    (167, 167, 191, 255), // 索引 109
    (192, 192, 223, 255), // 索引 110
    (231, 231, 255, 255), // 索引 111
    (24, 63, 16, 255),    // 索引 112
    (48, 88, 32, 255),    // 索引 113
    (72, 119, 48, 255),   // 索引 114
    (96, 144, 64, 255),   // 索引 115
    (120, 175, 80, 255),  // 索引 116
    (144, 200, 96, 255),  // 索引 117
    (168, 231, 112, 255), // 索引 118
    (216, 255, 143, 255), // 索引 119
    (63, 40, 40, 255),    // 索引 120
    (88, 64, 64, 255),    // 索引 121
    (119, 87, 87, 255),   // 索引 122
    (144, 111, 111, 255), // 索引 123
    (175, 128, 128, 255), // 索引 124
    (200, 152, 152, 255), // 索引 125
    (231, 175, 175, 255), // 索引 126
    (255, 216, 216, 255), // 索引 127
    (63, 63, 24, 255),    // 索引 128
    (88, 88, 48, 255),    // 索引 129
    (119, 119, 72, 255),  // 索引 130
    (144, 144, 96, 255),  // 索引 131
    (175, 175, 120, 255), // 索引 132
    (200, 200, 144, 255), // 索引 133
    (231, 231, 168, 255), // 索引 134
    (255, 255, 216, 255), // 索引 135
    (8, 16, 56, 255),     // 索引 136
    (24, 48, 87, 255),    // 索引 137
    (40, 80, 112, 255),   // 索引 138
    (56, 112, 143, 255),  // 索引 139
    (72, 144, 168, 255),  // 索引 140
    (88, 176, 199, 255),  // 索引 141
    (104, 208, 224, 255), // 索引 142
    (144, 255, 255, 255), // 索引 143
    (72, 16, 56, 255),    // 索引 144
    (96, 32, 87, 255),    // 索引 145
    (127, 48, 112, 255),  // 索引 146
    (151, 64, 143, 255),  // 索引 147
    (176, 80, 168, 255),  // 索引 148
    (200, 96, 199, 255),  // 索引 149
    (231, 112, 224, 255), // 索引 150
    (255, 152, 255, 255), // 索引 151
    (0, 0, 0, 255),       // 索引 152
    (0, 0, 0, 255),       // 索引 153
    (0, 0, 0, 255),       // 索引 154
    (0, 0, 0, 255),       // 索引 155
    (0, 0, 0, 255),       // 索引 156
    (0, 0, 0, 255),       // 索引 157
    (0, 0, 0, 255),       // 索引 158
    (215, 176, 88, 255),  // 索引 159
];

/// 解析 Pascal DB 文本格式
fn parse_pascal_db_text(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for line in s.lines() {
        let l = line.trim_start();
        if !(l.starts_with("db") || l.starts_with("DB")) {
            continue;
        }
        let rest = &l[2..];
        for tok in rest.split(|c: char| c == ',' || c.is_whitespace()) {
            let t = tok.trim().trim_end_matches(',');
            if t.is_empty() {
                continue;
            }
            if let Some(hex) = t.strip_prefix('$') {
                if let Ok(v) = u8::from_str_radix(hex, 16) {
                    out.push(v);
                    continue;
                }
            }
            if let Ok(v) = t.parse::<u8>() {
                out.push(v);
                continue;
            }
        }
    }
    out
}

/// Mode X 去平面化 - 将 VGA Mode X 4-plane 格式转换为线性 row-major 格式
fn modex_deplane(bytes: &[u8], w: usize, h: usize) -> Vec<u8> {
    assert!(
        w % 4 == 0,
        "ModeX requires width divisible by 4: w={}",
        w
    );
    assert!(
        bytes.len() == w * h,
        "ModeX sprite length mismatch: got={} expected={}",
        bytes.len(),
        w * h
    );

    let bytes_per_line = w / 4;
    let plane_size = bytes_per_line * h;
    let mut out = vec![0u8; w * h];

    for y in 0..h {
        for x in 0..w {
            let plane = x & 3;
            let bx = x >> 2;
            let src_idx = plane * plane_size + y * bytes_per_line + bx;
            out[y * w + x] = bytes[src_idx];
        }
    }

    out
}

/// 保存为 8-bit 索引 PNG (灰度图，像素值即调色板索引)
fn save_indexed_png(pixels: &[u8], width: u32, height: u32, output_path: &Path) {
    let file = File::create(output_path).expect("Failed to create PNG file");
    let w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width, height);
    // 使用 8-bit 灰度格式存储索引值
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    
    let mut writer = encoder.write_header().expect("Failed to write PNG header");
    writer.write_image_data(pixels).expect("Failed to write PNG data");
}

/// 保存为带调色板的索引 PNG (便于图片编辑器查看)
fn save_indexed_png_with_palette(pixels: &[u8], width: u32, height: u32, output_path: &Path) {
    let file = File::create(output_path).expect("Failed to create PNG file");
    let w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);
    
    // 构建 256 色调色板 (RGB)
    let mut palette_rgb = Vec::with_capacity(256 * 3);
    let mut palette_alpha = Vec::with_capacity(256);
    
    for i in 0..256 {
        if i < PALETTE.len() {
            let (r, g, b, a) = PALETTE[i];
            palette_rgb.push(r);
            palette_rgb.push(g);
            palette_rgb.push(b);
            palette_alpha.push(if i == 0 { 0 } else { a }); // 索引 0 透明
        } else {
            // 填充黑色
            palette_rgb.push(0);
            palette_rgb.push(0);
            palette_rgb.push(0);
            palette_alpha.push(255);
        }
    }
    
    encoder.set_palette(palette_rgb);
    encoder.set_trns(palette_alpha);
    
    let mut writer = encoder.write_header().expect("Failed to write PNG header");
    writer.write_image_data(pixels).expect("Failed to write PNG data");
}

/// 精灵尺寸配置
struct SpriteConfig {
    width: usize,
    height: usize,
}

impl SpriteConfig {
    fn new(w: usize, h: usize) -> Self {
        Self { width: w, height: h }
    }
}

fn main() {
    let assets_dir = Path::new("assets/sprites_pascal");
    let output_dir = Path::new("assets/sprites_indexed");
    let backgrounds_dir = Path::new("assets");
    
    // 创建输出目录
    fs::create_dir_all(output_dir).expect("Failed to create output directory");
    
    // 先处理背景高度表文件 (BOGEN.BK, BOGEN7.BK, BOGEN26.BK, MOUNT.BK)
    convert_background_files(backgrounds_dir, output_dir);
    
    // 转换 MPAL256 调色板
    convert_mpal256(assets_dir, output_dir);
    
    // 精灵名称到尺寸的映射
    let mut sprite_sizes: HashMap<&str, SpriteConfig> = HashMap::new();
    
    // 20x14 精灵 (常规地形/道具)
    let sprites_20x14 = [
        "BROWN", "PIPE", "GREEN", "SAND", "GRASS", "DES",
        "BRICK0", "BRICK1", "BRICK2",
        "GRASS1", "GRASS2", "GRASS3",
        "PALM0", "PALM1", "PALM2", "PALM3",
        "WOOD", "XBLOCK", "BLOCK", "COIN", "EXIT", "WPALM",
        "FENCE", "SMTREE", "TREE", "WINDOW", "LAVA", "LAVA2",
        "FALL", "NOTE", "PIN", "QUEST", "PALBRICK",
        "CHAMP", "POISON", "LIFE", "FLOWER", "STAR",
        "F", "GRKP", "RDKP", "LIFT1", "DONUT",
        "WHHIT", "WHFIRE", "CHIBIBO", "FISH", "RED",
        "PALPILL",
    ];
    for name in sprites_20x14 {
        sprite_sizes.insert(name, SpriteConfig::new(20, 14));
    }
    
    // 20x28 精灵 (玩家)
    let sprites_20x28 = [
        "SWMAR", "SJMAR", "LWMAR", "LJMAR", "FWMAR", "FJMAR",
        "SWLUI", "SJLUI", "LWLUI", "LJLUI", "FWLUI", "FJLUI",
    ];
    for name in sprites_20x28 {
        sprite_sizes.insert(name, SpriteConfig::new(20, 28));
    }
    
    // 20x24 精灵 (敌人 Koopa)
    let sprites_20x24 = ["GRKOOPA", "RDKOOPA"];
    for name in sprites_20x24 {
        sprite_sizes.insert(name, SpriteConfig::new(20, 24));
    }
    
    // 24x20 精灵 (食人花)
    let sprites_24x20 = ["PPLANT", "HIT"];
    for name in sprites_24x20 {
        sprite_sizes.insert(name, SpriteConfig::new(24, 20));
    }
    
    // 12x7 精灵 (火球/粒子)
    let sprites_12x7 = ["FIRE", "PART"];
    for name in sprites_12x7 {
        sprite_sizes.insert(name, SpriteConfig::new(12, 7));
    }
    
    // 特殊尺寸精灵
    sprite_sizes.insert("INTRO_000", SpriteConfig::new(108, 28));
    sprite_sizes.insert("INTRO_001", SpriteConfig::new(24, 28));
    sprite_sizes.insert("INTRO_002", SpriteConfig::new(84, 28));
    sprite_sizes.insert("START_000", SpriteConfig::new(116, 13));
    sprite_sizes.insert("START_001", SpriteConfig::new(108, 13));
    
    // 统计转换结果
    let mut converted_count = 0;
    let mut skipped_count = 0;
    let mut errors: Vec<String> = Vec::new();
    
    // 遍历 assets/sprites 目录
    for entry in fs::read_dir(assets_dir).expect("Failed to read sprites directory") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        
        if !path.is_file() {
            continue;
        }
        
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        
        // 只处理 .$00 格式的 Pascal include 文件 (优先) 或 .000 二进制文件
        let is_pascal_include = filename.contains(".$");
        let is_binary = filename.ends_with(".000") 
            || filename.ends_with(".001") 
            || filename.ends_with(".002")
            || filename.ends_with(".003")
            || filename.ends_with(".004")
            || filename.ends_with(".005");
        
        if !is_pascal_include && !is_binary {
            continue;
        }
        
        // 解析精灵名称和帧号
        let (base_name, frame) = if is_pascal_include {
            // 格式: NAME.$00 -> (NAME, 0)
            let parts: Vec<&str> = filename.split(".$").collect();
            if parts.len() != 2 {
                continue;
            }
            let frame = match u16::from_str_radix(parts[1], 16) {
                Ok(f) => f,
                Err(_) => continue,
            };
            (parts[0], frame)
        } else {
            // 格式: NAME.000 -> (NAME, 0)
            let parts: Vec<&str> = filename.split('.').collect();
            if parts.len() != 2 {
                continue;
            }
            let frame = match parts[1].parse::<u16>() {
                Ok(f) => f,
                Err(_) => continue,
            };
            (parts[0], frame)
        };
        
        // 查找精灵尺寸
        let sprite_name = format!("{}_{:03}", base_name, frame);
        let config = if let Some(c) = sprite_sizes.get(sprite_name.as_str()) {
            c
        } else if let Some(c) = sprite_sizes.get(base_name) {
            c
        } else {
            // 尝试识别特殊精灵
            if base_name == "INTRO" {
                match frame {
                    0 => &SpriteConfig::new(108, 28),
                    1 => &SpriteConfig::new(24, 28),
                    2 => &SpriteConfig::new(84, 28),
                    _ => {
                        skipped_count += 1;
                        continue;
                    }
                }
            } else if base_name == "START" {
                match frame {
                    0 => &SpriteConfig::new(116, 13),
                    1 => &SpriteConfig::new(108, 13),
                    _ => {
                        skipped_count += 1;
                        continue;
                    }
                }
            } else {
                println!("警告: 未知精灵尺寸: {} ({})", sprite_name, filename);
                skipped_count += 1;
                continue;
            }
        };
        
        // 检查是否已有同名 PNG (避免重复处理 .$00 和 .000)
        let output_path = output_dir.join(format!("{}.png", sprite_name));
        if output_path.exists() && !is_pascal_include {
            // 如果已存在且当前是二进制文件，跳过 (优先使用 .$00)
            skipped_count += 1;
            continue;
        }
        
        // 读取文件内容
        let content = match fs::read(&path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("读取失败 {}: {}", filename, e));
                continue;
            }
        };
        
        // 解析像素数据
        let raw_bytes = if is_pascal_include {
            // Pascal DB 文本格式
            match std::str::from_utf8(&content) {
                Ok(text) => parse_pascal_db_text(text),
                Err(_) => content.clone(), // 回退到二进制
            }
        } else {
            content
        };
        
        let expected_size = config.width * config.height;
        if raw_bytes.len() != expected_size {
            errors.push(format!(
                "尺寸不匹配 {}: 期望 {}x{}={} 实际 {}",
                sprite_name, config.width, config.height, expected_size, raw_bytes.len()
            ));
            continue;
        }
        
        // Mode X 去平面化
        let linear_pixels = modex_deplane(&raw_bytes, config.width, config.height);
        
        // 保存为灰度 PNG (像素值即调色板索引，确保 build.rs 正确读取)
        save_indexed_png(
            &linear_pixels,
            config.width as u32,
            config.height as u32,
            &output_path,
        );
        
        println!("转换: {} ({}x{}) -> {}", 
            filename, config.width, config.height, output_path.display());
        converted_count += 1;
    }
    
    // 输出统计
    println!("\n========================================");
    println!("转换完成!");
    println!("  成功转换: {} 个精灵", converted_count);
    println!("  跳过: {} 个文件", skipped_count);
    
    if !errors.is_empty() {
        println!("  错误: {} 个", errors.len());
        for err in &errors {
            println!("    - {}", err);
        }
    }
    
    println!("\n输出目录: {}", output_dir.display());
    println!("========================================");
}

/// 转换背景高度表文件 (BOGEN.BK, BOGEN7.BK, BOGEN26.BK, MOUNT.BK)
/// 将 Pascal DB 格式转换为灰度 PNG (宽度=数据长度, 高度=1)
fn convert_background_files(backgrounds_dir: &Path, output_dir: &Path) {
    println!("\n========================================");
    println!("转换背景高度表文件...");
    println!("========================================");
    
    let background_files = ["BOGEN.BK", "BOGEN7.BK", "BOGEN26.BK", "MOUNT.BK"];
    
    for filename in background_files {
        let input_path = backgrounds_dir.join(filename);
        
        if !input_path.exists() {
            println!("警告: 文件不存在: {}", input_path.display());
            continue;
        }
        
        // 读取文件内容
        let content = match fs::read_to_string(&input_path) {
            Ok(c) => c,
            Err(e) => {
                println!("错误: 读取失败 {}: {}", filename, e);
                continue;
            }
        };
        
        // 解析 Pascal DB 文本格式
        let bytes = parse_pascal_db_text(&content);
        
        if bytes.is_empty() {
            println!("警告: 解析失败 {} (无数据)", filename);
            continue;
        }
        
        // 输出文件名 (去掉 .BK 后缀，添加 .png)
        let output_name = filename.replace(".BK", ".png");
        let output_path = output_dir.join(&output_name);
        
        // 保存为灰度 PNG (宽度=数据长度, 高度=1)
        save_grayscale_png(&bytes, bytes.len() as u32, 1, &output_path);
        
        println!("转换: {} ({} 字节) -> {}", 
            filename, bytes.len(), output_path.display());
    }
}

/// 保存为灰度 PNG (用于背景高度表)
fn save_grayscale_png(pixels: &[u8], width: u32, height: u32, output_path: &Path) {
    let file = File::create(output_path).expect("Failed to create PNG file");
    let w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    
    let mut writer = encoder.write_header().expect("Failed to write PNG header");
    writer.write_image_data(pixels).expect("Failed to write PNG data");
}

/// 转换 MPAL256 调色板文件为 RGB PNG (256x1)
fn convert_mpal256(sprites_dir: &Path, output_dir: &Path) {
    println!("\n========================================");
    println!("转换 MPAL256 调色板...");
    println!("========================================");
    
    let mpal256_path = sprites_dir.join("MPAL256");
    if !mpal256_path.exists() {
        println!("警告: MPAL256 文件不存在: {}", mpal256_path.display());
        return;
    }
    
    // 读取并解析 Pascal DB 格式
    let content = match fs::read_to_string(&mpal256_path) {
        Ok(c) => c,
        Err(e) => {
            println!("错误: 读取 MPAL256 失败: {}", e);
            return;
        }
    };
    
    let bytes = parse_pascal_db_text(&content);
    
    if bytes.len() < 768 {
        println!("警告: MPAL256 数据不完整 (期望 768 字节, 实际 {} 字节)", bytes.len());
    }
    
    // 保持 6-bit VGA 值 (0-63)，不进行转换
    // 6-bit 到 8-bit 的转换在 game_runner.rs 的 get_palette_rgba 中统一进行
    let mut rgb_data = Vec::with_capacity(256 * 3);
    for i in 0..256 {
        let r = bytes.get(i * 3).copied().unwrap_or(0);
        let g = bytes.get(i * 3 + 1).copied().unwrap_or(0);
        let b = bytes.get(i * 3 + 2).copied().unwrap_or(0);
        
        // 直接使用 6-bit 值
        rgb_data.push(r);
        rgb_data.push(g);
        rgb_data.push(b);
    }
    
    // 保存为 256x1 RGB PNG
    let output_path = output_dir.join("MPAL256.png");
    save_rgb_png(&rgb_data, 256, 1, &output_path);
    
    println!("转换: MPAL256 (256 色 RGB) -> {}", output_path.display());
}

/// 保存为 RGB PNG (用于调色板)
fn save_rgb_png(pixels: &[u8], width: u32, height: u32, output_path: &Path) {
    let file = File::create(output_path).expect("Failed to create PNG file");
    let w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    
    let mut writer = encoder.write_header().expect("Failed to write PNG header");
    writer.write_image_data(pixels).expect("Failed to write PNG data");
}
