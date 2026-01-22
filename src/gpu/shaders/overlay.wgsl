// ============================================================================
// 叠加层着色器 (Overlay Shader)
// ============================================================================
//
// 功能: 在游戏画面上叠加UI元素（如Android触摸面板、FPS显示等）
//
// wgpu/WGSL 教学要点:
// 1. Alpha混合: 使用透明度混合叠加层和背景
// 2. 多Pass渲染: 这是渲染流程的最后一个Pass
// 3. RGBA纹理: 不使用调色板，直接使用RGBA颜色
//
// 渲染顺序:
// ┌─────────────────────────────────────────────────────────────────────┐
// │ Pass 1: 游戏渲染 -> render_texture                                  │
// │ Pass 2: 缩放输出 -> window surface (LoadOp::Clear)                  │
// │ Pass 3: 叠加层   -> window surface (LoadOp::Load, Alpha Blending)   │
// └─────────────────────────────────────────────────────────────────────┘
//
// 与scale.wgsl的区别:
// - scale.wgsl: Clear背景，输出游戏画面
// - overlay.wgsl: Load背景（保留之前的内容），Alpha混合叠加
// ============================================================================

// 绑定组: 叠加层纹理 + 采样器
@group(0) @binding(0) var overlay_texture: texture_2d<f32>;
@group(0) @binding(1) var tex_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// ============================================================================
// 顶点着色器
// 
// 覆盖整个窗口的全屏四边形
// 注意: 不使用scale/offset，因为叠加层本身就是窗口尺寸
// ============================================================================

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // 全屏四边形顶点
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0)
    );
    let pos = positions[vertex_index];

    var output: VertexOutput;
    output.clip_position = vec4<f32>(pos, 0.0, 1.0);
    
    // 计算UV坐标
    output.uv = (pos + 1.0) * 0.5;
    output.uv.y = 1.0 - output.uv.y;
    
    return output;
}

// ============================================================================
// 片段着色器
// 
// 直接采样叠加层纹理
// Alpha混合在RenderPipeline中配置（blend: ALPHA_BLENDING）
// 
// wgpu教学: Alpha混合公式
// final_color = src_color * src_alpha + dst_color * (1 - src_alpha)
// 其中dst_color是之前渲染的游戏画面
// ============================================================================

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(overlay_texture, tex_sampler, in.uv);
}
