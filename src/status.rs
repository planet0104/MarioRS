// 严格结构体化移植 Pascal STATUS.PAS - GPU版本

use crate::{gpu::RenderCommand, txt::FontStyle};

pub struct Status;

impl Status {
    pub fn init_status(&mut self) {
        // GPU模式下不需要初始化背景缓存
    }

    /// GPU渲染: 收集状态栏文本
    /// 使用UI层渲染，确保状态栏在所有游戏精灵之上
    pub fn collect_status_gpu(
        &self,
        commands: &mut Vec<RenderCommand>,
        _x_view: i32,
        player: usize,
        player_name: &[String],
        data_lives: &[i16],
        level_score: i32,
        data_coins: &[i16],
        world_number: &str,
        txt: &mut crate::txt::Txt,
        palette_index: u32,
    ) {
        const HEIGHT: i32 = 6;

        // 设置字体为粗体
        txt.set_font(0, crate::txt::FontStyle::BOLD);

        // 玩家名称
        // 重要：UI 必须使用屏幕坐标，不能随着 x_view 滚动
        // 使用UI层渲染，确保在所有游戏精灵（包括地下室砖墙）之上
        txt.write_text_ui_gpu(
            commands,
            10 + 4,
            HEIGHT,
            &player_name[player],
            31,
            palette_index,
        );

        // 生命数
        let mut lives = data_lives[player];
        if lives > 99 {
            lives = 99;
        }
        txt.write_text_ui_gpu(
            commands,
            54 + 4,
            HEIGHT,
            &format!("{:2}", lives),
            31,
            palette_index,
        );

        // 分数
        txt.write_text_ui_gpu(
            commands,
            84 + 6,
            HEIGHT,
            &format!("{:09}", level_score).replace(' ', "0"),
            31,
            palette_index,
        );

        // 金币图标
        txt.write_text_ui_gpu(commands, 140 + 40 + 10, HEIGHT, "\t", 13, palette_index);
        txt.write_text_ui_gpu(commands, 140 + 40 + 10, HEIGHT, "\x07", 14, palette_index);

        // 金币数
        txt.write_text_ui_gpu(
            commands,
            158 + 40 + 10,
            HEIGHT,
            &format!("{:2}", data_coins[player]),
            31,
            palette_index,
        );

        // 关卡
        let lev = world_number.chars().nth(2).unwrap_or(' ');
        txt.write_text_ui_gpu(
            commands,
            258,
            HEIGHT,
            &format!("LEVEL {}", lev),
            31,
            palette_index,
        );

        // 恢复正常字体
        txt.set_font(0, FontStyle::NORMAL);
        txt.write_text_ui_gpu(commands, 46 + 4, HEIGHT, "x", 31, palette_index);
        txt.write_text_ui_gpu(commands, 150 + 40 + 10, HEIGHT, "x", 31, palette_index);
    }
}
