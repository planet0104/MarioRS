// 配置文件读写模块
//
// 对应 Pascal MARIO.PAS 中的:
// - GetConfigName (line 96-106)
// - ReadConfig (line 108-148)
// - WriteConfig (line 150-168)
//
// 使用平台抽象层的存储接口，支持跨平台

use crate::buffers::GameData;
use crate::mario::{ConfigData, MAX_SAVE};
use crate::platform::{DesktopStorage, log_info, log_warn, log_error, StorageBackend};
use crate::persist as ps;

/// 配置文件存储键名
const CONFIG_KEY: &str = "mario.cfg";

/// 创建新的游戏数据（对应 Pascal NewData 过程）
pub fn new_game_data() -> GameData {
    GameData {
        num_players: 1,
        progress: [0, 0],
        lives: [3, 3],
        coins: [0, 0],
        score: [0, 0],
        mode: [0, 0], // mdSmall
        turbo: false,
    }
}

/// 读取配置文件
/// 
/// 对应 Pascal ReadConfig 过程（MARIO.PAS line 108-148）
/// 
/// 如果配置文件不存在或读取失败，返回默认配置
pub fn read_config() -> ConfigData {
    let storage = DesktopStorage::new();
    
    // 尝试读取配置文件
    if let Some(buffer) = storage.load(CONFIG_KEY) {
        // 尝试手动反序列化（小端）
        if let Ok(config) = deserialize_config(&buffer) {
            log_info("配置文件读取成功");
            return config;
        } else {
            log_warn("配置文件格式错误或版本不兼容，使用默认配置");
        }
    } else {
        log_info("配置文件不存在，使用默认配置");
    }
    
    // 返回默认配置
    // Pascal: 初始化所有存档槽位为空，设置默认选项
    let mut config = ConfigData::default();
    let empty_data = new_game_data();
    for i in 0..MAX_SAVE {
        config.games[i] = empty_data.clone();
        config.games[i].progress = [0, 0]; // 空存档
    }
    config.sline = true;  // 默认显示状态栏
    config.sound = true;  // 默认开启音效
    config.use_js = false; // 默认不使用手柄
    
    config
}

/// 写入配置文件
/// 
/// 对应 Pascal WriteConfig 过程（MARIO.PAS line 150-168）
pub fn write_config(config: &ConfigData) -> bool {
    let mut storage = DesktopStorage::new();
    
    // 序列化配置数据
    match serialize_config(config) {
        Ok(data) => {
            // 写入存储
            match storage.save(CONFIG_KEY, &data) {
                Ok(()) => {
                    log_info("配置文件保存成功");
                    return true;
                }
                Err(e) => {
                    log_error(&format!("写入配置文件失败: {}", e));
                }
            }
        }
        Err(e) => {
            log_error(&format!("序列化配置数据失败: {}", e));
        }
    }
    
    false
}

// use `persist` helpers for LE read/write

fn deserialize_config(data: &[u8]) -> Result<ConfigData, &'static str> {
    let mut cur = ps::Cursor::new(data);
    let mut cfg = ConfigData::default();
    let e = |_| "config parse error";

    // sound, sline
    cfg.sound = ps::read_bool(&mut cur).map_err(e)?;
    cfg.sline = ps::read_bool(&mut cur).map_err(e)?;

    // games: MAX_SAVE 个 GameData 条目
    for i in 0..MAX_SAVE {
        cfg.games[i].num_players = ps::read_i16_le(&mut cur).map_err(e)?;
        cfg.games[i].progress = ps::read_i16_pair(&mut cur).map_err(e)?;
        cfg.games[i].lives = ps::read_i16_pair(&mut cur).map_err(e)?;
        cfg.games[i].coins = ps::read_i16_pair(&mut cur).map_err(e)?;
        cfg.games[i].score = ps::read_i32_pair(&mut cur).map_err(e)?;
        cfg.games[i].mode = ps::read_u8_pair(&mut cur).map_err(e)?;
        cfg.games[i].turbo = ps::read_bool(&mut cur).map_err(e)?;
    }

    // use_js
    cfg.use_js = ps::read_bool(&mut cur).map_err(e)?;

    // jsdat (JoyRec) - 12 个 u16 字段
    let mut js = cfg.jsdat;
    js.x = ps::read_u16_le(&mut cur).map_err(e)?;
    js.y = ps::read_u16_le(&mut cur).map_err(e)?;
    js.x_center = ps::read_u16_le(&mut cur).map_err(e)?;
    js.y_center = ps::read_u16_le(&mut cur).map_err(e)?;
    js.x_min = ps::read_u16_le(&mut cur).map_err(e)?;
    js.y_min = ps::read_u16_le(&mut cur).map_err(e)?;
    js.x_max = ps::read_u16_le(&mut cur).map_err(e)?;
    js.y_max = ps::read_u16_le(&mut cur).map_err(e)?;
    js.x_left = ps::read_u16_le(&mut cur).map_err(e)?;
    js.y_up = ps::read_u16_le(&mut cur).map_err(e)?;
    js.x_right = ps::read_u16_le(&mut cur).map_err(e)?;
    js.y_down = ps::read_u16_le(&mut cur).map_err(e)?;
    cfg.jsdat = js;

    Ok(cfg)
}

