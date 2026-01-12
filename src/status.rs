// 严格结构体化移植 Pascal STATUS.PAS

use crate::{
    txt::FontStyle,
    vga256::{MAX_PAGE, SCREEN_WIDTH},
};

pub struct Status {
    // Pascal: BackGrAddr: array[0..MAX_PAGE] of Integer; 0 表示无效
    pub backgr_addr: Vec<i32>,
}

impl Status {
    pub fn init_status(&mut self) {
        // FillChar (BackGrAddr, SizeOf (BackGrAddr), #0);
        self.backgr_addr = vec![0; (MAX_PAGE + 1) as usize];
    }

    pub fn show_status(
        &mut self,
        current_page: usize,
        x_view: i32,
        player: usize,
        player_name: &[String],
        data_lives: &[i16],
        level_score: i32,
        data_coins: &[i16],
        world_number: &str,
        txt: &mut crate::txt::Txt,
        vga: &mut crate::vga256::VGA,
    ) {
        // Pascal: BackGrAddr[CurrentPage] := PushBackGr (XView, HEIGHT, SCREEN_WIDTH, 9);
        const HEIGHT: i32 = 6;
        // 重要：不能使用 Vec 版 push_backgr_world（会把 x/y 存成 u8，XView>255 后截断导致闪黑）
        self.backgr_addr[current_page] =
            vga.push_backgr_address_world(x_view, HEIGHT, SCREEN_WIDTH, 9);
        txt.set_font(0, crate::txt::FontStyle::BOLD);
        txt.write_text_world(vga, x_view + 10 + 4, HEIGHT, &player_name[player], 31);
        let mut i = data_lives[player];
        if i > 99 {
            i = 99;
        }
        txt.write_text_world(vga, x_view + 54 + 4, HEIGHT, &format!("{:2}", i), 31);
        txt.write_text_world(
            vga,
            x_view + 84 + 6,
            HEIGHT,
            &format!("{:09}", level_score).replace(' ', "0"),
            31,
        );
        txt.write_text_world(vga, x_view + 140 + 40 + 10, HEIGHT, "\t", 13); // #9
        txt.write_text_world(vga, x_view + 140 + 40 + 10, HEIGHT, "\x07", 14); // #7
        txt.write_text_world(
            vga,
            x_view + 158 + 40 + 10,
            HEIGHT,
            &format!("{:2}", data_coins[player]),
            31,
        );
        // txt.write_text(vga, x_view + 242, HEIGHT, &format!("WORLD {}", world_number), 31); // 注释掉的Pascal
        // Pascal: 'LEVEL ' + WorldNumber[3] (WorldNumber 是 "x-1"，取最后一位)
        let lev = world_number.chars().nth(2).unwrap_or(' ');
        txt.write_text_world(vga, x_view + 258, HEIGHT, &format!("LEVEL {}", lev), 31);
        txt.set_font(0, FontStyle::NORMAL);
        txt.write_text_world(vga, x_view + 46 + 4, HEIGHT, "x", 31);
        txt.write_text_world(vga, x_view + 150 + 40 + 10, HEIGHT, "x", 31);
    }

    pub fn hide_status(&mut self, vga: &mut crate::vga256::VGA) {
        let page = vga.current_page() as usize;
        let addr = *self.backgr_addr.get(page).unwrap_or(&0);
        if addr != 0 {
            vga.pop_backgr_address(addr);
            self.backgr_addr[page] = 0;
        }
    }
}
