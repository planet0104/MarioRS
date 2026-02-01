#![cfg(target_arch = "wasm32")]

//! 微信小游戏平台实现 - CPU 软件渲染版本
//!
//! 使用纯 CPU 软件渲染，然后将帧缓冲通过 Canvas 2D API 显示
//! 提供最佳的设备兼容性，适合低端设备

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::console;
// use web_sys::CanvasRenderingContext2d;
use js_sys;
use js_sys::Uint8ClampedArray;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// Base64 编码/解码用于微信存储
use base64::{Engine as _, engine::general_purpose};

use crate::cpu::CpuRenderer;
use crate::game_runner::GameState;
use crate::platform::FrameResult;
use crate::status::RenderMode;
use crate::platform::{
    AudioBackend, DisplayBackend, InputBackend, StorageBackend as PersistBackend,
    KeyCode as PlatformKeyCode, KeyEvent as PlatformKeyEvent,
};
use crate::gpu::types::{GAME_WIDTH, GAME_HEIGHT};

// ============================================================================
// 常量
// ============================================================================

const GAME_WIDTH_U32: u32 = GAME_WIDTH;
const GAME_HEIGHT_U32: u32 = GAME_HEIGHT;

// ============================================================================
// 日志函数
// ============================================================================

pub fn log_info(msg: &str) {
    console::log_1(&JsValue::from_str(msg));
}

pub fn log_warn(msg: &str) {
    console::warn_1(&JsValue::from_str(msg));
}

pub fn log_error(msg: &str) {
    console::error_1(&JsValue::from_str(msg));
}

/// 将 JsValue（可能是 Error 对象）转换为可读的字符串
fn js_value_to_string(e: &JsValue) -> String {
    if e.is_undefined() {
        return "undefined".to_string();
    }
    if let Some(s) = e.as_string() {
        return s;
    }

    // 尝试读取 .message 字段
    if let Ok(msg) = js_sys::Reflect::get(e, &JsValue::from_str("message")) {
        if let Some(s) = msg.as_string() {
            return s;
        }
    }

    // 尝试调用 toString()
    if let Ok(to_str) = js_sys::Reflect::get(e, &JsValue::from_str("toString")) {
        if let Some(func) = to_str.dyn_ref::<js_sys::Function>() {
            if let Ok(res) = func.call0(e) {
                if let Some(s) = res.as_string() {
                    return s;
                }
            }
        }
    }

    // 最后回退到 Debug 格式
    format!("{:?}", e)
}

// ============================================================================
// 帧计时器
// ============================================================================

struct FrameTimer {
    frame_duration_ms: f64,
    next_frame: f64,
}

impl FrameTimer {
    fn new(target_fps: f64) -> Self {
        let now = get_time_ms();
        Self {
            frame_duration_ms: 1000.0 / target_fps,
            next_frame: now,
        }
    }

    fn should_render(&self) -> bool {
        let now = get_time_ms();
        now >= self.next_frame
    }

    fn advance(&mut self) {
        let now = get_time_ms();
        self.next_frame = now + self.frame_duration_ms;
    }
}

struct FpsCounter {
    frame_count: u32,
    frame_time_accumulator: f32,
    last_update: f64,
    fps_display: u32,
    frame_time_display: f32,
}

impl FpsCounter {
    fn new() -> Self {
        let now = get_time_ms();
        Self {
            frame_count: 0,
            frame_time_accumulator: 0.0,
            last_update: now,
            fps_display: 0,
            frame_time_display: 0.0,
        }
    }

    fn update(&mut self, frame_time_ms: f64) {
        self.frame_count += 1;
        self.frame_time_accumulator += frame_time_ms as f32;
        
        let now = get_time_ms();
        let elapsed = now - self.last_update;
        
        if elapsed >= 1000.0 {
            self.fps_display = self.frame_count;
            self.frame_time_display = if self.frame_count > 0 {
                self.frame_time_accumulator / self.frame_count as f32
            } else {
                0.0
            };
            self.frame_count = 0;
            self.frame_time_accumulator = 0.0;
            self.last_update = now;
        }
    }

    fn fps(&self) -> u32 {
        self.fps_display
    }

    fn frame_time_ms(&self) -> f32 {
        self.frame_time_display
    }
}

// ============================================================================
// 全局对象访问 (兼容微信小游戏)
// ============================================================================

fn get_global() -> JsValue {
    js_sys::global().into()
}

/// 调用 JS 端的虚拟控制器渲染函数
fn render_virtual_controller() {
    let global = get_global();
    
    // 尝试从 GameGlobal 获取 renderVirtualController 函数
    if let Ok(game_global) = js_sys::Reflect::get(&global, &JsValue::from_str("GameGlobal")) {
        if !game_global.is_undefined() && !game_global.is_null() {
            if let Ok(render_fn) = js_sys::Reflect::get(&game_global, &JsValue::from_str("renderVirtualController")) {
                if let Some(func) = render_fn.dyn_ref::<js_sys::Function>() {
                    let _ = func.call0(&game_global);
                }
            }
        }
    }
}

/// 获取微信小游戏的 canvas
fn get_wx_canvas() -> Option<JsValue> {
    let global = get_global();
    
    // 尝试获取 GameGlobal.__wxGameCanvas
    if let Ok(game_global) = js_sys::Reflect::get(&global, &JsValue::from_str("GameGlobal")) {
        if !game_global.is_undefined() && !game_global.is_null() {
            if let Ok(canvas) = js_sys::Reflect::get(&game_global, &JsValue::from_str("__wxGameCanvas")) {
                if !canvas.is_undefined() && !canvas.is_null() {
                    log_info("使用微信小游戏 canvas (__wxGameCanvas)");
                    return Some(canvas);
                }
            }
        }
    }
    
    None
}

