// TmpObj Pascal 转换为 Rust
// 临时对象管理模块 - GPU版本
// 处理游戏中的临时对象如破碎方块、金币、命中效果等

use crate::buffers::{Buffers, H, NH, NV, W, WorldOptions};
use crate::figures::Figures;
use crate::glitter::GlitterSystem;
use crate::gpu::{RenderCommand, SpriteInstance};
use crate::music::MusicPlayer;
use crate::sprites::{SpriteAtlas, SpriteDataManager, SpriteId};

// 常量定义 - 临时对象类型
pub const TP_BROKEN: i32 = 1; // 破碎方块类型
pub const TP_COIN: i32 = 2; // 金币类型
pub const TP_HIT: i32 = 3; // 命中效果类型
pub const TP_FIRE: i32 = 4; // 火焰效果类型
pub const TP_NOTE: i32 = 5; // 音符类型

// 游戏参数常量
pub const BROKEN_DELAY: i32 = 3; // 破碎方块动画延迟
pub const COIN_SPEED: i32 = -4; // 金币初始速度
pub const COIN_DELAY: i32 = 12; // 金币动画延迟
pub const MAX_COIN_Y_VEL: i32 = 6; // 金币最大Y轴速度
pub const HIT_TIME: i32 = 4; // 命中效果持续时间

pub const MAX_TEMP_OBJ: usize = 20; // 最大临时对象数量
pub const MAX_REMOVE: usize = 10; // 最大移除操作数量

// 临时对象记录结构体 - GPU版本简化
#[derive(Debug, Clone)]
pub struct TempRec {
    pub alive: bool,         // 对象是否存活
    pub tp: i32,             // 对象类型
    pub x_pos: i32,          // X坐标位置
    pub y_pos: i32,          // Y坐标位置
    pub h_size: i32,         // 水平尺寸
    pub v_size: i32,         // 垂直尺寸
    pub x_vel: i32,          // X轴速度
    pub y_vel: i32,          // Y轴速度
    pub delay_counter: i32,  // 延迟计数器
}

impl Default for TempRec {
    fn default() -> Self {
        Self {
            alive: false,
            tp: 0,
            x_pos: 0,
            y_pos: 0,
            h_size: 0,
            v_size: 0,
            x_vel: 0,
            y_vel: 0,
            delay_counter: 0,
        }
    }
}

// 移除操作记录结构体
#[derive(Debug, Clone, Copy)]
pub struct RemoveRec {
    pub active: bool,   // 是否激活
    pub rem_count: i32, // 移除计数
    pub rem_x: i32,     // 移除区域X坐标
    pub rem_y: i32,     // 移除区域Y坐标
    pub rem_w: i32,     // 移除区域宽度
    pub rem_h: i32,     // 移除区域高度
    pub new_image: i32, // 新图像ID
}

impl Default for RemoveRec {
    fn default() -> Self {
        Self {
            active: false,
            rem_count: 0,
            rem_x: 0,
            rem_y: 0,
            rem_w: 0,
            rem_h: 0,
            new_image: 0,
        }
    }
}

// 临时对象管理器结构体
pub struct TmpObjManager {
    pub temp_obj: Vec<TempRec>, // 临时对象数组
    pub rem_list: Vec<RemoveRec>, // 移除列表数组
}

impl TmpObjManager {
    // 构造函数 - 初始化临时对象管理器
    pub fn new() -> Self {
        Self {
            temp_obj: vec![TempRec::default(); MAX_TEMP_OBJ],
            rem_list: vec![RemoveRec::default(); MAX_REMOVE],
        }
    }

    /// GPU版 - 初始化临时对象系统
    pub fn init_temp_obj(
        &mut self,
        options: &WorldOptions,
        sprites: &mut SpriteDataManager,
        figures: &mut Figures,
    ) {
        // 初始化所有临时对象为非存活状态
        for i in 0..MAX_TEMP_OBJ {
            self.temp_obj[i].alive = false;
        }

        // 初始化移除列表为非激活状态
        for i in 0..MAX_REMOVE {
            self.rem_list[i].active = false;
        }

        // 重新着色方块
        figures.recolor(&mut sprites.PART_000, None, options.brick_color);
    }

    // GPU模式下不需要保存背景

