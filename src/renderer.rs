// renderer.rs - 统一的渲染管线（P0-2 修复）
//
// 目标：把分散在 play.rs、backgr.rs、figures.rs 中的渲染调用收敛到这里，
// 明确渲染层级顺序，避免逻辑和渲染混杂。
//
// GPU渲染模式：当启用时，收集渲染指令而不是直接绘制，
// 最后由GpuRenderer统一提交到GPU

use crate::backgr::BackGr;
use crate::blocks::Blocks;
use crate::buffers::{Buffers, H, NH, NV, W};
use crate::enemies::Enemies;
use crate::figures::Figures;
use crate::glitter::GlitterSystem;
use crate::gpu::RenderCommand;
use crate::gpu::sprite_batch::SpriteBatch;
use crate::players::Players;
use crate::sprites::{SpriteAtlas, SpriteDataManager};
use crate::stars::Stars;
use crate::status::Status;
use crate::tmpobj::TmpObjManager;
use crate::txt::Txt;
use crate::vga256::VGA;
use crate::vga256::YBASE;

/// 渲染上下文 - 包含渲染一帧所需的所有引用
pub struct RenderContext<'a> {
    pub vga: &'a mut VGA,
    pub buffers: &'a mut Buffers,
    pub backgr: &'a mut BackGr,
    pub figures: &'a mut Figures,
    pub sprites: &'a mut SpriteDataManager,
    pub atlas: &'a SpriteAtlas,
    pub blocks: &'a mut Blocks,
    pub enemies: &'a mut Enemies,
    pub players: &'a mut Players,
    pub tmpobj: &'a mut TmpObjManager,
    pub stars: &'a mut Stars,
    pub glitters: &'a mut GlitterSystem,
    pub status: &'a mut Status,
    pub txt: &'a mut Txt,
}

/// 渲染器 - 负责统一管理所有渲染逻辑
pub struct Renderer {
    /// 是否显示对象（blocks、enemies、tmpobj）
    pub show_objects: bool,
    /// 是否显示分数
    pub show_score: bool,
    /// 是否显示状态栏
    pub show_status: bool,
    /// 是否显示重绘指示器
    pub show_retrace: bool,
    /// 是否仅绘制（intro模式）
    pub only_draw: bool,
}

impl Renderer {
    pub fn new() -> Self {
        Renderer {
            show_objects: true,
            show_score: false,
            show_status: true,
            show_retrace: false,
            only_draw: false,
        }
    }
    
    /// GPU模式：开始帧渲染（清空批次）
    pub fn begin_gpu_frame(&self, vga: &mut VGA) {
        vga.begin_gpu_frame();
    }
    
