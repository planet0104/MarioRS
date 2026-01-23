// 精灵资源模块 - 使用 include_bytes! 嵌入 PNG 文件，运行时解码
//
// 所有精灵 PNG 都在编译时嵌入二进制，启动时解码为像素数据

use std::collections::HashMap;
use std::sync::OnceLock;

// ============================================================================
// 背景高度表 PNG
// ============================================================================

pub static BOGEN_PNG: &[u8] = include_bytes!("../assets/sprites_indexed/BOGEN.png");
pub static BOGEN7_PNG: &[u8] = include_bytes!("../assets/sprites_indexed/BOGEN7.png");
pub static BOGEN26_PNG: &[u8] = include_bytes!("../assets/sprites_indexed/BOGEN26.png");
pub static MOUNT_PNG: &[u8] = include_bytes!("../assets/sprites_indexed/MOUNT.png");

// ============================================================================
// MPAL256 调色板 PNG
// ============================================================================

pub static MPAL256_PNG: &[u8] = include_bytes!("../assets/sprites_indexed/MPAL256.png");

// ============================================================================
// 精灵 PNG - 使用宏批量嵌入
// ============================================================================

macro_rules! include_sprite {
    ($name:ident, $path:literal) => {
        pub static $name: &[u8] = include_bytes!($path);
    };
}

