// renderer.rs - 统一的渲染管线（P0-2 修复）
//
// 目标：把分散在 play.rs、backgr.rs、figures.rs 中的渲染调用收敛到这里，
// 明确渲染层级顺序，避免逻辑和渲染混杂。
//
// GPU渲染模式：当启用时，收集渲染指令而不是直接绘制，
// 最后由GpuRenderer统一提交到GPU

use crate::backgr::BackGr;
use crate::blocks::Blocks;
use crate::buffers::{Buffers, DM_DOWN_OUT_OF_PIPE, DM_UP_INTO_PIPE, H, NH, NV, W};
use crate::enemies::Enemies;
use crate::figures::Figures;
use crate::glitter::GlitterSystem;
use crate::gpu::RenderCommand;
use crate::gpu::sprite_batch::FillCommand;
use crate::gpu::sprite_batch::SpriteBatch;
use crate::players::Players;
use crate::render_state::RenderState;
use crate::sprites::{SpriteAtlas, SpriteDataManager};
use crate::stars::Stars;
use crate::status::Status;
use crate::tmpobj::TmpObjManager;
use crate::txt::Txt;

/// 渲染上下文 - 包含渲染一帧所需的所有引用
pub struct RenderContext<'a> {
    pub render_state: &'a mut RenderState,
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
    /// 是否显示玩家
    pub show_players: bool,
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
            show_players: true,
            show_retrace: false,
            only_draw: false,
        }
    }

    /// GPU模式：开始帧渲染（清空批次）
    pub fn begin_gpu_frame(&self, render_state: &mut RenderState) {
        render_state.begin_gpu_frame();
    }

    /// GPU模式：获取收集的渲染批次
    pub fn get_sprite_batch<'a>(&self, render_state: &'a RenderState) -> &'a SpriteBatch {
        render_state.get_sprite_batch()
    }

    /// GPU版 - 渲染完整一帧（初始化阶段）
    pub fn render_init_frame(&mut self, ctx: &mut RenderContext, _page: i32) {
        ctx.render_state.begin_gpu_frame();
        let commands = self.collect_gpu_frame(ctx, ctx.atlas);
        Self::submit_gpu_commands(ctx.render_state, commands);
    }

    /// GPU版 - 渲染游戏主循环帧
    ///
    /// GPU模式：每帧完全重绘，不需要hide/erase操作
    pub fn render_game_frame(&mut self, ctx: &mut RenderContext) {
        ctx.render_state.begin_gpu_frame();
        let commands = self.collect_gpu_frame(ctx, ctx.atlas);
        Self::submit_gpu_commands(ctx.render_state, commands);
    }

    fn submit_gpu_commands(render_state: &mut RenderState, commands: Vec<RenderCommand>) {
        // 当前实现每帧上传 row0 调色板，因此统一使用 palette_index=0
        // fade/blink 等效果会直接体现在  render_state.palette.palette 的内容里
        let palette_index: u32 = 0;
        render_state.set_gpu_palette(palette_index);
        let batch = render_state.get_sprite_batch_mut();

        for cmd in commands {
            match cmd {
                RenderCommand::Sprite(s) => batch.push_sprite(s),
                RenderCommand::FillRect(r) => {
                    let fill = FillCommand {
                        x: r.position[0],
                        y: r.position[1],
                        width: r.size[0],
                        height: r.size[1],
                        color_index: r.color_index as u8,
                        palette_index: r.palette_index as u32,
                    };
                    batch.push_fill(fill);
                }
                RenderCommand::UIFillRect(r) => {
                    // UI层fills在所有sprites之后渲染
                    let fill = FillCommand {
                        x: r.position[0],
                        y: r.position[1],
                        width: r.size[0],
                        height: r.size[1],
                        color_index: r.color_index as u8,
                        palette_index: r.palette_index as u32,
                    };
                    batch.push_ui_fill(fill);
                }
                RenderCommand::DrawSprite(inst) => batch.push_instance(inst),
                RenderCommand::DrawSpriteFlipY(mut inst) => {
                    inst.flip[1] = 1.0;
                    batch.push_instance(inst);
                }
                RenderCommand::DrawSpritePart {
                    mut sprite,
                    visible_height,
                } => {
                    let full_h = sprite.size[1];
                    if visible_height >= full_h {
                        batch.push_instance(sprite);
                    } else if visible_height > 0.0 {
                        let clip_ratio = visible_height / full_h;
                        let clipped_uv_h = sprite.uv_size[1] * clip_ratio;
                        sprite.size[1] = visible_height;
                        sprite.uv_size[1] = clipped_uv_h;
                        batch.push_instance(sprite);
                    }
                }
            }
        }
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
        // 当前实现每帧上传 row0 调色板，因此统一使用 palette_index=0
        let palette_index: u32 = 0;
        let x_view = ctx.buffers.x_view;
        let y_view = ctx.buffers.y_view;
        let has_stars = ctx.buffers.options.stars != 0;

        // 1. 背景层：对齐 原版 的 DrawSky 逻辑
        for f in ctx.figures.collect_sky_fills(
            0,
            0,
            crate::render_state::SCREEN_WIDTH,
            crate::render_state::VIR_SCREEN_HEIGHT,
            &ctx.buffers.options,
        ) {
            commands.push(RenderCommand::FillRect(crate::gpu::FillRect::new(
                f.x,
                f.y,
                f.width,
                f.height,
                f.color_index,
                palette_index,
            )));
        }

        // 1.1 地下室砖墙背景（严格对齐 原版）
        //
        // 原版 FIGURES.DrawSky(Sky=6/7/8, BackGrType=4) 会调用 BACKGR.DrawBricks，
        // 用 PALBRICK_000 以 PutImage 语义平铺整块背景（索引0也要绘制）。
        //
        // wgpu 模式下如果用 0x18 单色 fill 替代，会导致你反馈的现象：
        // WINDOW_001 能看到，但墙面底纹变成纯色(#717171)。
        if matches!(ctx.buffers.options.sky_type, 6 | 7 | 8) && ctx.buffers.options.backgr_type == 4
        {
            use crate::gpu::sprite_batch::SpriteCommand;
            use crate::sprites::SpriteId;

            let uv = atlas.get(SpriteId::PALBRICK_000);
            let tw = uv.width as i32; // 20
            let th = uv.height as i32; // 14

            // 让砖块图案在“世界坐标”上保持对齐，随着 x_view/y_view 滚动。
            let x0 = -x_view.rem_euclid(tw);
            let y0 = -y_view.rem_euclid(th);
            let screen_w = crate::render_state::SCREEN_WIDTH;
            let screen_h = crate::render_state::VIR_SCREEN_HEIGHT;

            let mut y = y0;
            while y < screen_h {
                let mut x = x0;
                while x < screen_w {
                    commands.push(RenderCommand::Sprite(
                        SpriteCommand::new(x, y, uv).with_opaque(true),
                    ));
                    x += tw;
                }
                y += th;
            }
        }

        // 1.2 地下室柱子背景（对齐 原版 BACKGR.Pillar）
        //
        // 原版 FIGURES.DrawSky(Sky=6/7/8, BackGrType=6) 会逐 tile 调用 BACKGR.Pillar，
        // 按 (x/20)%3 在 PALPILL_000/001/002 之间切换，形成黑色垂直渐变的柱子纹理。
        //
        // GPU 版用整屏 tile 平铺实现同样的像素效果（索引0也要绘制）。
        if matches!(ctx.buffers.options.sky_type, 6 | 7 | 8) && ctx.buffers.options.backgr_type == 6
        {
            use crate::gpu::sprite_batch::SpriteCommand;
            use crate::sprites::SpriteId;

            let uv0 = atlas.get(SpriteId::PALPILL_000);
            let uv1 = atlas.get(SpriteId::PALPILL_001);
            let uv2 = atlas.get(SpriteId::PALPILL_002);

            let tw = uv0.width as i32; // 20
            let th = uv0.height as i32; // 14

            // 让柱子纹理在“世界坐标”上保持对齐，随着 x_view/y_view 滚动。
            let x0 = -x_view.rem_euclid(tw);
            let y0 = -y_view.rem_euclid(th);
            let screen_w = crate::render_state::SCREEN_WIDTH;
            let screen_h = crate::render_state::VIR_SCREEN_HEIGHT;

            let mut y = y0;
            while y < screen_h {
                let mut x = x0;
                while x < screen_w {
                    // 原版: match (x/20)%3
                    let world_x = x + x_view;
                    let which = world_x.div_euclid(tw).rem_euclid(3);
                    let uv = match which {
                        0 => uv0,
                        1 => uv1,
                        _ => uv2,
                    };
                    commands.push(RenderCommand::Sprite(
                        SpriteCommand::new(x, y, uv).with_opaque(true),
                    ));
                    x += tw;
                }
                y += th;
            }
        }

        // 2. 星星层（如果有）
        if has_stars {
            ctx.stars
                .collect_stars_gpu(&mut commands, ctx.buffers, palette_index);
        }

        // 3. 背景装饰层：BackGrMap 形状（对齐 原版 DrawBackGr 的云带/远景轮廓）
        for f in ctx
            .backgr
            .collect_put_backgr_fills(x_view, &ctx.buffers.options)
        {
            commands.push(RenderCommand::FillRect(crate::gpu::FillRect::new(
                f.x,
                f.y,
                f.width,
                f.height,
                f.color_index,
                palette_index,
            )));
        }

        // 3.1 云朵层（如果启用）
        // 注意：当前所有关卡的clouds值都是0，此代码为预留
        if ctx.backgr.clouds > 0 {
            // 使用简化的云朵渲染，以填充矩形模拟云朵形状
            // TODO: 如果需要精确对齐Pascal版本的TraceCloud效果，可扩展此处逻辑
            let cloud_fills = ctx.backgr.collect_cloud_fills(x_view);
            for f in cloud_fills {
                commands.push(RenderCommand::FillRect(crate::gpu::FillRect::new(
                    f.x,
                    f.y,
                    f.width,
                    f.height,
                    f.color_index,
                    palette_index,
                )));
            }
        }

        // Intro 特例：对齐 原版 WORLDS/INTRO.PAS::DrawIntroScreen 的 DrawBackGrMap
        // 原版 调用顺序是先画标题，再 DrawBackGrMap，但通过 GetPixel>=0xC0 的 mask 避免覆盖前景/标题。
        // GPU 无读回，这里把 DrawBackGrMap 放到“地形之前”渲染，达到与 mask 等价的像素效果。
        if self.only_draw
            && ctx.buffers.options.sky_type == 10
            && ctx.buffers.options.backgr_type == 10
        {
            // 原版: 云层/近景与山峰层使用同一套调用，但视觉上需要：
            // - 云层更圆（BOGEN26）
            // - 山峰更尖（MOUNT）
            let cloud_map = crate::backgr::backgr_map_bogen26();
            let mount_map = crate::backgr::backgr_map_mount();

            // shift=54, color=0xA0：云层（圆）
            for f in ctx.backgr.collect_backgr_map_fills_from_map(
                cloud_map,
                10 * H + 6,
                11 * H - 1,
                54,
                0xA0,
            ) {
                commands.push(RenderCommand::FillRect(crate::gpu::FillRect::new(
                    f.x,
                    f.y,
                    f.width,
                    f.height,
                    f.color_index,
                    palette_index,
                )));
            }
            // shift=55/53, color=0xA1：山峰层（尖）
            for f in ctx.backgr.collect_backgr_map_fills_from_map(
                mount_map,
                10 * H + 6,
                11 * H - 1,
                55,
                0xA1,
            ) {
                commands.push(RenderCommand::FillRect(crate::gpu::FillRect::new(
                    f.x,
                    f.y,
                    f.width,
                    f.height,
                    f.color_index,
                    palette_index,
                )));
            }
            for f in ctx.backgr.collect_backgr_map_fills_from_map(
                mount_map,
                10 * H + 6,
                11 * H - 1,
                53,
                0xA1,
            ) {
                commands.push(RenderCommand::FillRect(crate::gpu::FillRect::new(
                    f.x,
                    f.y,
                    f.width,
                    f.height,
                    f.color_index,
                    palette_index,
                )));
            }
        }

        // 4. 地形/方块层 (tilemap)
        // 收集可见区域的tile精灵
        let tile_start_x = x_view / W;
        let tile_start_y = 0;
        let visible_tiles_x = NH + 2;
        let visible_tiles_y = NV;

        let tile_commands = ctx.figures.collect_visible_tiles_gpu(
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
        commands.extend(tile_commands);

        // 5. 敌人层
        if self.show_objects {
            ctx.enemies
                .collect_enemy_sprites_gpu(&mut commands, ctx.buffers, atlas);
        }

        // 6. 玩家层
        if self.show_players {
            let player_sprites = ctx.players.collect_player_sprites_gpu(
                ctx.buffers,
                atlas,
                palette_index,
                ctx.enemies.star,
            );
            for sprite_cmd in player_sprites {
                commands.push(RenderCommand::Sprite(sprite_cmd));
            }
        }

        // 6.1 管道出入动画遮挡：对齐 原版（先画玩家，再重绘管道口在最上层进行遮挡）
        if matches!(ctx.buffers.demo, DM_UP_INTO_PIPE | DM_DOWN_OUT_OF_PIPE) {
            // 用地图数据动态判断“管道口”在哪一行，避免 map_y 偏差导致遮挡失败
            let mx = ctx.players.map_x;
            let my = ctx.players.map_y;
            let mut pipe_y: Option<i32> = None;
            if ctx.buffers.world_get(mx, my + 1) == b'0' {
                pipe_y = Some(my + 1);
            } else if ctx.buffers.world_get(mx, my - 1) == b'0' {
                pipe_y = Some(my - 1);
            }
            if let Some(ty) = pipe_y {
                for dx in 0..=1 {
                    let tx = mx + dx;
                    let overlay = ctx.figures.collect_tile_sprite_gpu(
                        tx,
                        ty,
                        &ctx.buffers.world_map,
                        ctx.sprites,
                        atlas,
                        &ctx.buffers.options,
                        ctx.buffers,
                    );
                    commands.extend(overlay);
                }
            }
        }

        // 7. 临时对象层
        if self.show_objects {
            ctx.tmpobj
                .collect_temp_obj_sprites_gpu(&mut commands, ctx.buffers, atlas);
        }

        // 8. 方块动画层（bump效果）
        if self.show_objects {
            ctx.blocks
                .collect_bump_sprites_gpu(&mut commands, x_view, y_view, atlas);
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
        ctx.glitters
            .collect_glitter_gpu(&mut commands, x_view, y_view, palette_index);
        // GPU 模式下每帧完全重绘，不需要 hide_glitter，但必须按 原版 的节奏递减闪光计数
        ctx.glitters.update_glitter_gpu();

        commands
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
