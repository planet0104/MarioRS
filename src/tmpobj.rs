// TmpObj Pascal 转换为 Rust
// 临时对象管理模块 - 处理游戏中的临时对象如破碎方块、金币、命中效果等

use crate::backgr::BackGr;
use crate::buffers::{Buffers, H, NH, NV, W, WorldOptions};
use crate::figures::Figures;
use crate::glitter::GlitterSystem;
use crate::music::MusicPlayer;
use crate::sprites::SpriteDataManager;
use crate::vga256::{MAX_PAGE, VGA};

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

// 临时对象记录结构体
#[derive(Debug, Clone)]
pub struct TempRec {
    pub alive: bool,                            // 对象是否存活
    pub visible: [bool; MAX_PAGE as usize + 1], // 各页面可见性
    pub tp: i32,                                // 对象类型
    // Pascal: BackGrAddr: array[0..MAX_PAGE] of Integer; 0 表示无效（句柄版，避免 x>255 截断）
    pub back_gr: Vec<i32>,
    pub x_pos: i32,                          // X坐标位置
    pub y_pos: i32,                          // Y坐标位置
    pub h_size: i32,                         // 水平尺寸
    pub v_size: i32,                         // 垂直尺寸
    pub x_vel: i32,                          // X轴速度
    pub y_vel: i32,                          // Y轴速度
    pub delay_counter: i32,                  // 延迟计数器
    pub old_x: [i32; MAX_PAGE as usize + 1], // 旧X坐标
    pub old_y: [i32; MAX_PAGE as usize + 1], // 旧Y坐标
}