// 20x14 精灵
include_sprite!(BROWN_000_PNG, "../assets/sprites_indexed/BROWN_000.png");
include_sprite!(BROWN_001_PNG, "../assets/sprites_indexed/BROWN_001.png");
include_sprite!(BROWN_002_PNG, "../assets/sprites_indexed/BROWN_002.png");
include_sprite!(BROWN_003_PNG, "../assets/sprites_indexed/BROWN_003.png");
include_sprite!(BROWN_004_PNG, "../assets/sprites_indexed/BROWN_004.png");
include_sprite!(PIPE_000_PNG, "../assets/sprites_indexed/PIPE_000.png");
include_sprite!(PIPE_001_PNG, "../assets/sprites_indexed/PIPE_001.png");
include_sprite!(PIPE_002_PNG, "../assets/sprites_indexed/PIPE_002.png");
include_sprite!(PIPE_003_PNG, "../assets/sprites_indexed/PIPE_003.png");
include_sprite!(GREEN_000_PNG, "../assets/sprites_indexed/GREEN_000.png");
include_sprite!(GREEN_001_PNG, "../assets/sprites_indexed/GREEN_001.png");
include_sprite!(GREEN_002_PNG, "../assets/sprites_indexed/GREEN_002.png");
include_sprite!(GREEN_003_PNG, "../assets/sprites_indexed/GREEN_003.png");
include_sprite!(GREEN_004_PNG, "../assets/sprites_indexed/GREEN_004.png");
include_sprite!(SAND_000_PNG, "../assets/sprites_indexed/SAND_000.png");
include_sprite!(SAND_001_PNG, "../assets/sprites_indexed/SAND_001.png");
include_sprite!(SAND_002_PNG, "../assets/sprites_indexed/SAND_002.png");
include_sprite!(SAND_003_PNG, "../assets/sprites_indexed/SAND_003.png");
include_sprite!(SAND_004_PNG, "../assets/sprites_indexed/SAND_004.png");
include_sprite!(GRASS_000_PNG, "../assets/sprites_indexed/GRASS_000.png");
include_sprite!(GRASS_001_PNG, "../assets/sprites_indexed/GRASS_001.png");
include_sprite!(GRASS_002_PNG, "../assets/sprites_indexed/GRASS_002.png");
include_sprite!(GRASS_003_PNG, "../assets/sprites_indexed/GRASS_003.png");
include_sprite!(GRASS_004_PNG, "../assets/sprites_indexed/GRASS_004.png");
include_sprite!(GRASS1_000_PNG, "../assets/sprites_indexed/GRASS1_000.png");
include_sprite!(GRASS1_001_PNG, "../assets/sprites_indexed/GRASS1_001.png");
include_sprite!(GRASS1_002_PNG, "../assets/sprites_indexed/GRASS1_002.png");
include_sprite!(GRASS2_000_PNG, "../assets/sprites_indexed/GRASS2_000.png");
include_sprite!(GRASS2_001_PNG, "../assets/sprites_indexed/GRASS2_001.png");
include_sprite!(GRASS2_002_PNG, "../assets/sprites_indexed/GRASS2_002.png");
include_sprite!(GRASS3_000_PNG, "../assets/sprites_indexed/GRASS3_000.png");
include_sprite!(GRASS3_001_PNG, "../assets/sprites_indexed/GRASS3_001.png");
include_sprite!(GRASS3_002_PNG, "../assets/sprites_indexed/GRASS3_002.png");
include_sprite!(DES_000_PNG, "../assets/sprites_indexed/DES_000.png");
include_sprite!(DES_001_PNG, "../assets/sprites_indexed/DES_001.png");
include_sprite!(DES_002_PNG, "../assets/sprites_indexed/DES_002.png");
include_sprite!(DES_003_PNG, "../assets/sprites_indexed/DES_003.png");
include_sprite!(DES_004_PNG, "../assets/sprites_indexed/DES_004.png");
include_sprite!(BRICK0_000_PNG, "../assets/sprites_indexed/BRICK0_000.png");
include_sprite!(BRICK0_001_PNG, "../assets/sprites_indexed/BRICK0_001.png");
include_sprite!(BRICK0_002_PNG, "../assets/sprites_indexed/BRICK0_002.png");
include_sprite!(BRICK1_000_PNG, "../assets/sprites_indexed/BRICK1_000.png");
include_sprite!(BRICK1_001_PNG, "../assets/sprites_indexed/BRICK1_001.png");
include_sprite!(BRICK1_002_PNG, "../assets/sprites_indexed/BRICK1_002.png");
include_sprite!(BRICK2_000_PNG, "../assets/sprites_indexed/BRICK2_000.png");
include_sprite!(BRICK2_001_PNG, "../assets/sprites_indexed/BRICK2_001.png");
include_sprite!(BRICK2_002_PNG, "../assets/sprites_indexed/BRICK2_002.png");
include_sprite!(PALM0_000_PNG, "../assets/sprites_indexed/PALM0_000.png");
include_sprite!(PALM0_001_PNG, "../assets/sprites_indexed/PALM0_001.png");
include_sprite!(PALM0_002_PNG, "../assets/sprites_indexed/PALM0_002.png");
include_sprite!(PALM1_000_PNG, "../assets/sprites_indexed/PALM1_000.png");
include_sprite!(PALM1_001_PNG, "../assets/sprites_indexed/PALM1_001.png");
include_sprite!(PALM1_002_PNG, "../assets/sprites_indexed/PALM1_002.png");
include_sprite!(PALM2_000_PNG, "../assets/sprites_indexed/PALM2_000.png");
include_sprite!(PALM2_001_PNG, "../assets/sprites_indexed/PALM2_001.png");
include_sprite!(PALM2_002_PNG, "../assets/sprites_indexed/PALM2_002.png");
include_sprite!(PALM3_000_PNG, "../assets/sprites_indexed/PALM3_000.png");
include_sprite!(PALM3_001_PNG, "../assets/sprites_indexed/PALM3_001.png");
include_sprite!(PALM3_002_PNG, "../assets/sprites_indexed/PALM3_002.png");
include_sprite!(WOOD_000_PNG, "../assets/sprites_indexed/WOOD_000.png");
include_sprite!(XBLOCK_000_PNG, "../assets/sprites_indexed/XBLOCK_000.png");
include_sprite!(BLOCK_000_PNG, "../assets/sprites_indexed/BLOCK_000.png");
include_sprite!(BLOCK_001_PNG, "../assets/sprites_indexed/BLOCK_001.png");
include_sprite!(COIN_000_PNG, "../assets/sprites_indexed/COIN_000.png");
include_sprite!(EXIT_000_PNG, "../assets/sprites_indexed/EXIT_000.png");
include_sprite!(EXIT_001_PNG, "../assets/sprites_indexed/EXIT_001.png");
include_sprite!(WPALM_000_PNG, "../assets/sprites_indexed/WPALM_000.png");
include_sprite!(FENCE_000_PNG, "../assets/sprites_indexed/FENCE_000.png");
include_sprite!(FENCE_001_PNG, "../assets/sprites_indexed/FENCE_001.png");
include_sprite!(SMTREE_000_PNG, "../assets/sprites_indexed/SMTREE_000.png");
include_sprite!(SMTREE_001_PNG, "../assets/sprites_indexed/SMTREE_001.png");
include_sprite!(TREE_000_PNG, "../assets/sprites_indexed/TREE_000.png");
include_sprite!(TREE_001_PNG, "../assets/sprites_indexed/TREE_001.png");
include_sprite!(TREE_002_PNG, "../assets/sprites_indexed/TREE_002.png");
include_sprite!(TREE_003_PNG, "../assets/sprites_indexed/TREE_003.png");
include_sprite!(WINDOW_000_PNG, "../assets/sprites_indexed/WINDOW_000.png");
include_sprite!(WINDOW_001_PNG, "../assets/sprites_indexed/WINDOW_001.png");
include_sprite!(LAVA_000_PNG, "../assets/sprites_indexed/LAVA_000.png");
include_sprite!(LAVA_001_PNG, "../assets/sprites_indexed/LAVA_001.png");
include_sprite!(LAVA2_000_PNG, "../assets/sprites_indexed/LAVA2_000.png");
include_sprite!(LAVA2_001_PNG, "../assets/sprites_indexed/LAVA2_001.png");
include_sprite!(LAVA2_002_PNG, "../assets/sprites_indexed/LAVA2_002.png");
include_sprite!(LAVA2_003_PNG, "../assets/sprites_indexed/LAVA2_003.png");
include_sprite!(LAVA2_004_PNG, "../assets/sprites_indexed/LAVA2_004.png");
include_sprite!(LAVA2_005_PNG, "../assets/sprites_indexed/LAVA2_005.png");
include_sprite!(FALL_000_PNG, "../assets/sprites_indexed/FALL_000.png");
include_sprite!(FALL_001_PNG, "../assets/sprites_indexed/FALL_001.png");
include_sprite!(NOTE_000_PNG, "../assets/sprites_indexed/NOTE_000.png");
include_sprite!(PIN_000_PNG, "../assets/sprites_indexed/PIN_000.png");
include_sprite!(QUEST_000_PNG, "../assets/sprites_indexed/QUEST_000.png");
include_sprite!(QUEST_001_PNG, "../assets/sprites_indexed/QUEST_001.png");
include_sprite!(PALBRICK_000_PNG, "../assets/sprites_indexed/PALBRICK_000.png");
include_sprite!(PALPILL_000_PNG, "../assets/sprites_indexed/PALPILL_000.png");
include_sprite!(PALPILL_001_PNG, "../assets/sprites_indexed/PALPILL_001.png");
include_sprite!(PALPILL_002_PNG, "../assets/sprites_indexed/PALPILL_002.png");
include_sprite!(CHAMP_000_PNG, "../assets/sprites_indexed/CHAMP_000.png");
include_sprite!(POISON_000_PNG, "../assets/sprites_indexed/POISON_000.png");
include_sprite!(LIFE_000_PNG, "../assets/sprites_indexed/LIFE_000.png");
include_sprite!(FLOWER_000_PNG, "../assets/sprites_indexed/FLOWER_000.png");
include_sprite!(STAR_000_PNG, "../assets/sprites_indexed/STAR_000.png");
include_sprite!(CHIBIBO_000_PNG, "../assets/sprites_indexed/CHIBIBO_000.png");
include_sprite!(CHIBIBO_001_PNG, "../assets/sprites_indexed/CHIBIBO_001.png");
include_sprite!(CHIBIBO_002_PNG, "../assets/sprites_indexed/CHIBIBO_002.png");
include_sprite!(CHIBIBO_003_PNG, "../assets/sprites_indexed/CHIBIBO_003.png");
include_sprite!(GRKP_000_PNG, "../assets/sprites_indexed/GRKP_000.png");
include_sprite!(GRKP_001_PNG, "../assets/sprites_indexed/GRKP_001.png");
include_sprite!(RDKP_000_PNG, "../assets/sprites_indexed/RDKP_000.png");
include_sprite!(RDKP_001_PNG, "../assets/sprites_indexed/RDKP_001.png");
include_sprite!(FISH_001_PNG, "../assets/sprites_indexed/FISH_001.png");
include_sprite!(DONUT_000_PNG, "../assets/sprites_indexed/DONUT_000.png");
include_sprite!(DONUT_001_PNG, "../assets/sprites_indexed/DONUT_001.png");
include_sprite!(LIFT1_000_PNG, "../assets/sprites_indexed/LIFT1_000.png");
include_sprite!(RED_000_PNG, "../assets/sprites_indexed/RED_000.png");
include_sprite!(RED_001_PNG, "../assets/sprites_indexed/RED_001.png");
include_sprite!(WHFIRE_000_PNG, "../assets/sprites_indexed/WHFIRE_000.png");
include_sprite!(WHHIT_000_PNG, "../assets/sprites_indexed/WHHIT_000.png");
include_sprite!(F_000_PNG, "../assets/sprites_indexed/F_000.png");
include_sprite!(F_001_PNG, "../assets/sprites_indexed/F_001.png");
include_sprite!(F_002_PNG, "../assets/sprites_indexed/F_002.png");
include_sprite!(F_003_PNG, "../assets/sprites_indexed/F_003.png");

