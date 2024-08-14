mod buffer;
pub use self::buffer::*;

mod layout;
pub use self::layout::*;

mod transform;
pub use self::transform::*;

use std::sync::Arc;
use std::sync::OnceLock;
use hecs::Entity;



/// 3차원 오브젝트를 나타내는 데이터입니다.
#[derive(Debug, Clone)]
pub struct GameObject {
    pub parent: Entity, 
    pub children: Vec<Entity>, 

    name: String, 
    buffer: Arc<GameObjectBuffer>, 
    bind_group: Arc<wgpu::BindGroup>, 
}

impl GameObject {
    /// 3차원 오브젝트의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(GameObject)"), 
                    entries: &[
                        // 0번 바인딩: 오브젝트 데이터 유니폼 버퍼
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None 
                            }, 
                            count: None,
                        }
                    ]
                }
            )
        })
    }
}

impl GameObject {
    /// 새로운 게임 오브젝트 데이터를 생성합니다.
    #[must_use]
    pub fn new(name: Option<&str>, device: &wgpu::Device) -> Self {
        // 디버깅 라벨을 생성합니다.
        let name = format!("GameObject({})", name.unwrap_or("Unknown"));

        // 유니폼 버퍼를 생성합니다.
        let buffer = GameObjectBuffer::new(Some(&format!("Uniform({})", name)), device);

        // 바인드 그룹을 생성합니다.
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", name)), 
                layout: &Self::layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: buffer.as_entire_binding(),
                    }, 
                ], 
            }
        ).into();

        Self { 
            parent: Entity::DANGLING,  
            children: Vec::with_capacity(8), 
            name, 
            buffer, 
            bind_group 
        }
    }

    /// 유니폼 버퍼를 갱신합니다.
    pub fn update(&self, data: GameObjectDataLayout) {
        let name = self.name.clone();
        let capturable = self.buffer.clone();
        self.buffer.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            if result.is_ok() {
                let mut view = capturable.slice(..).get_mapped_range_mut();
                let layout: &mut GameObjectDataLayout = bytemuck::from_bytes_mut(&mut view);
                *layout = data;
                drop(view);
                capturable.unmap();
            } else {
                log::warn!("Failed to write uniform buffer! (name: {})", name);
            }
        });
    }
}

impl GameObject {
    /// 3차원 오브젝트의 이름을 반환합니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 3차원 오브젝트의 유니폼 버퍼를 반환합니다.
    #[inline]
    #[must_use]
    pub fn buffer(&self) -> Arc<GameObjectBuffer> {
        self.buffer.clone()
    }

    /// 3차원 오브젝트의 [wgpu::BindGroup]을 반환합니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> Arc<wgpu::BindGroup> {
        self.bind_group.clone()
    }
}
