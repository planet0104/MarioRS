// ============================================================================
// 精灵渲染着色器 (Sprite Shader)
// ============================================================================
//
// 功能: 使用实例化渲染(Instance Rendering)批量绘制2D精灵
//
// wgpu/WGSL 教学要点:
// 1. 实例化渲染: 一次DrawCall绘制多个精灵，每个精灵有独立的变换参数
// 2. 索引调色板: 纹理存储的是调色板索引(0-255)，而非直接的颜色
// 3. 纹理采样: 使用textureSample从图集获取索引，使用textureLoad从调色板获取颜色
//
// 渲染流程:
// ┌─────────────────────────────────────────────────────────────────────┐
// │ 顶点着色器 (vs_main)                                                 │
// │   输入: vertex_index(0-5) + SpriteInstance(实例数据)                 │
// │   处理: 计算屏幕位置 + UV坐标(含翻转/旋转)                           │
// │   输出: clip_position + uv + palette参数                            │
// ├─────────────────────────────────────────────────────────────────────┤
// │ 片段着色器 (fs_main)                                                 │
// │   输入: 插值后的uv和palette参数                                      │
// │   处理: 从图集采样索引 -> 透明度判断 -> 调色板查找                   │
// │   输出: 最终颜色                                                     │
// └─────────────────────────────────────────────────────────────────────┘
// ============================================================================

// ----------------------------------------------------------------------------
// Uniform 绑定组 (Bind Group 0)
// 
// wgpu教学: BindGroup是GPU资源的组织方式
// - binding(0): Camera uniform - 包含视口信息，用于坐标转换
// - binding(1): 精灵图集纹理 - R8格式，存储调色板索引
// - binding(2): 调色板纹理 - RGBA格式，256x64，每行一个调色板状态
// - binding(3): 采样器 - 使用Nearest过滤（像素风格）
// ----------------------------------------------------------------------------

struct CameraUniform {
    view_offset: vec2<f32>,  // 视口偏移（世界坐标，当前未使用）
    screen_size: vec2<f32>,  // 屏幕尺寸（像素），用于NDC转换
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var palette_texture: texture_2d<f32>;
@group(0) @binding(3) var tex_sampler: sampler;

// ----------------------------------------------------------------------------
// 实例数据结构 (Instance Data)
// 
// wgpu教学: 实例化渲染的核心 - 每个精灵的独立参数
// 这些数据通过顶点缓冲区以Instance模式传入（step_mode = Instance）
// 每个精灵使用6个顶点（2个三角形），但共享同一份实例数据
// ----------------------------------------------------------------------------

struct SpriteInstance {
    @location(0) position: vec2<f32>,       // 屏幕位置（像素）
    @location(1) size: vec2<f32>,           // 精灵尺寸（像素）
    @location(2) uv_offset: vec2<f32>,      // 图集中的UV起始坐标（归一化）
    @location(3) uv_size: vec2<f32>,        // UV尺寸（归一化）
    @location(4) flip: vec2<f32>,           // 翻转标志: x=水平, y=垂直
    @location(5) palette_offset: f32,       // 调色板偏移（用于颜色变换效果）
    @location(6) palette_index: f32,        // 调色板行索引（用于多调色板支持）
    @location(7) opaque: f32,               // 不透明标志: 1=索引0也绘制
    @location(8) rotation: f32,             // 旋转: 0/1/2/3 = 0/90/180/270度
}

// 顶点着色器输出 / 片段着色器输入
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,  // 裁剪空间坐标
    @location(0) uv: vec2<f32>,                   // 纹理坐标
    @location(1) palette_offset: f32,             // 调色板偏移
    @location(2) palette_index: f32,              // 调色板索引
    @location(3) opaque: f32,                     // 不透明标志
}