// 20x28 角色精灵
include_sprite!(SJMAR_000_PNG, "../assets/sprites_indexed/SJMAR_000.png");
include_sprite!(SJMAR_001_PNG, "../assets/sprites_indexed/SJMAR_001.png");
include_sprite!(LJMAR_000_PNG, "../assets/sprites_indexed/LJMAR_000.png");
include_sprite!(LJMAR_001_PNG, "../assets/sprites_indexed/LJMAR_001.png");
include_sprite!(SWMAR_000_PNG, "../assets/sprites_indexed/SWMAR_000.png");
include_sprite!(SWMAR_001_PNG, "../assets/sprites_indexed/SWMAR_001.png");
include_sprite!(LWMAR_000_PNG, "../assets/sprites_indexed/LWMAR_000.png");
include_sprite!(LWMAR_001_PNG, "../assets/sprites_indexed/LWMAR_001.png");
include_sprite!(FJMAR_000_PNG, "../assets/sprites_indexed/FJMAR_000.png");
include_sprite!(FJMAR_001_PNG, "../assets/sprites_indexed/FJMAR_001.png");
include_sprite!(FWMAR_000_PNG, "../assets/sprites_indexed/FWMAR_000.png");
include_sprite!(FWMAR_001_PNG, "../assets/sprites_indexed/FWMAR_001.png");
include_sprite!(SJLUI_000_PNG, "../assets/sprites_indexed/SJLUI_000.png");
include_sprite!(SJLUI_001_PNG, "../assets/sprites_indexed/SJLUI_001.png");
include_sprite!(LJLUI_000_PNG, "../assets/sprites_indexed/LJLUI_000.png");
include_sprite!(LJLUI_001_PNG, "../assets/sprites_indexed/LJLUI_001.png");
include_sprite!(SWLUI_000_PNG, "../assets/sprites_indexed/SWLUI_000.png");
include_sprite!(SWLUI_001_PNG, "../assets/sprites_indexed/SWLUI_001.png");
include_sprite!(LWLUI_000_PNG, "../assets/sprites_indexed/LWLUI_000.png");
include_sprite!(LWLUI_001_PNG, "../assets/sprites_indexed/LWLUI_001.png");
include_sprite!(FJLUI_000_PNG, "../assets/sprites_indexed/FJLUI_000.png");
include_sprite!(FJLUI_001_PNG, "../assets/sprites_indexed/FJLUI_001.png");
include_sprite!(FWLUI_000_PNG, "../assets/sprites_indexed/FWLUI_000.png");
include_sprite!(FWLUI_001_PNG, "../assets/sprites_indexed/FWLUI_001.png");