    // GPU版 - 检查临时对象槽位是否可用
    fn available(&self, i: usize) -> bool {
        !self.temp_obj[i].alive
    }

    /// GPU版 - 创建新的临时对象
    pub fn new_temp_obj(
        &mut self,
        new_type: i32,
        x: i32,
        y: i32,
        xv: i32,
        yv: i32,
        wid: i32,
        ht: i32,
        buffers: &Buffers,
    ) {
        // 对破碎方块进行边界检查
        if new_type == TP_BROKEN {
            if xv > 0 {
                if x + 32 * xv > buffers.x_view + NH as i32 * W as i32 + 2 * W as i32 {
                    return;
                }
            } else if x + 32 * xv + 2 * (W as i32) < buffers.x_view {
                return;
            }
        }

        // 寻找可用的临时对象槽位
        let mut i = 0;
        while i < MAX_TEMP_OBJ && !self.available(i) {
            i += 1;
        }

        // 如果找到可用槽位，初始化新对象
        if i < MAX_TEMP_OBJ {
            let temp_obj = &mut self.temp_obj[i];
            temp_obj.alive = true;
            temp_obj.tp = new_type;
            temp_obj.x_pos = x;
            temp_obj.y_pos = y;
            temp_obj.x_vel = xv;
            temp_obj.y_vel = yv;
            temp_obj.h_size = wid;
            temp_obj.v_size = ht;
            temp_obj.delay_counter = 0;
        }
    }

    /// GPU渲染: 收集所有临时对象的精灵实例
    pub fn collect_temp_obj_sprites_gpu(
        &self,
        commands: &mut Vec<RenderCommand>,
        buffers: &Buffers,
        atlas: &SpriteAtlas,
    ) {
        for i in 0..MAX_TEMP_OBJ {
            if !self.temp_obj[i].alive {
                continue;
            }
            
            let temp_obj = &self.temp_obj[i];
            
            // 检查是否在可视区域内
            if temp_obj.x_pos + temp_obj.h_size < buffers.x_view
                || temp_obj.x_pos > buffers.x_view + crate::render_state::SCREEN_WIDTH as i32
                || temp_obj.y_pos + temp_obj.v_size < buffers.y_view
                || temp_obj.y_pos > buffers.y_view + crate::render_state::SCREEN_HEIGHT as i32
            {
                continue;
            }
            
            // 计算屏幕坐标
            let sx = (temp_obj.x_pos - buffers.x_view) as f32;
            let sy = (temp_obj.y_pos - buffers.y_view) as f32;
            
            // 根据对象类型选择精灵
            let sprite_id = match temp_obj.tp {
                TP_BROKEN => SpriteId::PART_000,
                TP_COIN => SpriteId::COIN_000,
                TP_HIT => SpriteId::WHHIT_000,
                TP_FIRE => SpriteId::WHFIRE_000,
                TP_NOTE => SpriteId::NOTE_000,
                _ => continue,
            };
            
            let uv = atlas.get(sprite_id);
            let (u, v, u_size, v_size) = uv.normalized(atlas.size());
            let inst = SpriteInstance::new(
                sx, sy,
                uv.width as f32, uv.height as f32,
                u, v, u_size, v_size
            );
            commands.push(RenderCommand::DrawSprite(inst));
        }
    }

    // 移动所有临时对象
    pub fn move_temp_obj(&mut self, glitter_sys: &mut GlitterSystem, buffers: &mut Buffers) {
        for i in 0..MAX_TEMP_OBJ {
            if self.temp_obj[i].alive {
                let temp_obj = &mut self.temp_obj[i];

                match temp_obj.tp {
                    TP_BROKEN => {
                        // 处理破碎方块物理
                        temp_obj.delay_counter += 1;
                        if temp_obj.delay_counter > BROKEN_DELAY {
                            temp_obj.delay_counter = 0;
                            temp_obj.y_vel += 1; // 重力加速度

                            // 边界检查
                            if temp_obj.y_pos > NV as i32 * H as i32 {
                                temp_obj.alive = false;
                            }
                        }
                    }
                    TP_COIN => {
                        // 处理金币物理
                        temp_obj.delay_counter += 1;
                        if temp_obj.delay_counter > COIN_DELAY {
                            temp_obj.y_vel += 1; // 重力加速度
                            if temp_obj.y_vel > MAX_COIN_Y_VEL {
                                temp_obj.alive = false;
                                // 创建金币闪光效果
                                glitter_sys.coin_glitter(
                                    temp_obj.x_pos + temp_obj.x_vel,
                                    temp_obj.y_pos + temp_obj.y_vel,
                                    buffers,
                                );
                            }
                        }
                    }
                    TP_HIT | TP_FIRE => {
                        // 处理命中和火焰效果
                        temp_obj.delay_counter += 1;
                        if temp_obj.delay_counter > HIT_TIME {
                            temp_obj.alive = false;
                        }
                    }
                    _ => {}
                }

                // 更新位置
                temp_obj.x_pos += temp_obj.x_vel;
                temp_obj.y_pos += temp_obj.y_vel;
            }
        }
    }

