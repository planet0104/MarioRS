// PC 桌面端平台实现
//
// 实现 platform.rs 中定义的所有 Backend traits
// 使用: pixels + winit + cpal + rand + std::fs
//
// 重要:只有这个模块依赖 winit,其他游戏模块通过 platform.rs 抽象访问

use super::{
    AudioBackend, DisplayBackend, InputBackend, 
    KeyCode as PlatformKeyCode, KeyEvent as PlatformKeyEvent,
    LogBackend, LogLevel, RandomBackend, StorageBackend, TimeBackend,
};
use crate::gpu::GpuRenderer;

// Windows 使用 hashbrown 避免 BCryptGenRandom 依赖(兼容 Win7)
// 但 desktop.rs 主要用于 wgpu-backend(非 Windows GDI),所以保留 std
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// Winit 相关导入(仅在此模块使用)
// ============================================================================

use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use pixels::wgpu::Backends;
use pixels::wgpu;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};
use winit::window::{Icon, Window, WindowId};

// ============================================================================
// 窗口图标 - 从游戏精灵生成
// ============================================================================

/// 从游戏精灵创建窗口图标(32x32 RGBA)
fn create_window_icon() -> Option<Icon> {
    use crate::sprites::{SpriteDataManager, PALETTE};
    
    // 加载精灵管理器获取 Mario 精灵
    let sprites = SpriteDataManager::new();
    let mario = &sprites.LWMAR_000; // 大马里奥行走(经典形象)
    
    // 源精灵尺寸: 20x28
    const SRC_W: usize = 20;
    const SRC_H: usize = 28;
    // 目标图标尺寸: 32x32
    const ICON_SIZE: usize = 32;
    
    let mut rgba = vec![0u8; ICON_SIZE * ICON_SIZE * 4];
    
    // 计算缩放和居中偏移
    let scale = 1; // 1:1 缩放保持像素清晰
    let offset_x = (ICON_SIZE - SRC_W * scale) / 2;
    let offset_y = (ICON_SIZE - SRC_H * scale) / 2;
    
    // 转换调色板索引为 RGBA
    for y in 0..SRC_H {
        for x in 0..SRC_W {
            let palette_idx = mario[y][x] as usize;
            let (r, g, b, a) = if palette_idx == 0 {
                (0, 0, 0, 0) // 透明
            } else if palette_idx < PALETTE.len() {
                PALETTE[palette_idx]
            } else {
                (0, 0, 0, 0)
            };
            
            // 缩放并居中绘制
            for sy in 0..scale {
                for sx in 0..scale {
                    let px = offset_x + x * scale + sx;
                    let py = offset_y + y * scale + sy;
                    if px < ICON_SIZE && py < ICON_SIZE {
                        let idx = (py * ICON_SIZE + px) * 4;
                        rgba[idx] = r;
                        rgba[idx + 1] = g;
                        rgba[idx + 2] = b;
                        rgba[idx + 3] = a;
                    }
                }
            }
        }
    }
    
    Icon::from_rgba(rgba, ICON_SIZE as u32, ICON_SIZE as u32).ok()
}

// ============================================================================
// 显示后端 - 使用 pixels + winit,支持等比例全屏
// ============================================================================

pub struct DesktopDisplay {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    fit_renderer: Option<FitRenderer>,
    fit_viewport: FitViewport,
    width: u32,
    height: u32,
    // GPU渲染器
    gpu_renderer: Option<GpuRenderer>,
}

#[derive(Clone, Copy, Debug)]
struct FitViewport {
    surface_w: u32,
    surface_h: u32,
    draw_w: u32,
    draw_h: u32,
    bar_x: u32,
    bar_y: u32,
}

impl FitViewport {
    fn new(game_w: u32, game_h: u32, surface_w: u32, surface_h: u32) -> Self {
        // 非整数等比缩放,尽可能放大但不超出 surface
        let gw = game_w as f64;
        let gh = game_h as f64;
        let sw = surface_w as f64;
        let sh = surface_h as f64;

        let scale = (sw / gw).min(sh / gh);
        let mut draw_w = (gw * scale).floor().max(1.0) as u32;
        let mut draw_h = (gh * scale).floor().max(1.0) as u32;

        if draw_w > surface_w {
            draw_w = surface_w;
        }
        if draw_h > surface_h {
            draw_h = surface_h;
        }

        let bar_x = surface_w.saturating_sub(draw_w) / 2;
        let bar_y = surface_h.saturating_sub(draw_h) / 2;

        Self {
            surface_w,
            surface_h,
            draw_w,
            draw_h,
            bar_x,
            bar_y,
        }
    }