fn serialize_config(config: &ConfigData) -> Result<Vec<u8>, &'static str> {
    let mut buf: Vec<u8> = Vec::new();
    let e = |_| "io error";

    ps::write_bool(&mut buf, config.sound).map_err(e)?;
    ps::write_bool(&mut buf, config.sline).map_err(e)?;

    for i in 0..MAX_SAVE {
        let g = &config.games[i];
        ps::write_i16_le(&mut buf, g.num_players).map_err(e)?;
        ps::write_i16_pair(&mut buf, g.progress).map_err(e)?;
        ps::write_i16_pair(&mut buf, g.lives).map_err(e)?;
        ps::write_i16_pair(&mut buf, g.coins).map_err(e)?;
        ps::write_i32_pair(&mut buf, g.score).map_err(e)?;
        ps::write_u8_pair(&mut buf, g.mode).map_err(e)?;
        ps::write_bool(&mut buf, g.turbo).map_err(e)?;
    }

    ps::write_bool(&mut buf, config.use_js).map_err(e)?;

    let js = &config.jsdat;
    ps::write_u16_le(&mut buf, js.x).map_err(e)?;
    ps::write_u16_le(&mut buf, js.y).map_err(e)?;
    ps::write_u16_le(&mut buf, js.x_center).map_err(e)?;
    ps::write_u16_le(&mut buf, js.y_center).map_err(e)?;
    ps::write_u16_le(&mut buf, js.x_min).map_err(e)?;
    ps::write_u16_le(&mut buf, js.y_min).map_err(e)?;
    ps::write_u16_le(&mut buf, js.x_max).map_err(e)?;
    ps::write_u16_le(&mut buf, js.y_max).map_err(e)?;
    ps::write_u16_le(&mut buf, js.x_left).map_err(e)?;
    ps::write_u16_le(&mut buf, js.y_up).map_err(e)?;
    ps::write_u16_le(&mut buf, js.x_right).map_err(e)?;
    ps::write_u16_le(&mut buf, js.y_down).map_err(e)?;

    Ok(buf)
}

/// 检查存档是否为空
/// 
/// Pascal 逻辑：if (Progress[plMario] = 0) and (Progress[plLuigi] = 0) then 'EMPTY'
pub fn is_save_empty(game: &GameData) -> bool {
    game.progress[0] == 0 && game.progress[1] == 0
}

/// 获取存档显示信息
/// 
/// 返回类似 "LEVEL 3 * 2P" 的字符串（Pascal Intro 菜单中的存档显示）
pub fn get_save_display(game: &GameData) -> String {
    if is_save_empty(game) {
        return "EMPTY".to_string();
    }
    
    // 计算最高进度
    let mut level = game.progress[0].max(game.progress[1]);
    let is_turbo = level >= crate::mario::NUM_LEV as i16;
    
    if is_turbo {
        level -= crate::mario::NUM_LEV as i16;
    }
    
    // 生成显示字符串
    let turbo_mark = if is_turbo { "*" } else { "" };
    let players = game.num_players;
    
    format!("LEVEL {} {} {}P", level + 1, turbo_mark, players)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_game_data() {
        let data = new_game_data();
        assert_eq!(data.num_players, 1);
        assert_eq!(data.lives[0], 3);
        assert_eq!(data.progress[0], 0);
    }

    #[test]
    fn test_is_save_empty() {
        let mut data = new_game_data();
        assert!(is_save_empty(&data));
        
        data.progress[0] = 1;
        assert!(!is_save_empty(&data));
    }

    #[test]
    fn test_get_save_display() {
        let mut data = new_game_data();
        assert_eq!(get_save_display(&data), "EMPTY");
        
        data.progress[0] = 2;
        data.num_players = 1;
        assert_eq!(get_save_display(&data), "LEVEL 3  1P");
    }
}