/// 获取 canvas 尺寸
fn get_canvas_size(canvas: &JsValue) -> (u32, u32) {
    let width = js_sys::Reflect::get(canvas, &JsValue::from_str("width"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(320.0) as u32;
    let height = js_sys::Reflect::get(canvas, &JsValue::from_str("height"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(182.0) as u32;
    (width.max(1), height.max(1))
}

/// 获取设备像素比（DPR）
fn get_device_pixel_ratio() -> f64 {
    let global = get_global();
    
    // 微信小游戏：从 wx.getSystemInfoSync().pixelRatio 获取
    if let Ok(wx) = js_sys::Reflect::get(&global, &JsValue::from_str("wx")) {
        if !wx.is_undefined() && !wx.is_null() {
            if let Ok(get_system_info_fn) = js_sys::Reflect::get(&wx, &JsValue::from_str("getSystemInfoSync")) {
                if let Some(func) = get_system_info_fn.dyn_ref::<js_sys::Function>() {
                    if let Ok(info) = func.call0(&wx) {
                        if let Ok(pixel_ratio) = js_sys::Reflect::get(&info, &JsValue::from_str("pixelRatio")) {
                            if let Some(dpr) = pixel_ratio.as_f64() {
                                return dpr;
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 标准浏览器：从 window.devicePixelRatio 获取
    if let Ok(dpr_val) = js_sys::Reflect::get(&global, &JsValue::from_str("devicePixelRatio")) {
        if let Some(dpr) = dpr_val.as_f64() {
            return dpr;
        }
    }
    
    // 回退默认值
    1.0
}

/// 获取 Canvas 2D 上下文
/// 获取 Canvas 2D 上下文（微信小游戏兼容版）
fn get_2d_context(canvas: &JsValue) -> Result<JsValue, String> {
    // 尝试多种方式获取上下文
    
    // 方式1：标准浏览器方式
    if let Ok(get_context) = js_sys::Reflect::get(canvas, &JsValue::from_str("getContext")) {
        if let Some(func) = get_context.dyn_ref::<js_sys::Function>() {
            let ctx = func.call1(canvas, &JsValue::from_str("2d"));
            if let Ok(ctx) = ctx {
                if !ctx.is_null() && !ctx.is_undefined() {
                    log_info("使用标准getContext获取2D上下文成功");
                    return Ok(ctx);
                }
            }
        }
    }
    
    // 方式2：微信小游戏可能的方式
    log_info("尝试微信小游戏特定方式获取上下文");
    
    // 检查是否有微信特定的方法
    let wx_get_context = js_sys::Reflect::get(canvas, &JsValue::from_str("getContext"))
        .or_else(|_| js_sys::Reflect::get(canvas, &JsValue::from_str("_getContext")))
        .or_else(|_| js_sys::Reflect::get(canvas, &JsValue::from_str("wxGetContext")));
    
    if let Ok(get_context_fn) = wx_get_context {
        if let Some(func) = get_context_fn.dyn_ref::<js_sys::Function>() {
            // 尝试带参数的版本
            let options = js_sys::Object::new();
            js_sys::Reflect::set(&options, &"type".into(), &"2d".into()).ok();
            js_sys::Reflect::set(&options, &"alpha".into(), &true.into()).ok();
            js_sys::Reflect::set(&options, &"antialias".into(), &false.into()).ok();
            
            let ctx = func.call1(canvas, &JsValue::from_str("2d"))
                .or_else(|_| func.call1(canvas, &options))
                .or_else(|_| func.call0(canvas));
            
            if let Ok(ctx) = ctx {
                if !ctx.is_null() && !ctx.is_undefined() {
                    log_info("使用微信特定方式获取上下文成功");
                    return Ok(ctx);
                }
            }
        }
    }
    
    // 方式3：直接通过全局对象获取
    log_info("尝试直接获取上下文属性");
    let ctx_prop = js_sys::Reflect::get(canvas, &JsValue::from_str("context"))
        .or_else(|_| js_sys::Reflect::get(canvas, &JsValue::from_str("ctx")))
        .or_else(|_| js_sys::Reflect::get(canvas, &JsValue::from_str("_ctx")));
    
    if let Ok(ctx) = ctx_prop {
        if !ctx.is_null() && !ctx.is_undefined() {
            // 验证这是否真的是2D上下文
            if let Ok(_) = js_sys::Reflect::get(&ctx, &"drawImage".into()) {
                log_info("通过直接属性获取上下文成功");
                return Ok(ctx);
            }
        }
    }
    
    Err("无法获取 Canvas 2D 上下文（所有方法都失败）".to_string())
}

/// 设置图像平滑（增强版 - 支持移动端和微信小游戏）
fn set_image_smoothing(ctx: &JsValue, enabled: bool) {
    // 标准属性
    let _ = js_sys::Reflect::set(ctx, &JsValue::from_str("imageSmoothingEnabled"), &JsValue::from(enabled));
    
    // Webkit 前缀（iOS Safari, 微信小游戏等）
    let _ = js_sys::Reflect::set(ctx, &JsValue::from_str("webkitImageSmoothingEnabled"), &JsValue::from(enabled));
    
    // Mozilla 前缀
    let _ = js_sys::Reflect::set(ctx, &JsValue::from_str("mozImageSmoothingEnabled"), &JsValue::from(enabled));
    
    // Microsoft 前缀
    let _ = js_sys::Reflect::set(ctx, &JsValue::from_str("msImageSmoothingEnabled"), &JsValue::from(enabled));
    
    // Opera 前缀
    let _ = js_sys::Reflect::set(ctx, &JsValue::from_str("oImageSmoothingEnabled"), &JsValue::from(enabled));
    
    // 如果禁用平滑，设置 imageSmoothingQuality 为最低质量（最接近像素风格）
    if !enabled {
        // 可选值: "low" | "medium" | "high"
        // 使用 "low" 来获得最接近像素风格的效果
        let _ = js_sys::Reflect::set(ctx, &JsValue::from_str("imageSmoothingQuality"), &JsValue::from_str("low"));
        
        log_info("图像平滑已禁用（像素风格），imageSmoothingQuality=low");
    } else {
        log_info("图像平滑已启用");
    }
}

/// 设置 Canvas 元素的 CSS 样式为像素风格（移动端兼容性）
fn set_canvas_pixel_style(canvas: &JsValue) {
    // 获取 canvas.style 对象
    if let Ok(style) = js_sys::Reflect::get(canvas, &JsValue::from_str("style")) {
        if !style.is_undefined() && !style.is_null() {
            // 设置多个 image-rendering 属性以确保跨浏览器兼容性
            
            // 标准属性（现代浏览器）
            let _ = js_sys::Reflect::set(&style, &JsValue::from_str("imageRendering"), &JsValue::from_str("pixelated"));
            
            // Webkit/Chrome 旧版本
            let _ = js_sys::Reflect::set(&style, &JsValue::from_str("imageRendering"), &JsValue::from_str("-webkit-optimize-contrast"));
            
            // Firefox 旧版本
            let _ = js_sys::Reflect::set(&style, &JsValue::from_str("imageRendering"), &JsValue::from_str("-moz-crisp-edges"));
            
            // Microsoft Edge 旧版本
            let _ = js_sys::Reflect::set(&style, &JsValue::from_str("imageRendering"), &JsValue::from_str("-ms-interpolation-mode"));
            
            // 最兼容的设置（优先使用 pixelated）
            let _ = js_sys::Reflect::set(&style, &JsValue::from_str("imageRendering"), &JsValue::from_str("crisp-edges"));
            
            // 最后再设置一次 pixelated（现代浏览器首选）
            let _ = js_sys::Reflect::set(&style, &JsValue::from_str("imageRendering"), &JsValue::from_str("pixelated"));
            
            log_info("Canvas CSS 像素风格已设置");
        }
    }
}

/// 获取当前时间（毫秒）
fn get_time_ms() -> f64 {
    // 尝试使用 performance.now()
    let global = get_global();
    if let Ok(perf) = js_sys::Reflect::get(&global, &JsValue::from_str("performance")) {
        if !perf.is_undefined() && !perf.is_null() {
            if let Ok(now_fn) = js_sys::Reflect::get(&perf, &JsValue::from_str("now")) {
                if now_fn.is_function() {
                    if let Some(func) = now_fn.dyn_ref::<js_sys::Function>() {
                        if let Ok(result) = func.call0(&perf) {
                            if let Some(time) = result.as_f64() {
                                return time;
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 也尝试从 GameGlobal.performance 获取
    if let Ok(game_global) = js_sys::Reflect::get(&global, &JsValue::from_str("GameGlobal")) {
        if !game_global.is_undefined() && !game_global.is_null() {
            if let Ok(perf) = js_sys::Reflect::get(&game_global, &JsValue::from_str("performance")) {
                if !perf.is_undefined() && !perf.is_null() {
                    if let Ok(now_fn) = js_sys::Reflect::get(&perf, &JsValue::from_str("now")) {
                        if now_fn.is_function() {
                            if let Some(func) = now_fn.dyn_ref::<js_sys::Function>() {
                                if let Ok(result) = func.call0(&perf) {
                                    if let Some(time) = result.as_f64() {
                                        return time;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 回退到 Date.now()
    js_sys::Date::now()
}

// ============================================================================
// 持久化后端 - 使用微信存储 API
// ============================================================================

pub struct WxStorage;

impl WxStorage {
    pub fn new() -> Self {
        Self
    }
}

impl PersistBackend for WxStorage {
    fn load(&self, key: &str) -> Option<Vec<u8>> {
        log_info(&format!("[WxStorage] Attempting to load key: '{}'", key));
        
        let global = get_global();
        let wx = match js_sys::Reflect::get(&global, &JsValue::from_str("wx")) {
            Ok(wx) => {
                log_info("[WxStorage] Successfully got 'wx' global object");
                if wx.is_undefined() || wx.is_null() {
                    log_warn("[WxStorage] 'wx' object is undefined or null");
                    return None;
                }
                wx
            }
            Err(e) => {
                log_error(&format!("[WxStorage] Failed to get 'wx' global object: {}", js_value_to_string(&e)));
                return None;
            }
        };
        
        let get_storage_fn = match js_sys::Reflect::get(&wx, &JsValue::from_str("getStorageSync")) {
            Ok(func) => {
                log_info("[WxStorage] Successfully got 'getStorageSync' function");
                if func.is_undefined() || func.is_null() {
                    log_warn("[WxStorage] 'getStorageSync' function is undefined or null");
                    return None;
                }
                func
            }
            Err(e) => {
                log_error(&format!("[WxStorage] Failed to get 'getStorageSync' function: {}", js_value_to_string(&e)));
                return None;
            }
        };
        
        let get_storage = match get_storage_fn.dyn_into::<js_sys::Function>() {
            Ok(f) => {
                log_info("[WxStorage] Successfully cast 'getStorageSync' to Function");
                f
            }
            Err(_) => {
                log_error("[WxStorage] Failed to cast 'getStorageSync' to Function");
                return None;
            }
        };
        
        let result = match get_storage.call1(&wx, &JsValue::from_str(key)) {
            Ok(r) => {
                log_info(&format!("[WxStorage] Successfully called 'getStorageSync' for key '{}'", key));
                r
            }
            Err(e) => {
                log_error(&format!("[WxStorage] Failed to call 'getStorageSync' for key '{}': {}", key, js_value_to_string(&e)));
                return None;
            }
        };
        
        if result.is_undefined() {
            log_info(&format!("[WxStorage] Key '{}' not found (returned undefined)", key));
            return None;
        }
        
        if result.is_null() {
            log_info(&format!("[WxStorage] Key '{}' not found (returned null)", key));
            return None;
        }
        
        let base64_string = match result.as_string() {
            Some(s) => {
                if s.is_empty() {
                    log_warn(&format!("[WxStorage] Key '{}' returned empty string", key));
                    return None;
                }
                log_info(&format!("[WxStorage] Key '{}' returned string value, length: {}", key, s.len()));
                s
            }
            None => {
                log_error(&format!("[WxStorage] Key '{}' returned non-string value (type: {:?})", key, result.js_typeof()));
                log_error(&format!("[WxStorage] Value debug: {:?}", result));
                return None;
            }
        };
        
        log_info(&format!("[WxStorage] Attempting Base64 decode for key '{}' (base64_len: {})", key, base64_string.len()));
        match general_purpose::STANDARD.decode(&base64_string) {
            Ok(decoded) => {
                log_info(&format!("[WxStorage] Successfully loaded and decoded key '{}', original_data_len: {}", key, decoded.len()));
                Some(decoded)
            }
            Err(e) => {
                log_error(&format!("[WxStorage] Base64 decode FAILED for key '{}': {}", key, e));
                log_error(&format!("[WxStorage] Base64 string (first 100 chars): '{}'", &base64_string.chars().take(100).collect::<String>()));
                None
            }
        }
    }

    fn save(&mut self, key: &str, data: &[u8]) -> Result<(), String> {
        log_info(&format!("[WxStorage] Attempting to save key: '{}', data_len: {}", key, data.len()));
        
        let global = get_global();
        let wx = match js_sys::Reflect::get(&global, &JsValue::from_str("wx")) {
            Ok(wx) => {
                log_info("[WxStorage] Successfully got 'wx' global object");
                if wx.is_undefined() || wx.is_null() {
                    log_warn("[WxStorage] 'wx' object is undefined or null");
                    return Err("wx object is not available".to_string());
                }
                wx
            }
            Err(e) => {
                log_error(&format!("[WxStorage] Failed to get 'wx' global object: {}", js_value_to_string(&e)));
                return Err("Failed to get wx global object".to_string());
            }
        };
        
        let set_storage_fn = match js_sys::Reflect::get(&wx, &JsValue::from_str("setStorageSync")) {
            Ok(func) => {
                log_info("[WxStorage] Successfully got 'setStorageSync' function");
                if func.is_undefined() || func.is_null() {
                    log_warn("[WxStorage] 'setStorageSync' function is undefined or null");
                    return Err("setStorageSync function is not available".to_string());
                }
                func
            }
            Err(e) => {
                log_error(&format!("[WxStorage] Failed to get 'setStorageSync' function: {}", js_value_to_string(&e)));
                return Err("Failed to get setStorageSync function".to_string());
            }
        };
        
        let set_storage = match set_storage_fn.dyn_into::<js_sys::Function>() {
            Ok(f) => {
                log_info("[WxStorage] Successfully cast 'setStorageSync' to Function");
                f
            }
            Err(_) => {
                log_error("[WxStorage] Failed to cast 'setStorageSync' to Function");
                return Err("Failed to cast setStorageSync to Function".to_string());
            }
        };
        
        log_info(&format!("[WxStorage] Encoding data to Base64, original length: {}", data.len()));
        let base64_string = general_purpose::STANDARD.encode(data);
        log_info(&format!("[WxStorage] Base64 encoding complete, encoded length: {}", base64_string.len()));
        
        match set_storage.call2(&wx, &JsValue::from_str(key), &JsValue::from_str(&base64_string)) {
            Ok(_) => {
                log_info(&format!("[WxStorage] SUCCESSFULLY saved key '{}', original_len: {}, base64_len: {}", key, data.len(), base64_string.len()));
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("[WxStorage] FAILED to save key '{}': {}", key, js_value_to_string(&e));
                log_error(&err_msg);
                log_error(&format!("[WxStorage] Failed data info - key: '{}', original_len: {}, base64_len: {}", key, data.len(), base64_string.len()));
                Err(err_msg)
            }
        }
    }

    fn remove(&mut self, key: &str) -> Result<(), String> {
        log_info(&format!("[WxStorage] Attempting to remove key: '{}'", key));
        
        let global = get_global();
        let wx = match js_sys::Reflect::get(&global, &JsValue::from_str("wx")) {
            Ok(wx) => {
                log_info("[WxStorage] Successfully got 'wx' global object");
                if wx.is_undefined() || wx.is_null() {
                    log_warn("[WxStorage] 'wx' object is undefined or null");
                    return Err("wx object is not available".to_string());
                }
                wx
            }
            Err(e) => {
                log_error(&format!("[WxStorage] Failed to get 'wx' global object: {}", js_value_to_string(&e)));
                return Err("Failed to get wx global object".to_string());
            }
        };
        
        let remove_storage_fn = match js_sys::Reflect::get(&wx, &JsValue::from_str("removeStorageSync")) {
            Ok(func) => {
                log_info("[WxStorage] Successfully got 'removeStorageSync' function");
                if func.is_undefined() || func.is_null() {
                    log_warn("[WxStorage] 'removeStorageSync' function is undefined or null");
                    return Err("removeStorageSync function is not available".to_string());
                }
                func
            }
            Err(e) => {
                log_error(&format!("[WxStorage] Failed to get 'removeStorageSync' function: {}", js_value_to_string(&e)));
                return Err("Failed to get removeStorageSync function".to_string());
            }
        };
        
        let remove_storage = match remove_storage_fn.dyn_into::<js_sys::Function>() {
            Ok(f) => {
                log_info("[WxStorage] Successfully cast 'removeStorageSync' to Function");
                f
            }
            Err(_) => {
                log_error("[WxStorage] Failed to cast 'removeStorageSync' to Function");
                return Err("Failed to cast removeStorageSync to Function".to_string());
            }
        };
        
        match remove_storage.call1(&wx, &JsValue::from_str(key)) {
            Ok(_) => {
                log_info(&format!("[WxStorage] SUCCESSFULLY removed key '{}'", key));
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("[WxStorage] FAILED to remove key '{}': {}", key, js_value_to_string(&e));
                log_warn(&err_msg);
                log_warn(&format!("[WxStorage] Continue execution despite removal failure for key '{}'", key));
                // Note: According to original code, we return Ok even on error
                Ok(())
            }
        }
    }

    fn exists(&self, key: &str) -> bool {
        log_info(&format!("[WxStorage] Checking existence of key: '{}'", key));
        let exists = self.load(key).is_some();
        log_info(&format!("[WxStorage] Key '{}' exists: {}", key, exists));
        exists
    }
}

// ============================================================================
// 显示后端 - CPU 渲染 + Canvas 2D 显示
// ============================================================================

pub struct WxDisplay {
    canvas: JsValue,
    ctx: JsValue,
    width: u32,
    height: u32,
    // 离屏 Canvas 用于缩放渲染
    offscreen_canvas: Option<JsValue>,
    offscreen_ctx: Option<JsValue>,
    // 缓存 Canvas 2D 函数，避免每帧反射
    fn_create_image_data: Option<js_sys::Function>,
    fn_put_image_data: Option<js_sys::Function>,
    fn_draw_image: Option<js_sys::Function>,
    // 缩放参数（初始化时计算一次）
    dst_x: f64,
    dst_y: f64,
    dst_w: f64,
    dst_h: f64,
    // 调试日志计时器
    last_debug_log_time: f64,
    // 像素完美缩放参数（Rust 端手动缩放）
    /// 缩放倍数（整数倍，如 2 或 3）
    scale_factor: u32,
    /// 缩放后的游戏画面宽度
    scaled_game_width: u32,
    /// 缩放后的游戏画面高度
    scaled_game_height: u32,
    /// 缩放后的居中 X 偏移
    scaled_dst_x: f64,
    /// 缩放后的居中 Y 偏移
    scaled_dst_y: f64,
    // 性能优化：缓存缩放后的缓冲区，避免每帧重新分配
    /// 缓存的缩放后缓冲区（复用内存）
    scaled_buffer: Vec<u8>,
    /// 缓存的单行缩放缓冲区（复用内存）
    scaled_row_buffer: Vec<u8>,
    // 性能优化 v3：缓存 JS 对象，避免每帧创建
    /// 缓存的 Uint8ClampedArray（复用 JS 对象）
    cached_typed_array: Option<Uint8ClampedArray>,
    /// 缓存的 ImageData（复用 JS 对象）
    cached_image_data: Option<JsValue>,
}

impl WxDisplay {
    pub fn new() -> Result<Self, String> {
        let canvas = get_wx_canvas()
            .ok_or("无法获取微信小游戏 canvas")?;
        
        let (width, height) = get_canvas_size(&canvas);
        log_info(&format!("Canvas 尺寸: {}x{}", width, height));
        
        // 获取 2D 上下文
        let ctx = get_2d_context(&canvas)?;
        
        // 设置图像平滑为 false (像素风格)
        set_image_smoothing(&ctx, false);
        
        // 设置 Canvas 的 CSS image-rendering 样式为像素风格
        // 这对于移动端特别重要
        set_canvas_pixel_style(&canvas);
        
        // 计算缩放参数（保持宽高比）
        let game_aspect = GAME_WIDTH_U32 as f64 / GAME_HEIGHT_U32 as f64;
        let screen_aspect = width as f64 / height as f64;
        
        let (dst_w, dst_h, dst_x, dst_y) = if screen_aspect > game_aspect {
            let h = height as f64;
            let w = h * game_aspect;
            let x = (width as f64 - w) / 2.0;
            (w, h, x, 0.0)
        } else {
            let w = width as f64;
            let h = w / game_aspect;
            let y = (height as f64 - h) / 2.0;
            (w, h, 0.0, y)
        };
        
        // 计算像素完美缩放参数（整数倍缩放）
        // 注意：putImageData 使用物理像素坐标，所以直接基于物理像素计算缩放倍数
        // 这样可以充分利用高 DPI 屏幕的分辨率
        let scale_x = (width as f64 / GAME_WIDTH_U32 as f64).floor() as u32;
        let scale_y = (height as f64 / GAME_HEIGHT_U32 as f64).floor() as u32;
        let scale_factor = scale_x.min(scale_y).max(1); // 使用最大可能的缩放倍数
        
        let scaled_game_width = GAME_WIDTH_U32 * scale_factor;
        let scaled_game_height = GAME_HEIGHT_U32 * scale_factor;
        
        // 计算缩放后的居中位置（物理像素坐标）
        let scaled_dst_x = ((width as f64 - scaled_game_width as f64) / 2.0).max(0.0);
        let scaled_dst_y = ((height as f64 - scaled_game_height as f64) / 2.0).max(0.0);
        
        // 获取 DPR 用于日志显示
        let dpr = get_device_pixel_ratio();
        let css_width = width as f64 / dpr;
        let css_height = height as f64 / dpr;
        
        log_info(&format!(
            "像素完美缩放: {}x 倍数 (物理像素: {}x{}, CSS: {:.0}x{:.0}, DPR: {:.1})",
            scale_factor, width, height, css_width, css_height, dpr
        ));
        log_info(&format!(
            "缩放后尺寸: {}x{}, 居中位置: ({:.0}, {:.0})",
            scaled_game_width, scaled_game_height, scaled_dst_x, scaled_dst_y
        ));
        
        // 预分配缩放缓冲区（避免每帧重新分配内存）
        let scaled_buffer_size = (scaled_game_width * scaled_game_height * 4) as usize;
        let scaled_row_buffer_size = (scaled_game_width * 4) as usize;
        
        log_info(&format!(
            "预分配缩放缓冲区: scaled_buffer={}KB, scaled_row={}KB",
            scaled_buffer_size / 1024,
            scaled_row_buffer_size / 1024
        ));
        
        // 预创建 Uint8ClampedArray 和 ImageData（避免每帧创建 JS 对象）
        let cached_typed_array = Uint8ClampedArray::new_with_length(scaled_buffer_size as u32);
        
        let cached_image_data = if let Ok(create_fn) = js_sys::Reflect::get(&ctx, &JsValue::from_str("createImageData")) {
            if let Some(func) = create_fn.dyn_ref::<js_sys::Function>() {
                func.call2(&ctx, 
                    &JsValue::from(scaled_game_width), 
                    &JsValue::from(scaled_game_height))
                    .ok()
            } else {
                None
            }
        } else {
            None
        };
        
        if cached_image_data.is_some() {
            log_info("已缓存 ImageData 对象（避免每帧创建）");
        }
        
        // 检测并记录 WASM SIMD 支持状态
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        {
            log_info("✅ WASM SIMD 已启用 (v128 指令集)");
            log_info(&format!("   - 水平缩放: 每次处理 {} 个像素批量写入", 4));
            log_info(&format!("   - 垂直复制: 每次复制 {} 字节", 16));
        }
        
        #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
        {
            log_info("⚠️  WASM SIMD 未启用，使用标准优化版本");
        }
        
        // 初始化虚拟控制器叠加层
        // 必须在这里初始化，因为现在不再使用 init_offscreen()
        init_virtual_controller_overlay();
        
        Ok(Self {
            canvas,
            ctx,
            width,
            height,
            offscreen_canvas: None,
            offscreen_ctx: None,
            fn_create_image_data: None,
            fn_put_image_data: None,
            fn_draw_image: None,
            dst_x,
            dst_y,
            dst_w,
            dst_h,
            last_debug_log_time: 0.0,
            scale_factor,
            scaled_game_width,
            scaled_game_height,
            scaled_dst_x,
            scaled_dst_y,
            scaled_buffer: vec![0u8; scaled_buffer_size],
            scaled_row_buffer: vec![0u8; scaled_row_buffer_size],
            cached_typed_array: Some(cached_typed_array),
            cached_image_data,
        })
    }
    
    /// 初始化离屏 Canvas（延迟初始化）
    fn init_offscreen(&mut self) -> Result<(), String> {
        if self.offscreen_canvas.is_some() {
            return Ok(());
        }
        
        // 创建离屏 Canvas
        let global = get_global();
        let wx = js_sys::Reflect::get(&global, &JsValue::from_str("wx"))
            .map_err(|_| "wx not found")?;
        
        if wx.is_undefined() || wx.is_null() {
            return Err("wx is undefined".to_string());
        }
        
        let create_canvas = js_sys::Reflect::get(&wx, &JsValue::from_str("createCanvas"))
            .map_err(|_| "wx.createCanvas not found")?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| "wx.createCanvas is not a function")?;
        
        let offscreen = create_canvas.call0(&wx)
            .map_err(|_| "wx.createCanvas failed")?;
        
        // 设置离屏 Canvas 尺寸为游戏分辨率
        let _ = js_sys::Reflect::set(&offscreen, &JsValue::from_str("width"), &JsValue::from(GAME_WIDTH_U32));
        let _ = js_sys::Reflect::set(&offscreen, &JsValue::from_str("height"), &JsValue::from(GAME_HEIGHT_U32));
        
        // 获取离屏 Canvas 的 2D 上下文
        let offscreen_ctx = get_2d_context(&offscreen)?;
        
        // 设置离屏 Canvas 的图像平滑为 false（像素风格）
        set_image_smoothing(&offscreen_ctx, false);
        
        // 设置离屏 Canvas 的 CSS 像素风格
        set_canvas_pixel_style(&offscreen);
        
        // 缓存离屏 Canvas 的 createImageData 函数
        let fn_create = js_sys::Reflect::get(&offscreen_ctx, &JsValue::from_str("createImageData"))
            .ok()
            .and_then(|v| v.dyn_into::<js_sys::Function>().ok());
        
        // 缓存离屏 Canvas 的 putImageData 函数
        let fn_put = js_sys::Reflect::get(&offscreen_ctx, &JsValue::from_str("putImageData"))
            .ok()
            .and_then(|v| v.dyn_into::<js_sys::Function>().ok());
        
        // 缓存主 Canvas 的 drawImage 函数
        let fn_draw = js_sys::Reflect::get(&self.ctx, &JsValue::from_str("drawImage"))
            .ok()
            .and_then(|v| v.dyn_into::<js_sys::Function>().ok());
        
        self.offscreen_canvas = Some(offscreen);
        self.offscreen_ctx = Some(offscreen_ctx);
        self.fn_create_image_data = fn_create;
        self.fn_put_image_data = fn_put;
        self.fn_draw_image = fn_draw;
        
        log_info(&format!("[CPU] 离屏Canvas初始化完成, 游戏分辨率: {}x{}, 目标区域: ({:.0}, {:.0}, {:.0}, {:.0})", 
            GAME_WIDTH_U32, GAME_HEIGHT_U32, self.dst_x, self.dst_y, self.dst_w, self.dst_h));


        // 初始化虚拟控制器叠加层
        init_virtual_controller_overlay();
        Ok(())
    }
    
    /// 使用最近邻算法进行像素完美缩放（优化版本 v5 - WASM SIMD）
    /// 将原始 framebuffer 缩放到指定倍数
    /// 
    /// 性能优化策略：
    /// 1. 先水平缩放每一行（每个像素复制scale次）
    /// 2. 然后垂直复制整行（将缩放后的行复制scale次）
    /// 3. **v2**: 复用预分配的缓冲区，避免每帧重新分配内存
    /// 4. **v4**: 使用 unsafe ptr::copy_nonoverlapping 批量复制，强制内联
    /// 5. **v5**: 使用 WASM SIMD 指令并行处理 4 个像素（v128）
    #[inline(always)]
    fn scale_framebuffer_nearest_neighbor(&mut self, framebuffer: &[u8]) {
        // WASM SIMD 优化版本（如果可用）
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        {
            self.scale_framebuffer_simd(framebuffer);
            return;
        }
        
        // 回退到标准版本（非 SIMD）
        #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
        {
            self.scale_framebuffer_standard(framebuffer);
        }
    }
    
    /// 标准缩放算法（v4 优化版本）
    #[inline(always)]
    fn scale_framebuffer_standard(&mut self, framebuffer: &[u8]) {
        let scale = self.scale_factor as usize;
        let src_w = GAME_WIDTH_U32 as usize;
        let src_h = GAME_HEIGHT_U32 as usize;
        let dst_w = self.scaled_game_width as usize;
        
        // 复用预分配的缓冲区（性能关键：避免每帧分配内存）
        let scaled = &mut self.scaled_buffer;
        let scaled_row = &mut self.scaled_row_buffer;
        
        // 对每一行进行处理
        for src_y in 0..src_h {
            let src_row_start = src_y * src_w * 4;
            
            // 步骤1: 水平缩放这一行（使用 unsafe 批量复制优化）
            // 每个源像素水平复制scale次
            for src_x in 0..src_w {
                let src_idx = src_row_start + src_x * 4;
                
                // 读取源像素的 RGBA 值（手动展开优化）
                let r = framebuffer[src_idx];
                let g = framebuffer[src_idx + 1];
                let b = framebuffer[src_idx + 2];
                let a = framebuffer[src_idx + 3];
                
                // 将这个像素水平复制scale次（使用 unsafe 批量复制）
                let dst_x_start = src_x * scale * 4;
                unsafe {
                    for dx in 0..scale {
                        let dst_idx = dst_x_start + dx * 4;
                        let dst_ptr = scaled_row.as_mut_ptr().add(dst_idx);
                        // 批量复制4字节RGBA（避免边界检查）
                        *dst_ptr = r;
                        *dst_ptr.add(1) = g;
                        *dst_ptr.add(2) = b;
                        *dst_ptr.add(3) = a;
                    }
                }
            }
            
            // 步骤2: 垂直复制这一行（复制scale次，使用 unsafe 指针操作）
            let dst_y_start = src_y * scale;
            let row_bytes = dst_w * 4;
            unsafe {
                let src_ptr = scaled_row.as_ptr();
                for dy in 0..scale {
                    let dst_row_start = (dst_y_start + dy) * row_bytes;
                    let dst_ptr = scaled.as_mut_ptr().add(dst_row_start);
                    // 使用最快的内存复制（无边界检查）
                    std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, row_bytes);
                }
            }
        }
    }
    
    /// WASM SIMD 优化版本（真正的并行处理）
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    #[inline(always)]
    fn scale_framebuffer_simd(&mut self, framebuffer: &[u8]) {
        use std::arch::wasm32::*;
        
        let scale = self.scale_factor as usize;
        let src_w = GAME_WIDTH_U32 as usize;
        let src_h = GAME_HEIGHT_U32 as usize;
        let dst_w = self.scaled_game_width as usize;
        
        let scaled = &mut self.scaled_buffer;
        let scaled_row = &mut self.scaled_row_buffer;
        
        // 对每一行进行处理
        for src_y in 0..src_h {
            let src_row_start = src_y * src_w * 4;
            
            // 步骤1: 水平缩放（SIMD 优化：批量复制）
            let row_bytes = dst_w * 4;
            
            unsafe {
                // 使用 SIMD 批量复制整行
                let chunks_16 = row_bytes / 16;
                let remainder = row_bytes % 16;
                
                for i in 0..chunks_16 {
                    let dst_idx = i * 16;
                    // 简单高效的方式：先用标量填充，再用 SIMD 复制
                    // 这比复杂的 shuffle 快
                    if dst_idx < scaled_row.len() {
                        // 对于 6x 缩放，每个源像素占 24 字节（6*4）
                        // 16 字节包含约 1.33 个源像素，太复杂
                        // 回退到标量方式填充行缓冲
                        break;
                    }
                }
                
                // 标量方式水平缩放（更简单高效）
                for src_x in 0..src_w {
                    let src_idx = src_row_start + src_x * 4;
                    let r = framebuffer[src_idx];
                    let g = framebuffer[src_idx + 1];
                    let b = framebuffer[src_idx + 2];
                    let a = framebuffer[src_idx + 3];
                    
                    let dst_start = src_x * scale * 4;
                    for dx in 0..scale {
                        let dst_idx = dst_start + dx * 4;
                        *scaled_row.get_unchecked_mut(dst_idx) = r;
                        *scaled_row.get_unchecked_mut(dst_idx + 1) = g;
                        *scaled_row.get_unchecked_mut(dst_idx + 2) = b;
                        *scaled_row.get_unchecked_mut(dst_idx + 3) = a;
                    }
                }
            }
            
            // 步骤2: 垂直复制（SIMD 批量复制 - 这是关键优化！）
            let dst_y_start = src_y * scale;
            let row_bytes = dst_w * 4;
            
            unsafe {
                let src_ptr = scaled_row.as_ptr();
                
                // 对每一行使用 SIMD 批量复制
                for dy in 0..scale {
                    let dst_row_start = (dst_y_start + dy) * row_bytes;
                    let dst_ptr = scaled.as_mut_ptr().add(dst_row_start);
                    
                    // 使用 v128 批量复制（每次 16 字节）
                    let chunks_16 = row_bytes / 16;
                    for i in 0..chunks_16 {
                        let vec = v128_load(src_ptr.add(i * 16) as *const v128);
                        v128_store(dst_ptr.add(i * 16) as *mut v128, vec);
                    }
                    
                    // 处理剩余字节
                    let remainder = row_bytes % 16;
                    if remainder > 0 {
                        let remainder_start = chunks_16 * 16;
                        std::ptr::copy_nonoverlapping(
                            src_ptr.add(remainder_start),
                            dst_ptr.add(remainder_start),
                            remainder
                        );
                    }
                }
            }
        }
    }
    
    /// 将帧缓冲绘制到 Canvas（像素完美版本 v4）
    /// 流程: framebuffer -> Rust端像素完美缩放 -> ImageData -> 主Canvas (1:1 putImageData)
    /// v3 优化：复用缓存的 Uint8ClampedArray 和 ImageData，避免每帧创建 JS 对象
    /// v4 优化：内联渲染函数，减少函数调用开销
    #[inline(always)]
    pub fn render_framebuffer(&mut self, framebuffer: &[u8]) -> Result<(), String> {
        // 确保帧缓冲大小正确
        let expected_size = (GAME_WIDTH_U32 * GAME_HEIGHT_U32 * 4) as usize;
        if framebuffer.len() != expected_size {
            return Err(format!("帧缓冲大小不匹配: expected {}, got {}", 
                expected_size, framebuffer.len()));
        }
        
        // 步骤0: 清除整个 Canvas，避免虚拟控制器拖影
        // putImageData 不会清除背景，所以需要先清除
        if let Ok(clear_fn) = js_sys::Reflect::get(&self.ctx, &JsValue::from_str("clearRect")) {
            if let Some(func) = clear_fn.dyn_ref::<js_sys::Function>() {
                let _ = func.call4(&self.ctx,
                    &JsValue::from(0),
                    &JsValue::from(0),
                    &JsValue::from(self.width),
                    &JsValue::from(self.height));
            }
        }
        
        // 步骤1: 在 Rust 中使用最近邻算法缩放 framebuffer
        // 注意：现在是就地修改缓存的缓冲区，不返回新Vec
        self.scale_framebuffer_nearest_neighbor(framebuffer);
        
        // 步骤2: 复用缓存的 TypedArray，直接更新数据
        let typed_array = self.cached_typed_array.as_ref()
            .ok_or("cached_typed_array is None")?;
        typed_array.copy_from(&self.scaled_buffer);
        
        // 步骤3: 获取或创建 ImageData
        let img_data = if let Some(cached) = &self.cached_image_data {
            // 复用缓存的 ImageData
            cached.clone()
        } else {
            // 如果缓存失败，回退到每帧创建（兼容性）
            let create_fn = js_sys::Reflect::get(&self.ctx, &JsValue::from_str("createImageData"))
                .ok()
                .and_then(|v| v.dyn_into::<js_sys::Function>().ok())
                .ok_or("createImageData not found")?;
            
            create_fn.call2(&self.ctx, 
                &JsValue::from(self.scaled_game_width), 
                &JsValue::from(self.scaled_game_height))
                .map_err(|e| format!("createImageData failed: {}", js_value_to_string(&e)))?
        };
        
        // 步骤4: 获取 ImageData.data 并更新像素数据
        let data_val = js_sys::Reflect::get(&img_data, &JsValue::from_str("data"))
            .map_err(|_| "无法获取 ImageData.data")?;
        
        // 使用 TypedArray.set() 方法批量复制（最快的方式）
        if let Ok(set_fn) = js_sys::Reflect::get(&data_val, &JsValue::from_str("set")) {
            if let Some(func) = set_fn.dyn_ref::<js_sys::Function>() {
                let _ = func.call1(&data_val, typed_array);
            }
        }
        
        // 步骤5: 使用 putImageData 将缩放后的图像直接绘制到主 Canvas
        let put_fn = js_sys::Reflect::get(&self.ctx, &JsValue::from_str("putImageData"))
            .ok()
            .and_then(|v| v.dyn_into::<js_sys::Function>().ok())
            .ok_or("putImageData not found")?;
        
        put_fn.call3(&self.ctx, &img_data, 
            &JsValue::from(self.scaled_dst_x as i32), 
            &JsValue::from(self.scaled_dst_y as i32))
            .map_err(|e| format!("putImageData failed: {}", js_value_to_string(&e)))?;
        
        Ok(())
    }
}

impl DisplayBackend for WxDisplay {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn present(&mut self) -> Result<(), String> {
        // CPU 渲染版本不需要 present，渲染在 render_framebuffer 中完成
        Ok(())
    }

    fn request_redraw(&self) {
        // 微信小游戏使用 requestAnimationFrame，不需要显式请求重绘
    }
}

// ============================================================================
// 输入后端 - 触摸控制
// ============================================================================

pub struct WxInput {
    key_states: HashMap<PlatformKeyCode, bool>,
    event_queue: Vec<PlatformKeyEvent>,
    should_close: bool,
}

impl WxInput {
    pub fn new() -> Self {
        Self {
            key_states: HashMap::new(),
            event_queue: Vec::new(),
            should_close: false,
        }
    }
    
    pub fn handle_button_event(&mut self, button_id: i32, pressed: bool) {
        let key = match button_id {
            1 => PlatformKeyCode::Left,
            2 => PlatformKeyCode::Right,
            3 => PlatformKeyCode::Up,
            4 => PlatformKeyCode::Down,
            5 => PlatformKeyCode::AltLeft,      // A = Jump (跳跃)
            6 => PlatformKeyCode::Space,        // B = Fire (发射)
            7 => PlatformKeyCode::ControlLeft,  // X = Run (奔跑/加速)
            8 => PlatformKeyCode::ShiftLeft,    // Y = Special (特殊)
            _ => return,
        };
        
        self.key_states.insert(key, pressed);
        self.event_queue.push(PlatformKeyEvent { key, pressed });
    }
    
    /// 将微信小游戏键盘 code 字符串映射到 PlatformKeyCode
    fn wx_code_to_keycode(code: &str) -> Option<PlatformKeyCode> {
        match code {
            // 方向键
            "ArrowLeft" => Some(PlatformKeyCode::Left),
            "ArrowRight" => Some(PlatformKeyCode::Right),
            "ArrowUp" => Some(PlatformKeyCode::Up),
            "ArrowDown" => Some(PlatformKeyCode::Down),
            
            // WASD - 独立的字母键
            "KeyA" => Some(PlatformKeyCode::KeyA),
            "KeyW" => Some(PlatformKeyCode::KeyW),
            "KeyD" => Some(PlatformKeyCode::KeyD),
            "KeyS" => Some(PlatformKeyCode::KeyS),
            
            // 作弊码需要的字母键 (B、C、E、F)
            "KeyB" => Some(PlatformKeyCode::KeyB),
            "KeyC" => Some(PlatformKeyCode::KeyC),
            "KeyE" => Some(PlatformKeyCode::KeyE),
            "KeyF" => Some(PlatformKeyCode::KeyF),
            
            // 动作键 - 使用独立的按键
            "KeyZ" => Some(PlatformKeyCode::KeyZ),
            "KeyX" => Some(PlatformKeyCode::KeyX),
            "Space" => Some(PlatformKeyCode::Space),
            "ControlLeft" | "ControlRight" => Some(PlatformKeyCode::ControlLeft),
            "ShiftLeft" | "ShiftRight" => Some(PlatformKeyCode::ShiftLeft),
            "AltLeft" | "AltRight" => Some(PlatformKeyCode::AltLeft),
            
            // 功能键
            "Enter" => Some(PlatformKeyCode::Enter),
            "Escape" => Some(PlatformKeyCode::Escape),
            "KeyP" => Some(PlatformKeyCode::KeyP),
            "Tab" => Some(PlatformKeyCode::Tab),
            
            // 数字键 0-9
            "Digit0" => Some(PlatformKeyCode::Digit0),
            "Digit1" => Some(PlatformKeyCode::Digit1),
            "Digit2" => Some(PlatformKeyCode::Digit2),
            "Digit3" => Some(PlatformKeyCode::Digit3),
            "Digit4" => Some(PlatformKeyCode::Digit4),
            "Digit5" => Some(PlatformKeyCode::Digit5),
            "Digit6" => Some(PlatformKeyCode::Digit6),
            "Digit7" => Some(PlatformKeyCode::Digit7),
            "Digit8" => Some(PlatformKeyCode::Digit8),
            "Digit9" => Some(PlatformKeyCode::Digit9),
            
            _ => None, // 忽略未映射的按键
        }
    }
    
    /// 处理键盘事件 (微信小游戏 PC 端 wx.onKeyDown/wx.onKeyUp)
    pub fn handle_key_event(&mut self, code: &str, pressed: bool) {
        let key = match Self::wx_code_to_keycode(code) {
            Some(k) => k,
            None => {
                log_warn(&format!("未映射的键盘按键: {}", code));
                return;
            }
        };
        
        self.key_states.insert(key, pressed);
        self.event_queue.push(PlatformKeyEvent { key, pressed });
    }
}

impl InputBackend for WxInput {
    fn poll_events(&mut self) -> Vec<crate::platform::KeyEvent> {
        std::mem::take(&mut self.event_queue)
    }

    fn is_key_pressed(&self, key: crate::platform::KeyCode) -> bool {
        *self.key_states.get(&key).unwrap_or(&false)
    }

    fn should_close(&self) -> bool {
        self.should_close
    }

    fn request_close(&mut self) {
        self.should_close = true;
    }
}

// ============================================================================
// 音频后端 - 微信 WebAudioContext API
// ============================================================================

pub struct WxAudio {
    audio_context: Option<JsValue>,
    gain_node: Option<JsValue>,
    current_oscillator: Option<JsValue>,
    volume: f32,
    muted: bool,
}

impl WxAudio {
    pub fn new() -> Self {
        let mut audio = Self {
            audio_context: None,
            gain_node: None,
            current_oscillator: None,
            volume: 0.5,
            muted: false,
        };
        audio.init_audio_context();
        audio
    }
    
    /// 初始化 WebAudioContext
    fn init_audio_context(&mut self) {
        let global = get_global();
        
        // 调用 wx.createWebAudioContext()
        if let Ok(wx) = js_sys::Reflect::get(&global, &JsValue::from_str("wx")) {
            if !wx.is_undefined() && !wx.is_null() {
                if let Ok(create_fn) = js_sys::Reflect::get(&wx, &JsValue::from_str("createWebAudioContext")) {
                    if let Some(func) = create_fn.dyn_ref::<js_sys::Function>() {
                        if let Ok(ctx) = func.call0(&wx) {
                            if !ctx.is_undefined() && !ctx.is_null() {
                                log_info("WebAudioContext 创建成功");
                                
                                // 创建 GainNode 用于音量控制
                                if let Some(gain) = self.create_gain_node(&ctx) {
                                    // 连接到 destination
                                    if let Ok(dest) = js_sys::Reflect::get(&ctx, &JsValue::from_str("destination")) {
                                        self.connect_nodes(&gain, &dest);
                                        self.gain_node = Some(gain);
                                        self.update_gain_value();
                                    }
                                }
                                
                                self.audio_context = Some(ctx);
                                return;
                            }
                        }
                    }
                }
            }
        }
        
        log_warn("WebAudioContext 创建失败，音频将不可用");
    }
    
    /// 创建 GainNode
    fn create_gain_node(&self, ctx: &JsValue) -> Option<JsValue> {
        if let Ok(create_fn) = js_sys::Reflect::get(ctx, &JsValue::from_str("createGain")) {
            if let Some(func) = create_fn.dyn_ref::<js_sys::Function>() {
                if let Ok(gain) = func.call0(ctx) {
                    if !gain.is_undefined() && !gain.is_null() {
                        return Some(gain);
                    }
                }
            }
        }
        None
    }
    
    /// 连接两个音频节点
    fn connect_nodes(&self, source: &JsValue, dest: &JsValue) {
        if let Ok(connect_fn) = js_sys::Reflect::get(source, &JsValue::from_str("connect")) {
            if let Some(func) = connect_fn.dyn_ref::<js_sys::Function>() {
                let _ = func.call1(source, dest);
            }
        }
    }
    
    /// 更新 GainNode 的值
    fn update_gain_value(&self) {
        if let Some(gain) = &self.gain_node {
            if let Ok(gain_param) = js_sys::Reflect::get(gain, &JsValue::from_str("gain")) {
                let value = if self.muted { 0.0 } else { self.volume };
                let _ = js_sys::Reflect::set(&gain_param, &JsValue::from_str("value"), &JsValue::from_f64(value as f64));
            }
        }
    }
    
    /// 创建并播放振荡器（手机优化版本）
    fn play_oscillator(&mut self, frequency: u32, duration_ms: u32) {
        // 借用 audio_context
        let ctx = match &self.audio_context {
            Some(c) => c.clone(),
            None => return,
        };
        
        // 4. 检查音频上下文状态，如果是suspended则自动恢复
        if let Ok(state) = js_sys::Reflect::get(&ctx, &JsValue::from_str("state")) {
            if let Some(state_str) = state.as_string() {
                if state_str == "suspended" {
                    self.resume();
                }
            }
        }
        
        // 1. 先将播放时长整体增加1.5倍（所有声音都加长），然后根据频率设置最小播放时长
        let extended_duration = ((duration_ms as f64) * 1.5) as u32;
        let actual_duration = if frequency < 200 {
            extended_duration.max(200) // 极低频至少 200ms
        } else if frequency < 300 {
            extended_duration.max(150) // 低频至少 150ms
        } else {
            extended_duration.max(100) // 普通频率至少 100ms
        };
        
        // 创建 OscillatorNode
        if let Ok(create_fn) = js_sys::Reflect::get(&ctx, &JsValue::from_str("createOscillator")) {
            if let Some(func) = create_fn.dyn_ref::<js_sys::Function>() {
                if let Ok(osc) = func.call0(&ctx) {
                    if osc.is_undefined() || osc.is_null() {
                        return;
                    }
                    
                    // 设置波形为方波
                    let _ = js_sys::Reflect::set(&osc, &JsValue::from_str("type"), &JsValue::from_str("square"));
                    
                    // 设置频率（低频自动提升1.5倍以适应手机喇叭）
                    // let adjusted_freq = if frequency < 300 {
                    //     (frequency as f64) * 2.0
                    // } else {
                    //     frequency as f64
                    // };
                    let adjusted_freq = frequency as f64;
                    
                    if let Ok(freq_param) = js_sys::Reflect::get(&osc, &JsValue::from_str("frequency")) {
                        let _ = js_sys::Reflect::set(&freq_param, &JsValue::from_str("value"), &JsValue::from_f64(adjusted_freq));
                    }
                    
                    // 2. 为每个音符创建独立的 GainNode 实现音频包络（淡入淡出）
                    if let Some(envelope_gain) = self.create_gain_node(&ctx) {
                        // 连接：Oscillator -> EnvelopeGain -> MasterGain -> Destination
                        self.connect_nodes(&osc, &envelope_gain);
                        if let Some(master_gain) = &self.gain_node {
                            self.connect_nodes(&envelope_gain, master_gain);
                        }
                        
                        // 获取当前时间
                        let current_time = js_sys::Reflect::get(&ctx, &JsValue::from_str("currentTime"))
                            .ok()
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        
                        // 设置音频包络（ADSR）
                        if let Ok(gain_param) = js_sys::Reflect::get(&envelope_gain, &JsValue::from_str("gain")) {
                            // Attack: 10ms 淡入（从 0 到 1）
                            let _ = js_sys::Reflect::set(&gain_param, &JsValue::from_str("value"), &JsValue::from_f64(0.0));
                            
                            if let Ok(ramp_fn) = js_sys::Reflect::get(&gain_param, &JsValue::from_str("linearRampToValueAtTime")) {
                                if let Some(func) = ramp_fn.dyn_ref::<js_sys::Function>() {
                                    let attack_time = current_time + 0.01; // 10ms
                                    let _ = func.call2(&gain_param, &JsValue::from_f64(1.0), &JsValue::from_f64(attack_time));
                                }
                            }
                            
                            // Release: 最后 20ms 淡出（从 1 到 0）
                            let release_start = current_time + (actual_duration as f64 / 1000.0) - 0.02;
                            let release_end = current_time + (actual_duration as f64 / 1000.0);
                            
                            if let Ok(set_value_fn) = js_sys::Reflect::get(&gain_param, &JsValue::from_str("setValueAtTime")) {
                                if let Some(func) = set_value_fn.dyn_ref::<js_sys::Function>() {
                                    let _ = func.call2(&gain_param, &JsValue::from_f64(1.0), &JsValue::from_f64(release_start));
                                }
                            }
                            
                            if let Ok(ramp_fn) = js_sys::Reflect::get(&gain_param, &JsValue::from_str("linearRampToValueAtTime")) {
                                if let Some(func) = ramp_fn.dyn_ref::<js_sys::Function>() {
                                    let _ = func.call2(&gain_param, &JsValue::from_f64(0.0), &JsValue::from_f64(release_end));
                                }
                            }
                        }
                        
                        // 启动振荡器
                        if let Ok(start_fn) = js_sys::Reflect::get(&osc, &JsValue::from_str("start")) {
                            if let Some(func) = start_fn.dyn_ref::<js_sys::Function>() {
                                let _ = func.call1(&osc, &JsValue::from_f64(current_time));
                            }
                        }
                        
                        // 设置停止时间
                        let stop_time = current_time + (actual_duration as f64 / 1000.0);
                        if let Ok(stop_fn) = js_sys::Reflect::get(&osc, &JsValue::from_str("stop")) {
                            if let Some(func) = stop_fn.dyn_ref::<js_sys::Function>() {
                                let _ = func.call1(&osc, &JsValue::from_f64(stop_time));
                            }
                        }
                        
                        // 3. 不再保存current_oscillator，让声音自然播放完毕
                        // self.current_oscillator = Some(osc);
                    }
                }
            }
        }
    }
    
    /// 停止当前振荡器
    fn stop_current_oscillator(&mut self) {
        if let Some(osc) = self.current_oscillator.take() {
            // 尝试停止振荡器
            if let Ok(stop_fn) = js_sys::Reflect::get(&osc, &JsValue::from_str("stop")) {
                if let Some(func) = stop_fn.dyn_ref::<js_sys::Function>() {
                    let _ = func.call0(&osc);
                }
            }
            // 断开连接
            if let Ok(disconnect_fn) = js_sys::Reflect::get(&osc, &JsValue::from_str("disconnect")) {
                if let Some(func) = disconnect_fn.dyn_ref::<js_sys::Function>() {
                    let _ = func.call0(&osc);
                }
            }
        }
    }
    
    /// 恢复音频上下文（用户交互后调用）
    pub fn resume(&self) {
        if let Some(ctx) = &self.audio_context {
            if let Ok(resume_fn) = js_sys::Reflect::get(ctx, &JsValue::from_str("resume")) {
                if let Some(func) = resume_fn.dyn_ref::<js_sys::Function>() {
                    let _ = func.call0(ctx);
                }
            }
        }
    }
}

impl AudioBackend for WxAudio {
    fn beep(&mut self, frequency: u32, duration_ms: u32) {
        if self.muted || frequency == 0 {
            return;
        }
        self.play_oscillator(frequency, duration_ms);
    }

    fn play_sequence(&mut self, notes: &[(u32, u32)]) {
        if self.muted || notes.is_empty() {
            return;
        }
        
        // 克隆以避免借用冲突
        let ctx = match &self.audio_context {
            Some(c) => c.clone(),
            None => return,
        };
        
        // 4. 检查音频上下文状态
        if let Ok(state) = js_sys::Reflect::get(&ctx, &JsValue::from_str("state")) {
            if let Some(state_str) = state.as_string() {
                if state_str == "suspended" {
                    self.resume();
                }
            }
        }
        
        // 获取当前时间
        let mut current_time = js_sys::Reflect::get(&ctx, &JsValue::from_str("currentTime"))
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        
        // 为每个音符创建振荡器并安排播放
        for &(mut frequency, duration_ms) in notes {
            // 1. 先将播放时长整体增加1.5倍（所有声音都加长），然后根据频率设置最小播放时长
            let actual_duration = if frequency == 0 {
                duration_ms // 休止符保持原始时长
            } else {
                let extended_duration = ((duration_ms as f64) * 1.5) as u32;
                // if frequency < 200 {
                //     extended_duration.max(200) // 极低频至少 200ms
                // } else if frequency < 300 {
                //     extended_duration.max(150) // 低频至少 150ms
                // } else {
                //     extended_duration.max(100) // 普通频率至少 100ms
                // }
                if frequency < 400{
                    frequency = (frequency as f64 * 3.0) as u32; // 低频提升1.5倍
                }
                extended_duration
            };
            
            if frequency == 0 {
                // 休止符，只增加时间
                current_time += actual_duration as f64 / 1000.0;
                continue;
            }
            
            // 创建振荡器
            if let Ok(create_fn) = js_sys::Reflect::get(&ctx, &JsValue::from_str("createOscillator")) {
                if let Some(func) = create_fn.dyn_ref::<js_sys::Function>() {
                    if let Ok(osc) = func.call0(&ctx) {
                        if osc.is_undefined() || osc.is_null() {
                            continue;
                        }
                        
                        // 设置方波
                        let _ = js_sys::Reflect::set(&osc, &JsValue::from_str("type"), &JsValue::from_str("square"));
                        
                        // 设置频率（低频自动提升2.0倍以适应手机喇叭）
                        // let adjusted_freq = if frequency < 300 {
                        //     (frequency as f64) * 2.0
                        // } else {
                        //     frequency as f64
                        // };
                        let adjusted_freq = frequency as f64;
                        
                        if let Ok(freq_param) = js_sys::Reflect::get(&osc, &JsValue::from_str("frequency")) {
                            let _ = js_sys::Reflect::set(&freq_param, &JsValue::from_str("value"), &JsValue::from_f64(adjusted_freq));
                        }
                        
                        // 2. 创建独立的 GainNode 用于音频包络
                        if let Some(envelope_gain) = self.create_gain_node(&ctx) {
                            // 连接：Oscillator -> EnvelopeGain -> MasterGain -> Destination
                            self.connect_nodes(&osc, &envelope_gain);
                            if let Some(master_gain) = &self.gain_node {
                                self.connect_nodes(&envelope_gain, master_gain);
                            }
                            
                            // 设置音频包络
                            if let Ok(gain_param) = js_sys::Reflect::get(&envelope_gain, &JsValue::from_str("gain")) {
                                // Attack: 10ms
                                let _ = js_sys::Reflect::set(&gain_param, &JsValue::from_str("value"), &JsValue::from_f64(0.0));
                                
                                if let Ok(ramp_fn) = js_sys::Reflect::get(&gain_param, &JsValue::from_str("linearRampToValueAtTime")) {
                                    if let Some(func) = ramp_fn.dyn_ref::<js_sys::Function>() {
                                        let _ = func.call2(&gain_param, &JsValue::from_f64(1.0), &JsValue::from_f64(current_time + 0.01));
                                    }
                                }
                                
                                // Release: 最后 20ms
                                let release_start = current_time + (actual_duration as f64 / 1000.0) - 0.02;
                                let release_end = current_time + (actual_duration as f64 / 1000.0);
                                
                                if let Ok(set_fn) = js_sys::Reflect::get(&gain_param, &JsValue::from_str("setValueAtTime")) {
                                    if let Some(func) = set_fn.dyn_ref::<js_sys::Function>() {
                                        let _ = func.call2(&gain_param, &JsValue::from_f64(1.0), &JsValue::from_f64(release_start));
                                    }
                                }
                                
                                if let Ok(ramp_fn) = js_sys::Reflect::get(&gain_param, &JsValue::from_str("linearRampToValueAtTime")) {
                                    if let Some(func) = ramp_fn.dyn_ref::<js_sys::Function>() {
                                        let _ = func.call2(&gain_param, &JsValue::from_f64(0.0), &JsValue::from_f64(release_end));
                                    }
                                }
                            }
                            
                            // 安排开始和结束时间
                            if let Ok(start_fn) = js_sys::Reflect::get(&osc, &JsValue::from_str("start")) {
                                if let Some(func) = start_fn.dyn_ref::<js_sys::Function>() {
                                    let _ = func.call1(&osc, &JsValue::from_f64(current_time));
                                }
                            }
                            
                            let stop_time = current_time + (actual_duration as f64 / 1000.0);
                            if let Ok(stop_fn) = js_sys::Reflect::get(&osc, &JsValue::from_str("stop")) {
                                if let Some(func) = stop_fn.dyn_ref::<js_sys::Function>() {
                                    let _ = func.call1(&osc, &JsValue::from_f64(stop_time));
                                }
                            }
                            
                            current_time = stop_time;
                        }
                    }
                }
            }
        }
    }

    fn stop(&mut self) {
        self.stop_current_oscillator();
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.update_gain_value();
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.muted = !enabled;
        self.update_gain_value();
        
        if enabled {
            self.resume();
        }
    }

    fn is_enabled(&self) -> bool {
        !self.muted
    }
}

// ============================================================================
// 游戏应用
// ============================================================================

struct WxGameApp {
    display: WxDisplay,
    input: Rc<RefCell<WxInput>>,
    audio: WxAudio,
    cpu_renderer: CpuRenderer,
    game_state: Option<GameState>,
    fps_counter: FpsCounter,
    running: bool,
}

impl WxGameApp {
    fn new() -> Result<Self, String> {
        let display = WxDisplay::new()?;
        
        // 创建 CPU 渲染器
        let cpu_renderer = CpuRenderer::new(GAME_WIDTH_U32, GAME_HEIGHT_U32);
        log_info(&format!("[CPU] CPU software renderer initialized: {}x{}", GAME_WIDTH_U32, GAME_HEIGHT_U32));
        
        let input = Rc::new(RefCell::new(WxInput::new()));
        
        // 设置全局 WxInput 实例，供 JS 直接调用
        unsafe {
            set_global_wxinput(input.clone());
            log_info("Global WxInput initialized");
        }
        
        Ok(Self {
            display,
            input,
            audio: WxAudio::new(),
            cpu_renderer,
            game_state: None,
            fps_counter: FpsCounter::new(),
            running: true,
        })
    }
    
    fn init_game(&mut self) {
        log_info("初始化游戏状态...");
        let mut game_state = GameState::new();
        
        // 上传图集和调色板到 CPU 渲染器（通过 submit_to_cpu）
        log_info("上传图集和调色板到 CPU 渲染器...");
        game_state.submit_to_cpu(&mut self.cpu_renderer);
        
        self.game_state = Some(game_state);
        log_info("游戏状态初始化完成");
    }
    
    fn is_running(&self) -> bool {
        self.running && !self.input.borrow().should_close()
    }
    
    fn frame_update(&mut self) {
        // 使用当前时间
        let current_time = get_time_ms();
        self.frame_update_with_time(current_time, 16.67);
    }
    
    fn frame_update_with_time(&mut self, _current_time: f64, _delta_time: f64) {
        // 测量实际渲染耗时
        let render_start = get_time_ms();
        
        if let Some(state) = &mut self.game_state {
            state.set_fps_display(self.fps_counter.fps(), self.fps_counter.frame_time_ms());
            state.set_render_mode(RenderMode::CPU);
            
            // 处理输入
            let events = self.input.borrow_mut().poll_events();
            for event in events {
                state.handle_key_event(&event);
            }
            
            // 更新游戏逻辑
            let result = state.frame_update();
            
            // CPU 渲染
            self.cpu_renderer.clear();
            state.submit_to_cpu(&mut self.cpu_renderer);
            
            // 将帧缓冲绘制到 Canvas
            let framebuffer = self.cpu_renderer.framebuffer();
            if let Err(e) = self.display.render_framebuffer(framebuffer) {

                //减少日志频率
                static RENDER_ERROR_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                let count = RENDER_ERROR_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if count % 100 == 0 {
                    log_error(&format!("渲染帧缓冲失败: {:?}", e));
                    }

                
            }
            
            // 渲染虚拟控制器叠加层
            render_virtual_controller();
            
            if result == FrameResult::Exit {
                // state.shutdown();
                // self.running = false;
            }
        }
        
        // 使用实际渲染耗时更新FPS计数器
        let render_time = get_time_ms() - render_start;
        self.fps_counter.update(render_time);
    }
}

// ============================================================================
// requestAnimationFrame
// ============================================================================

fn request_animation_frame(callback: &Closure<dyn FnMut(f64)>) -> Result<(), JsValue> {
    let global = get_global();
    
    // 尝试从全局对象获取 requestAnimationFrame
    if let Ok(raf) = js_sys::Reflect::get(&global, &JsValue::from_str("requestAnimationFrame")) {
        if raf.is_function() {
            let raf_fn = raf.dyn_ref::<js_sys::Function>().unwrap();
            let _ = raf_fn.call1(&global, callback.as_ref())?;
            return Ok(());
        }
    }
    
    // 微信小游戏：尝试从 GameGlobal 获取
    if let Ok(game_global) = js_sys::Reflect::get(&global, &JsValue::from_str("GameGlobal")) {
        if !game_global.is_undefined() && !game_global.is_null() {
            if let Ok(raf) = js_sys::Reflect::get(&game_global, &JsValue::from_str("requestAnimationFrame")) {
                if raf.is_function() {
                    let raf_fn = raf.dyn_ref::<js_sys::Function>().unwrap();
                    let _ = raf_fn.call1(&game_global, callback.as_ref())?;
                    return Ok(());
                }
            }
        }
    }
    
    // 微信小游戏：尝试从 wx 对象获取
    if let Ok(wx) = js_sys::Reflect::get(&global, &JsValue::from_str("wx")) {
        if !wx.is_undefined() && !wx.is_null() {
            if let Ok(raf) = js_sys::Reflect::get(&wx, &JsValue::from_str("requestAnimationFrame")) {
                if raf.is_function() {
                    let raf_fn = raf.dyn_ref::<js_sys::Function>().unwrap();
                    let _ = raf_fn.call1(&wx, callback.as_ref())?;
                    return Ok(());
                }
            }
        }
    }
    
    log_error("requestAnimationFrame not found on global, GameGlobal or wx");
    Err(JsValue::from_str("requestAnimationFrame not available"))
}

/// 游戏主循环状态
struct GameLoopState {
    app: Rc<RefCell<WxGameApp>>,
    last_time: f64,
}

fn game_loop(app: Rc<RefCell<WxGameApp>>) {
    // 初始化循环状态
    let state = Rc::new(RefCell::new(GameLoopState {
        app,
        last_time: 0.0,
    }));
    
    schedule_frame(state);
}

fn schedule_frame(state: Rc<RefCell<GameLoopState>>) {
    let state_clone = state.clone();
    
    let closure = Closure::wrap(Box::new(move |current_time: f64| {
        let mut loop_state = state_clone.borrow_mut();
        
        // 初始化 last_time
        if loop_state.last_time == 0.0 {
            loop_state.last_time = current_time;
        }
        
        let delta_time = current_time - loop_state.last_time;
        loop_state.last_time = current_time;
        
        // 每次 rAF 回调都执行更新（帧率由 wx.setPreferredFramesPerSecond 控制）
        let running = {
            let app_ref = loop_state.app.borrow();
            app_ref.is_running()
        };
        
        if running {
            {
                let mut app_ref = loop_state.app.borrow_mut();
                app_ref.frame_update_with_time(current_time, delta_time);
            }
            
            // 继续下一帧
            drop(loop_state);
            schedule_frame(state_clone.clone());
        }
    }) as Box<dyn FnMut(f64)>);

    if let Err(e) = request_animation_frame(&closure) {
        log_error(&format!("无法请求动画帧: {:?}", e));
    }

    closure.forget();
}

// ============================================================================
// WASM 入口点
// ============================================================================

#[wasm_bindgen]
pub fn run_wxgame_cpu() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    
    log_info("Mario RS - 微信小游戏 CPU 版本启动");
    
    match WxGameApp::new() {
        Ok(mut app) => {
            app.init_game();
            
            let app_rc = Rc::new(RefCell::new(app));
            
            log_info("开始游戏主循环 (CPU 渲染)");
            game_loop(app_rc);
        }
        Err(e) => {
            log_error(&format!("初始化失败: {}", e));
        }
    }
}

/// JS 调用：处理按钮事件 (虚拟触摸按钮)
#[wasm_bindgen]
pub fn on_button_event_cpu(button_id: i32, pressed: bool) {
    
    unsafe {
        if let Some(input) = get_global_wxinput() {
            input.borrow_mut().handle_button_event(button_id, pressed);
        } else {
            log_error("WxInput not initialized for button event");
        }
    }
}

/// 全局 WxInput 实例，用于从 JS 直接访问
static mut GLOBAL_WXINPUT: Option<Rc<RefCell<WxInput>>> = None;

/// 设置全局 WxInput 实例（在游戏初始化时调用）
unsafe fn set_global_wxinput(input: Rc<RefCell<WxInput>>) {
    unsafe {
        GLOBAL_WXINPUT = Some(input);
    }
}

/// 获取全局 WxInput 实例
unsafe fn get_global_wxinput() -> Option<Rc<RefCell<WxInput>>> {
    unsafe {
        // 使用原始指针避免创建共享引用（Rust 2024 兼容性）
        let ptr = std::ptr::addr_of!(GLOBAL_WXINPUT);
        (*ptr).as_ref().cloned()
    }
}

/// JS 调用：处理键盘事件 (微信小游戏 PC 端 wx.onKeyDown/wx.onKeyUp)
#[wasm_bindgen]
pub fn on_key_event_cpu(code: &str, pressed: bool) {
    
    unsafe {
        if let Some(input) = get_global_wxinput() {
            input.borrow_mut().handle_key_event(code, pressed);
        } else {
            log_error("WxInput not initialized");
        }
    }
}

/// JS 调用：恢复音频上下文 (用户交互后调用)
#[wasm_bindgen]
pub fn resume_audio_cpu() {
    let global = get_global();
    if let Ok(game_global) = js_sys::Reflect::get(&global, &JsValue::from_str("GameGlobal")) {
        if !game_global.is_undefined() && !game_global.is_null() {
            if let Ok(audio) = js_sys::Reflect::get(&game_global, &JsValue::from_str("wxgame_audio_cpu")) {
                if !audio.is_undefined() && !audio.is_null() {
                    if let Ok(resume_fn) = js_sys::Reflect::get(&audio, &JsValue::from_str("resume")) {
                        if let Some(func) = resume_fn.dyn_ref::<js_sys::Function>() {
                            let _ = func.call0(&audio);
                            log_info("音频上下文已恢复 (CPU 版本)");
                        }
                    }
                }
            }
        }
    }
}


/// 初始化虚拟控制器叠加层
fn init_virtual_controller_overlay() {
    let global = get_global();
    
    // 尝试调用 VirtualController.lateInit()
    if let Ok(game_global) = js_sys::Reflect::get(&global, &JsValue::from_str("GameGlobal")) {
        if !game_global.is_undefined() && !game_global.is_null() {
            if let Ok(vc) = js_sys::Reflect::get(&game_global, &JsValue::from_str("VirtualController")) {
                if !vc.is_undefined() && !vc.is_null() {
                    if let Ok(late_init) = js_sys::Reflect::get(&vc, &JsValue::from_str("lateInit")) {
                        if let Some(func) = late_init.dyn_ref::<js_sys::Function>() {
                            let ret = func.call0(&vc);
                            if let Err(e) = ret {
                                log_error(&format!("VirtualController.lateInit() failed: {:?}", e));
                            } else {
                                log_info("虚拟控制器叠加层初始化完成");
                            }
                        }
                    }
                }
            }
        }
    }
}