use crate::render_state::MAX_PAGE;

pub const W: i32 = 20;
pub const H: i32 = 14;
pub const NH: i32 = 16;
pub const NV: i32 = 13;
pub const MAX_WORLD_SIZE: i32 = 236;
pub const EX: i32 = 1;
pub const EY1: i32 = 8;
pub const EY2: i32 = 3;
pub const DIR_LEFT: i32 = 0;
pub const DIR_RIGHT: i32 = 1;
pub const MD_SMALL: usize = 0;
pub const MD_LARGE: usize = 1;
pub const MD_FIRE: usize = 2;
pub const PL_MARIO: i32 = 0;
pub const PL_LUIGI: i32 = 1;

// Pascal: QuitGame: Boolean = FALSE; BeeperSound: Boolean = TRUE;
pub const QUIT_GAME_DEFAULT: bool = false;
pub const BEEPER_SOUND_DEFAULT: bool = true;

// === Pascal缓冲区类型移植 ===

/// ImageBuffer = array [1..H, 1..W] of u8;
pub type ImageBuffer = [[u8; W as usize]; H as usize];

/// ScreenBuffer = array [0..MAX_PAGE] of ImageBuffer;
pub type ScreenBuffer = [ImageBuffer; MAX_PAGE as usize + 1];

/// PicBuffer = array [1..2*H, 1..W] of u8;
/// 玩家(Mario/Luigi)精灵为 20×28 (=W×2H)，必须使用该缓冲区类型，否则只会显示上半截。
pub type PicBuffer = [[u8; W as usize]; (2 * H) as usize];

// 扩展精灵缓冲区类型
// 部分精灵在 Pascal 原版中不是 20x14
// 这里补充常用尺寸，供 sprites/enemies/tmpobj 等模块使用
pub type ImageBuffer20x24 = [[u8; W as usize]; 24];
pub type ImageBuffer24x20 = [[u8; 24]; 20];
pub type ImageBuffer12x7 = [[u8; 12]; 7];

/// PictureBuffer = array [plMario..plLuigi, mdSmall..mdFire, 0..3, dirLeft..dirRight] of PicBuffer;
pub type PictureBuffer = [[[[PicBuffer; 2]; 4]; 3]; 2];

pub trait PictureBufferFill {
    fn fill(&mut self, value: u8);
}

impl PictureBufferFill for PictureBuffer {
    fn fill(&mut self, value: u8) {
        for pl in self.iter_mut() {
            for md in pl.iter_mut() {
                for n in md.iter_mut() {
                    for dir in n.iter_mut() {
                        for row in dir.iter_mut() {
                            for cell in row.iter_mut() {
                                *cell = value;
                            }
                        }
                    }
                }
            }
        }
    }
}

// 2玩家 × 3形态 × 4帧 × 2方向

/// MapBuffer = array [1..MaxWorldSize, 1..NV] of Char;
///
/// Pascal 是 1..MaxWorldSize（包含 MaxWorldSize 这一列），并且 ReadWorld 会访问 M^[X+1,*]。
/// Rust 这里保留第 0 列作为占位，真实数据从列 1 开始写入。
pub type MapBuffer = [[char; NV as usize]; MAX_WORLD_SIZE as usize + 1];

/// StarBuffer = array [0..MAX_PAGE, 0..319] of u8;
pub type StarBuffer = [[u8; 320]; MAX_PAGE as usize + 1];

/// WorldBuffer = array [-EX..MaxWorldSize-1+EX, -EY1..NV-1+EY2] of u8;
pub type WorldBuffer = Vec<Vec<u8>>; // 需动态分配，或用具体大小

pub trait WorldBufferExt {
    fn insert_at(&mut self, pos: (i32, i32), ch: u8);
    fn get_at(&self, pos: (i32, i32)) -> u8;
}

impl WorldBufferExt for WorldBuffer {
    fn insert_at(&mut self, pos: (i32, i32), ch: u8) {
        let (x, y) = pos;
        if x >= 0 && y >= 0 {
            let (xu, yu) = (x as usize, y as usize);
            if xu < self.len() && yu < self[0].len() {
                self[xu][yu] = ch;
            }
        }
    }

    fn get_at(&self, pos: (i32, i32)) -> u8 {
        let (x, y) = pos;
        if x >= 0 && y >= 0 {
            let (xu, yu) = (x as usize, y as usize);
            if xu < self.len() && yu < self[0].len() {
                return self[xu][yu];
            }
        }
        0
    }
}