    // 添加移除操作到队列
    pub fn remove(&mut self, x: i32, y: i32, w: i32, h: i32, new_img: i32) {
        if y < 0 {
            return;
        }

        // 寻找可用的移除槽位
        let mut i = 0;
        while i < MAX_REMOVE && self.rem_list[i].active {
            i += 1;
        }

        // 如果找到可用槽位，添加移除操作
        if i < MAX_REMOVE {
            let rem_rec = &mut self.rem_list[i];
            rem_rec.rem_x = x;
            rem_rec.rem_y = y;
            rem_rec.rem_w = w;
            rem_rec.rem_h = h;
            rem_rec.new_image = new_img;
            rem_rec.rem_count = 4;  // GPU模式：只需要几帧的延迟
            rem_rec.active = true;
        }
    }

    /// GPU版 - 破坏方块 - 创建破碎效果
    pub fn break_block(
        &mut self,
        x: i32,
        y: i32,
        buffers: &mut Buffers,
        music_player: &MusicPlayer,
    ) {
        // Pascal: WorldMap^[X,Y] := ' ';
        buffers.world_set(x, y, b' ');

        let pixel_x = x * W as i32;
        let pixel_y = y * H as i32;

        // 移除原方块
        self.remove(pixel_x, pixel_y, W as i32, H as i32, 0);

        // 创建四个破碎片段
        let w_half = W as i32 / 2;
        let h_half = H as i32 / 2;

        // 左上片段
        self.new_temp_obj(TP_BROKEN, pixel_x, pixel_y, -2, -6, 12, h_half, buffers);
        // 右上片段
        self.new_temp_obj(TP_BROKEN, pixel_x + w_half, pixel_y, 2, -6, 12, h_half, buffers);
        // 左下片段
        self.new_temp_obj(TP_BROKEN, pixel_x, pixel_y + h_half, -2, -4, 12, h_half, buffers);
        // 右下片段
        self.new_temp_obj(TP_BROKEN, pixel_x + w_half, pixel_y + h_half, 2, -4, 12, h_half, buffers);

        // 播放音效
        music_player.beep(110);
    }

    /// GPU版 - 命中金币 - 处理金币收集
    pub fn hit_coin(
        &mut self,
        x: i32,
        y: i32,
        throw_up: bool,
        glitter_sys: &mut GlitterSystem,
        buffers: &mut Buffers,
        music_player: &MusicPlayer,
    ) {
        // Pascal: X/Y 是像素世界坐标
        let map_x = x / W as i32;
        let map_y = y / H as i32;

        if throw_up {
            // 创建向上抛的金币动画
            self.new_temp_obj(
                TP_COIN, x, y - H as i32, 0, COIN_SPEED, W as i32, H as i32, buffers,
            );
        } else {
            // 直接移除金币
            buffers.world_set(map_x, map_y, b' ');
            self.remove(x, y, W as i32, H as i32, 0);
            glitter_sys.coin_glitter(x, y, buffers);
        }

        // 播放音效和处理得分
        music_player.beep(2420);
        buffers.data.coins[buffers.player] += 1;
        buffers.add_score(50);

        // 每100枚金币加1命，并把金币数清零
        if buffers.data.coins[buffers.player] % 100 == 0 {
            self.add_life(buffers, music_player);
            buffers.data.coins[buffers.player] = 0;
        }
    }

    // 增加生命
    pub fn add_life(&self, buffers: &mut Buffers, music_player: &MusicPlayer) {
        // 生命增加逻辑
        buffers.data.lives[buffers.player] += 1;
        music_player.play_life();
    }
}
