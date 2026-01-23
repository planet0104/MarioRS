// ============================================================================
// GPU缓冲区池 (Buffer Pool)
// ============================================================================
//
// wgpu教学: 缓冲区管理策略
//
// 为什么需要缓冲区池?
// 1. 避免每帧调用 create_buffer()，减少CPU/GPU同步开销
// 2. 减少内存分配和释放的性能损耗
// 3. 更好的内存局部性，可能提升缓存命中率
//
// 本模块提供预分配缓冲区的管理功能:
// - 初始容量创建
// - 动态扩容（当数据量超过容量时）
// - 数据上传
//
// 使用模式:
// 1. 初始化时预分配合理容量的缓冲区
// 2. 每帧检查是否需要扩容
// 3. 使用 queue.write_buffer() 上传数据
// 4. 渲染时使用 buffer.slice(..)
//
// ============================================================================

use crate::gpu::types::{FillRect, SpriteInstance};

// ============================================================================
// 缓冲区池初始容量常量
//
// wgpu教学: 容量设计原则
// - 根据典型场景设置合理的初始值
// - 过小会导致频繁扩容
// - 过大会浪费GPU显存
// ============================================================================

/// 精灵缓冲区初始容量 - 2048个精灵实例
pub const INITIAL_SPRITE_CAPACITY: usize = 2048;

/// 填充矩形缓冲区初始容量 - 256个矩形
pub const INITIAL_FILL_CAPACITY: usize = 256;

/// UI填充缓冲区初始容量 - 128个矩形
pub const INITIAL_UI_FILL_CAPACITY: usize = 128;

// ============================================================================
// 缓冲区池结构体
//
// wgpu教学: 缓冲区池封装
// 将缓冲区和容量信息封装在一起，便于管理
// ============================================================================

/// 精灵缓冲区池
pub struct SpriteBufferPool {
    /// 预分配的GPU缓冲区
    pub buffer: wgpu::Buffer,
    /// 当前容量（实例数量）
    pub capacity: usize,
}

impl SpriteBufferPool {
    /// 创建新的精灵缓冲区池
    ///
    /// wgpu教学: 缓冲区创建参数
    /// - label: 调试标签，便于在GPU调试工具中识别
    /// - size: 缓冲区大小（字节）
    /// - usage: 缓冲区用途，这里是 VERTEX（顶点缓冲区）+ COPY_DST（可写入）
    /// - mapped_at_creation: false 表示创建时不映射内存
    pub fn new(device: &wgpu::Device, initial_capacity: usize) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_buffer_pooled"),
            size: (initial_capacity * std::mem::size_of::<SpriteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            capacity: initial_capacity,
        }
    }

    /// 确保容量足够，必要时扩容
    ///
    /// wgpu教学: 动态扩容策略
    /// 1. 当需求容量超过当前容量时触发扩容
    /// 2. 新容量使用 next_power_of_two 确保是2的幂（优化对齐）
    /// 3. 创建新缓冲区，旧缓冲区会被自动释放
    ///
    /// 注意: 扩容有性能开销，应避免频繁触发
    pub fn ensure_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required > self.capacity {
            let new_capacity = required.next_power_of_two().max(64);
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite_buffer_pooled"),
                size: (new_capacity * std::mem::size_of::<SpriteInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.capacity = new_capacity;
        }
    }

    /// 上传数据到缓冲区
    ///
    /// wgpu教学: write_buffer
    /// - 这是更新已存在缓冲区数据的高效方式
    /// - 数据会被复制到GPU显存
    /// - 比每帧创建新缓冲区更高效
    pub fn write(&self, queue: &wgpu::Queue, data: &[SpriteInstance]) {
        if !data.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
        }
    }
}

/// 填充矩形缓冲区池
pub struct FillBufferPool {
    /// 预分配的GPU缓冲区
    pub buffer: wgpu::Buffer,
    /// 当前容量（矩形数量）
    pub capacity: usize,
}

impl FillBufferPool {
    /// 创建新的填充缓冲区池
    pub fn new(device: &wgpu::Device, initial_capacity: usize) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fill_buffer_pooled"),
            size: (initial_capacity * std::mem::size_of::<FillRect>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            capacity: initial_capacity,
        }
    }

    /// 确保容量足够，必要时扩容
    pub fn ensure_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required > self.capacity {
            let new_capacity = required.next_power_of_two().max(64);
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("fill_buffer_pooled"),
                size: (new_capacity * std::mem::size_of::<FillRect>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.capacity = new_capacity;
        }
    }

    /// 上传数据到缓冲区
    pub fn write(&self, queue: &wgpu::Queue, data: &[FillRect]) {
        if !data.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
        }
    }
}

// ============================================================================
// 缓冲区池管理器
//
// wgpu教学: 统一管理所有缓冲区池
// 将所有预分配缓冲区集中管理，简化渲染器代码
// ============================================================================

/// 缓冲区池管理器 - 统一管理所有预分配的GPU缓冲区
pub struct BufferPoolManager {
    /// 精灵实例缓冲区池
    pub sprites: SpriteBufferPool,
    /// 填充矩形缓冲区池（背景层）
    pub fills: FillBufferPool,
    /// UI填充矩形缓冲区池（UI层）
    pub ui_fills: FillBufferPool,
}

impl BufferPoolManager {
    /// 创建缓冲区池管理器
    ///
    /// 使用默认的初始容量创建所有缓冲区池
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            sprites: SpriteBufferPool::new(device, INITIAL_SPRITE_CAPACITY),
            fills: FillBufferPool::new(device, INITIAL_FILL_CAPACITY),
            ui_fills: FillBufferPool::new(device, INITIAL_UI_FILL_CAPACITY),
        }
    }

    /// 准备帧数据 - 确保容量并上传数据
    ///
    /// wgpu教学: 帧数据准备流程
    /// 1. 检查并确保所有缓冲区容量足够
    /// 2. 上传本帧的渲染数据到GPU
    ///
    /// 这个方法应该在render_frame之前调用
    pub fn prepare_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sprites: &[SpriteInstance],
        fills: &[FillRect],
        ui_fills: &[FillRect],
    ) {
        // 确保容量（至少为1，避免0大小缓冲区）
        self.sprites.ensure_capacity(device, sprites.len().max(1));
        self.fills.ensure_capacity(device, fills.len().max(1));
        self.ui_fills.ensure_capacity(device, ui_fills.len().max(1));

        // 上传数据
        self.sprites.write(queue, sprites);
        self.fills.write(queue, fills);
        self.ui_fills.write(queue, ui_fills);
    }
}

// ============================================================================
// 辅助函数 - 用于直接在GpuRenderer中使用
// ============================================================================

/// 创建精灵缓冲区（预分配）
pub fn create_sprite_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sprite_buffer_pooled"),
        size: (capacity * std::mem::size_of::<SpriteInstance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

/// 创建填充缓冲区（预分配）
pub fn create_fill_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("fill_buffer_pooled"),
        size: (capacity * std::mem::size_of::<FillRect>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
