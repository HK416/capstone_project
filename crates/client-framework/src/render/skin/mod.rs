mod buffer;
pub use self::buffer::*;

mod layout;
pub use self::layout::*;

use std::sync::Arc;
use std::sync::OnceLock;
use hecs::Entity;

use crate::render::mesh::Mesh;



/// 스키닝 컴포넌트 타입입니다.
pub type SkinComponent = Arc<Skin>;

/// 3차원 메쉬의 스키닝 데이터입니다.
#[derive(Debug)]
pub struct Skin {
    mesh: Arc<Mesh>, 
    root_bone: Entity, 
    bones: Vec<Entity>, 
    bone_matrix_buffer: Arc<BoneMatrixBuffer>, 
    bind_group: wgpu::BindGroup, 
}

impl Skin {
    /// 스키닝의 [wgpu::BindGroupLayout]을 반환합니다.
    pub fn layout(device: &wgpu::Device) -> &'static wgpu::BindGroupLayout {
        static LAYOUT: OnceLock<wgpu::BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: Some("BindGroupLayout(Skinning)"), 
                    entries: &[
                        // 0번 바인딩: 뼈 데이터
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, 
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None, 
                            }, 
                            count: None, 
                        }, 
                        // 1번 바인딩: 현재 뼈 애니메이션 변환 행렬
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, 
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None, 
                            }, 
                            count: None, 
                        }, 
                        // 2번 바인딩: 뼈 바인드 포즈의 역행렬
                        wgpu::BindGroupLayoutEntry {
                            binding: 2, 
                            visibility: wgpu::ShaderStages::VERTEX, 
                            ty: wgpu::BindingType::Buffer { 
                                ty: wgpu::BufferBindingType::Uniform, 
                                has_dynamic_offset: false, 
                                min_binding_size: None, 
                            }, 
                            count: None, 
                        }, 
                    ]
                }
            )
        })
    }
}

impl Skin {
    /// 새로운 스키닝 데이터를 생성합니다.
    /// 
    /// # Panics
    /// 주어진 메쉬에 바인딩 포즈 버퍼가 없는 경우 [`panic!`]을 호출합니다.
    /// 
    #[must_use]
    pub fn new<I>(
        mesh: Arc<Mesh>, 
        root_bone: Entity, 
        bones: I, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        bone_data: BoneDataLayout
    ) -> SkinComponent 
    where 
        I: IntoIterator<Item = Entity>, 
        I::IntoIter: ExactSizeIterator 
    {
        // 디버깅 라벨을 생성합니다.
        let name = format!("Skin({})", mesh.name());

        // 뼈 정보 유니폼 버퍼를 생성합니다. 
        let bone_data = BoneBuffer::from_data(
            Some(&format!("Uniform({})", &name)), 
            device, 
            queue, 
            bone_data
        );

        // 현재 애니메이션의 뼈 변환 행렬 유니폼 버퍼를 생성합니다.
        let bone_matrix_buffer = BoneMatrixBuffer::new(
            Some(&format!("Uniform({})", &name)), 
            device
        );

        // 초기 뼈의 위치 변환 행렬 유니폼 버퍼를 가져옵니다.
        let bindposes = mesh.bindpose()
            .expect("The given mesh is not a skinned mesh!");

        // 바인드 그룹을 생성합니다.
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", &name)), 
                layout: &Self::layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: bone_data.as_entire_binding(), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: bone_matrix_buffer.as_entire_binding(), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 2, 
                        resource: bindposes.as_entire_binding(), 
                    }, 
                ],
            }, 
        ).into();

        Self { 
            mesh, 
            root_bone, 
            bones: bones.into_iter().collect(), 
            bone_matrix_buffer, 
            bind_group 
        }.into()
    }

    // 뼈 애니메이션 변환을 갱신합니다.
    pub fn update(&self, queue: &wgpu::Queue, data: BoneMatrixDataLayout) {
        let name = self.mesh.name().to_string();
        let capturable = self.bone_matrix_buffer.clone();
        self.bone_matrix_buffer.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            if result.is_ok() {
                let mut view = capturable.slice(..).get_mapped_range_mut();
                let layout: &mut BoneMatrixDataLayout = bytemuck::from_bytes_mut(&mut view);
                *layout = data;
                drop(view);
                capturable.unmap();
            } else {
                log::warn!("Failed to write uniform buffer! (name: Skin({}))", name);
            }
        });
        queue.submit([]);
    }
}

impl Skin {
    /// 컴포넌트에 연결된 메쉬를 반환합니다.
    #[inline]
    #[must_use]
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    /// 최상위 뼈 엔티티를 반환합니다.
    #[inline]
    #[must_use]
    pub fn root_bone(&self) -> &Entity {
        &self.root_bone
    }

    /// 뼈 엔티티를 반환합니다.
    #[inline]
    #[must_use]
    pub fn bones(&self) -> &[Entity] {
        &self.bones
    }

    /// 컴포넌트의 유니폼 버퍼를 반환합니다.
    #[inline]
    #[must_use]
    pub fn bone_matrix_buffer(&self) -> &BoneMatrixBuffer {
        &self.bone_matrix_buffer
    }

    /// 컴포넌트의 [wgpu::BindGroup]을 반환합니다.
    #[inline]
    #[must_use]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
