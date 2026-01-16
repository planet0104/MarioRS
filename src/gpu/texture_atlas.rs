// 纹理图集管理 - 将多个精灵打包到单个纹理中

use std::collections::HashMap;

// 图集中精灵的UV信息
#[derive(Clone, Copy, Debug)]
pub struct SpriteUV {
    // 在图集中的像素位置
    pub x: u32,
    pub y: u32,
    // 精灵尺寸
    pub width: u32,
    pub height: u32,
}

impl SpriteUV {
    // 计算归一化UV坐标 (0.0-1.0)
    pub fn normalized(&self, atlas_size: u32) -> (f32, f32, f32, f32) {
        let atlas_f = atlas_size as f32;
        (
            self.x as f32 / atlas_f,
            self.y as f32 / atlas_f,
            self.width as f32 / atlas_f,
            self.height as f32 / atlas_f,
        )
    }
}

// 简单的行打包器 (Shelf Packer)
struct ShelfPacker {
    atlas_size: u32,
    current_x: u32,
    current_y: u32,
    shelf_height: u32,
}

impl ShelfPacker {
    fn new(atlas_size: u32) -> Self {
        Self {
            atlas_size,
            current_x: 0,
            current_y: 0,
            shelf_height: 0,
        }
    }

    // 分配空间给精灵
    fn allocate(&mut self, width: u32, height: u32) -> Option<(u32, u32)> {
        // 检查当前行是否够空间
        if self.current_x + width > self.atlas_size {
            // 换到下一行
            self.current_y += self.shelf_height;
            self.current_x = 0;
            self.shelf_height = 0;
        }

        // 检查是否超出图集
        if self.current_y + height > self.atlas_size {
            return None;
        }

        let result = (self.current_x, self.current_y);
        self.current_x += width;
        self.shelf_height = self.shelf_height.max(height);
        Some(result)
    }
}

// 纹理图集构建器
pub struct TextureAtlas {
    // 图集尺寸
    pub size: u32,
    // 图集像素数据 (R8格式, 存储调色板索引)
    pub data: Vec<u8>,
    // 精灵名称到UV的映射
    sprites: HashMap<String, SpriteUV>,
    // 打包器
    packer: ShelfPacker,
}

impl TextureAtlas {
    pub fn new(size: u32) -> Self {
        Self {
            size,
            data: vec![0; (size * size) as usize],
            sprites: HashMap::new(),
            packer: ShelfPacker::new(size),
        }
    }

    // 添加精灵到图集
    pub fn add_sprite(&mut self, name: &str, width: u32, height: u32, pixels: &[u8]) -> Option<SpriteUV> {
        // 分配空间
        let (x, y) = self.packer.allocate(width, height)?;

        // 复制像素数据
        for row in 0..height {
            let src_start = (row * width) as usize;
            let dst_start = ((y + row) * self.size + x) as usize;
            self.data[dst_start..dst_start + width as usize]
                .copy_from_slice(&pixels[src_start..src_start + width as usize]);
        }

        let uv = SpriteUV { x, y, width, height };
        self.sprites.insert(name.to_string(), uv);
        Some(uv)
    }

    // 获取精灵UV
    pub fn get_sprite(&self, name: &str) -> Option<&SpriteUV> {
        self.sprites.get(name)
    }

    // 获取所有精灵
    pub fn sprites(&self) -> &HashMap<String, SpriteUV> {
        &self.sprites
    }

    // 获取图集数据
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}