// 玩家名
pub const PLAYER_NAME: [&str; 2] = ["MARIO", "LUIGI"];

// === 角色判定相关常量 ===
/// 可以抓住你的字符集合（Pascal: [#0..#13, '0'..'Z']）
/// 注意：Pascal 的 '0'..'Z' 是按 ASCII 连续区间，包含 ':', ';', '<', '=', '>', '?', '@' 等符号。
pub const CAN_HOLD_YOU: [u8; 57] = [
    // #0..#13
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
    // '0'..'Z' (0x30..=0x5A)
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E, 0x3F,
    0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F,
    0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A,
];
/// 可以站立的字符集合（Pascal: [#14..#16, 'a'..'f']）
pub const CAN_STAND_ON: [u8; 9] = [
    0x0E, 0x0F, 0x10, // #14, #15, #16
    b'a', b'b', b'c', b'd', b'e', b'f',
];
/// 隐藏字符集合（Pascal: ['$']）
pub const HIDDEN: [u8; 1] = [b'$'];

// === Demo状态常量 ===
pub const DM_NO_DEMO: i32 = 0;
pub const DM_DOWN_INTO_PIPE: i32 = 1;
pub const DM_UP_OUT_OF_PIPE: i32 = 2;
pub const DM_UP_INTO_PIPE: i32 = 3;
pub const DM_DOWN_OUT_OF_PIPE: i32 = 4;
pub const DM_DEAD: i32 = 5;

// GameData结构体
#[derive(Debug, Clone, Default)]
pub struct GameData {
    /// Pascal: Integer (Turbo Pascal -> 16bit signed)
    pub num_players: i16,
    /// Pascal: array[...] of Integer
    pub progress: [i16; 2],
    /// Pascal: array[...] of Integer
    pub lives: [i16; 2],
    /// Pascal: array[...] of Integer
    pub coins: [i16; 2],
    /// Pascal: array[...] of LongInt
    pub score: [i32; 2],
    /// Pascal: array[...] of Byte (mdSmall/mdLarge/mdFire)
    pub mode: [u8; 2],
    /// Pascal: Boolean - Turbo mode (enabled when Progress >= NUM_LEV)
    pub turbo: bool,
}

impl GameData {
    pub fn new() -> Self {
        Self {
            num_players: 1,
            progress: [0, 0],
            lives: [3, 3],
            coins: [0, 0],
            score: [0, 0],
            mode: [0, 0],
            turbo: false,
        }
    }
}

// Buffers主结构体（部分字段，按需扩展）
pub struct Buffers {
    pub player: usize,
    pub data: GameData,
    pub world_number: String,
    pub level_score: i32,
    // Pascal绝对地址定时器变量，Rust用普通字段模拟
    pub timer: i64,   // Timer: LongInt
    pub w_timer: u16, // wTimer: Word
    pub b_timer: u8,  // bTimer: Byte
    // Pascal全局变量
    pub game_done: bool,
    pub passed: bool,
    pub world_map: WorldBuffer,      // WorldMapPtr
    pub save_world_map: WorldBuffer, // WorldMapPtr
    pub options: WorldOptions,       // WorldOptions 结构体需定义
    pub save_options: WorldOptions,  // WorldOptions 结构体需定义
    pub x_view: i32,
    pub y_view: i32,
    pub last_x_view: [i32; (MAX_PAGE + 1) as usize],
    pub star_backgr: Box<StarBuffer>, // StarBufferPtr
    pub size: u16,
    pub pictures: Box<PictureBuffer>, // PictureBufferPtr
    pub demo: i32,
    pub text_counter: i32,
    pub lava_counter: u8,
    // 兼容旧Pascal全局变量
    pub player_name: Vec<String>,
    pub quit_game: bool,
    pub beeper_sound: bool,
    pub playing_macro: bool,
}

// 需要定义WorldOptions结构体
#[derive(Debug, Clone, Default)]
pub struct WorldOptions {
    /// Pascal: Word
    pub init_x: u16,
    /// Pascal: Word
    pub init_y: u16,
    pub sky_type: u8,
    pub wall_type1: u8,
    pub wall_type2: u8,
    pub wall_type3: u8,
    pub pipe_color: u8,
    pub ground_color1: u8,
    pub ground_color2: u8,
    /// Pascal: Byte
    pub horizon: u8,
    pub backgr_type: u8,
    pub backgr_color1: u8,
    pub backgr_color2: u8,
    pub stars: u8,
    pub clouds: u8,
    pub design: u8,
    pub c2r: u8,
    pub c2g: u8,
    pub c2b: u8,
    pub c3r: u8,
    pub c3g: u8,
    pub c3b: u8,
    pub brick_color: u8,
    pub wood_color: u8,
    pub xblock_color: u8,
    pub build_wall: bool,
    /// Pascal: Word
    pub x_size: u16,
}