impl Default for TempRec {
    fn default() -> Self {
        Self {
            alive: false,
            visible: [false; MAX_PAGE as usize + 1],
            tp: 0,
            back_gr: vec![0; MAX_PAGE as usize + 1],
            x_pos: 0,
            y_pos: 0,
            h_size: 0,
            v_size: 0,
            x_vel: 0,
            y_vel: 0,
            delay_counter: 0,
            old_x: [0; MAX_PAGE as usize + 1],
            old_y: [0; MAX_PAGE as usize + 1],
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
                                // pub current_page: usize,                 // 当前页面 - 待实现
                                // pub working_page: usize,                 // 工作页面 - 待实现
}

impl TmpObjManager {
    // 构造函数 - 初始化临时对象管理器
    pub fn new() -> Self {
        Self {
            temp_obj: vec![TempRec::default(); MAX_TEMP_OBJ],
            rem_list: vec![RemoveRec::default(); MAX_REMOVE],
        }
    }

    // 初始化临时对象系统
    pub fn init_temp_obj(
        &mut self,
        options: &WorldOptions,
        sprites: &mut SpriteDataManager,
        figures: &mut Figures,
    ) {
        // 初始化所有临时对象为非存活状态
        for i in 0..MAX_TEMP_OBJ {
            self.temp_obj[i].alive = false;
            for j in 0..=MAX_PAGE as usize {
                self.temp_obj[i].visible[j] = false;
            }
        }

        // 初始化移除列表为非激活状态
        for i in 0..MAX_REMOVE {
            self.rem_list[i].active = false;
        }

        // 重新着色方块
        figures.recolor(&mut sprites.PART_000, None, options.brick_color);
    }

    // 读取背景图像数据
    fn read_back_gr(&mut self, i: usize, vga: &mut VGA) {
        let temp_obj = &mut self.temp_obj[i];
        // 读取背景图像
        let current_page = vga.current_page() as usize;
        temp_obj.back_gr[current_page] = vga.push_backgr_address_world(
            temp_obj.x_pos,
            temp_obj.y_pos,
            temp_obj.h_size + 4,
            temp_obj.v_size,
        );
        temp_obj.old_x[current_page] = temp_obj.x_pos;
        temp_obj.old_y[current_page] = temp_obj.y_pos;
    }

    // 检查临时对象槽位是否可用
    fn available(&self, i: usize) -> bool {
        let temp_obj = &self.temp_obj[i];
        let mut used = temp_obj.alive;

        // 检查所有页面的可见性
        for j in 0..=MAX_PAGE as usize {
            used = used || temp_obj.visible[j];
        }

        !used
    }

    // 创建新的临时对象
    pub fn new_temp_obj(
        &mut self,
        new_type: i32,
        x: i32,
        y: i32,
        xv: i32,
        yv: i32,
        wid: i32,
        ht: i32,
        vga: &mut VGA,
        buffers: &mut Buffers,
    ) {
        // 对破碎方块进行边界检查
        if new_type == TP_BROKEN {
            // 边界检查
            if xv > 0 {
                if x + 32 * xv > buffers.x_view + NH as i32 * W as i32 + 2 * W as i32 {
                    return;
                }
            } else {
                if x + 32 * xv + 2 * (W as i32) < buffers.x_view {
                    return;
                }
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

            // 重置所有页面的可见性
            for j in 0..=MAX_PAGE as usize {
                temp_obj.visible[j] = false;
            }

            // 设置对象属性
            temp_obj.tp = new_type;
            temp_obj.x_pos = x;
            temp_obj.y_pos = y;
            temp_obj.x_vel = xv;
            temp_obj.y_vel = yv;
            temp_obj.h_size = wid;
            temp_obj.v_size = ht;
            temp_obj.delay_counter = 0;

            // 读取背景
            self.read_back_gr(i, vga);
        }
    }

    // 显示所有临时对象
    pub fn show_temp_obj(&mut self, vga: &mut VGA, sprites: &SpriteDataManager) {
        for i in 0..MAX_TEMP_OBJ {
            if self.temp_obj[i].alive {
                self.read_back_gr(i, vga);

                let temp_obj = &self.temp_obj[i];

                // 根据对象类型绘制相应图像
                match temp_obj.tp {
                    TP_BROKEN => {
                        vga.draw_image_imagebuffer_world(
                            temp_obj.x_pos,
                            temp_obj.y_pos,
                            &sprites.PART_000,
                        );
                    }
                    TP_COIN => {
                        vga.draw_image_imagebuffer_world(
                            temp_obj.x_pos,
                            temp_obj.y_pos,
                            &sprites.COIN_000,
                        );
                    }
                    TP_HIT => {
                        vga.draw_image_imagebuffer_world(
                            temp_obj.x_pos,
                            temp_obj.y_pos,
                            &sprites.WHHIT_000,
                        );
                    }
                    TP_FIRE => {
                        vga.draw_image_imagebuffer_world(
                            temp_obj.x_pos,
                            temp_obj.y_pos,
                            &sprites.WHFIRE_000,
                        );
                    }
                    TP_NOTE => {
                        vga.draw_image_imagebuffer_world(
                            temp_obj.x_pos,
                            temp_obj.y_pos,
                            &sprites.NOTE_000,
                        );
                    }
                    _ => {}
                }

                // 设置当前页面可见性
                self.temp_obj[i].visible[vga.current_page() as usize] = true;
            }
        }
    }

    // 隐藏所有临时对象
    pub fn hide_temp_obj(&mut self, vga: &mut VGA) {
        for i in (0..MAX_TEMP_OBJ).rev() {
            // 页面可见性检查和背景恢复
            let current_page = vga.current_page() as usize;
            if self.temp_obj[i].visible[current_page] {
                let addr = self.temp_obj[i].back_gr[current_page];
                if addr != 0 {
                    vga.pop_backgr_address(addr);
                    self.temp_obj[i].back_gr[current_page] = 0;
                }
                self.temp_obj[i].visible[current_page] = false;
            }
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
            rem_rec.rem_count = (MAX_PAGE + 1) as i32 + 1;
            rem_rec.active = true;
        }
    }

    // 执行移除操作
    pub fn run_remove(
        &mut self,
        vga: &mut VGA,
        backgr: &mut BackGr,
        sprites: &SpriteDataManager,
        options: &WorldOptions,
    ) {
        for i in 0..MAX_REMOVE {
            if self.rem_list[i].active {
                let rem_rec = &mut self.rem_list[i];

                // 根据新图像类型执行相应绘制操作
                match rem_rec.new_image {
                    0 => {
                        backgr.draw_backgr_block(
                            rem_rec.rem_x,
                            rem_rec.rem_y,
                            rem_rec.rem_w,
                            rem_rec.rem_h,
                            vga,
                            options,
                            sprites,
                        );
                    }
                    1 => {
                        vga.draw_image_imagebuffer_world(
                            rem_rec.rem_x,
                            rem_rec.rem_y,
                            &sprites.QUEST_001,
                        );
                    }
                    2 => {
                        vga.draw_image_imagebuffer_world(
                            rem_rec.rem_x,
                            rem_rec.rem_y,
                            &sprites.QUEST_000,
                        );
                    }
                    5 => {
                        vga.draw_image_imagebuffer_world(
                            rem_rec.rem_x,
                            rem_rec.rem_y,
                            &sprites.NOTE_000,
                        );
                    }
                    _ => {}
                }

                rem_rec.rem_count -= 1;
                if rem_rec.rem_count < 1 {
                    rem_rec.active = false;
                }
            }
        }
    }

    // 破坏方块 - 创建破碎效果
    pub fn break_block(
        &mut self,
        x: i32,
        y: i32,
        vga: &mut VGA,
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
        self.new_temp_obj(
            TP_BROKEN, pixel_x, pixel_y, -2, -6, 12, h_half, vga, buffers,
        );
        // 右上片段
        self.new_temp_obj(
            TP_BROKEN,
            pixel_x + w_half,
            pixel_y,
            2,
            -6,
            12,
            h_half,
            vga,
            buffers,
        );
        // 左下片段
        self.new_temp_obj(
            TP_BROKEN,
            pixel_x,
            pixel_y + h_half,
            -2,
            -4,
            12,
            h_half,
            vga,
            buffers,
        );
        // 右下片段
        self.new_temp_obj(
            TP_BROKEN,
            pixel_x + w_half,
            pixel_y + h_half,
            2,
            -4,
            12,
            h_half,
            vga,
            buffers,
        );

        // 播放音效
        music_player.beep(110);
    }

    // 命中金币 - 处理金币收集
    pub fn hit_coin(
        &mut self,
        x: i32,
        y: i32,
        throw_up: bool,
        vga: &mut VGA,
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
                TP_COIN,
                x,
                y - H as i32,
                0,
                COIN_SPEED,
                W as i32,
                H as i32,
                vga,
                buffers,
            );
        } else {
            // 直接移除金币
            buffers.world_set(map_x, map_y, b' ');
            self.remove(x, y, W as i32, H as i32, 0);
            glitter_sys.coin_glitter(x, y, buffers);
        }

        // 播放音效和处理得分
        music_player.beep(2420);
        // Pascal 中 StartMusic(CoinMusic) 被注释掉了，只播放 beep(2420)
        // music_player.play_coin();
        buffers.data.coins[buffers.player] += 1; // 玩家金币数加 1
        buffers.add_score(50);

        // 每 100 枚金币加 1 命，并把金币数清零
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