    /// GPU模式：获取收集的渲染批次
    pub fn get_sprite_batch<'a>(&self, vga: &'a VGA) -> &'a SpriteBatch {
        vga.get_sprite_batch()
    }

    /// GPU版 - 渲染完整一帧（初始化阶段）
    pub fn render_init_frame(&mut self, ctx: &mut RenderContext, _page: i32) {
        // GPU模式每帧完全重绘
        ctx.vga.begin_gpu_frame();
        
        let has_stars = ctx.buffers.options.stars != 0;
        let opt1 = ctx.buffers.options.clone();
        
        // 1. 背景层
        self.render_background_layer(ctx);
       
        // 2. Tile层
        self.render_tile_layer(ctx);
   
        // 3. Overlay层
        self.render_overlay_layer(ctx);
      
        // 4. 实体层
        if has_stars {
            ctx.stars.show_stars(ctx.vga, ctx.buffers);
        }
        ctx.enemies.show_enemies(ctx.vga, ctx.buffers, ctx.sprites, ctx.glitters, ctx.atlas);
        if !self.only_draw {
            ctx.players.draw_player(
                ctx.buffers,
                ctx.vga,
                ctx.sprites,
                ctx.figures,
                &opt1,
                ctx.backgr,
                ctx.enemies,
                ctx.atlas,
            );
        }
      
        // 5. Present
        ctx.vga.show_page();

        ctx.figures.set_trace_enabled(false);
    }

    /// GPU版 - 渲染游戏主循环帧
    /// 
    /// GPU模式：每帧完全重绘，不需要hide/erase操作
    pub fn render_game_frame(&mut self, ctx: &mut RenderContext) {
        // GPU模式每帧完全重绘，开始新帧
        ctx.vga.begin_gpu_frame();
        
        let has_stars = ctx.buffers.options.stars != 0;
        let x_view = ctx.buffers.x_view;

        // 1. 背景层：天空 + 云朵
        self.render_background_layer(ctx);

        // 2. Tile层：地形方块
        self.render_tile_layer(ctx);

        // 3. Overlay层：山峰/背景
        self.render_overlay_layer(ctx);

        // 4. 移除效果
        ctx.tmpobj.run_remove(ctx.vga, ctx.atlas);

        // 5. 实体层
        if self.show_objects {
            ctx.blocks.draw_blocks(ctx.vga, ctx.atlas);
            ctx.enemies.show_enemies(ctx.vga, ctx.buffers, ctx.sprites, ctx.glitters, ctx.atlas);
        }
        
        let opt_for_player = ctx.buffers.options.clone();
        ctx.players.draw_player(
            ctx.buffers,
            ctx.vga,
            ctx.sprites,
            ctx.figures,
            &opt_for_player,
            ctx.backgr,
            ctx.enemies,
            ctx.atlas,
        );

        // 6. UI层：状态栏
        if self.show_status {
            let player = ctx.buffers.player;
            let level_score: i32 = ctx.buffers.level_score.try_into().unwrap_or(0);
            ctx.status.show_status(
                x_view,
                player,
                &ctx.buffers.player_name,
                &ctx.buffers.data.lives,
                level_score,
                &ctx.buffers.data.coins,
                &ctx.buffers.world_number,
                ctx.txt,
                ctx.vga,
                ctx.atlas,
            );
        }
        
        // 7. 临时对象层
        if self.show_objects {
            ctx.tmpobj.show_temp_obj(ctx.vga, ctx.atlas);
        }

        // 8. 特效层
        if has_stars {
            ctx.stars.show_stars(ctx.vga, ctx.buffers);
        }
        ctx.glitters.show_glitter(ctx.vga);
        ctx.glitters.update_glitter_gpu();  // 更新闪光计数器

        // 9. 更新视口记录
        ctx.buffers.last_x_view[ctx.vga.current_page() as usize] = ctx.buffers.x_view;
    }

    /// GPU模式：收集帧渲染命令
    /// 
    /// 替代render_game_frame，不直接绘制到VGA framebuffer，
    /// 而是收集所有渲染命令到Vec<RenderCommand>中，
    /// 最后由GpuRenderer统一提交到GPU
    /// 
    /// 渲染层级顺序:
    /// 1. 天空填充
    /// 2. 星星层
    /// 3. 背景装饰层(山峰/云朵)
    /// 4. Tilemap地形层
    /// 5. 敌人层
    /// 6. 玩家层
    /// 7. 临时对象层
    /// 8. 方块动画层
    /// 9. UI层(状态栏)
    /// 10. 闪光特效层
    pub fn collect_gpu_frame(
        &mut self,
        ctx: &mut RenderContext,
        atlas: &SpriteAtlas,
    ) -> Vec<RenderCommand> {
        let mut commands = Vec::with_capacity(2048);
        let palette_index = ctx.vga.palette.get_fade_palette_index();
        let x_view = ctx.buffers.x_view;
        let y_view = ctx.buffers.y_view;
        let has_stars = ctx.buffers.options.stars != 0;

        // 1. 背景层：天空填充
        let sky_color = match ctx.buffers.options.sky_type {
            0 => 0x90,  // 白天蓝色
            1 => 0x90,  // 白天
            2 => 0x70,  // 黄昏橙色
            3 => 0x00,  // 夜晚黑色
            4 => 0x90,  // 水下蓝
            5 => 0xC0,  // 城堡灰
            6 | 7 | 8 => 0x00,  // 地下室黑色
            _ => 0x90,
        };
        // 全屏天空填充
        let fill = crate::gpu::FillRect::new(
            0.0, 0.0,
            crate::vga256::SCREEN_WIDTH as f32, 
            crate::vga256::VIR_SCREEN_HEIGHT as f32,
            sky_color, palette_index,
        );
        commands.push(RenderCommand::FillRect(fill));
        
        // 渐变层(horizon以下使用不同颜色)
        let horizon = ctx.buffers.options.horizon as i32;
        if horizon > 0 && horizon < crate::vga256::VIR_SCREEN_HEIGHT as i32 {
            let ground_color = match ctx.buffers.options.sky_type {
                0 | 1 | 4 => 0xF0,  // 草地绿
                2 => 0xE0,          // 沙地黄
                3 => 0x18,          // 夜晚地面
                _ => 0xF0,
            };
            let fill = crate::gpu::FillRect::new(
                0.0, horizon as f32,
                crate::vga256::SCREEN_WIDTH as f32,
                (crate::vga256::VIR_SCREEN_HEIGHT - horizon) as f32,
                ground_color, palette_index,
            );
            commands.push(RenderCommand::FillRect(fill));
        }

        // 2. 星星层（如果有）
        if has_stars {
            ctx.stars.collect_stars_gpu(&mut commands, ctx.buffers, palette_index);
        }

        // 3. 背景装饰层（山峰、云朵等）
        // 使用backgr模块的GPU收集方法
        let cloud_sprites = ctx.backgr.collect_cloud_sprites(x_view);
        for sprite_cmd in cloud_sprites {
            if let Some(fill) = Self::sprite_to_fill(&sprite_cmd, palette_index) {
                commands.push(RenderCommand::FillRect(fill));
            }
        }

        // 4. 地形/方块层 (tilemap)
        // 收集可见区域的tile精灵
        let tile_start_x = x_view / W;
        let tile_start_y = 0;
        let visible_tiles_x = NH + 2;
        let visible_tiles_y = NV;
        
        let tile_sprites = ctx.figures.collect_visible_tiles_gpu(
            tile_start_x,
            tile_start_y,
            visible_tiles_x,
            visible_tiles_y,
            &ctx.buffers.world_map,
            ctx.sprites,
            atlas,
            &ctx.buffers.options,
            ctx.buffers,
        );
        for sprite_cmd in tile_sprites {
            commands.push(RenderCommand::Sprite(sprite_cmd));
        }

        // 5. 敌人层
        if self.show_objects {
            ctx.enemies.collect_enemy_sprites_gpu(
                &mut commands,
                ctx.buffers,
                atlas,
            );
        }

        // 6. 玩家层
        let player_sprites = ctx.players.collect_player_sprites_gpu(
            ctx.buffers,
            atlas,
            palette_index,
        );
        for sprite_cmd in player_sprites {
            commands.push(RenderCommand::Sprite(sprite_cmd));
        }

        // 7. 临时对象层
        if self.show_objects {
            ctx.tmpobj.collect_temp_obj_sprites_gpu(
                &mut commands,
                ctx.buffers,
                atlas,
            );
        }

        // 8. 方块动画层（bump效果）
        if self.show_objects {
            ctx.blocks.collect_bump_sprites_gpu(
                &mut commands,
                x_view,
                y_view,
                atlas,
            );
        }

        // 9. 状态栏UI
        if self.show_status {
            let player = ctx.buffers.player;
            let level_score: i32 = ctx.buffers.level_score.try_into().unwrap_or(0);
            ctx.status.collect_status_gpu(
                &mut commands,
                x_view,
                player,
                &ctx.buffers.player_name,
                &ctx.buffers.data.lives,
                level_score,
                &ctx.buffers.data.coins,
                &ctx.buffers.world_number,
                ctx.txt,
                palette_index,
            );
        }

        // 10. 闪光特效层
        ctx.glitters.collect_glitter_gpu(&mut commands, x_view, y_view, palette_index);

        commands
    }
    
    /// 辅助函数：将云朵精灵转换为填充命令（简化版本）
    fn sprite_to_fill(
        sprite_cmd: &crate::gpu::sprite_batch::SpriteCommand,
        palette_index: u32,
    ) -> Option<crate::gpu::FillRect> {
        // 云朵使用白色填充（调色板索引0xFF通常是白色）
        Some(crate::gpu::FillRect::new(
            sprite_cmd.x, sprite_cmd.y,
            sprite_cmd.uv.width as f32, sprite_cmd.uv.height as f32,
            0xFF, palette_index,
        ))
    }

    /// 背景层渲染（天空 + 云朵）
    fn render_background_layer(&mut self, ctx: &mut RenderContext) {
        let x_view = ctx.buffers.x_view;
        // 提前读取并复制需要的 options，避免借用冲突
        let sky_type = ctx.buffers.options.sky_type;
        let opt1 = ctx.buffers.options.clone();

        // 绘制天空
        // Pascal 对齐：在地下室(本项目 sky_type=8)时，DrawSky 会走 Sky=6/7/8 分支，
        // 实际效果应是"砖墙/砖块背景"，而不是额外一层全屏底色。
        // Rust 的渲染管线里 Tile 层的 Redraw 已经会逐格调用 draw_sky 做底色，
        // 这里如果再对全屏调用一次 draw_sky，容易造成"灰白蒙版/发白"的覆盖效果。
        // 因此：地下室 sky_type=8 时跳过这一层全屏 draw_sky，只保留 tile 逐格底色绘制。
        if sky_type != 8 {
            ctx.figures.draw_sky(
                x_view,
                0,
                NH * W,
                NV * H,
                ctx.vga,
                &opt1,
                ctx.backgr,
                ctx.sprites,
            );
        }

        // 绘制云朵（P0-2 修复：不再修改 buffers.x_view）
        ctx.backgr.start_clouds(x_view, ctx.vga, ctx.buffers);
    }

    /// Tile 层渲染（地形方块）
    fn render_tile_layer(&mut self, ctx: &mut RenderContext) {
        let x_view = ctx.buffers.x_view;
        // 提前复制 options，避免借用冲突（redraw 需要同时访问 options 和 buffers）
        let opt1 = ctx.buffers.options.clone();
        
        for x in (x_view / W - 1)..=(x_view / W + NH) {
            for y in 0..15 {
                ctx.figures.redraw(
                    x,
                    y,
                    &ctx.buffers.world_map,
                    ctx.vga,
                    ctx.backgr,
                    ctx.sprites,
                    &opt1,
                    ctx.buffers,
                );
            }
        }
    }

    /// Overlay 层渲染（山峰/背景）
    fn render_overlay_layer(&mut self, ctx: &mut RenderContext) {
        // 山峰/背景
        // Pascal 对齐：地下室(SkyType=8, BackGrType=4)不应叠加远景背景层（否则会造成"灰/白蒙版"）。
        // 仅在非地下室时绘制 DrawBackGr。
        // 注意：需要 clone 避免借用冲突（draw_backgr 需要 &mut Buffers 和 &WorldOptions）
        if ctx.buffers.options.sky_type != 8 {
            let opt1 = ctx.buffers.options.clone();
            ctx.backgr.draw_backgr(false, ctx.vga, ctx.buffers, &opt1);
            ctx.backgr.read_color_map(ctx.buffers, ctx.vga);
        }
    }

    /// 滚动渲染（move_screen 的渲染部分）
    ///
    /// 这个方法只负责渲染，不包含逻辑处理（如启动敌人、设置视口等）
    /// 逻辑部分留在 play.rs 的 move_screen_logic 中
    ///
    /// 设计说明：
    /// Pascal Mode X 使用硬件视口滚动（SetView 改变虚拟显存起始地址，像素不移动）。
    /// Rust 只有固定 320x200 framebuffer，没有硬件视口，无法实现同样的机制。
    /// 
    /// 使用像素搬移（scroll_screen_x）会导致问题：
    /// - 远景层有视差效果，不能 1:1 搬移
    /// - 边界处理复杂，容易产生黑边/残影
    ///
    /// 因此采用**完全重绘**策略：每帧基于新的 XView 直接重绘可见区域，
    /// 保证渲染结果正确且稳定。现代 CPU 性能足以支持这种方式。
    pub fn render_scroll(&mut self, ctx: &mut RenderContext, _scroll: i32, _page: usize) {
        // 提前复制需要的 options 值，避免借用冲突
        let opt1 = ctx.buffers.options.clone();
        let x_view = ctx.buffers.x_view;
        let sw = ctx.vga.width as i32;
        let sh = ctx.vga.height as i32;

        // 1) 天空底色：覆盖整屏，确保不会残留旧像素
        ctx.figures.draw_sky(
            x_view,
            0,
            sw,
            sh,
            ctx.vga,
            &opt1,
            ctx.backgr,
            ctx.sprites,
        );

        // 2) Tile 层：严格按地图符号重绘可见 tile 范围
        let tile_left = x_view.div_euclid(W) - 1;
        let tile_right = x_view.div_euclid(W) + NH;
        for tx in tile_left..=tile_right {
            for ty in 0..NV {
                ctx.figures.redraw(
                    tx,
                    ty,
                    &ctx.buffers.world_map,
                    ctx.vga,
                    ctx.backgr,
                    ctx.sprites,
                    &opt1,
                    ctx.buffers,
                );
            }
        }

        // 3) 远景层：DrawBackGr(FALSE)（对应 Pascal MoveScreen 中的处理）
        // 注意：Horizon 需要临时偏移，与 Pascal 保持一致
        let mut opt_back = opt1.clone();
        let base_h = opt_back.horizon as i32;
        opt_back.horizon = (base_h + ctx.vga.get_y_offset() - YBASE) as u8;
        ctx.backgr.draw_backgr(false, ctx.vga, ctx.buffers, &opt_back);
        ctx.backgr.read_color_map(ctx.buffers, ctx.vga);
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
