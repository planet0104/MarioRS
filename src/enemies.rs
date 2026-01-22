use crate::buffers::*;
use crate::figures::Figures;
use crate::glitter::*;
use crate::gpu::{RenderCommand, SpriteInstance};
use crate::music::*;
use crate::render_state::*;
use crate::sprites::SpriteDataManager;
use crate::tmpobj::TP_FIRE;
use crate::tmpobj::TP_HIT;
use crate::tmpobj::TmpObjManager;
use crate::utils::*;

// Constants
pub const START_ENEMIES_AT: i32 = 2;
pub const FORGET_ENEMIES_AT: i32 = 5;

// Pascal风格的敌人类型常量定义
pub const TP_DEAD: i32 = 0;
pub const TP_DYING: i32 = 1;
pub const TP_CHIBIBO: i32 = 2;
pub const TP_FLAT_CHIBIBO: i32 = 3;
pub const TP_DEAD_CHIBIBO: i32 = 4;
pub const TP_RISING_CHAMP: i32 = 5;
pub const TP_CHAMP: i32 = 6;
pub const TP_RISING_LIFE: i32 = 7;
pub const TP_LIFE: i32 = 8;
pub const TP_RISING_FLOWER: i32 = 9;
pub const TP_FLOWER: i32 = 10;
pub const TP_RISING_STAR: i32 = 11;
pub const TP_STAR: i32 = 12;
pub const TP_FIREBALL: i32 = 13;
pub const TP_DYING_FIREBALL: i32 = 14;
pub const TP_VERT_FISH: i32 = 15;
pub const TP_DEAD_VERT_FISH: i32 = 16;
pub const TP_VERT_FIREBALL: i32 = 17;
pub const TP_VERT_PLANT: i32 = 18;
pub const TP_DEAD_VERT_PLANT: i32 = 19;
pub const TP_RED: i32 = 20;
pub const TP_DEAD_RED: i32 = 21;

pub const TP_KOOPA: i32 = 50;
pub const TP_SLEEPING_KOOPA: i32 = 51;
pub const TP_WAKING_KOOPA: i32 = 52;
pub const TP_RUNNING_KOOPA: i32 = 53;
pub const TP_DYING_KOOPA: i32 = 54;
pub const TP_DEAD_KOOPA: i32 = 55;

pub const TP_LIFT_START: i32 = 60;
pub const TP_BLOCK_LIFT: i32 = 60;
pub const TP_DONUT: i32 = 61;
// 62~68 可补充其它Lift类型
pub const TP_LIFT_END: i32 = 69;

// Directions
const LEFT: usize = 0;
const RIGHT: usize = 1;

// Koopa colors
const K_GREEN: usize = 0;
const K_RED: usize = 1;

// Status constants
// 敌人状态常量定义
pub const ENEMY_GROUNDED: i32 = 0;
pub const ENEMY_FALLING: i32 = 1;

const MAX_ENEMIES: usize = 11;
const MAX_ENEMIES_AT_ONCE: usize = 25;

#[derive(Clone, Debug)]
struct EnemyRec {
    tp: i32,
    sub_tp: i32,
    x_pos: i32,
    y_pos: i32,
    last_x_pos: i32,
    last_y_pos: i32,
    map_x: i32,
    map_y: i32,
    x_vel: i32,
    y_vel: i32,
    move_delay: i32,
    delay_counter: i32,
    counter: i32,
    status: i32,
    dir_counter: u8,
}

impl Default for EnemyRec {
    fn default() -> Self {
        EnemyRec {
            tp: TP_DEAD,
            sub_tp: 0,
            x_pos: 0,
            y_pos: 0,
            last_x_pos: 0,
            last_y_pos: 0,
            map_x: 0,
            map_y: 0,
            x_vel: 0,
            y_vel: 0,
            move_delay: 0,
            delay_counter: 0,
            counter: 0,
            status: ENEMY_GROUNDED,
            dir_counter: 0,
        }
    }
}

pub struct Enemies {
    enemy_pictures: [[ImageBuffer; 2]; MAX_ENEMIES + 1],
    enemies: Vec<EnemyRec>,
    active_enemies: Vec<usize>,
    num_enemies: usize,
    time_counter: u8,
    fire_ball_list: [ImageBuffer; 4],
    pub koopa_list: [[[ImageBuffer20x24; 2]; 2]; 2],
    pub cd_champ: u8,
    pub cd_life: u8,
    pub cd_flower: u8,
    pub cd_star: u8,
    pub cd_enemy: i32,
    pub cd_hit: i32,
    pub cd_lift: i32,
    pub cd_stop_jump: u8,
    pub player_x1: i32,
    pub player_y1: i32,
    pub player_x2: i32,
    pub player_y2: i32,
    pub player_x_vel: i32,
    pub player_y_vel: i32,
    pub star: bool,
    pub turbo: bool,
}

impl Enemies {
    pub fn new(sprites: &SpriteDataManager) -> Self {
        Enemies {
            enemy_pictures: [[ImageBuffer::default(); 2]; MAX_ENEMIES + 1],
            enemies: vec![EnemyRec::default(); MAX_ENEMIES_AT_ONCE],
            active_enemies: Vec::new(),
            num_enemies: 0,
            time_counter: 0,
            fire_ball_list: [
                sprites.F_000.clone(),
                sprites.F_001.clone(),
                sprites.F_002.clone(),
                sprites.F_003.clone(),
            ],
            koopa_list: [
                [
                    // LEFT
                    [sprites.GRKOOPA_000.clone(), sprites.GRKOOPA_001.clone()], // kGreen
                    [sprites.RDKOOPA_000.clone(), sprites.RDKOOPA_001.clone()], // kRed
                ],
                [
                    // RIGHT
                    [sprites.GRKOOPA_000.clone(), sprites.GRKOOPA_001.clone()], // kGreen
                    [sprites.RDKOOPA_000.clone(), sprites.RDKOOPA_001.clone()], // kRed
                ],
            ],

            cd_champ: 0,
            cd_life: 0,
            cd_flower: 0,
            cd_star: 0,
            cd_enemy: 0,
            cd_hit: 0,
            cd_lift: 0,
            cd_stop_jump: 0,
            player_x1: 0,
            player_y1: 0,
            player_x2: 0,
            player_y2: 0,
            player_x_vel: 0,
            player_y_vel: 0,
            star: false,
            turbo: false,
        }
    }

    fn kill(&mut self, i: usize, buffers: &mut Buffers) {
        let enemy = &mut self.enemies[i];
        match enemy.tp {
            TP_CHIBIBO => {
                enemy.tp = TP_DEAD_CHIBIBO;
                enemy.x_vel = -1 + 2 * (((enemy.x_pos + enemy.x_vel) % W > W / 2) as i32);
                enemy.y_vel = -4;
                enemy.move_delay = 0;
                enemy.delay_counter = 0;
                buffers.add_score(100);
            }
            TP_RED => {
                enemy.tp = TP_DEAD_RED;
                enemy.x_vel = -1 + 2 * (((enemy.x_pos + enemy.x_vel) % W > W / 2) as i32);
                enemy.y_vel = -4;
                enemy.move_delay = 0;
                enemy.delay_counter = 0;
                buffers.add_score(100);
            }
            TP_KOOPA | TP_SLEEPING_KOOPA | TP_WAKING_KOOPA | TP_RUNNING_KOOPA => {
                enemy.tp = TP_DEAD_KOOPA;
                enemy.x_vel = -1 + 2 * (((enemy.x_pos + enemy.x_vel) % W > W / 2) as i32);
                enemy.y_vel = -4;
                enemy.move_delay = 0;
                enemy.delay_counter = 0;
                buffers.add_score(100);
            }
            TP_VERT_FISH => {
                enemy.tp = TP_DEAD_VERT_FISH;
                enemy.x_vel = 0;
                enemy.y_vel = 0;
                enemy.move_delay = 2;
                enemy.delay_counter = 0;
                enemy.status = ENEMY_FALLING;
                buffers.add_score(100);
            }
            TP_VERT_PLANT => {
                enemy.tp = TP_DEAD_VERT_PLANT;
                enemy.delay_counter = 0;
                enemy.y_vel = 0;
                // 对齐 原版：死亡特效必须固定在击中位置，不能回到管道口
                enemy.status = 0;
                enemy.last_x_pos = enemy.x_pos;
                enemy.last_y_pos = enemy.y_pos;
                buffers.add_score(100);
            }
            _ => {}
        }
    }