    fn as_uniform_params(&self) -> [f32; 4] {
        // params: [scale_x, scale_y, translate_x, translate_y]
        // translate_x/translate_y 是目标区域中心点在 NDC 的坐标
        let sw = self.surface_w.max(1) as f32;
        let sh = self.surface_h.max(1) as f32;
        let dw = self.draw_w.max(1) as f32;
        let dh = self.draw_h.max(1) as f32;

        let scale_x = dw / sw;
        let scale_y = dh / sh;

        let center_x = (self.bar_x as f32 + dw * 0.5) / sw;
        let center_y_topdown = (self.bar_y as f32 + dh * 0.5) / sh;

        let translate_x = center_x * 2.0 - 1.0;
        let translate_y = 1.0 - center_y_topdown * 2.0;

        [scale_x, scale_y, translate_x, translate_y]
    }
}

fn pack_f32x4_le(v: [f32; 4]) -> [u8; 16] {
    let mut out = [0u8; 16];
    let mut i = 0usize;
    while i < 4 {
        let b = v[i].to_le_bytes();
        out[i * 4] = b[0];
        out[i * 4 + 1] = b[1];
        out[i * 4 + 2] = b[2];
        out[i * 4 + 3] = b[3];
        i += 1;
    }
    out
}

const FIT_SCALE_WGSL: &str = r#"
struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @builtin(position) position: vec4<f32>,
}

@group(0) @binding(0) var r_tex_color: texture_2d<f32>;
@group(0) @binding(1) var r_tex_sampler: sampler;
// params: x=scale_x y=scale_y z=translate_x w=translate_y
@group(0) @binding(2) var<uniform> r_params: vec4<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VertexOutput {
    var pos = vec2<f32>(0.0, 0.0);
    if (vid == 0u) {
        pos = vec2<f32>(-1.0, -1.0);
    } else if (vid == 1u) {
        pos = vec2<f32>(3.0, -1.0);
    } else {
        pos = vec2<f32>(-1.0, 3.0);
    }

    var out: VertexOutput;
    out.tex_coord = fma(pos, vec2<f32>(0.5, -0.5), vec2<f32>(0.5, 0.5));
    out.position = vec4<f32>(pos * r_params.xy + r_params.zw, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(@location(0) tex_coord: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(r_tex_color, r_tex_sampler, tex_coord);
}
"#;

struct FitRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl FitRenderer {
    fn new(
        device: &wgpu::Device,
        source_texture: &wgpu::Texture,
        render_target_format: wgpu::TextureFormat,
    ) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mariors_fit_scale_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(FIT_SCALE_WGSL)),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mariors_fit_scale_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mariors_fit_scale_uniform"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mariors_fit_scale_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mariors_fit_scale_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mariors_fit_scale_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_target_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let texture_view = source_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mariors_fit_scale_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            pipeline,
            bind_group,
            uniform_buffer,
        }
    }

    fn update_viewport(&self, queue: &wgpu::Queue, viewport: FitViewport) {
        let params = viewport.as_uniform_params();
        let bytes = pack_f32x4_le(params);
        queue.write_buffer(&self.uniform_buffer, 0, &bytes);
    }

    fn render(&self, encoder: &mut wgpu::CommandEncoder, render_target: &wgpu::TextureView, viewport: FitViewport) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("mariors_fit_scale_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // 只在目标渲染区域绘制,避免 full-screen triangle 在边缘采样导致伪影
        if viewport.draw_w > 0 && viewport.draw_h > 0 {
            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.bind_group, &[]);
            rpass.set_scissor_rect(viewport.bar_x, viewport.bar_y, viewport.draw_w, viewport.draw_h);
            rpass.draw(0..3, 0..1);
        }
    }
}

