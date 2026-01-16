// 游戏上下文 - 封装所有游戏子系统的可变引用
// 解决函数参数过多问题，提高代码可维护性

use crate::{
    backgr::BackGr,
    blocks::Blocks,
    buffers::Buffers,
    enemies::Enemies,
    figures::Figures,
    glitter::GlitterSystem,
    joystick::JoystickState,
    keyboard::Keyboard,
    music::MusicPlayer,
    players::Players,
    sprites::{SpriteAtlas, SpriteDataManager},
    stars::Stars,
    status::Status,
    tmpobj::TmpObjManager,
    txt::Txt,
    vga256::VGA,
};

/// 游戏上下文 - 封装所有游戏子系统的可变引用
/// 
/// 用于简化函数签名，避免传递过多参数
/// 使用生命周期 `'a` 确保所有引用在上下文存活期间有效
pub struct GameContext<'a> {
    // 渲染相关（vga.palette 包含调色板）
    pub vga: &'a mut VGA,
    pub txt: &'a mut Txt,
    
    // 游戏状态
    pub buffers: &'a mut Buffers,
    pub players: &'a mut Players,
    pub enemies: &'a mut Enemies,
    
    // 场景元素
    pub backgr: &'a mut BackGr,
    pub figures: &'a mut Figures,
    pub stars: &'a mut Stars,
    pub blocks: &'a mut Blocks,
    
    // 特效和临时对象
    pub glitters: &'a mut GlitterSystem,
    pub tmpobj: &'a mut TmpObjManager,
    pub status: &'a mut Status,
    
    // 资源
    pub sprites: &'a mut SpriteDataManager,
    pub atlas: &'a SpriteAtlas,
    pub music: &'a mut MusicPlayer,
    
    // 输入
    pub keyboard: &'a mut Keyboard,
    pub joystick: &'a mut JoystickState,
    
    // 游戏信息
    pub cur_player: u8,
}

impl<'a> GameContext<'a> {
    /// 创建游戏上下文
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        vga: &'a mut VGA,
        txt: &'a mut Txt,
        buffers: &'a mut Buffers,
        players: &'a mut Players,
        enemies: &'a mut Enemies,
        backgr: &'a mut BackGr,
        figures: &'a mut Figures,
        stars: &'a mut Stars,
        blocks: &'a mut Blocks,
        glitters: &'a mut GlitterSystem,
        tmpobj: &'a mut TmpObjManager,
        status: &'a mut Status,
        sprites: &'a mut SpriteDataManager,
        atlas: &'a SpriteAtlas,
        music: &'a mut MusicPlayer,
        keyboard: &'a mut Keyboard,
        joystick: &'a mut JoystickState,
        cur_player: u8,
    ) -> Self {
        Self {
            vga,
            txt,
            buffers,
            players,
            enemies,
            backgr,
            figures,
            stars,
            blocks,
            glitters,
            tmpobj,
            status,
            sprites,
            atlas,
            music,
            keyboard,
            joystick,
            cur_player,
        }
    }
}