    fn show_star(
        &self,
        x: i32,
        y: i32,
        _render_state: &mut RenderState,
        buffers: &mut Buffers,
        tmp_obj_manager: &mut TmpObjManager,
        music_player: &MusicPlayer,
    ) {
        music_player.beep(100);
        if x + W > buffers.x_view && x < buffers.x_view + SCREEN_WIDTH as i32 {
            tmp_obj_manager.new_temp_obj(TP_HIT, x, y, 0, 0, W, H, buffers);
        }
    }

    fn show_fire(
        &self,
        x: i32,
        y: i32,
        _render_state: &mut RenderState,
        buffers: &mut Buffers,
        tmp_obj_manager: &mut TmpObjManager,
        music_player: &MusicPlayer,
    ) {
        music_player.beep(50);
        let x = x - 4;
        let y = y - 4;
        if x + W > buffers.x_view && x < buffers.x_view + SCREEN_WIDTH as i32 {
            tmp_obj_manager.new_temp_obj(TP_FIRE, x, y, 0, 0, W, H, buffers);
        }
    }

    pub fn init_enemy_figures(&mut self, figures: &mut Figures, sprites: &SpriteDataManager) {
        // 加载 Chibibo 敌人图片
        self.enemy_pictures[1][RIGHT] = sprites.CHIBIBO_000.clone();
        self.enemy_pictures[2][RIGHT] = sprites.CHIBIBO_001.clone();
        self.enemy_pictures[4][RIGHT] = sprites.CHIBIBO_002.clone();
        self.enemy_pictures[5][RIGHT] = sprites.CHIBIBO_003.clone();

        // Fish
        self.enemy_pictures[3][LEFT] = sprites.FISH_001.clone();
        self.enemy_pictures[3][RIGHT] = figures.mirror(&self.enemy_pictures[3][LEFT]);

        // Red
        self.enemy_pictures[6][LEFT] = sprites.RED_000.clone();
        self.enemy_pictures[7][LEFT] = sprites.RED_001.clone();

        // Green Koopa
        self.enemy_pictures[8][RIGHT] = sprites.GRKP_000.clone();
        self.enemy_pictures[9][RIGHT] = sprites.GRKP_001.clone();

        // Red Koopa
        self.enemy_pictures[10][RIGHT] = sprites.RDKP_000.clone();
        self.enemy_pictures[11][RIGHT] = sprites.RDKP_001.clone();

        // 镜像处理
        for i in 1..=MAX_ENEMIES {
            if i == 6 || i == 7 {
                self.enemy_pictures[i][RIGHT] = figures.mirror(&self.enemy_pictures[i][LEFT]);
            } else if i != 3 {
                self.enemy_pictures[i][LEFT] = figures.mirror(&self.enemy_pictures[i][RIGHT]);
            }
        }

        // Koopa 镜像
        for i in 0..=1 {
            for j in K_GREEN..=K_RED {
                self.koopa_list[RIGHT][j][i] = figures.mirror(&self.koopa_list[LEFT][j][i]);
            }
        }
    }

    pub fn clear_enemies(&mut self) {
        for enemy in &mut self.enemies {
            enemy.tp = TP_DEAD;
        }
        self.num_enemies = 0;
        self.active_enemies.clear();
        self.cd_champ = 0;
        self.cd_life = 0;
        self.cd_flower = 0;
        self.cd_star = 0;
        self.cd_enemy = 0;
        self.cd_hit = 0;
        self.cd_lift = 0;
        self.cd_stop_jump = 0;
    }

    pub fn stop_enemies(&mut self, buffers: &mut Buffers) {
        for i in 0..self.num_enemies {
            let j = self.active_enemies[i] as usize;
            let enemy = &self.enemies[j];
            match enemy.tp {
                TP_CHIBIBO => {
                    buffers.world_set(enemy.map_x, enemy.map_y, 0x80);
                }
                TP_VERT_FISH => {
                    buffers.world_set(enemy.map_x, enemy.map_y - 2, 0x81);
                }
                TP_VERT_FIREBALL => {
                    buffers.world_set(enemy.map_x, enemy.map_y - 2, 0x82);
                }
                TP_VERT_PLANT => {
                    let ch = (0x84 + enemy.sub_tp) as u8;
                    buffers.world_set(enemy.map_x, enemy.map_y - 2, ch);
                }
                TP_RED => {
                    buffers.world_set(enemy.map_x, enemy.map_y, 0x87);
                }
                TP_KOOPA | TP_SLEEPING_KOOPA | TP_WAKING_KOOPA | TP_RUNNING_KOOPA => {
                    let ch = (0x88 + enemy.sub_tp) as u8;
                    buffers.world_set(enemy.map_x, enemy.map_y, ch);
                }
                TP_BLOCK_LIFT => {
                    buffers.world_set(enemy.map_x, enemy.map_y, 0xb0);
                }
                TP_DONUT => {
                    buffers.world_set(enemy.map_x, enemy.map_y, 0xb1);
                }
                _ => {}
            }
        }
        self.clear_enemies();
    }

    pub fn new_enemy(
        &mut self,
        init_type: i32,
        sub_type: i32,
        init_x: i32,
        init_y: i32,
        init_x_vel: i32,
        init_y_vel: i32,
        init_delay: i32,
        music_player: &MusicPlayer,
    ) {
        let mut init_x_vel = init_x_vel;
        let mut init_y_vel = init_y_vel;
        let mut init_delay = init_delay;

        if self.turbo {
            init_x_vel *= 2;
            init_y_vel *= 2;
            init_delay /= 2;
        }

        if init_type == TP_FIREBALL {
            let mut fire_ball_count = 0;
            for i in 0..self.num_enemies {
                let j = self.active_enemies[i] as usize;
                if self.enemies[j].tp == TP_FIREBALL {
                    fire_ball_count += 1;
                }
            }
            if fire_ball_count >= 2 {
                return;
            }
            music_player.play_fire();
        }

        // Find first available enemy slot
        let mut i = 0;
        while i < MAX_ENEMIES_AT_ONCE && self.enemies[i].tp != TP_DEAD {
            i += 1;
        }
        if i >= MAX_ENEMIES_AT_ONCE {
            return; // No available slots
        }

        let enemy = &mut self.enemies[i];
        enemy.tp = init_type;
        enemy.sub_tp = sub_type;
        enemy.map_x = init_x;
        enemy.map_y = init_y;
        enemy.x_pos = init_x * W;
        enemy.y_pos = init_y * H;
        enemy.x_vel = init_x_vel;
        enemy.y_vel = init_y_vel;
        enemy.move_delay = init_delay;
        enemy.delay_counter = 0;
        enemy.dir_counter = 0;
        enemy.status = ENEMY_GROUNDED;
        // GPU渲染每帧完全重绘，不需要背景保存/恢复
        enemy.counter = 0;

        match init_type {
            TP_VERT_PLANT => {
                enemy.x_pos += 8;
                enemy.status = ENEMY_GROUNDED;
            }
            TP_FIREBALL => {
                if init_x_vel > 0 {
                    enemy.x_pos = self.player_x2;
                } else {
                    enemy.x_pos = self.player_x1;
                }
            }
            _ => {}
        }
        enemy.last_x_pos = enemy.x_pos;
        enemy.last_y_pos = enemy.y_pos;

        self.active_enemies.push(i);
        self.num_enemies = self.active_enemies.len();
    }

