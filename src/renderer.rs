// renderer.rs - 统一的渲染管线（P0-2 修复）
//
// 目标：把分散在 play.rs、backgr.rs、figures.rs 中的渲染调用收敛到这里，
// 明确渲染层级顺序，避免逻辑和渲染混杂。

use crate::backgr::BackGr;
use crate::blocks::Blocks;
use crate::buffers::{Buffers, H, NH, NV, W};
use crate::enemies::Enemies;
use crate::figures::Figures;
use crate::glitter::GlitterSystem;
use crate::players::Players;
use crate::sprites::SpriteDataManager;
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

    /// 渲染完整一帧（初始化阶段）
    /// 对应 Pascal PLAY.PAS 中初始化循环的渲染部分
    pub fn render_init_frame(&mut self, ctx: &mut RenderContext, _page: i32) {
        // 提前复制需要的值，避免借用冲突
        let has_stars = ctx.buffers.options.stars != 0;
        let opt1 = ctx.buffers.options.clone();
        
        // 1. 背景层
        self.render_background_layer(ctx);
       
        // 2. Tile 层（地形）
        self.render_tile_layer(ctx);
   
        // 3. Overlay 层（debug、山峰、背景）
        self.render_overlay_layer(ctx);
      
        // 4. 实体层
        if has_stars {
            ctx.stars.show_stars(ctx.vga, ctx.buffers);
        }
        ctx.enemies
            .show_enemies(ctx.vga, ctx.buffers, ctx.sprites, ctx.glitters);
        if !self.only_draw {
            ctx.players.draw_player(
                ctx.buffers,
                ctx.vga,
                ctx.sprites,
                ctx.figures,
                &opt1,
                ctx.backgr,
                ctx.enemies,
            );
        }

        // 关键日志：present 前抽样 framebuffer 是否被写入
        let sx0 = ctx.buffers.x_view + 10;
        let s0 = ctx.vga.get_pixel_world(sx0, 40);
        let s1 = ctx.vga.get_pixel_world(sx0, 80);
        let s2 = ctx.vga.get_pixel_world(sx0, 120);
      
        // 5. Present
        ctx.vga.show_page();

        // 确保关闭 sprite tracing
        ctx.figures.set_trace_enabled(false);
    }

    /// 渲染游戏主循环帧
    /// 对应 Pascal PLAY.PAS 主循环中的渲染部分
    ///
    /// 注意：show_score 需要在调用此方法前单独处理（因为需要 Play 的方法）
    pub fn render_game_frame(&mut self, ctx: &mut RenderContext) {
        let page = ctx.vga.current_page() as usize;
        let scroll = ctx.buffers.x_view - ctx.buffers.last_x_view[page];
        // 提前读取需要的 options 字段值，避免持久借用冲突
        let has_stars = ctx.buffers.options.stars != 0;

        // 0. 先擦除上一帧的实体/UI（Pascal：先 PopBackGr，再 ResetStack）。
        // 关键：敌人/临时对象改为"句柄版背景"，避免 x>255 时 Vec 版 push/pop 截断导致写回错位。
        ctx.glitters.hide_glitter(ctx.vga);
        if has_stars {
            ctx.stars.hide_stars(ctx.vga, ctx.buffers);
        }
        if self.show_objects {
            ctx.tmpobj.hide_temp_obj(ctx.vga);
        }
        if self.show_status {
            ctx.status.hide_status(ctx.vga);
        }
        ctx.players.erase_player(ctx.vga);
        if self.show_objects {
            ctx.enemies.hide_enemies(ctx.vga);
            ctx.blocks.erase_blocks(ctx.vga);
        }

        // 1) 背景/地形重绘：
        // - 非滚屏帧：只需要 DrawBackGr(FALSE)（Horizon 临时偏移）
        // - 滚屏帧：render_scroll 内会先移动 framebuffer，再补齐新露出条带（含 sky/tile/backgr）
        if scroll == 0 {
            // 只在需要修改horizon时才clone
            let mut opt_back = ctx.buffers.options.clone();
            let base_h = opt_back.horizon as i32;
            opt_back.horizon = (base_h + ctx.vga.get_y_offset() - YBASE) as u8;
            ctx.backgr.draw_backgr(false, ctx.vga, ctx.buffers, &opt_back);
        } else {
            self.render_scroll(ctx, scroll, page);
        }

        ctx.tmpobj
            .run_remove(ctx.vga, ctx.backgr, ctx.sprites, &ctx.buffers.options);

        // 1. 重置栈（准备绘制）。
        // - 非滚屏帧：必须在“擦除上一帧”之后调用，否则会使上一帧的 backgr handle 失效。
        // - 滚屏帧：我们跳过了旧句柄擦除，直接重绘基底，因此这里重置即可。
        ctx.vga.reset_stack();

        // 2. 实体层（blocks + enemies + player）
        if self.show_objects {
            ctx.blocks
                .draw_blocks(ctx.vga, ctx.backgr, &ctx.buffers.options, ctx.sprites);
            ctx.enemies
                .show_enemies(ctx.vga, ctx.buffers, ctx.sprites, ctx.glitters);
        }
        // 注意：draw_player 需要同时访问 &mut Buffers 和 &WorldOptions
        // 由于借用规则，需要先 clone options
        let opt_for_player = ctx.buffers.options.clone();
        ctx.players.draw_player(
            ctx.buffers,
            ctx.vga,
            ctx.sprites,
            ctx.figures,
            &opt_for_player,
            ctx.backgr,
            ctx.enemies,
        );

        // 3. UI 层（状态、临时对象）
        // show_score 需要在外部调用
        if self.show_status {
            let current_page = ctx.vga.current_page() as usize;
            let x_view = ctx.buffers.x_view;
            let player = ctx.buffers.player;
            let level_score: i32 = ctx.buffers.level_score.try_into().unwrap_or(0);

            // 直接传递引用，避免不必要的 clone
            ctx.status.show_status(
                current_page,
                x_view,
                player,
                &ctx.buffers.player_name,
                &ctx.buffers.data.lives,
                level_score,
                &ctx.buffers.data.coins,
                &ctx.buffers.world_number,
                ctx.txt,
                ctx.vga,
            );
        }
        if self.show_objects {
            ctx.tmpobj.show_temp_obj(ctx.vga, ctx.sprites);
        }

        // 4. 特效层
        if has_stars {
            ctx.stars.show_stars(ctx.vga, ctx.buffers);
        }
        ctx.glitters.show_glitter(ctx.vga);

        // 5. 更新视口记录
        ctx.buffers.last_x_view[ctx.vga.current_page() as usize] = ctx.buffers.x_view;
        // 注意1：Pascal 的 ShowTotalBack(结算文字) 发生在 ShowPage 之前。
        // 注意2：这里不再调用 show_page；由 play.rs 在“需要绘制 show_score 后”再统一 present，保证顺序严格一致。
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
