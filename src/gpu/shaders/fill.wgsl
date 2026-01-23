// ============================================================================
// 填充矩形着色器 (Fill Shader)
// ============================================================================
//
// 功能: 绘制纯色填充矩形，用于天空、背景、UI等
//
// wgpu/WGSL 教学要点:
// 1. 简化的实例化渲染: 每个矩形只需要位置、大小和颜色索引
// 2. 调色板查找: 复用精灵着色器的调色板机制
// 3. 无纹理采样: 只使用调色板纹理，不需要精灵图集
//
// 与sprite.wgsl的区别:
// - 更少的实例属性（无UV、翻转、旋转）
// - 更少的绑定资源（无图集纹理、无采样器）
// - 更简单的片段着色器（直接查调色板）
// ============================================================================

// ----------------------------------------------------------------------------
// Uniform 绑定组 (Bind Group 0)
// 
// 注意: 填充着色器只需要Camera和Palette，不需要图集纹理
// ----------------------------------------------------------------------------

struct CameraUniform {
    view_offset: vec2<f32>,  // 视口偏移（当前未使用）
    screen_size: vec2<f32>,  // 屏幕尺寸（像素）
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var palette_texture: texture_2d<f32>;

// ----------------------------------------------------------------------------
// 填充矩形实例数据
// 
// 比精灵实例简单得多，只需要4个属性
// ----------------------------------------------------------------------------

struct FillInstance {
    @location(0) position: vec2<f32>,    // 屏幕位置（像素）
    @location(1) size: vec2<f32>,        // 矩形尺寸（像素）
    @location(2) color_index: f32,       // 调色板颜色索引(0-255)
    @location(3) palette_index: f32,     // 调色板行索引
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color_index: f32,
    @location(1) palette_index: f32,
}

// ============================================================================
// 顶点着色器
// 
// 与sprite.wgsl类似，但更简单:
// - 不需要处理UV坐标
// - 不需要处理翻转和旋转
// ============================================================================

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, instance: FillInstance) -> VertexOutput {
    // 四边形顶点（2个三角形）
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0)
    );
    let pos = positions[vertex_index];
    
    // 屏幕坐标 -> NDC坐标
    let screen_pos = instance.position + pos * instance.size;
    let ndc = (screen_pos / camera.screen_size) * 2.0 - 1.0;
    
    var output: VertexOutput;
    output.clip_position = vec4<f32>(ndc.x, -ndc.y, 0.0, 1.0);
    output.color_index = instance.color_index;
    output.palette_index = instance.palette_index;
    return output;
}

// ============================================================================
// 片段着色器
// 
// 非常简单: 直接用颜色索引查找调色板
// 不需要透明度处理（填充矩形总是不透明的）
// ============================================================================

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 直接使用颜色索引查找调色板
    let palette_idx = u32(in.color_index) % 256u;
    let palette_row = u32(in.palette_index);
    let color = textureLoad(palette_texture, vec2<i32>(i32(palette_idx), i32(palette_row)), 0);
    return color;
}