impl Buffers {
    /// 创建并初始化所有缓冲区和资源
    pub fn new() -> Self {
        // WorldBuffer: [-EX..MAX_WORLD_SIZE-1+EX, -EY1..NV-1+EY2]

        eprintln!("[DEBUG] Buffers::new: 开始.....");

        let w_width = (MAX_WORLD_SIZE + 2 * EX) as usize;

        eprintln!("[DEBUG] Buffers::new: w_width = {}", w_width);
        eprintln!("[DEBUG] Buffers::new: EX = {}", EX);
        eprintln!("[DEBUG] Buffers::new: EY1 = {}", EY1);
        eprintln!("[DEBUG] Buffers::new: EY2 = {}", EY2);
        eprintln!("[DEBUG] Buffers::new: NV = {}", NV);
        eprintln!("[DEBUG] Buffers::new: MAX_WORLD_SIZE = {}", MAX_WORLD_SIZE);
        let w_height = (NV + EY1 + EY2) as usize;

        eprintln!("[DEBUG] Buffers::new: w_height = {}", w_height);
        let world_map = vec![vec![0; w_height]; w_width];

        let save_world_map = vec![vec![0; w_height]; w_width];

        // StarBuffer: [[u8; 320]; MAX_PAGE+1]
        let star_backgr = Box::new([[0u8; 320]; MAX_PAGE as usize + 1]);

        // PictureBuffer: 2玩家 × 3形态 × 4帧 × 2方向 × (2*H × W)
        // 使用堆分配避免在栈上构造大数组导致栈溢出
        let pictures: Box<PictureBuffer> = {
            let mut buf = Box::<std::mem::MaybeUninit<PictureBuffer>>::new_uninit();
            unsafe {
                std::ptr::write_bytes(buf.as_mut_ptr(), 0, 1);
                // 兼容旧Rust版本，不依赖 Box<MaybeUninit<T>>::assume_init
                let ptr = Box::into_raw(buf) as *mut PictureBuffer;
                Box::from_raw(ptr)
            }
        };

        Self {
            player: 0,
            data: GameData::new(), // 必须使用 new() 而非 default()，确保 num_players = 1
            world_number: String::new(),
            level_score: 0,
            timer: 0,
            w_timer: 0,
            b_timer: 0,
            game_done: false,
            passed: false,
            world_map,
            save_world_map: save_world_map,
            options: WorldOptions::default(),
            save_options: WorldOptions::default(),
            x_view: 0,
            y_view: 0,
            last_x_view: Default::default(),
            star_backgr,
            size: 0, // 可选: 计算总内存大小
            pictures,
            demo: 0,
            text_counter: 0,
            lava_counter: 0,
            player_name: PLAYER_NAME.iter().map(|s| s.to_string()).collect(),
            quit_game: QUIT_GAME_DEFAULT,
            beeper_sound: BEEPER_SOUND_DEFAULT,
            playing_macro: false,
        }
    }
}