impl DesktopDisplay {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            window: None,
            pixels: None,
            fit_renderer: None,
            fit_viewport: FitViewport::new(width, height, width, height),
            width,
            height,
            gpu_renderer: None,
        }
    }
    
    /// 获取GPU渲染器引用
    pub fn gpu_renderer(&self) -> Option<&GpuRenderer> {
        self.gpu_renderer.as_ref()
    }
    
    /// 获取GPU渲染器可变引用
    pub fn gpu_renderer_mut(&mut self) -> Option<&mut GpuRenderer> {
        self.gpu_renderer.as_mut()
    }

    /// 使用 ActiveEventLoop 创建窗口(winit 0.30 要求在事件循环内创建)
    pub fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        use winit::dpi::LogicalSize;
        
        let size = LogicalSize::new(self.width as f64, self.height as f64);
        
        // 创建窗口图标(从游戏精灵生成)
        let icon = create_window_icon();
        
        // 关键修复:先创建不可见窗口,初始化 pixels 并填充黑色后再显示
        // 这样可以避免启动时的白色闪烁
        let window_attributes = Window::default_attributes()
            .with_title("Mario")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_window_icon(icon)
            .with_visible(false);  // 先不显示窗口
        
        let window = Arc::new(event_loop.create_window(window_attributes)?);
        let window_size = window.inner_size();
        let window_outer_size = window.outer_size();
        let scale_factor = window.scale_factor();
        
        let surface_texture = SurfaceTexture::new(
            window_size.width,
            window_size.height,
            Arc::clone(&window),
        );

        let mut pixels = PixelsBuilder::new(self.width, self.height, surface_texture)
            .wgpu_backend(Backends::VULKAN | Backends::GL)
            .clear_color(pixels::wgpu::Color::BLACK)  // 设置清除颜色为黑色
            .build()?;
        
        // 初始化 framebuffer 为黑色
        for pixel in pixels.frame_mut().chunks_exact_mut(4) {
            pixel[0] = 0; // R
            pixel[1] = 0; // G
            pixel[2] = 0; // B
            pixel[3] = 255; // A
        }
        
        // 立即渲染黑色帧到 GPU surface,预热渲染管线
        // 这可以减少首次显示时的白色闪烁
        let _ = pixels.render();
        
        // 注意:不在这里显示窗口,而是在游戏初始化完成后显示
        // 这样可以避免加载期间的白色闪烁
        self.fit_viewport = FitViewport::new(self.width, self.height, window_size.width, window_size.height);
        let render_target_format = pixels.surface_texture_format();
        let fit_renderer = FitRenderer::new(&pixels.context().device, &pixels.context().texture, render_target_format);
        fit_renderer.update_viewport(&pixels.context().queue, self.fit_viewport);

        self.window = Some(window);
        self.fit_renderer = Some(fit_renderer);
        self.pixels = Some(pixels);
        Ok(())
    }
    
    /// 显示窗口(在游戏初始化完成后调用)
    pub fn show_window(&mut self) {
        if let Some(window) = &self.window {
            // 多次渲染黑色帧确保 GPU 完全准备好
            // 这可以避免首帧白色闪烁
            if let Some(pixels) = &mut self.pixels {
                for _ in 0..3 {
                    let _ = pixels.render();
                }
            }
            window.set_visible(true);
            // 显示后立即请求重绘
            window.request_redraw();
        }
    }

    pub fn has_window(&self) -> bool {
        self.window.is_some()
    }
    
    /// 处理窗口大小调整,重新创建 surface texture 以支持等比例缩放
    pub fn resize(&mut self, new_width: u32, new_height: u32) -> Result<(), Box<dyn std::error::Error>> {
        if let (Some(window), Some(pixels)) = (&self.window, &mut self.pixels) {
            // 注意:这里保持当前 pixels 的缩放方式(等比例缩放+letterbox)
            // 我们只更新 wgpu surface 尺寸,并打出关键尺寸日志用于排查
            let window_inner = window.inner_size();
            let window_outer = window.outer_size();
            let scale_factor = window.scale_factor();

            // 计算理论等比缩放后的渲染矩形(用于对照 pixels 的实际表现)
            let game_w = self.width as f64;
            let game_h = self.height as f64;
            let surface_w = new_width as f64;
            let surface_h = new_height as f64;
            let scale_fit = (surface_w / game_w).min(surface_h / game_h);
            let draw_fit_w = (game_w * scale_fit).floor().max(0.0) as u32;
            let draw_fit_h = (game_h * scale_fit).floor().max(0.0) as u32;
            let bar_fit_x = new_width.saturating_sub(draw_fit_w) / 2;
            let bar_fit_y = new_height.saturating_sub(draw_fit_h) / 2;

            // 如果 pixels 使用整数倍缩放,只有窗口尺寸是游戏尺寸的整数倍时才会无黑边
            let scale_int = scale_fit.floor().max(1.0) as u32;
            let draw_int_w = self.width.saturating_mul(scale_int);
            let draw_int_h = self.height.saturating_mul(scale_int);
            let bar_int_x = new_width.saturating_sub(draw_int_w) / 2;
            let bar_int_y = new_height.saturating_sub(draw_int_h) / 2;

            pixels.resize_surface(new_width, new_height)?;

            // 使用自定义非整数等比缩放(最小方案:保留 pixels,替换最后的缩放渲染)
            self.fit_viewport = FitViewport::new(self.width, self.height, new_width, new_height);
            if let Some(fit_renderer) = &self.fit_renderer {
                fit_renderer.update_viewport(&pixels.context().queue, self.fit_viewport);
            }

            // 无需调整游戏逻辑分辨率 (self.width x self.height)
            // pixels 会自动进行等比例缩放并添加 letterbox
        }
        Ok(())
    }
}