// 20x24 敌人精灵
include_sprite!(GRKOOPA_000_PNG, "../assets/sprites_indexed/GRKOOPA_000.png");
include_sprite!(GRKOOPA_001_PNG, "../assets/sprites_indexed/GRKOOPA_001.png");
include_sprite!(RDKOOPA_000_PNG, "../assets/sprites_indexed/RDKOOPA_000.png");
include_sprite!(RDKOOPA_001_PNG, "../assets/sprites_indexed/RDKOOPA_001.png");

// 24x20 精灵
include_sprite!(PPLANT_000_PNG, "../assets/sprites_indexed/PPLANT_000.png");
include_sprite!(PPLANT_001_PNG, "../assets/sprites_indexed/PPLANT_001.png");
include_sprite!(PPLANT_002_PNG, "../assets/sprites_indexed/PPLANT_002.png");
include_sprite!(PPLANT_003_PNG, "../assets/sprites_indexed/PPLANT_003.png");
include_sprite!(HIT_000_PNG, "../assets/sprites_indexed/HIT_000.png");

// 12x7 小精灵
include_sprite!(FIRE_000_PNG, "../assets/sprites_indexed/FIRE_000.png");
include_sprite!(FIRE_001_PNG, "../assets/sprites_indexed/FIRE_001.png");
include_sprite!(PART_000_PNG, "../assets/sprites_indexed/PART_000.png");

