//! GPU渲染测试 - 验证精灵和填充渲染是否正常
//!
//! 运行方式: cargo run --example test_gpu_render
//!
//! 这个测试程序创建一个简单的窗口，渲染几个精灵和填充矩形，
//! 用于调试GPU渲染流程。
//!
//! 注意：这个测试程序不依赖pixels库，直接使用winit和wgpu。

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
    dpi::LogicalSize,
};
use wgpu::Backends;

// 引入游戏模块
use mario::gpu::{GpuRenderer, SpriteInstance, FillRect, GAME_WIDTH, GAME_HEIGHT, ATLAS_SIZE};
use mario::sprites::{SpriteDataManager, SpriteId, PALETTE};

const WINDOW_WIDTH: u32 = 640;
const WINDOW_HEIGHT: u32 = 480;

struct TestApp {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    device: Option<Arc<wgpu::Device>>,
    queue: Option<Arc<wgpu::Queue>>,
    config: Option<wgpu::SurfaceConfiguration>,
    gpu_renderer: Option<GpuRenderer>,
    sprites: Option<SpriteDataManager>,
    frame_count: u32,
}

impl TestApp {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            device: None,
            queue: None,
            config: None,
            gpu_renderer: None,
            sprites: None,
            frame_count: 0,
        }
    }

    fn init_wgpu(&mut self, window: Arc<Window>) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: Backends::VULKAN | Backends::GL | Backends::DX12,
            ..Default::default()
        });

        // 创建surface
        let surface = instance.create_surface(window.clone()).expect("创建surface失败");

        // 请求适配器
        let adapter = futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })).expect("找不到合适的GPU适配器");

        println!("使用GPU适配器: {:?}", adapter.get_info());

        // 请求设备
        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("test_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        )).expect("请求设备失败");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // 配置surface
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // 创建GPU渲染器
        let gpu_renderer = GpuRenderer::new(device.clone(), queue.clone(), config.format);

        // 加载精灵
        let sprites = SpriteDataManager::new();
        let atlas = sprites.build_atlas();

        // 上传图集到GPU
        gpu_renderer.upload_atlas(atlas.data(), ATLAS_SIZE, ATLAS_SIZE);

        // 创建并上传调色板
        let palette_rgba = create_palette_rgba();
        gpu_renderer.upload_palette(0, &palette_rgba);

        println!("GPU渲染器初始化完成");
        println!("图集大小: {}x{}", ATLAS_SIZE, ATLAS_SIZE);
        println!("游戏分辨率: {}x{}", GAME_WIDTH, GAME_HEIGHT);

        self.window = Some(window);
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
        self.gpu_renderer = Some(gpu_renderer);
        self.sprites = Some(sprites);
    }

    fn render(&mut self) {
        let surface = match &self.surface {
            Some(s) => s,
            None => return,
        };
        let gpu_renderer = match &mut self.gpu_renderer {
            Some(r) => r,
            None => return,
        };
        let config = match &self.config {
            Some(c) => c,
            None => return,
        };
        let sprites = match &self.sprites {
            Some(s) => s,
            None => return,
        };

        // 获取surface纹理
        let output = match surface.get_current_texture() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("获取surface纹理失败: {:?}", e);
                return;
            }
        };
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 更新缩放参数
        gpu_renderer.update_scale(config.width, config.height);

        // 开始新帧
        gpu_renderer.begin_frame();

        // 1. 添加天空背景填充 (蓝色，调色板索引 35)
        gpu_renderer.draw_fill(FillRect::new(0.0, 0.0, GAME_WIDTH as f32, GAME_HEIGHT as f32, 35, 0));

        // 2. 添加地面填充 (棕色，调色板索引 7)
        gpu_renderer.draw_fill(FillRect::new(0.0, 150.0, GAME_WIDTH as f32, 32.0, 7, 0));

        // 3. 添加一些测试精灵
        let atlas = sprites.build_atlas();
        
        // 砖块精灵 (BROWN)
        let brick_uv = atlas.get(SpriteId::BROWN_000);
        let brick_sprite = SpriteInstance::new(
            50.0, 120.0,                           // 位置
            20.0, 14.0,                            // 尺寸
            brick_uv.x as f32 / ATLAS_SIZE as f32, // UV x
            brick_uv.y as f32 / ATLAS_SIZE as f32, // UV y
            brick_uv.width as f32 / ATLAS_SIZE as f32,  // UV width
            brick_uv.height as f32 / ATLAS_SIZE as f32, // UV height
        );
        gpu_renderer.draw_sprite(brick_sprite);

        // 多个砖块
        for i in 0..5 {
            let x = 50.0 + i as f32 * 20.0;
            let sprite = SpriteInstance::new(
                x, 120.0,
                20.0, 14.0,
                brick_uv.x as f32 / ATLAS_SIZE as f32,
                brick_uv.y as f32 / ATLAS_SIZE as f32,
                brick_uv.width as f32 / ATLAS_SIZE as f32,
                brick_uv.height as f32 / ATLAS_SIZE as f32,
            );
            gpu_renderer.draw_sprite(sprite);
        }

        // 问号砖块
        let quest_uv = atlas.get(SpriteId::QUEST_000);
        let quest_sprite = SpriteInstance::new(
            160.0, 80.0,
            20.0, 14.0,
            quest_uv.x as f32 / ATLAS_SIZE as f32,
            quest_uv.y as f32 / ATLAS_SIZE as f32,
            quest_uv.width as f32 / ATLAS_SIZE as f32,
            quest_uv.height as f32 / ATLAS_SIZE as f32,
        );
        gpu_renderer.draw_sprite(quest_sprite);

        // Mario精灵 (SWMAR - 小马里奥行走)
        // 注意: 这需要SpriteId中有对应的定义
        // 使用帧计数来动画
        let mario_frame = (self.frame_count / 10) % 2;
        let mario_id = if mario_frame == 0 { SpriteId::SWMAR_000 } else { SpriteId::SWMAR_001 };
        let mario_uv = atlas.get(mario_id);
        let mario_sprite = SpriteInstance::new(
            100.0, 136.0 - mario_uv.height as f32, // 站在地面上
            mario_uv.width as f32, mario_uv.height as f32,
            mario_uv.x as f32 / ATLAS_SIZE as f32,
            mario_uv.y as f32 / ATLAS_SIZE as f32,
            mario_uv.width as f32 / ATLAS_SIZE as f32,
            mario_uv.height as f32 / ATLAS_SIZE as f32,
        );
        gpu_renderer.draw_sprite(mario_sprite);

        // 敌人 (Chibibo/栗子敌人)
        let enemy_frame = (self.frame_count / 15) % 2;
        let enemy_id = if enemy_frame == 0 { SpriteId::CHIBIBO_000 } else { SpriteId::CHIBIBO_001 };
        let enemy_uv = atlas.get(enemy_id);
        let enemy_sprite = SpriteInstance::new(
            200.0, 136.0 - enemy_uv.height as f32,
            enemy_uv.width as f32, enemy_uv.height as f32,
            enemy_uv.x as f32 / ATLAS_SIZE as f32,
            enemy_uv.y as f32 / ATLAS_SIZE as f32,
            enemy_uv.width as f32 / ATLAS_SIZE as f32,
            enemy_uv.height as f32 / ATLAS_SIZE as f32,
        );
        gpu_renderer.draw_sprite(enemy_sprite);

        // 金币
        let coin_uv = atlas.get(SpriteId::COIN_000);
        let coin_sprite = SpriteInstance::new(
            160.0, 50.0,
            coin_uv.width as f32, coin_uv.height as f32,
            coin_uv.x as f32 / ATLAS_SIZE as f32,
            coin_uv.y as f32 / ATLAS_SIZE as f32,
            coin_uv.width as f32 / ATLAS_SIZE as f32,
            coin_uv.height as f32 / ATLAS_SIZE as f32,
        );
        gpu_renderer.draw_sprite(coin_sprite);

        // 渲染帧到GPU纹理
        gpu_renderer.render_frame();

        // 渲染到窗口surface
        gpu_renderer.render_to_surface(&view);

        // 提交
        output.present();

        self.frame_count += 1;

        // 每60帧打印一次统计
        if self.frame_count % 60 == 0 {
            println!("帧 {}: 渲染正常", self.frame_count);
        }
    }
}