impl DisplayBackend for DesktopDisplay {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn present(&mut self) -> Result<(), String> {
        if let (Some(pixels), Some(fit_renderer)) = (&self.pixels, &self.fit_renderer) {
            let viewport = self.fit_viewport;
            pixels.render_with(|encoder, render_target, _context| {
                fit_renderer.render(encoder, render_target, viewport);
                Ok(())
            }).map_err(|e| {
                log_error(&format!("[display] present_failed {}", e));
                e.to_string()
            })
        } else {
            Ok(())
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

// ============================================================================
// 音频后端 - 使用平台音频模块
// ============================================================================

pub use super::audio::PlatformAudio as DesktopAudio;

// ============================================================================
// 时间后端 - 使用 std::time
// ============================================================================

pub struct DesktopTime {
    start: Instant,
}

impl DesktopTime {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl TimeBackend for DesktopTime {
    fn now_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }

    fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}

// ============================================================================
// 随机数后端 - 使用 rand (SmallRng 减小体积)
// ============================================================================

use rand::Rng;
use rand::rngs::SmallRng;
use rand::SeedableRng;

pub struct DesktopRandom {
    rng: SmallRng,
}

impl DesktopRandom {
    pub fn new() -> Self {
        // 使用系统时间作为种子,避免依赖rand的std_rng特性
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        Self {
            rng: SmallRng::seed_from_u64(seed),
        }
    }
}

impl RandomBackend for DesktopRandom {
    fn random_range(&mut self, max: i32) -> i32 {
        if max <= 0 {
            return 0;
        }
        self.rng.gen_range(0..max)
    }

    fn random_range_f32(&mut self, max: f32) -> f32 {
        if max <= 0.0 {
            return 0.0;
        }
        self.rng.gen_range(0.0..max)
    }

    fn random_f32(&mut self) -> f32 {
        self.rng.gen_range(0.0..1.0)
    }
}

// ============================================================================
// 存储后端 - 使用 std::fs
// ============================================================================

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

pub struct DesktopStorage {
    base_path: PathBuf,
}

impl DesktopStorage {
    pub fn new() -> Self {
        // 优先使用当前工作目录(更稳定,适合开发和发布环境)
        // 如果获取失败,则使用可执行文件所在目录
        let base_path = std::env::current_dir()
            .ok()
            .or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            })
            .unwrap_or_else(|| PathBuf::from("."));
        Self { base_path }
    }

    fn get_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key)
    }
}

impl StorageBackend for DesktopStorage {
    fn load(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.get_path(key);
        let mut file = File::open(&path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;
        Some(buffer)
    }

    fn save(&mut self, key: &str, data: &[u8]) -> Result<(), String> {
        let path = self.get_path(key);
        let mut file = File::create(&path).map_err(|e| e.to_string())?;
        file.write_all(data).map_err(|e| e.to_string())
    }

    fn remove(&mut self, key: &str) -> Result<(), String> {
        let path = self.get_path(key);
        fs::remove_file(&path).map_err(|e| e.to_string())
    }

    fn exists(&self, key: &str) -> bool {
        self.get_path(key).exists()
    }
}

// ============================================================================
// 日志后端 - 使用 println
// ============================================================================

pub struct DesktopLog;

impl DesktopLog {
    pub fn new() -> Self {
        Self
    }
}

impl LogBackend for DesktopLog {
    fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Debug => println!("[DEBUG] {}", message),
            LogLevel::Info => println!("[INFO] {}", message),
            LogLevel::Warn => println!("[WARN] {}", message),
            LogLevel::Error => eprintln!("[ERROR] {}", message),
        }
    }
}