impl Buffers {
    /// 读取世界地图数据
    pub fn read_world(&mut self, map: &mut MapBuffer, w: &mut WorldBuffer, opt: &WorldOptions) {
        // 1. 拷贝 opt 到 self.options
        self.options = opt.clone();

        // 2. 填充 w 为 ' '（空格，ASCII 32）
        let w_width = w.len() as i32;
        let w_height = if w_width > 0 { w[0].len() as i32 } else { 0 };
        for i in 0..w_width {
            for j in 0..w_height {
                w[i as usize][j as usize] = b' ';
            }
        }

        // 3. 填充左侧边界（i: -EX..-1）为 '@'
        for i in 0..EX {
            let wi = i; // i=-EX 对应 w[0]
            for j in 0..(NV + EY1 + EY2) {
                if wi < w_width && j < w_height {
                    w[wi as usize][j as usize] = b'@';
                }
            }
        }

        // 4. 主循环，填充地图内容
        let mut x = 0;
        // Pascal: While (M^[X + 1, 1] <> #0) and (X < MaxWorldSize) do
        while x < MAX_WORLD_SIZE && map[x as usize + 1][0] != '\0' {
            // for i := 1 to NV do W^[X, NV-i] := M^[X+1, i];
            for i in 0..NV {
                let wx = x + EX;
                let wy = NV - 1 - i + EY1;
                if wx < w_width && wy < w_height {
                    w[wx as usize][wy as usize] = map[x as usize + 1][i as usize] as u8;
                }
            }
            // W^[X, -EY1] := #0;
            let wx = x + EX;
            let wy = 0; // -EY1 对应 w[wx][0]
            if wx < w_width && wy < w_height {
                w[wx as usize][wy as usize] = 0;
            }
            // for i := 1 to EY2 do W^[X, NV-1+i] := W^[X, NV-1];
            for i in 1..=EY2 {
                let wx = x + EX;
                let wy = NV - 1 + i + EY1;
                let src_y = NV - 1 + EY1;
                if wx < w_width && wy < w_height && src_y < w_height {
                    w[wx as usize][wy as usize] = w[wx as usize][src_y as usize];
                }
            }
            x += 1;
        }

        // 5. 设置 self.options.x_size (Pascal: Word)
        self.options.x_size = x as u16;

        // 6. 填充右侧边界（i: X..X+EX-1）为 '@'
        for i in x..(x + EX) {
            let wi = i + EX;
            for j in 0..(NV + EY1 + EY2) {
                if wi < w_width && j < w_height {
                    w[wi as usize][j as usize] = b'@';
                }
            }
        }
    }

    /// 交换世界地图和选项
    pub fn swap(&mut self) {
        // 交换 Options <-> SaveOptions
        std::mem::swap(&mut self.options, &mut self.save_options);

        // 交换 world_map <-> save_world_map
        let w_width = self.world_map.len();
        let w_height = if w_width > 0 {
            self.world_map[0].len()
        } else {
            0
        };
        // i: -EX..MAX_WORLD_SIZE-1+EX
        let x_offset = EX;
        let y_offset = EY1;
        let x_start = -EX;
        let x_end = MAX_WORLD_SIZE - 1 + EX;
        let y_start = -EY1;
        let y_end = NV - 1 + EY2;
        for i in x_start..=x_end {
            let wi = (i + x_offset) as usize;
            if wi >= w_width {
                continue;
            }
            for j in y_start..=y_end {
                let wj = (j + y_offset) as usize;
                if wj >= w_height {
                    continue;
                }
                let c = self.world_map[wi][wj];
                self.world_map[wi][wj] = self.save_world_map[wi][wj];
                self.save_world_map[wi][wj] = c;
            }
        }
    }

    /// 初始化关卡分数
    pub fn init_level_score(&mut self) {
        self.level_score = 0;
        // 若需要同步玩家分数，可在此处实现
    }

    /// 增加分数
    pub fn add_score(&mut self, n: i32) {
        self.level_score += n;
        // // 同步玩家分数，取消下行注释
        // let player = self.player as usize;
        // if player < 2 { self.data.score[player] += self.level_score; }
    }

    /// 读取 WorldMap 指定坐标的字符（对齐 Pascal 的负索引数组语义）
    ///
    /// Pascal: WorldMap^ [X, Y] 的 X/Y 允许为负（X:-EX.., Y:-EY1..），通过指针偏移实现。
    /// Rust: world_map 的第 0 行对应 Pascal 的 X=-EX，第 0 列对应 Pascal 的 Y=-EY1。
    #[inline]
    pub fn world_get(&self, x: i32, y: i32) -> u8 {
        let xx = x + EX;
        let yy = y + EY1;
        if xx < 0
            || yy < 0
            || (xx as usize) >= self.world_map.len()
            || self.world_map.is_empty()
            || (yy as usize) >= self.world_map[0].len()
        {
            0
        } else {
            self.world_map[xx as usize][yy as usize]
        }
    }

    /// 设置 WorldMap 指定坐标的字符（对齐 Pascal 的负索引数组语义）
    #[inline]
    pub fn world_set(&mut self, x: i32, y: i32, ch: u8) {
        let xx = x + EX;
        let yy = y + EY1;
        if xx < 0
            || yy < 0
            || (xx as usize) >= self.world_map.len()
            || self.world_map.is_empty()
            || (yy as usize) >= self.world_map[0].len()
        {
            return;
        }
        self.world_map[xx as usize][yy as usize] = ch;
    }
}