// INTRO 和 START 特殊精灵
include_sprite!(INTRO_000_PNG, "../assets/sprites_indexed/INTRO_000.png");
include_sprite!(INTRO_001_PNG, "../assets/sprites_indexed/INTRO_001.png");
include_sprite!(INTRO_002_PNG, "../assets/sprites_indexed/INTRO_002.png");
include_sprite!(START_000_PNG, "../assets/sprites_indexed/START_000.png");
include_sprite!(START_001_PNG, "../assets/sprites_indexed/START_001.png");

// ============================================================================
// PNG 解码缓存
// ============================================================================

static SPRITE_CACHE: OnceLock<HashMap<&'static str, Vec<u8>>> = OnceLock::new();

/// 初始化精灵缓存（解码所有 PNG）
fn init_sprite_cache() -> HashMap<&'static str, Vec<u8>> {
    let mut cache = HashMap::new();
    
    // 解码并缓存所有精灵
    macro_rules! decode_sprite {
        ($name:literal, $data:expr) => {
            cache.insert($name, decode_grayscale_png($data));
        };
    }
    
    // 背景高度表
    decode_sprite!("BOGEN", BOGEN_PNG);
    decode_sprite!("BOGEN7", BOGEN7_PNG);
    decode_sprite!("BOGEN26", BOGEN26_PNG);
    decode_sprite!("MOUNT", MOUNT_PNG);
    
    // 20x14 精灵
    decode_sprite!("BROWN_000", BROWN_000_PNG);
    decode_sprite!("BROWN_001", BROWN_001_PNG);
    decode_sprite!("BROWN_002", BROWN_002_PNG);
    decode_sprite!("BROWN_003", BROWN_003_PNG);
    decode_sprite!("BROWN_004", BROWN_004_PNG);
    decode_sprite!("PIPE_000", PIPE_000_PNG);
    decode_sprite!("PIPE_001", PIPE_001_PNG);
    decode_sprite!("PIPE_002", PIPE_002_PNG);
    decode_sprite!("PIPE_003", PIPE_003_PNG);
    decode_sprite!("GREEN_000", GREEN_000_PNG);
    decode_sprite!("GREEN_001", GREEN_001_PNG);
    decode_sprite!("GREEN_002", GREEN_002_PNG);
    decode_sprite!("GREEN_003", GREEN_003_PNG);
    decode_sprite!("GREEN_004", GREEN_004_PNG);
    decode_sprite!("SAND_000", SAND_000_PNG);
    decode_sprite!("SAND_001", SAND_001_PNG);
    decode_sprite!("SAND_002", SAND_002_PNG);
    decode_sprite!("SAND_003", SAND_003_PNG);
    decode_sprite!("SAND_004", SAND_004_PNG);
    decode_sprite!("GRASS_000", GRASS_000_PNG);
    decode_sprite!("GRASS_001", GRASS_001_PNG);
    decode_sprite!("GRASS_002", GRASS_002_PNG);
    decode_sprite!("GRASS_003", GRASS_003_PNG);
    decode_sprite!("GRASS_004", GRASS_004_PNG);
    decode_sprite!("GRASS1_000", GRASS1_000_PNG);
    decode_sprite!("GRASS1_001", GRASS1_001_PNG);
    decode_sprite!("GRASS1_002", GRASS1_002_PNG);
    decode_sprite!("GRASS2_000", GRASS2_000_PNG);
    decode_sprite!("GRASS2_001", GRASS2_001_PNG);
    decode_sprite!("GRASS2_002", GRASS2_002_PNG);
    decode_sprite!("GRASS3_000", GRASS3_000_PNG);
    decode_sprite!("GRASS3_001", GRASS3_001_PNG);
    decode_sprite!("GRASS3_002", GRASS3_002_PNG);
    decode_sprite!("DES_000", DES_000_PNG);
    decode_sprite!("DES_001", DES_001_PNG);
    decode_sprite!("DES_002", DES_002_PNG);
    decode_sprite!("DES_003", DES_003_PNG);
    decode_sprite!("DES_004", DES_004_PNG);
    decode_sprite!("BRICK0_000", BRICK0_000_PNG);
    decode_sprite!("BRICK0_001", BRICK0_001_PNG);
    decode_sprite!("BRICK0_002", BRICK0_002_PNG);
    decode_sprite!("BRICK1_000", BRICK1_000_PNG);
    decode_sprite!("BRICK1_001", BRICK1_001_PNG);
    decode_sprite!("BRICK1_002", BRICK1_002_PNG);
    decode_sprite!("BRICK2_000", BRICK2_000_PNG);
    decode_sprite!("BRICK2_001", BRICK2_001_PNG);
    decode_sprite!("BRICK2_002", BRICK2_002_PNG);
    decode_sprite!("PALM0_000", PALM0_000_PNG);
    decode_sprite!("PALM0_001", PALM0_001_PNG);
    decode_sprite!("PALM0_002", PALM0_002_PNG);
    decode_sprite!("PALM1_000", PALM1_000_PNG);
    decode_sprite!("PALM1_001", PALM1_001_PNG);
    decode_sprite!("PALM1_002", PALM1_002_PNG);
    decode_sprite!("PALM2_000", PALM2_000_PNG);
    decode_sprite!("PALM2_001", PALM2_001_PNG);
    decode_sprite!("PALM2_002", PALM2_002_PNG);
    decode_sprite!("PALM3_000", PALM3_000_PNG);
    decode_sprite!("PALM3_001", PALM3_001_PNG);
    decode_sprite!("PALM3_002", PALM3_002_PNG);
    decode_sprite!("WOOD_000", WOOD_000_PNG);
    decode_sprite!("XBLOCK_000", XBLOCK_000_PNG);
    decode_sprite!("BLOCK_000", BLOCK_000_PNG);
    decode_sprite!("BLOCK_001", BLOCK_001_PNG);
    decode_sprite!("COIN_000", COIN_000_PNG);
    decode_sprite!("EXIT_000", EXIT_000_PNG);
    decode_sprite!("EXIT_001", EXIT_001_PNG);
    decode_sprite!("WPALM_000", WPALM_000_PNG);
    decode_sprite!("FENCE_000", FENCE_000_PNG);
    decode_sprite!("FENCE_001", FENCE_001_PNG);
    decode_sprite!("SMTREE_000", SMTREE_000_PNG);
    decode_sprite!("SMTREE_001", SMTREE_001_PNG);
    decode_sprite!("TREE_000", TREE_000_PNG);
    decode_sprite!("TREE_001", TREE_001_PNG);
    decode_sprite!("TREE_002", TREE_002_PNG);
    decode_sprite!("TREE_003", TREE_003_PNG);
    decode_sprite!("WINDOW_000", WINDOW_000_PNG);
    decode_sprite!("WINDOW_001", WINDOW_001_PNG);
    decode_sprite!("LAVA_000", LAVA_000_PNG);
    decode_sprite!("LAVA_001", LAVA_001_PNG);
    decode_sprite!("LAVA2_000", LAVA2_000_PNG);
    decode_sprite!("LAVA2_001", LAVA2_001_PNG);
    decode_sprite!("LAVA2_002", LAVA2_002_PNG);
    decode_sprite!("LAVA2_003", LAVA2_003_PNG);
    decode_sprite!("LAVA2_004", LAVA2_004_PNG);
    decode_sprite!("LAVA2_005", LAVA2_005_PNG);
    decode_sprite!("FALL_000", FALL_000_PNG);
    decode_sprite!("FALL_001", FALL_001_PNG);
    decode_sprite!("NOTE_000", NOTE_000_PNG);
    decode_sprite!("PIN_000", PIN_000_PNG);
    decode_sprite!("QUEST_000", QUEST_000_PNG);
    decode_sprite!("QUEST_001", QUEST_001_PNG);
    decode_sprite!("PALBRICK_000", PALBRICK_000_PNG);
    decode_sprite!("PALPILL_000", PALPILL_000_PNG);
    decode_sprite!("PALPILL_001", PALPILL_001_PNG);
    decode_sprite!("PALPILL_002", PALPILL_002_PNG);
    decode_sprite!("CHAMP_000", CHAMP_000_PNG);
    decode_sprite!("POISON_000", POISON_000_PNG);
    decode_sprite!("LIFE_000", LIFE_000_PNG);
    decode_sprite!("FLOWER_000", FLOWER_000_PNG);
    decode_sprite!("STAR_000", STAR_000_PNG);
    decode_sprite!("CHIBIBO_000", CHIBIBO_000_PNG);
    decode_sprite!("CHIBIBO_001", CHIBIBO_001_PNG);
    decode_sprite!("CHIBIBO_002", CHIBIBO_002_PNG);
    decode_sprite!("CHIBIBO_003", CHIBIBO_003_PNG);
    decode_sprite!("GRKP_000", GRKP_000_PNG);
    decode_sprite!("GRKP_001", GRKP_001_PNG);
    decode_sprite!("RDKP_000", RDKP_000_PNG);
    decode_sprite!("RDKP_001", RDKP_001_PNG);
    decode_sprite!("FISH_001", FISH_001_PNG);
    decode_sprite!("DONUT_000", DONUT_000_PNG);
    decode_sprite!("DONUT_001", DONUT_001_PNG);
    decode_sprite!("LIFT1_000", LIFT1_000_PNG);
    decode_sprite!("RED_000", RED_000_PNG);
    decode_sprite!("RED_001", RED_001_PNG);
    decode_sprite!("WHFIRE_000", WHFIRE_000_PNG);
    decode_sprite!("WHHIT_000", WHHIT_000_PNG);
    decode_sprite!("F_000", F_000_PNG);
    decode_sprite!("F_001", F_001_PNG);
    decode_sprite!("F_002", F_002_PNG);
    decode_sprite!("F_003", F_003_PNG);
    
    // 20x28 角色精灵
    decode_sprite!("SJMAR_000", SJMAR_000_PNG);
    decode_sprite!("SJMAR_001", SJMAR_001_PNG);
    decode_sprite!("LJMAR_000", LJMAR_000_PNG);
    decode_sprite!("LJMAR_001", LJMAR_001_PNG);
    decode_sprite!("SWMAR_000", SWMAR_000_PNG);
    decode_sprite!("SWMAR_001", SWMAR_001_PNG);
    decode_sprite!("LWMAR_000", LWMAR_000_PNG);
    decode_sprite!("LWMAR_001", LWMAR_001_PNG);
    decode_sprite!("FJMAR_000", FJMAR_000_PNG);
    decode_sprite!("FJMAR_001", FJMAR_001_PNG);
    decode_sprite!("FWMAR_000", FWMAR_000_PNG);
    decode_sprite!("FWMAR_001", FWMAR_001_PNG);
    decode_sprite!("SJLUI_000", SJLUI_000_PNG);
    decode_sprite!("SJLUI_001", SJLUI_001_PNG);
    decode_sprite!("LJLUI_000", LJLUI_000_PNG);
    decode_sprite!("LJLUI_001", LJLUI_001_PNG);
    decode_sprite!("SWLUI_000", SWLUI_000_PNG);
    decode_sprite!("SWLUI_001", SWLUI_001_PNG);
    decode_sprite!("LWLUI_000", LWLUI_000_PNG);
    decode_sprite!("LWLUI_001", LWLUI_001_PNG);
    decode_sprite!("FJLUI_000", FJLUI_000_PNG);
    decode_sprite!("FJLUI_001", FJLUI_001_PNG);
    decode_sprite!("FWLUI_000", FWLUI_000_PNG);
    decode_sprite!("FWLUI_001", FWLUI_001_PNG);
    
    // 20x24 敌人精灵
    decode_sprite!("GRKOOPA_000", GRKOOPA_000_PNG);
    decode_sprite!("GRKOOPA_001", GRKOOPA_001_PNG);
    decode_sprite!("RDKOOPA_000", RDKOOPA_000_PNG);
    decode_sprite!("RDKOOPA_001", RDKOOPA_001_PNG);
    
    // 24x20 精灵
    decode_sprite!("PPLANT_000", PPLANT_000_PNG);
    decode_sprite!("PPLANT_001", PPLANT_001_PNG);
    decode_sprite!("PPLANT_002", PPLANT_002_PNG);
    decode_sprite!("PPLANT_003", PPLANT_003_PNG);
    decode_sprite!("HIT_000", HIT_000_PNG);
    
    // 12x7 小精灵
    decode_sprite!("FIRE_000", FIRE_000_PNG);
    decode_sprite!("FIRE_001", FIRE_001_PNG);
    decode_sprite!("PART_000", PART_000_PNG);
    
    // INTRO 和 START 特殊精灵
    decode_sprite!("INTRO_000", INTRO_000_PNG);
    decode_sprite!("INTRO_001", INTRO_001_PNG);
    decode_sprite!("INTRO_002", INTRO_002_PNG);
    decode_sprite!("START_000", START_000_PNG);
    decode_sprite!("START_001", START_001_PNG);
    
    cache
}