// ============================================================================
// 输入后端
// ============================================================================

pub struct DesktopInput {
    key_states: HashSet<PlatformKeyCode>,
    pending_events: Vec<PlatformKeyEvent>,
    should_close: bool,
}

impl DesktopInput {
    pub fn new() -> Self {
        Self {
            key_states: HashSet::new(),
            pending_events: Vec::new(),
            should_close: false,
        }
    }

    /// 处理 winit 键盘事件
    pub fn handle_winit_key_event(&mut self, event: &KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        let key = winit_keycode_to_platform(&event.physical_key);
        
        if pressed {
            self.key_states.insert(key);
        } else {
            self.key_states.remove(&key);
        }

        self.pending_events.push(PlatformKeyEvent { key, pressed });
    }
}

impl InputBackend for DesktopInput {
    fn poll_events(&mut self) -> Vec<PlatformKeyEvent> {
        std::mem::take(&mut self.pending_events)
    }

    fn is_key_pressed(&self, key: PlatformKeyCode) -> bool {
        self.key_states.contains(&key)
    }

    fn should_close(&self) -> bool {
        self.should_close
    }

    fn request_close(&mut self) {
        self.should_close = true;
    }
}

/// 将 winit KeyCode 转换为平台无关的 KeyCode
fn winit_keycode_to_platform(physical_key: &PhysicalKey) -> PlatformKeyCode {
    match physical_key {
        PhysicalKey::Code(keycode) => match keycode {
            WinitKeyCode::ArrowLeft => PlatformKeyCode::Left,
            WinitKeyCode::ArrowRight => PlatformKeyCode::Right,
            WinitKeyCode::ArrowUp => PlatformKeyCode::Up,
            WinitKeyCode::ArrowDown => PlatformKeyCode::Down,
            WinitKeyCode::Space => PlatformKeyCode::Space,
            WinitKeyCode::AltLeft => PlatformKeyCode::AltLeft,
            WinitKeyCode::AltRight => PlatformKeyCode::AltRight,
            WinitKeyCode::ControlLeft => PlatformKeyCode::ControlLeft,
            WinitKeyCode::ControlRight => PlatformKeyCode::ControlRight,
            WinitKeyCode::ShiftLeft => PlatformKeyCode::ShiftLeft,
            WinitKeyCode::ShiftRight => PlatformKeyCode::ShiftRight,
            WinitKeyCode::Escape => PlatformKeyCode::Escape,
            WinitKeyCode::Enter => PlatformKeyCode::Enter,
            WinitKeyCode::Tab => PlatformKeyCode::Tab,
            WinitKeyCode::F1 => PlatformKeyCode::F1,
            WinitKeyCode::F2 => PlatformKeyCode::F2,
            WinitKeyCode::F11 => PlatformKeyCode::F11,
            WinitKeyCode::Backspace => PlatformKeyCode::Backspace,
            WinitKeyCode::KeyA => PlatformKeyCode::KeyA,
            WinitKeyCode::KeyB => PlatformKeyCode::KeyB,
            WinitKeyCode::KeyC => PlatformKeyCode::KeyC,
            WinitKeyCode::KeyD => PlatformKeyCode::KeyD,
            WinitKeyCode::KeyE => PlatformKeyCode::KeyE,
            WinitKeyCode::KeyF => PlatformKeyCode::KeyF,
            WinitKeyCode::KeyG => PlatformKeyCode::KeyG,
            WinitKeyCode::KeyH => PlatformKeyCode::KeyH,
            WinitKeyCode::KeyI => PlatformKeyCode::KeyI,
            WinitKeyCode::KeyJ => PlatformKeyCode::KeyJ,
            WinitKeyCode::KeyK => PlatformKeyCode::KeyK,
            WinitKeyCode::KeyL => PlatformKeyCode::KeyL,
            WinitKeyCode::KeyM => PlatformKeyCode::KeyM,
            WinitKeyCode::KeyN => PlatformKeyCode::KeyN,
            WinitKeyCode::KeyO => PlatformKeyCode::KeyO,
            WinitKeyCode::KeyP => PlatformKeyCode::KeyP,
            WinitKeyCode::KeyQ => PlatformKeyCode::KeyQ,
            WinitKeyCode::KeyR => PlatformKeyCode::KeyR,
            WinitKeyCode::KeyS => PlatformKeyCode::KeyS,
            WinitKeyCode::KeyT => PlatformKeyCode::KeyT,
            WinitKeyCode::KeyU => PlatformKeyCode::KeyU,
            WinitKeyCode::KeyV => PlatformKeyCode::KeyV,
            WinitKeyCode::KeyW => PlatformKeyCode::KeyW,
            WinitKeyCode::KeyX => PlatformKeyCode::KeyX,
            WinitKeyCode::KeyY => PlatformKeyCode::KeyY,
            WinitKeyCode::KeyZ => PlatformKeyCode::KeyZ,
            WinitKeyCode::Digit0 => PlatformKeyCode::Digit0,
            WinitKeyCode::Digit1 => PlatformKeyCode::Digit1,
            WinitKeyCode::Digit2 => PlatformKeyCode::Digit2,
            WinitKeyCode::Digit3 => PlatformKeyCode::Digit3,
            WinitKeyCode::Digit4 => PlatformKeyCode::Digit4,
            WinitKeyCode::Digit5 => PlatformKeyCode::Digit5,
            WinitKeyCode::Digit6 => PlatformKeyCode::Digit6,
            WinitKeyCode::Digit7 => PlatformKeyCode::Digit7,
            WinitKeyCode::Digit8 => PlatformKeyCode::Digit8,
            WinitKeyCode::Digit9 => PlatformKeyCode::Digit9,
            _ => PlatformKeyCode::Unknown,
        },
        PhysicalKey::Unidentified(_) => PlatformKeyCode::Unknown,
    }
}