impl ApplicationHandler for TestApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = Window::default_attributes()
                .with_title("GPU渲染测试 - Mario精灵")
                .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
                .with_resizable(true);

            let window = Arc::new(event_loop.create_window(window_attributes).expect("创建窗口失败"));
            
            self.init_wgpu(window);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("关闭窗口");
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let (Some(surface), Some(device), Some(config)) = 
                    (&self.surface, &self.device, &mut self.config) 
                {
                    config.width = new_size.width.max(1);
                    config.height = new_size.height.max(1);
                    surface.configure(device, config);
                    println!("窗口大小调整: {}x{}", new_size.width, new_size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// 从游戏调色板创建RGBA格式的调色板数据
fn create_palette_rgba() -> [[u8; 4]; 256] {
    let mut rgba = [[0u8; 4]; 256];
    for i in 0..256 {
        if i < PALETTE.len() {
            let (r, g, b, a) = PALETTE[i];
            rgba[i] = [r, g, b, a];
        } else {
            rgba[i] = [0, 0, 0, 255];
        }
    }
    rgba
}

fn main() {
    println!("=========================================");
    println!("    GPU渲染测试程序");
    println!("=========================================");
    println!();
    println!("这个测试程序将渲染:");
    println!("  - 蓝色天空背景");
    println!("  - 棕色地面");
    println!("  - 砖块精灵");
    println!("  - 问号砖块");
    println!("  - Mario精灵 (动画)");
    println!("  - 敌人精灵 (动画)");
    println!("  - 金币精灵");
    println!();
    println!("如果画面显示正常，说明GPU渲染流程工作正常。");
    println!("如果画面异常，请检查:");
    println!("  1. 着色器代码是否正确");
    println!("  2. 图集数据是否正确上传");
    println!("  3. 调色板数据是否正确");
    println!("  4. UV坐标计算是否正确");
    println!();

    let event_loop = EventLoop::new().expect("创建事件循环失败");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = TestApp::new();
    event_loop.run_app(&mut app).expect("运行失败");
}