    /// GPU渲染: 收集所有敌人的精灵实例
    /// 替代show_enemies，不再直接绘制到VGA，而是收集RenderCommand
    /// 使用SpriteId和着色器翻转，而非动态生成的镜像ImageBuffer
    /// 注意：火花效果(glitter)在move_enemies中通过glitter_sys创建
    pub fn collect_enemy_sprites_gpu(
        &self,
        commands: &mut Vec<RenderCommand>,
        buffers: &Buffers,
        atlas: &crate::sprites::SpriteAtlas,
    ) {
        use crate::sprites::SpriteId;

        for &j in self.active_enemies.iter() {
            let j = j as usize;
            if j >= self.enemies.len() {
                continue;
            }
            let enemy = &self.enemies[j];

            // 检查是否在可视区域内
            if (enemy.x_pos as i32 + W < buffers.x_view)
                || (enemy.x_pos > buffers.x_view + SCREEN_WIDTH as i32)
                || (enemy.y_pos >= buffers.y_view + SCREEN_HEIGHT as i32)
            {
                continue;
            }

            // 计算屏幕坐标
            let sx = (enemy.x_pos - buffers.x_view) as f32;
            let sy = (enemy.y_pos - buffers.y_view) as f32;

            match enemy.tp {
                TP_CHIBIBO => {
                    // 对齐 原版: TP_CHIBIBO 使用 FigList[1 + 3*sub_tp] 的单个图像，
                    // 并用 dir_counter 在左右镜像之间切换（作为走路动画效果）。
                    let sprite_id = if enemy.sub_tp == 0 {
                        SpriteId::CHIBIBO_000
                    } else {
                        SpriteId::CHIBIBO_002
                    };
                    let flip_x = !(enemy.dir_counter % 32 < 16);
                    let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, flip_x, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_FLAT_CHIBIBO => {
                    // 被踩扁的栗子怪
                    let sprite_id = if enemy.sub_tp == 0 {
                        SpriteId::CHIBIBO_001
                    } else {
                        SpriteId::CHIBIBO_003
                    };
                    let flip_x = !(enemy.dir_counter % 32 < 16);
                    let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, flip_x, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_DEAD_CHIBIBO => {
                    // 死亡栗子怪 (上下翻转)
                    let inst =
                        self.create_enemy_sprite(atlas, SpriteId::CHIBIBO_000, sx, sy, true, true);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_RISING_CHAMP => {
                    if enemy.y_pos != (enemy.map_y * H) {
                        let visible_h = (H - (enemy.y_pos % H) - 1) as f32;
                        let sprite_id = if enemy.sub_tp == 0 {
                            SpriteId::CHAMP_000
                        } else {
                            SpriteId::POISON_000
                        };
                        let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, false, false);
                        commands.push(RenderCommand::DrawSpritePart {
                            sprite: inst,
                            visible_height: visible_h,
                        });
                    }
                }
                TP_CHAMP => {
                    let sprite_id = if enemy.sub_tp == 0 {
                        SpriteId::CHAMP_000
                    } else {
                        SpriteId::POISON_000
                    };
                    let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, false, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_RISING_LIFE => {
                    if enemy.y_pos != (enemy.map_y * H) {
                        let visible_h = (H - (enemy.y_pos % H) - 1) as f32;
                        let inst = self.create_enemy_sprite(
                            atlas,
                            SpriteId::LIFE_000,
                            sx,
                            sy,
                            false,
                            false,
                        );
                        commands.push(RenderCommand::DrawSpritePart {
                            sprite: inst,
                            visible_height: visible_h,
                        });
                    }
                }
                TP_LIFE => {
                    let inst =
                        self.create_enemy_sprite(atlas, SpriteId::LIFE_000, sx, sy, false, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_RISING_FLOWER => {
                    if enemy.y_pos != (enemy.map_y * H) {
                        let visible_h = (H - (enemy.y_pos % H) - 1) as f32;
                        let inst = self.create_enemy_sprite(
                            atlas,
                            SpriteId::FLOWER_000,
                            sx,
                            sy,
                            false,
                            false,
                        );
                        commands.push(RenderCommand::DrawSpritePart {
                            sprite: inst,
                            visible_height: visible_h,
                        });
                    }
                }
                TP_FLOWER => {
                    let inst =
                        self.create_enemy_sprite(atlas, SpriteId::FLOWER_000, sx, sy, false, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_RISING_STAR => {
                    if enemy.y_pos != (enemy.map_y * H) {
                        let visible_h = (H - (enemy.y_pos % H) - 1) as f32;
                        let inst = self.create_enemy_sprite(
                            atlas,
                            SpriteId::STAR_000,
                            sx,
                            sy,
                            false,
                            false,
                        );
                        commands.push(RenderCommand::DrawSpritePart {
                            sprite: inst,
                            visible_height: visible_h,
                        });
                    }
                }
                TP_STAR => {
                    let inst =
                        self.create_enemy_sprite(atlas, SpriteId::STAR_000, sx, sy, false, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_FIREBALL => {
                    let sprite_id = if enemy.x_pos % 4 < 2 {
                        SpriteId::FIRE_000
                    } else {
                        SpriteId::FIRE_001
                    };
                    let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, false, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_VERT_FISH => {
                    if (enemy.y_vel != 0) || (enemy.y_pos < NV * H - H) {
                        let flip_x = self.player_x1 > enemy.x_pos;
                        let inst = self.create_enemy_sprite(
                            atlas,
                            SpriteId::FISH_001,
                            sx,
                            sy,
                            flip_x,
                            false,
                        );
                        commands.push(RenderCommand::DrawSprite(inst));
                    }
                }
                TP_DEAD_VERT_FISH => {
                    if (enemy.y_pos < NV * H - H) || (enemy.y_vel != 0) {
                        let flip_x = self.player_x1 <= enemy.x_pos;
                        let inst = self.create_enemy_sprite(
                            atlas,
                            SpriteId::FISH_001,
                            sx,
                            sy,
                            flip_x,
                            true,
                        );
                        commands.push(RenderCommand::DrawSprite(inst));
                    }
                }
                TP_VERT_FIREBALL => {
                    if (enemy.delay_counter - enemy.move_delay).abs() <= 1 {
                        // 随机选择火球帧 (F_000~F_003)
                        let sprite_id = match random_usize(4) {
                            0 => SpriteId::F_000,
                            1 => SpriteId::F_001,
                            2 => SpriteId::F_002,
                            _ => SpriteId::F_003,
                        };
                        let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, false, false);
                        commands.push(RenderCommand::DrawSprite(inst));
                        // 火花效果在move_enemies中通过glitter_sys创建
                    }
                }
                TP_VERT_PLANT => {
                    let sprite_id = if self.time_counter % 32 < 16 {
                        match enemy.sub_tp {
                            0 | 1 => SpriteId::PPLANT_002,
                            _ => SpriteId::PPLANT_000,
                        }
                    } else {
                        match enemy.sub_tp {
                            0 | 1 => SpriteId::PPLANT_003,
                            _ => SpriteId::PPLANT_001,
                        }
                    };
                    let visible_h = (enemy.map_y * H) - enemy.y_pos - 1;
                    if visible_h >= 0 {
                        let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, false, false);
                        commands.push(RenderCommand::DrawSpritePart {
                            sprite: inst,
                            visible_height: visible_h as f32,
                        });
                    }
                }
                TP_DEAD_VERT_PLANT => {
                    if self.enemies[j].status < 12 {
                        let inst = self.create_enemy_sprite(
                            atlas,
                            SpriteId::HIT_000,
                            sx,
                            sy,
                            false,
                            false,
                        );
                        commands.push(RenderCommand::DrawSprite(inst));
                    }
                }
                TP_RED => {
                    let sprite_id = if enemy.dir_counter % 16 <= 8 {
                        SpriteId::RED_000
                    } else {
                        SpriteId::RED_001
                    };
                    let flip_x = enemy.x_vel > 0;
                    let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, flip_x, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_DEAD_RED => {
                    let sprite_id = if enemy.dir_counter % 16 <= 8 {
                        SpriteId::RED_000
                    } else {
                        SpriteId::RED_001
                    };
                    let flip_x = enemy.x_vel > 0;
                    let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, flip_x, true);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_KOOPA => {
                    // 对齐 原版: 乌龟本体（有头）使用 GRKOOPA/RDKOOPA + y_pos-10
                    let sprite_id = match (enemy.sub_tp, enemy.dir_counter % 16 <= 8) {
                        (0, true) => SpriteId::GRKOOPA_000,
                        (0, false) => SpriteId::GRKOOPA_001,
                        (_, true) => SpriteId::RDKOOPA_000,
                        (_, false) => SpriteId::RDKOOPA_001,
                    };
                    let flip_x = enemy.x_vel > 0;
                    let inst =
                        self.create_enemy_sprite(atlas, sprite_id, sx, sy - 10.0, flip_x, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_WAKING_KOOPA | TP_RUNNING_KOOPA => {
                    // 对齐 原版: shell 跑动/抖动帧 = GRKP_000/001 + 左右镜像切换
                    let base0 = if enemy.sub_tp == 0 {
                        SpriteId::GRKP_000
                    } else {
                        SpriteId::RDKP_000
                    };
                    let base1 = if enemy.sub_tp == 0 {
                        SpriteId::GRKP_001
                    } else {
                        SpriteId::RDKP_001
                    };
                    let sprite_id = if enemy.dir_counter % 16 <= 8 {
                        base1
                    } else {
                        base0
                    };
                    let flip_x = !(enemy.dir_counter % 32 <= 16);
                    let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, flip_x, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_SLEEPING_KOOPA => {
                    // 原版: enemy_pictures[8 + 2*sub_tp][0]（固定使用镜像帧）
                    let sprite_id = if enemy.sub_tp == 0 {
                        SpriteId::GRKP_000
                    } else {
                        SpriteId::RDKP_000
                    };
                    let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, true, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_DEAD_KOOPA => {
                    // 原版: up_side_down(enemy_pictures[8 + 2*sub_tp][(dir_counter%16<=8)])
                    let sprite_id = if enemy.sub_tp == 0 {
                        SpriteId::GRKP_000
                    } else {
                        SpriteId::RDKP_000
                    };
                    let flip_x = !(enemy.dir_counter % 16 <= 8);
                    let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, flip_x, true);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_BLOCK_LIFT => {
                    let inst =
                        self.create_enemy_sprite(atlas, SpriteId::LIFT1_000, sx, sy, false, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                TP_DONUT => {
                    let sprite_id = if enemy.status == 0 {
                        SpriteId::DONUT_000
                    } else {
                        SpriteId::DONUT_001
                    };
                    let inst = self.create_enemy_sprite(atlas, sprite_id, sx, sy, false, false);
                    commands.push(RenderCommand::DrawSprite(inst));
                }
                _ => {}
            }
        }
    }

    /// 辅助方法：从图集创建精灵实例
    fn create_enemy_sprite(
        &self,
        atlas: &crate::sprites::SpriteAtlas,
        sprite_id: crate::sprites::SpriteId,
        x: f32,
        y: f32,
        flip_x: bool,
        flip_y: bool,
    ) -> SpriteInstance {
        let uv = atlas.get(sprite_id);
        let (u, v, u_size, v_size) = uv.normalized(atlas.size());
        SpriteInstance::new(
            x,
            y,
            uv.width as f32,
            uv.height as f32,
            u,
            v,
            u_size,
            v_size,
        )
        .with_flip(flip_x, flip_y)
    }

    fn check(
        &mut self,
        i: usize,
        render_state: &mut RenderState,
        buffers: &mut Buffers,
        tmp_obj_manager: &mut TmpObjManager,
        music_player: &MusicPlayer,
        glitter_sys: &mut GlitterSystem,
    ) {
        let safe = EY1 as i32;
        let h_safe = H * safe;
        let mut new_ch1: u8;
        let mut new_ch2: u8;
        let ch: u8;
        let j: i32;
        let l: i32;
        let side: i32;
        let mut at_x: i32;
        let mut new_x: i32;
        let new_x1: i32;
        let new_x2: i32;
        let mut y1: i32;
        let mut y2: i32;
        // let mut new_y: i32;
        let mut hold1: bool;
        let mut hold2: bool;
        let mut x: i32;
        let mut y: i32;

        if i >= self.enemies.len() {
            return;
        }

        let enemy = self.enemies[i].clone();

        match enemy.tp {
            TP_RISING_CHAMP | TP_RISING_LIFE | TP_RISING_FLOWER | TP_RISING_STAR => {
                // Pascal: if ((YPos / H) = (YPos div H)) and (YPos <> MapY * H) then ...
                // 等价条件：YPos 是 H 的整数倍，且已经从砖块顶部“升起”离开起点。
                if (enemy.y_pos % H == 0) && enemy.y_pos != enemy.map_y * H {
                    // Pascal: WorldMap^[MapX + 1, MapY - 1] in CanHoldYou
                    let world_map_val = buffers.world_get(enemy.map_x + 1, enemy.map_y - 1);

                    self.enemies[i].x_vel = 1 - 2 * if CAN_HOLD_YOU.contains(&world_map_val) {
                        1
                    } else {
                        0
                    };

                    match enemy.tp {
                        TP_RISING_CHAMP => {
                            self.enemies[i].tp = TP_CHAMP;
                        }
                        TP_RISING_LIFE => {
                            self.enemies[i].tp = TP_LIFE;
                            self.enemies[i].x_vel = 2 * self.enemies[i].x_vel;
                        }
                        TP_RISING_FLOWER => {
                            self.enemies[i].x_vel = 0;
                            self.enemies[i].tp = TP_FLOWER;
                        }
                        TP_RISING_STAR => {
                            self.enemies[i].tp = TP_STAR;
                            self.enemies[i].x_vel = 2 * self.enemies[i].x_vel;
                        }
                        _ => {}
                    }
                    self.enemies[i].y_vel = -7;
                    self.enemies[i].move_delay = 1;
                    self.enemies[i].status = ENEMY_FALLING;
                } else {
                    // Pascal原始逻辑：当蘑菇仍在砖块中上升时，偶数帧调用 Beep(130 - 20 * j)
                    // 其中 j = YPos % H。负频率在16位下会折返成高频，保留这一特性以贴近原声。
                    j = enemy.y_pos % H;
                    if j % 2 == 0 {
                        let freq = 130 - 20 * j;
                        let wrapped = freq.rem_euclid(1 << 16) as u16;
                        music_player.beep(wrapped as u32);
                    }
                    return;
                }
            }

            TP_FIREBALL => {
                // 重要：此处必须使用“更新后的 x_vel”参与后续判断。
                // 否则会出现：碰墙后把 self.enemies[i].x_vel 置 0，但下面仍用旧的 enemy.x_vel 判断，
                // 导致不进入 dying 分支，从而火球被通用逻辑反弹并在全图来回弹跳（与 Pascal 不一致）。
                let mut x_vel = self.enemies[i].x_vel;

                at_x = (enemy.x_pos + W / 4) / W;
                new_x = (enemy.x_pos + W / 4 + x_vel) / W;

                if at_x != new_x || self.player_x1 % W == 0 {
                    y1 = (enemy.y_pos + H / 4 + h_safe) / H - safe;
                    new_ch1 = buffers.world_get(new_x, y1);

                    if CAN_HOLD_YOU.contains(&new_ch1) {
                        x_vel = 0;
                        self.enemies[i].x_vel = 0;
                    }
                }

                new_x = enemy.x_pos;
                // new_y = enemy.y_pos;
                at_x = (enemy.x_pos + W / 4 + x_vel) / W;
                let new_y = (enemy.y_pos + 2 + H / 4 + enemy.y_vel + h_safe) / H - safe;

                new_ch1 = buffers.world_get(at_x, new_y);

                if enemy.y_vel > 0
                    && (CAN_HOLD_YOU.contains(&new_ch1) || CAN_STAND_ON.contains(&new_ch1))
                {
                    self.enemies[i].y_pos =
                        ((enemy.y_pos + enemy.y_vel - 5 + h_safe) / H - safe) * H;
                    self.enemies[i].y_vel = -2;
                } else if enemy.x_pos % 3 == 0 {
                    self.enemies[i].y_vel += 1;
                }

                if x_vel == 0
                    || new_x < buffers.x_view - W
                    || new_x > buffers.x_view + NH * W + W
                    || new_y > NV * H
                {
                    self.enemies[i].delay_counter = -(MAX_PAGE as i32 + 1);
                    self.enemies[i].tp = TP_DYING_FIREBALL;
                }
                return;
            }

            TP_STAR => {
                glitter_sys.start_glitter(enemy.x_pos, enemy.y_pos, W, H, buffers);
            }

            _ => {}
        }

        // Handle non-vertical enemies movement
        if !matches!(
            enemy.tp,
            TP_VERT_FISH
                | TP_DEAD_VERT_FISH
                | TP_VERT_FIREBALL
                | TP_VERT_PLANT
                | TP_DEAD_VERT_PLANT
        ) {
            side = if enemy.x_vel > 0 { W - 1 } else { 0 };
            at_x = (enemy.x_pos + side) / W;
            new_x = (enemy.x_pos + side + enemy.x_vel) / W;

            if at_x != new_x || matches!(enemy.status, ENEMY_FALLING) {
                y1 = (enemy.y_pos + h_safe) / H - safe;
                y2 = (enemy.y_pos + h_safe + H - 1) / H - safe;

                new_ch1 = buffers.world_get(new_x, y1);
                new_ch2 = buffers.world_get(new_x, y2);

                hold1 = CAN_HOLD_YOU.contains(&new_ch1);
                hold2 = CAN_HOLD_YOU.contains(&new_ch2);

                if hold1 || hold2 {
                    if matches!(enemy.tp, TP_RUNNING_KOOPA) {
                        self.show_star(
                            enemy.x_pos + enemy.x_vel,
                            enemy.y_pos,
                            render_state,
                            buffers,
                            tmp_obj_manager,
                            music_player,
                        );
                        l = (enemy.y_pos + h_safe + H / 2) / H - safe;

                        ch = buffers.world_get(new_x, l);

                        if enemy.x_pos >= buffers.x_view
                            && enemy.x_pos + W <= buffers.x_view + NH * W
                        {
                            match ch {
                                b'J' => {
                                    tmp_obj_manager.break_block(new_x, l, buffers, music_player);
                                }
                                b'?' => {
                                    let above_ch = buffers.world_get(new_x, l - 1);

                                    match above_ch as u8 {
                                        b' ' => {
                                            tmp_obj_manager.hit_coin(
                                                new_x * W,
                                                l * H,
                                                true,
                                                glitter_sys,
                                                buffers,
                                                music_player,
                                            );
                                        }
                                        0xE0 => {
                                            if buffers.data.mode[buffers.player] == 0 {
                                                // mdSmall
                                                self.new_enemy(
                                                    TP_RISING_CHAMP,
                                                    0,
                                                    new_x,
                                                    l,
                                                    0,
                                                    -1,
                                                    1,
                                                    music_player,
                                                );
                                            } else {
                                                self.new_enemy(
                                                    TP_RISING_FLOWER,
                                                    0,
                                                    new_x,
                                                    l,
                                                    0,
                                                    -1,
                                                    1,
                                                    music_player,
                                                );
                                            }
                                        }
                                        0xE1 => {
                                            self.new_enemy(
                                                TP_RISING_LIFE,
                                                0,
                                                new_x,
                                                l,
                                                0,
                                                -1,
                                                2,
                                                music_player,
                                            );
                                        }
                                        _ => {}
                                    }
                                    tmp_obj_manager.remove(new_x * W, l * H, W, H, 1);
                                    buffers.world_set(new_x, l, b'@');
                                }
                                _ => {}
                            }
                        }
                    }
                    self.enemies[i].x_vel = 0;
                }
            }

            at_x = (enemy.x_pos + enemy.x_vel) / W;
            new_x = (enemy.x_pos + enemy.x_vel + W - 1) / W;
            let new_y = (enemy.y_pos + 1 + H + enemy.y_vel + h_safe) / H - safe;

            new_ch1 = buffers.world_get(at_x, new_y);

            new_ch2 = buffers.world_get(new_x, new_y);

            hold1 = CAN_HOLD_YOU.contains(&new_ch1) || CAN_STAND_ON.contains(&new_ch1);
            hold2 = CAN_HOLD_YOU.contains(&new_ch2) || CAN_STAND_ON.contains(&new_ch2);

            // Handle lift types
            if matches!(enemy.tp, TP_LIFT_START..=TP_LIFT_END) {
                if enemy.y_vel != 0 && !matches!(enemy.tp, TP_DONUT) {
                    if enemy.y_vel < 0 {
                        hold1 = (enemy.y_pos + enemy.y_vel) / (H) < enemy.map_y;
                    }
                    if hold1 {
                        self.enemies[i].y_vel = -enemy.y_vel;
                    }
                }
            } else {
                match enemy.status {
                    ENEMY_GROUNDED => {
                        if !(hold1 || hold2) {
                            self.enemies[i].status = ENEMY_FALLING;
                            self.enemies[i].y_vel = 1;
                        }
                        if enemy.sub_tp == 1 && matches!(enemy.tp, TP_KOOPA) {
                            if enemy.x_vel > 0 && (enemy.x_pos % W >= 11 && enemy.x_pos % W <= 19) {
                                if !hold2 && hold1 {
                                    self.enemies[i].x_vel = 0;
                                }
                            }
                            if enemy.x_vel < 0 && (enemy.x_pos % W >= 1 && enemy.x_pos % W <= 9) {
                                if !hold1 && hold2 {
                                    self.enemies[i].x_vel = 0;
                                }
                            }
                        }
                    }
                    ENEMY_FALLING => {
                        if hold1 || hold2 {
                            self.enemies[i].status = ENEMY_GROUNDED;
                            self.enemies[i].y_pos =
                                ((enemy.y_pos + enemy.y_vel + 1 + h_safe) / H - safe) * H;
                            if matches!(enemy.tp, TP_STAR) {
                                self.enemies[i].y_vel = -(5 * enemy.y_vel) / 2;
                                self.enemies[i].status = ENEMY_FALLING;
                            } else {
                                self.enemies[i].y_vel = 0;
                            }
                        } else {
                            self.enemies[i].y_vel += 1;
                            if self.enemies[i].y_vel > 4 {
                                self.enemies[i].y_vel = 4;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Enemy collision detection
        new_x1 = enemy.x_pos + enemy.x_vel;
        new_x2 = new_x1 + W - 1
            + 4 * if matches!(enemy.tp, TP_VERT_PLANT) {
                1
            } else {
                0
            };
        y1 = enemy.y_pos + enemy.y_vel;
        y2 = y1 + H - 1;

        if matches!(
            enemy.tp,
            TP_CHIBIBO
                | TP_FLAT_CHIBIBO
                | TP_VERT_FISH
                | TP_VERT_PLANT
                | TP_DEAD_VERT_PLANT
                | TP_RED
                | TP_KOOPA..=TP_RUNNING_KOOPA
        ) {
            for k in 0..self.num_enemies {
                if k >= self.active_enemies.len() {
                    continue;
                }
                let j = self.active_enemies[k];
                if j != i && j < self.enemies.len() {
                    if matches!(
                        self.enemies[j].tp,
                        TP_CHIBIBO | TP_FLAT_CHIBIBO | TP_RED | TP_KOOPA..=TP_RUNNING_KOOPA
                    ) {
                        x = self.enemies[j].x_pos + self.enemies[j].x_vel;
                        y = self.enemies[j].y_pos + self.enemies[j].y_vel;

                        if new_x1 < x + W && new_x2 > x && y1 < y + H && y2 > y {
                            if matches!(self.enemies[j].tp, TP_RUNNING_KOOPA) {
                                self.show_star(
                                    enemy.x_pos,
                                    enemy.y_pos,
                                    render_state,
                                    buffers,
                                    tmp_obj_manager,
                                    music_player,
                                );
                                if matches!(enemy.tp, TP_RUNNING_KOOPA) {
                                    self.show_star(
                                        self.enemies[j].x_pos,
                                        self.enemies[j].y_pos,
                                        render_state,
                                        buffers,
                                        tmp_obj_manager,
                                        music_player,
                                    );
                                    self.kill(j, buffers);
                                }
                                self.kill(i, buffers);
                            } else if !matches!(enemy.tp, TP_RUNNING_KOOPA) {
                                self.enemies[i].x_vel = -enemy.x_vel;
                                self.enemies[j].x_vel = -self.enemies[j].x_vel;
                                self.enemies[i].y_vel = -enemy.y_vel;
                                self.enemies[j].y_vel = -self.enemies[j].y_vel;

                                if (x - new_x1).abs() < W {
                                    if x > new_x1 {
                                        self.enemies[i].x_pos = enemy.x_pos - enemy.x_vel;
                                        self.enemies[i].x_vel = -self.enemies[i].x_vel.abs();
                                    } else if x < new_x1 {
                                        self.enemies[i].x_pos = enemy.x_pos - enemy.x_vel;
                                        self.enemies[i].x_vel = self.enemies[i].x_vel.abs();
                                    }
                                }
                            }
                        }
                    } else if matches!(self.enemies[j].tp, TP_FIREBALL) {
                        x = self.enemies[j].x_pos + self.enemies[j].x_vel;
                        y = self.enemies[j].y_pos + self.enemies[j].y_vel;

                        if new_x1 <= x + W / 2 && new_x2 >= x && y1 <= y + H / 2 && y2 >= y {
                            self.enemies[j].tp = TP_DYING_FIREBALL;
                            self.enemies[j].delay_counter = -(MAX_PAGE as i32 + 1);
                            self.show_star(
                                enemy.x_pos,
                                enemy.y_pos,
                                render_state,
                                buffers,
                                tmp_obj_manager,
                                music_player,
                            );
                            self.kill(i, buffers);
                        }
                    }
                }
            }
        }
    }

    pub fn move_enemies(
        &mut self,
        render_state: &mut RenderState,
        music_player: &MusicPlayer,
        buffers: &mut Buffers,
        glitter_sys: &mut GlitterSystem,
        tmp_obj_manager: &mut TmpObjManager,
    ) {
        // Pascal 的 Byte 自然溢出（0..255 循环），Rust debug 下需要显式使用 wrapping_add
        self.time_counter = self.time_counter.wrapping_add(1);
        // println!(
        //     "[ENEMIES] move_enemies tick time_counter={} num_enemies={} x_view={}",
        //     self.time_counter,
        //     self.num_enemies,
        //     buffers.x_view
        // );

        // 第一个循环：移动敌人（严格对齐 Pascal ENEMIES.PAS::MoveEnemies）
        for i in 0..self.num_enemies {
            let j = self.active_enemies[i] as usize;
            if j >= self.enemies.len() {
                continue;
            }

            // Pascal: Inc(DelayCounter)
            self.enemies[j].delay_counter += 1;
            let new_x = self.enemies[j].x_pos + self.enemies[j].x_vel;

            if self.enemies[j].delay_counter > self.enemies[j].move_delay {
                // Pascal: XPos := LastXPos; YPos := LastYPos; Inc(DirCounter)
                {
                    let e = &mut self.enemies[j];
                    e.x_pos = e.last_x_pos;
                    e.y_pos = e.last_y_pos;
                    e.dir_counter = e.dir_counter.wrapping_add(1);
                }

                // 处理垂直类型的敌人
                if matches!(
                    self.enemies[j].tp,
                    TP_VERT_FISH | TP_VERT_FIREBALL | TP_VERT_PLANT
                ) {
                    if self.enemies[j].tp == TP_VERT_PLANT {
                        match self.enemies[j].status {
                            0 => {
                                match self.enemies[j].sub_tp {
                                    0 => {
                                        if (self.enemies[j].x_pos > self.player_x2 + W)
                                            || (self.enemies[j].x_pos + 24 + (W) < self.player_x1)
                                        {
                                            self.enemies[j].status += 1;
                                        }
                                    }
                                    1 => {
                                        if (self.enemies[j].x_pos > self.player_x2)
                                            || (self.enemies[j].x_pos + 24 < self.player_x1)
                                        {
                                            self.enemies[j].status += 1;
                                        }
                                    }
                                    2 => {
                                        self.enemies[j].status += 1;
                                    }
                                    _ => {}
                                }
                                self.enemies[j].y_vel = 0;
                                self.enemies[j].delay_counter = 0;
                                self.enemies[j].move_delay = 1;
                            }
                            1 => {
                                self.enemies[j].y_vel = -1;
                                self.enemies[j].delay_counter = 0;
                                self.enemies[j].move_delay = 2;
                                if self.enemies[j].y_pos + self.enemies[j].y_vel
                                    <= (self.enemies[j].map_y * H - 19)
                                {
                                    self.enemies[j].y_vel = 0;
                                    self.enemies[j].delay_counter = 0;
                                    self.enemies[j].move_delay = 2;
                                    self.enemies[j].counter = 0;
                                    self.enemies[j].status += 1;
                                }
                            }
                            2 => {
                                self.enemies[j].counter += 1;
                                if self.enemies[j].counter > 200 {
                                    self.enemies[j].status += 1;
                                }
                                self.enemies[j].move_delay = 0;
                                self.enemies[j].delay_counter = 0;
                            }
                            3 => {
                                self.enemies[j].y_vel = 1;
                                self.enemies[j].delay_counter = 0;
                                self.enemies[j].move_delay = 2;
                                if self.enemies[j].y_pos > (self.enemies[j].map_y * H) {
                                    self.enemies[j].status += 1;
                                }
                            }
                            4 => {
                                self.enemies[j].y_vel = 0;
                                self.enemies[j].move_delay = 100 + random_i32(100);
                                self.enemies[j].delay_counter = 0;
                                self.enemies[j].status = 0;
                            }
                            _ => {}
                        }
                    } else {
                        if self.enemies[j].y_pos + H >= NV * H {
                            if self.enemies[j].y_vel > 0 {
                                self.enemies[j].y_vel = 0;
                                self.enemies[j].move_delay = 100 + random_i32(300);
                                self.enemies[j].delay_counter = 0;
                            } else {
                                self.enemies[j].y_vel = -10;
                                self.enemies[j].move_delay = 1;
                                self.enemies[j].delay_counter = 0;
                                if self.enemies[j].tp == TP_VERT_FIREBALL {
                                    music_player.beep(100);
                                    self.enemies[j].y_vel = -9;
                                }
                            }
                        }
                    }

                    // TP_VERT_FIREBALL 火花效果（对齐 原版 show_enemies 逻辑）
                    if self.enemies[j].tp == TP_VERT_FIREBALL {
                        if (self.enemies[j].delay_counter - self.enemies[j].move_delay).abs() <= 1 {
                            // 在火球附近生成随机火花
                            glitter_sys.new_glitter(
                                self.enemies[j].x_pos + random_i32(W),
                                self.enemies[j].y_pos + random_i32(H),
                                57 + random_u8(7),
                                14 + random_u8(20),
                                buffers,
                            );
                            glitter_sys.new_star(
                                self.enemies[j].x_pos + random_i32(W),
                                self.enemies[j].y_pos + random_i32(H),
                                57 + random_u8(7),
                                14 + random_u8(20),
                                buffers,
                            );
                        }
                    }
                }

                // 处理睡眠中的库巴
                if self.enemies[j].tp == TP_SLEEPING_KOOPA {
                    self.enemies[j].counter += 1;
                    if self.enemies[j].counter > 150 {
                        self.enemies[j].tp = TP_WAKING_KOOPA;
                        self.enemies[j].x_vel = 1;
                        self.enemies[j].counter = 0;
                    }
                }

                // 处理醒来中的库巴
                if self.enemies[j].tp == TP_WAKING_KOOPA {
                    self.enemies[j].x_vel = -self.enemies[j].x_vel;
                    self.enemies[j].move_delay = 1;
                    self.enemies[j].delay_counter = 0;
                    self.enemies[j].counter += 1;
                    if self.enemies[j].counter > 50 {
                        self.enemies[j].tp = TP_KOOPA;
                        if self.player_x1 > self.enemies[j].x_pos {
                            self.enemies[j].x_vel = 1;
                        } else {
                            self.enemies[j].x_vel = -1;
                        }
                    }
                }

                // 处理死亡食人花动画（对齐原版：show_enemies中的TP_DEAD_VERT_PLANT逻辑）
                // wgpu版本使用collect_enemy_sprites_gpu渲染，需要在move_enemies中更新状态
                if self.enemies[j].tp == TP_DEAD_VERT_PLANT {
                    self.enemies[j].delay_counter = 0;
                    self.enemies[j].move_delay = 0;
                    self.enemies[j].y_vel = 0;
                    self.enemies[j].status += 1;
                    // 动画播放完毕后销毁敌人
                    if self.enemies[j].status > 14 {
                        self.enemies[j].tp = TP_DYING;
                    }
                }

                // 处理死亡状态
                if matches!(
                    self.enemies[j].tp,
                    TP_DYING | TP_DYING_FIREBALL | TP_DYING_KOOPA
                ) {
                    self.enemies[j].tp = TP_DEAD;
                } else if matches!(self.enemies[j].tp, TP_FLAT_CHIBIBO)
                    || (new_x <= -W)
                    || (new_x < buffers.x_view - FORGET_ENEMIES_AT * W)
                    || (new_x > buffers.x_view + NH * W + FORGET_ENEMIES_AT * W)
                    || (self.enemies[j].y_pos + self.enemies[j].y_vel > NV * H)
                {
                    // 恢复世界地图
                    let map_x = self.enemies[j].map_x;
                    let map_y = self.enemies[j].map_y;
                    match self.enemies[j].tp {
                        TP_CHIBIBO => {
                            buffers.world_set(map_x, map_y, 0x80);
                        }
                        TP_VERT_FISH => {
                            buffers.world_set(map_x, map_y - 2, 0x81);
                        }
                        TP_VERT_FIREBALL => {
                            buffers.world_set(map_x, map_y - 2, 0x82);
                        }
                        TP_VERT_PLANT => {
                            let ch = (0x84 + self.enemies[j].sub_tp) as u8;
                            buffers.world_set(map_x, map_y - 2, ch);
                        }
                        TP_RED => {
                            buffers.world_set(map_x, map_y, 0x87);
                        }
                        TP_KOOPA | TP_SLEEPING_KOOPA | TP_WAKING_KOOPA | TP_RUNNING_KOOPA => {
                            let ch = (0x88 + self.enemies[j].sub_tp) as u8;
                            buffers.world_set(map_x, map_y, ch);
                        }
                        TP_BLOCK_LIFT => {
                            buffers.world_set(map_x, map_y, 0xb0);
                        }
                        TP_DONUT => {
                            buffers.world_set(map_x, map_y, 0xb1);
                        }
                        _ => {}
                    }

                    if self.enemies[j].tp == TP_KOOPA {
                        self.enemies[j].tp = TP_DYING_KOOPA;
                    } else if self.enemies[j].tp != TP_FIREBALL {
                        self.enemies[j].tp = TP_DYING;
                    } else {
                        self.enemies[j].tp = TP_DYING_FIREBALL;
                    }
                    self.enemies[j].delay_counter = -(MAX_PAGE as i32 + 1);
                } else {
                    self.enemies[j].delay_counter = 0;
                    let old_x_vel = self.enemies[j].x_vel;

                    // 垂直移动逻辑
                    if matches!(
                        self.enemies[j].tp,
                        TP_VERT_FISH | TP_DEAD_VERT_FISH | TP_VERT_FIREBALL
                    ) {
                        if (self.enemies[j].dir_counter % 3 == 0)
                            && (self.enemies[j].y_pos + H < NV * H)
                        {
                            self.enemies[j].y_vel += 1;
                        }
                    }

                    if matches!(
                        self.enemies[j].tp,
                        TP_DEAD_CHIBIBO | TP_DEAD_RED | TP_DEAD_KOOPA
                    ) {
                        if self.enemies[j].x_pos % 6 == 0 {
                            self.enemies[j].y_vel += 1;
                        }
                    } else {
                        self.check(
                            j,
                            render_state,
                            buffers,
                            tmp_obj_manager,
                            music_player,
                            glitter_sys,
                        );
                    }

                    // Pascal: XPos := XPos + XVel; YPos := YPos + YVel
                    self.enemies[j].x_pos += self.enemies[j].x_vel;
                    self.enemies[j].y_pos += self.enemies[j].y_vel;

                    if self.enemies[j].x_vel == 0 {
                        self.enemies[j].x_vel = -old_x_vel;
                        if self.enemies[j].tp == TP_DYING_FIREBALL {
                            self.show_fire(
                                self.enemies[j].x_pos,
                                self.enemies[j].y_pos,
                                render_state,
                                buffers,
                                tmp_obj_manager,
                                music_player,
                            );
                        }
                    }
                }

                self.enemies[j].last_x_pos = self.enemies[j].x_pos;
                self.enemies[j].last_y_pos = self.enemies[j].y_pos;
            } else if (self.enemies[j].x_vel != 0) || (self.enemies[j].y_vel != 0) {
                // Pascal: 插值移动
                let dc = self.enemies[j].delay_counter;
                let md = self.enemies[j].move_delay;
                let xv = self.enemies[j].x_vel;
                let yv = self.enemies[j].y_vel;
                let lx = self.enemies[j].last_x_pos;
                let ly = self.enemies[j].last_y_pos;
                self.enemies[j].x_pos = lx + (dc * xv) / (md + 1);
                self.enemies[j].y_pos = ly + (dc * yv) / (md + 1);
            }
        }

        // 第二个循环：碰撞检测
        for i in 0..self.num_enemies {
            let j = self.active_enemies[i] as usize;
            if j >= self.enemies.len() {
                continue;
            }

            if matches!(
                self.enemies[j].tp,
                TP_CHIBIBO
                    | TP_CHAMP
                    | TP_LIFE
                    | TP_FLOWER
                    | TP_STAR
                    | TP_VERT_FISH
                    | TP_VERT_FIREBALL
                    | TP_VERT_PLANT
                    | TP_RED
            ) || self.enemies[j].tp.is_in_range(TP_KOOPA, TP_RUNNING_KOOPA)
                || self.enemies[j].tp.is_in_range(TP_LIFT_START, TP_LIFT_END)
            {
                if (self.player_x1 < self.enemies[j].x_pos + W)
                    && (self.player_x2 > self.enemies[j].x_pos)
                    && (self.player_y1 + self.player_y_vel < self.enemies[j].y_pos + H)
                    && (self.player_y2 + self.player_y_vel > self.enemies[j].y_pos)
                {
                    if self.star && !self.enemies[j].tp.is_in_range(TP_LIFT_START, TP_LIFT_END) {
                        music_player.beep(800);
                        self.kill(j, buffers);
                        self.cd_hit = 1;
                    }

                    match self.enemies[j].tp {
                        TP_SLEEPING_KOOPA | TP_WAKING_KOOPA => {
                            self.enemies[j].tp = TP_RUNNING_KOOPA;
                            self.enemies[j].x_vel = 5
                                * (2 * if self.enemies[j].x_pos > self.player_x1 {
                                    1
                                } else {
                                    0
                                } - 1);
                            self.enemies[j].move_delay = 0;
                            self.enemies[j].delay_counter = 0;
                            music_player.beep(800);
                            self.cd_enemy = 1;
                            buffers.add_score(100);
                        }
                        TP_CHAMP => {
                            if self.enemies[j].sub_tp == 0 {
                                self.cd_champ = 1;
                                buffers.add_score(1000);
                            } else {
                                self.cd_hit = 1;
                            }
                            self.enemies[j].tp = TP_DYING;
                            self.enemies[j].delay_counter = -(MAX_PAGE as i32 + 1);
                            glitter_sys.coin_glitter(
                                self.enemies[j].x_pos,
                                self.enemies[j].y_pos,
                                buffers,
                            );
                        }
                        TP_LIFE => {
                            self.cd_life = 1;
                            self.enemies[j].tp = TP_DYING;
                            self.enemies[j].delay_counter = -(MAX_PAGE as i32 + 1);
                            glitter_sys.coin_glitter(
                                self.enemies[j].x_pos,
                                self.enemies[j].y_pos,
                                buffers,
                            );
                            buffers.add_score(1000);
                        }
                        TP_FLOWER => {
                            self.cd_flower = 1;
                            self.enemies[j].tp = TP_DYING;
                            self.enemies[j].delay_counter = -(MAX_PAGE as i32 + 1);
                            glitter_sys.coin_glitter(
                                self.enemies[j].x_pos,
                                self.enemies[j].y_pos,
                                buffers,
                            );
                            buffers.add_score(1000);
                        }
                        TP_STAR => {
                            self.cd_star = 1;
                            self.enemies[j].tp = TP_DYING;
                            self.enemies[j].delay_counter = -(MAX_PAGE as i32 + 1);
                            glitter_sys.coin_glitter(
                                self.enemies[j].x_pos,
                                self.enemies[j].y_pos,
                                buffers,
                            );
                            buffers.add_score(1000);
                        }
                        TP_VERT_FIREBALL => {
                            self.cd_hit = 1;
                        }
                        _ => {
                            if ((self.player_y_vel > self.enemies[j].y_vel)
                                || (self.player_y_vel > 0))
                                && (self.player_y2 <= self.enemies[j].y_pos + H)
                            {
                                match self.enemies[j].tp {
                                    TP_CHIBIBO => {
                                        self.enemies[j].tp = TP_FLAT_CHIBIBO;
                                        self.enemies[j].x_vel = 0;
                                        self.enemies[j].delay_counter = -2
                                            - 15 * if self.enemies[j].y_vel == 0 { 1 } else { 0 };
                                        music_player.beep(800);
                                        self.cd_enemy = 1;
                                        buffers.add_score(100);
                                    }
                                    TP_VERT_FISH => {
                                        if self.enemies[j].y_pos + H < NV * H {
                                            self.kill(j, buffers);
                                            music_player.beep(800);
                                            self.cd_enemy = 1;
                                        }
                                    }
                                    TP_KOOPA | TP_RUNNING_KOOPA => {
                                        self.enemies[j].tp = TP_SLEEPING_KOOPA;
                                        self.enemies[j].x_vel = 0;
                                        self.enemies[j].counter = 0;
                                        music_player.beep(800);
                                        self.cd_enemy = 1;
                                        buffers.add_score(100);
                                    }
                                    _ => {
                                        if self.enemies[j]
                                            .tp
                                            .is_in_range(TP_LIFT_START, TP_LIFT_END)
                                        {
                                            if self.enemies[j].tp == TP_DONUT {
                                                self.enemies[j].status = 2;
                                                if (self.enemies[j].counter > 20)
                                                    && (self.enemies[j].y_vel == 0)
                                                {
                                                    self.enemies[j].y_vel += 1;
                                                }
                                            }
                                            self.cd_stop_jump =
                                                if self.player_y_vel != 2 { 1 } else { 0 };
                                            self.cd_lift = 1;
                                            self.player_y1 = self.enemies[j].y_pos - 2 * H;
                                            self.player_y2 = self.enemies[j].y_pos - 1;
                                            self.player_x_vel = self.enemies[j].x_vel;
                                            if self.enemies[j].move_delay != 0 {
                                                self.player_x_vel = self.enemies[j].x_vel
                                                    * (self.enemies[j].x_pos % 2);
                                            }
                                            self.player_y_vel = self.enemies[j].y_vel;
                                        }
                                    }
                                }
                            } else {
                                if !((self.enemies[j].tp == TP_VERT_FISH)
                                    && !((self.enemies[j].delay_counter
                                        - self.enemies[j].move_delay)
                                        .abs()
                                        <= 1)
                                    || self.enemies[j].tp.is_in_range(TP_LIFT_START, TP_LIFT_END))
                                {
                                    self.cd_hit = 1;
                                    if self.star {
                                        self.kill(j, buffers);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 清理死亡的敌人
        let mut i = 0;
        while i < self.active_enemies.len() {
            let j = self.active_enemies[i] as usize;
            if j < self.enemies.len() && self.enemies[j].tp == TP_DEAD {
                self.active_enemies.remove(i);
            } else {
                i += 1;
            }
        }
        self.num_enemies = self.active_enemies.len();
    }

    pub fn start_enemies(
        &mut self,
        x: i32,
        dir: i8,
        buffers: &mut Buffers,
        music_player: &MusicPlayer,
        options: &WorldOptions,
    ) {
        if x < 0 || x > options.x_size as i32 {
            return;
        }

        for i in 0..NV {
            let mut remove = true;

            // 重要：必须使用带 EX/EY1 偏移的 WorldMap 访问，否则会读错格子导致敌人占位符无法清除
            let ch = buffers.world_get(x, i) as u8;

            match ch {
                0x80 => self.new_enemy(TP_CHIBIBO, 0, x, i, dir as i32, 0, 2, music_player),
                0x81 => self.new_enemy(
                    TP_VERT_FISH,
                    0,
                    x,
                    i + 2,
                    0,
                    0,
                    50 + (random_i32(100)),
                    music_player,
                ),
                0x82 => self.new_enemy(
                    TP_VERT_FIREBALL,
                    0,
                    x,
                    i + 2,
                    0,
                    0,
                    50 + (random_i32(100)),
                    music_player,
                ),
                0x83 => self.new_enemy(TP_CHIBIBO, 1, x, i, dir as i32, 0, 2, music_player),
                0x84 | 0x85 | 0x86 => {
                    let sub_type = ch - 0x84;
                    self.new_enemy(
                        TP_VERT_PLANT,
                        sub_type as i32,
                        x,
                        i + 2,
                        0,
                        0,
                        20 + random_i32(50),
                        music_player,
                    );
                }
                0x87 => self.new_enemy(TP_RED, 0, x, i, dir as i32, 0, 2, music_player),
                0x88 | 0x89 | 0x8A => {
                    let sub_type = ch - 0x88;
                    self.new_enemy(
                        TP_KOOPA,
                        sub_type as i32,
                        x,
                        i,
                        dir as i32,
                        0,
                        2,
                        music_player,
                    );
                }
                0xB0 => {
                    // 检查左右相邻格是否在 CanHoldYou 集合中
                    let left = if x > 0 {
                        buffers.world_get(x - 1, i)
                    } else {
                        0
                    };
                    let right = buffers.world_get(x + 1, i);
                    let horizontal = CAN_HOLD_YOU.contains(&left) || CAN_HOLD_YOU.contains(&right);
                    if horizontal {
                        self.new_enemy(TP_BLOCK_LIFT, 0, x, i, -dir as i32, 0, 0, music_player);
                    } else {
                        self.new_enemy(TP_BLOCK_LIFT, 0, x, i, 0, -dir as i32, 0, music_player);
                    }
                }
                0xB1 => self.new_enemy(TP_DONUT, 0, x, i, 0, 0, 0, music_player),
                _ => remove = false,
            }

            if remove {
                // Pascal: if Remove then WorldMap^[X, i] := ' ';
                buffers.world_set(x, i, b' ');
            }
        }
    }

    pub fn hit_above(&mut self, map_x: i32, map_y: i32, buffers: &mut Buffers) {
        let y = map_y * H;
        let x = map_x * W;

        // 这里可能会调用 self.kill() 修改 active_enemies，因此必须先复制一份索引列表避免借用冲突
        let active = self.active_enemies.clone();
        for j in active {
            let j = j as usize;
            if j >= self.enemies.len() {
                continue;
            }
            let enemy = &mut self.enemies[j];
            if enemy.y_pos == y {
                let left = enemy.x_pos + enemy.x_vel;
                let right = left + W;
                if right > x && left < x + W {
                    match enemy.tp {
                        TP_CHAMP | TP_LIFE | TP_FLOWER | TP_STAR | TP_KOOPA | TP_SLEEPING_KOOPA
                        | TP_WAKING_KOOPA | TP_RUNNING_KOOPA => {
                            if (enemy.x_vel > 0 && left + W / 2 <= x)
                                || (enemy.x_vel < 0 && left + W / 2 >= x)
                            {
                                enemy.x_vel = -enemy.x_vel;
                            }
                            enemy.y_vel = -7;
                            enemy.status = ENEMY_FALLING;
                            if matches!(
                                enemy.tp,
                                TP_KOOPA | TP_SLEEPING_KOOPA | TP_WAKING_KOOPA | TP_RUNNING_KOOPA
                            ) {
                                enemy.tp = TP_SLEEPING_KOOPA;
                                enemy.x_vel = 0;
                            }
                        }
                        TP_CHIBIBO | TP_RED => {
                            self.kill(j, buffers);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