// ============================================================================
// 全局便捷函数 - 使用线程局部存储
// ============================================================================

use std::cell::RefCell;

thread_local! {
    static RANDOM: RefCell<DesktopRandom> = RefCell::new(DesktopRandom::new());
    static TIME: DesktopTime = DesktopTime::new();
    static LOG: DesktopLog = DesktopLog::new();
}

pub fn random_i32(max: i32) -> i32 {
    RANDOM.with(|r| r.borrow_mut().random_range(max))
}

pub fn random_usize(max: usize) -> usize {
    random_i32(max as i32) as usize
}

pub fn random_u32(max: u32) -> u32 {
    random_i32(max as i32) as u32
}

pub fn random_u8(max: u8) -> u8 {
    random_i32(max as i32) as u8
}

pub fn random_f32(max: f32) -> f32 {
    RANDOM.with(|r| r.borrow_mut().random_range_f32(max))
}

pub fn now_ms() -> f64 {
    TIME.with(|t| t.now_ms())
}

pub fn log_debug(msg: &str) {
    LOG.with(|l| l.debug(msg));
}

pub fn log_info(msg: &str) {
    LOG.with(|l| l.info(msg));
}

pub fn log_warn(msg: &str) {
    LOG.with(|l| l.warn(msg));
}

pub fn log_error(msg: &str) {
    LOG.with(|l| l.error(msg));
}

// ============================================================================
// 游戏应用程序 - 封装事件循环
// ============================================================================

use crate::game_runner::{GameState, print_startup_info, GAME_WIDTH, GAME_HEIGHT};
use crate::platform::FrameResult;

/// 游戏应用程序状态(平台层只负责事件循环,游戏逻辑在 game_runner 中)
struct GameApp {
    display: DesktopDisplay,
    #[allow(dead_code)]
    input: DesktopInput,
    game_state: Option<GameState>,
    frame_duration: Duration,
    next_frame: Instant,
    #[allow(dead_code)]
    running: bool,
    is_fullscreen: bool,
}

impl GameApp {
    fn new() -> Self {
        Self {
            display: DesktopDisplay::new(GAME_WIDTH, GAME_HEIGHT),
            input: DesktopInput::new(),
            game_state: None,
            frame_duration: Duration::from_secs_f64(1.0 / 60.0),
            next_frame: Instant::now(),
            running: true,
            is_fullscreen: false,
        }
    }
    