/// 解码灰度 PNG 为像素数组（直接获取原始像素值，不进行颜色转换）
fn decode_grayscale_png(data: &[u8]) -> Vec<u8> {
    use image::GenericImageView;
    
    let img = image::load_from_memory(data)
        .expect("Failed to decode PNG");
    
    // 直接获取第一通道的原始值（对于灰度图就是灰度值/索引值）
    let (width, height) = img.dimensions();
    let mut pixels = Vec::with_capacity((width * height) as usize);
    
    for y in 0..height {
        for x in 0..width {
            // 获取像素的第一通道（R 或 灰度值）
            let pixel = img.get_pixel(x, y);
            pixels.push(pixel[0]);
        }
    }
    
    pixels
}

/// 获取精灵像素数据
pub fn get_sprite(name: &str) -> Option<&'static Vec<u8>> {
    let cache = SPRITE_CACHE.get_or_init(init_sprite_cache);
    cache.get(name)
}

/// 获取背景高度表
pub fn get_background(name: &str) -> &'static [u8] {
    get_sprite(name).map(|v| v.as_slice()).unwrap_or(&[])
}

// ============================================================================
// MPAL256 调色板
// ============================================================================

static MPAL256_CACHE: OnceLock<[[u8; 3]; 256]> = OnceLock::new();

/// 解码 MPAL256 调色板
fn decode_mpal256() -> [[u8; 3]; 256] {
    let img = image::load_from_memory(MPAL256_PNG)
        .expect("Failed to decode MPAL256 PNG");
    let rgb = img.to_rgb8();
    let pixels = rgb.into_raw();
    
    let mut palette = [[0u8; 3]; 256];
    for i in 0..256 {
        palette[i][0] = pixels.get(i * 3).copied().unwrap_or(0);
        palette[i][1] = pixels.get(i * 3 + 1).copied().unwrap_or(0);
        palette[i][2] = pixels.get(i * 3 + 2).copied().unwrap_or(0);
    }
    palette
}

/// 获取 MPAL256 调色板
pub fn get_mpal256_palette() -> &'static [[u8; 3]; 256] {
    MPAL256_CACHE.get_or_init(decode_mpal256)
}