// ============================================================================
// 顶点着色器
// 
// wgpu教学: 
// - @builtin(vertex_index): GPU自动提供的顶点索引(0-5)
// - 我们用6个顶点组成2个三角形，形成一个四边形
// - NDC坐标系: x/y范围是[-1, 1]，左下角是(-1,-1)，右上角是(1,1)
// - 注意: wgpu的Y轴向下，所以我们翻转了Y坐标 (-ndc.y)
// ============================================================================

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, instance: SpriteInstance) -> VertexOutput {
    // 四边形的6个顶点位置（2个三角形）
    // 顺序: 左上, 右上, 左下, 右上, 右下, 左下
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0)
    );
    let pos = positions[vertex_index];
    
    // 计算屏幕位置（像素坐标）
    let screen_pos = instance.position + pos * instance.size;
    
    // 转换为NDC坐标（Normalized Device Coordinates）
    // 公式: ndc = (screen_pos / screen_size) * 2.0 - 1.0
    // 将[0, screen_size]映射到[-1, 1]
    let ndc = (screen_pos / camera.screen_size) * 2.0 - 1.0;
    
    // 计算UV坐标（考虑旋转和翻转）
    // 约定: 先旋转(0/90/180/270)，再翻转
    var uv = pos;
    
    // 旋转处理 (r: 0=0度, 1=90度, 2=180度, 3=270度)
    let r = u32(instance.rotation + 0.5) & 3u;
    if (r == 1u) {
        uv = vec2<f32>(uv.y, 1.0 - uv.x);       // 顺时针90度
    } else if (r == 2u) {
        uv = vec2<f32>(1.0 - uv.x, 1.0 - uv.y); // 180度
    } else if (r == 3u) {
        uv = vec2<f32>(1.0 - uv.y, uv.x);       // 顺时针270度
    }
    
    // 翻转处理
    if (instance.flip.x > 0.5) { uv.x = 1.0 - uv.x; }  // 水平翻转
    if (instance.flip.y > 0.5) { uv.y = 1.0 - uv.y; }  // 垂直翻转
    
    // 应用UV偏移和缩放（定位到图集中的具体精灵）
    uv = instance.uv_offset + uv * instance.uv_size;
    
    var output: VertexOutput;
    output.clip_position = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);  // Y轴翻转
    output.uv = uv;
    output.palette_offset = instance.palette_offset;
    output.palette_index = instance.palette_index;
    output.opaque = instance.opaque;
    return output;
}

// ============================================================================
// 片段着色器
// 
// wgpu教学:
// - textureSample: 使用采样器采样纹理，支持过滤和寻址模式
// - textureLoad: 直接读取纹理像素，不使用采样器
// - discard: 丢弃当前片段，不写入颜色缓冲区（实现透明）
// 
// 索引调色板技术:
// 1. 图集纹理(R8格式)存储的是调色板索引(0-255)
// 2. 调色板纹理(RGBA8格式)存储256种颜色
// 3. 用图集采样得到的索引，去调色板中查找真实颜色
// 
// 优点: 
// - 可以通过改变调色板实现淡入淡出、闪烁等效果
// - 图集只需要1字节/像素，节省显存
// ============================================================================

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 步骤1: 从精灵图集采样调色板索引
    // R8格式纹理，r通道存储0-1的值，乘以255得到索引
    let index_color = textureSample(atlas_texture, tex_sampler, in.uv);
    let raw_idx = i32(index_color.r * 255.0 + 0.5);
    
    // 步骤2: 透明度处理
    // 重要: 必须用原始索引判断透明，在调色板偏移之前！
    // 否则palette_offset会把0偏移成非0，导致本该透明的像素变成不透明
    if (raw_idx == 0 && in.opaque < 0.5) { 
        discard;  // 索引0为透明色（除非opaque标志为1）
    }

    // 步骤3: 应用调色板偏移（用于颜色变换效果，如无敌闪烁）
    var palette_i = raw_idx;
    if (raw_idx != 0) {
        let off = i32(in.palette_offset);
        palette_i = (raw_idx + off) % 256;
        if (palette_i < 0) { palette_i = palette_i + 256; }
    }
    let palette_idx = u32(palette_i);
    
    // 步骤4: 从调色板纹理查找最终颜色
    // 使用textureLoad直接读取，避免采样器插值
    let palette_row = u32(in.palette_index);
    let color = textureLoad(palette_texture, vec2<i32>(i32(palette_idx), i32(palette_row)), 0);
    
    return color;
}