    /// 切换全屏/窗口模式
    fn toggle_fullscreen(&mut self) {
        use winit::window::Fullscreen;
        
        if let Some(window) = &self.display.window {
            if self.is_fullscreen {
                // 退出全屏
                window.set_fullscreen(None);
                self.is_fullscreen = false;
            } else {
                // 进入全屏(使用无边框全屏模式)
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                self.is_fullscreen = true;
            }
        }
    }
}

impl ApplicationHandler for GameApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // 窗口创建(winit 0.30 要求在 resumed 中创建)
        if !self.display.has_window() {
            if let Err(e) = self.display.create_window(event_loop) {
                eprintln!("创建窗口失败: {}", e);
                event_loop.exit();
                return;
            }

            // 初始化游戏状态(游戏逻辑封装在 game_runner 模块中)
            // 注意:窗口在此期间保持不可见,避免白色闪烁
            self.game_state = Some(GameState::new());
            
            // 游戏初始化完成后再显示窗口
            self.display.show_window();
            
            // 打印启动信息
            print_startup_info();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(state) = &mut self.game_state {
                    state.shutdown(); // 保存配置后再退出
                }
                self.running = false;
                event_loop.exit();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                // DPI 或显示器缩放发生变化时,窗口的物理像素尺寸可能变化
                // 这里打日志并兜底调用 resize,避免 surface 尺寸没跟上导致画面偏小或黑边异常
                if let Some(window) = &self.display.window {
                    let inner = window.inner_size();
                    let outer = window.outer_size();
                    if inner.width > 0 && inner.height > 0 {
                        if let Err(e) = self.display.resize(inner.width, inner.height) {
                            eprintln!("调整窗口大小失败: {}", e);
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { event: key_event, .. } => {
                let platform_key = winit_keycode_to_platform(&key_event.physical_key);
                let is_pressed = key_event.state == ElementState::Pressed;
                
                // F11 切换全屏
                if is_pressed && platform_key == PlatformKeyCode::F11 {
                    self.toggle_fullscreen();
                    return;
                }
                
                // ESC 退出全屏(仅全屏模式下)
                if is_pressed && platform_key == PlatformKeyCode::Escape && self.is_fullscreen {
                    self.toggle_fullscreen();
                    return;
                }
                
                // 转换为平台无关的 KeyEvent 并更新游戏键盘状态
                if let Some(state) = &mut self.game_state {
                    let platform_event = crate::platform::KeyEvent {
                        key: platform_key,
                        pressed: is_pressed,
                    };
                    state.handle_key_event(&platform_event);
                }
            }
            WindowEvent::Resized(new_size) => {
                // 窗口大小改变时,调整 surface texture 尺寸
                // pixels 库会自动处理等比例缩放和 letterbox
                if new_size.width > 0 && new_size.height > 0 {
                    if let Some(window) = &self.display.window {
                    }
                    if let Err(e) = self.display.resize(new_size.width, new_size.height) {
                        eprintln!("调整窗口大小失败: {}", e);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // 帧率限制
                let now = Instant::now();
                if now < self.next_frame {
                    self.display.request_redraw();
                    return;
                }
                self.next_frame = now + self.frame_duration;

                // 游戏帧更新
                if let Some(state) = &mut self.game_state {
                    let result = state.frame_update();

                    // 渲染到framebuffer
                    let display_frame = self.display.framebuffer_mut();
                    state.render_to_rgba(display_frame);

                    // 显示 - pixels 会自动进行等比例缩放和添加 letterbox
                    let _ = self.display.present();

                    if result == FrameResult::Exit {
                        state.shutdown(); // 保存配置后再退出
                        self.running = false;
                        event_loop.exit();
                    }
                }

                // 请求下一帧
                self.display.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // 请求重绘以保持游戏循环运行
        self.display.request_redraw();
    }
}

/// 运行游戏(平台入口函数)
pub fn run_game() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统(仅在启用 logging feature 时)
    #[cfg(feature = "logging")]
    {
        use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
        
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"))
            .add_directive("wgpu_hal::vulkan=error".parse().unwrap());
        
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .try_init()
            .ok();
    }

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = GameApp::new();
    // winit 0.30: run_app 会消费 event_loop 并在退出时返回
    let _ = event_loop.run_app(&mut app);

    Ok(())
}