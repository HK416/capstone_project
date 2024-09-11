mod buffer;
pub use self::buffer::*;

mod layout;
pub use self::layout::*;

mod transform;
pub use self::transform::*;

use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering as MemOrdering;
use hecs::Entity;
use hecs::World;



/// 오브젝트 컴포넌트 타입입니다.
pub type GameObjectComponent = Arc<GameObject>;

/// 3차원 오브젝트를 나타내는 데이터입니다.
#[derive(Debug)]
pub struct GameObject {
    parent: AtomicU64, 
    sibling: AtomicU64, 
    child: AtomicU64, 

    name: String, 
    buffer: Arc<GameObjectBuffer>, 
    bind_group: wgpu::BindGroup, 
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
    pub fn new(name: Option<&str>, device: &wgpu::Device) -> GameObjectComponent {
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
        );

        Self { 
            parent: AtomicU64::new(0), 
            sibling: AtomicU64::new(0), 
            child: AtomicU64::new(0), 
            name, 
            buffer, 
            bind_group 
        }.into()
    }

    /// 유니폼 버퍼를 갱신합니다.
    pub fn update(&self, queue: &wgpu::Queue, data: GameObjectDataLayout) {
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
        queue.submit([]);
    }
}

impl GameObject {
    /// 부모 오브젝트 엔티티를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_parent(&self) -> Option<Entity> {
        Entity::from_bits(self.parent.load(MemOrdering::Acquire))
    }

    /// 형제 오브젝트 엔티티를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_sibling(&self) -> Option<Entity> {
        Entity::from_bits(self.sibling.load(MemOrdering::Acquire))
    }

    /// 자식 오브젝트 엔티티를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_child(&self) -> Option<Entity> {
        Entity::from_bits(self.child.load(MemOrdering::Acquire))
    }

    /// 부모 오브젝트 엔티티를 설정합니다.
    #[inline]
    pub fn set_parent(&self, entity: Entity) {
        self.parent.store(entity.to_bits().get(), MemOrdering::Release);
    }

    /// 형제 오브젝트 엔티티를 설정합니다.
    #[inline]
    pub fn set_sibling(&self, entity: Entity) {
        self.sibling.store(entity.to_bits().get(), MemOrdering::Release);
    }

    /// 자식 오브젝트 엔티티를 설정합니다.
    #[inline]
    pub fn set_child(&self, entity: Entity) {
        self.child.store(entity.to_bits().get(), MemOrdering::Release);
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
    pub fn buffer(&self) -> &GameObjectBuffer {
        &self.buffer
    }

    /// 3차원 오브젝트의 [wgpu::BindGroup]을 반환합니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}



macro_rules! ok_or_return {
    ($result:expr) => {
        match ($result) {
            Ok(it) => it, 
            _ => return,
        }
    };
}

/// 오브젝트 계층 구조를 갱신합니다.
pub fn update_hierarchy(world: &mut World, parent: Option<gmm::Matrix>, entity: Entity) {
    // 현제 엔티티의 오브젝트 컴포넌트를 가져옵니다.
    let game_object = (*ok_or_return!(world.get::<&GameObjectComponent>(entity))).clone();
    
    // 현제 엔티티의 월드 변환 행렬을 갱신합니다.
    if let Some(parent) = parent {
        let transform = *ok_or_return!(world.get::<&Transform>(entity));
        let world_transform = ok_or_return!(world.query_one_mut::<&mut WorldTransform>(entity));
        (**world_transform) = parent * (*transform);
    }

    // 형제 엔티티의 월드 변환 행렬을 갱신합니다.
    if let Some(sibling_entity) = game_object.get_sibling() {
        update_hierarchy(world, parent, sibling_entity);
    }

    // 자식 엔티티의 월드 변환 행렬을 갱신합니다.
    if let Some(child_entity) = game_object.get_child() {
        let world_transform = **ok_or_return!(world.get::<&WorldTransform>(entity));
        update_hierarchy(world, Some(world_transform), child_entity);
    }
}

/// 오브젝트 계층 구조를 정리합니다.
pub fn cleanup_hierarchy(world: &mut World, entity: Entity) {
    // 현제 엔티티의 오브젝트 컴포넌트를 가져옵니다.
    let game_object = (*ok_or_return!(world.get::<&GameObjectComponent>(entity))).clone();
    
    if let Some(sibling_entity) = game_object.get_sibling() {
        cleanup_hierarchy(world, sibling_entity);
    }

    if let Some(child_entity) = game_object.get_child() {
        cleanup_hierarchy(world, child_entity);
    }

    world.despawn(entity).unwrap();
}
