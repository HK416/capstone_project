use std::{
    collections::HashMap, 
    sync::Arc
};

use crate::render::{
    mesh::{
        BoneUniform, 
        MeshUniform, 
        NonSkinnedMesh, 
        SkinnedMesh, 
        SkinnedMeshUniform, 
    }, 
    pool::MeshPool
};

use super::{
    Attribute, 
    Indices, 
    MeshRenderer, 
    VertexAttributeValues, 
    Vertices
};



/// 메쉬를 생성하는 빌더입니다.
#[derive(Debug, Clone)]
pub struct MeshBuilder {
    /// 메쉬의 이름입니다.
    pub(crate) name: String, 

    /// 메쉬의 정점 데이터입니다.
    pub(crate) vertices: Vertices, 

    /// 메쉬의 정점 속성 데이터입니다.
    pub(crate) attributes: HashMap<Attribute, VertexAttributeValues>, 

    /// 메쉬의 하위 메쉬 데이터입니다.
    pub(crate) submeshes: Vec<Indices>, 

    /// 메쉬의 바인드 포즈 데이터입니다.
    pub(crate) bindposes: Vec<gmm::Float4x4>, 
}

impl MeshBuilder {
    /// 새로운 메쉬 빌더를 생성합니다.
    /// 
    /// # Panics
    /// 주어진 정점 데이터가 비어있는 경우 `panic!`을 호출합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn new<N, V>(name: N, values: V) -> Self 
    where 
        N: Into<String>, 
        V: IntoIterator<Item = gmm::Float3>, 
        V::IntoIter: ExactSizeIterator,
    {
        let name = name.into();
        let vertices = Vertices(values.into_iter().collect());
        assert!(!vertices.is_empty(), "The given vertex data is empty!");

        Self { 
            name, 
            vertices, 
            attributes: HashMap::with_capacity(8), 
            submeshes: Vec::with_capacity(8), 
            bindposes: Vec::new() 
        }
    }

    /// 메쉬 빌더에 정점 속성을 데이터를 추가합니다.
    /// 이미 해당 정점 속성 데이터가 존재할 경우 데이터를 덮어씁니다.
    /// 
    /// # Panics
    /// 주어진 정점 속성 데이터가 비어있는 경우 `panic!`을 호출합니다.
    /// 
    #[must_use]
    pub fn with_attribute(mut self, values: VertexAttributeValues) -> Self {
        assert!(!values.is_empty(), "The given attribute data is empty!");
        self.attributes.insert(values.attribute(), values);
        self
    }

    /// 메쉬 빌더에 하위 메쉬 데이터를 추가합니다.
    /// 
    /// # Panics
    /// 주어진 하위 메쉬 데이터가 비어있는 경우 `panic!`을 호출합니다.
    ///  
    #[must_use]
    pub fn with_submesh(mut self, values: Indices) -> Self {
        assert!(!values.is_empty(), "The given submesh data is empty!");
        self.submeshes.push(values);
        self
    }

    /// 메쉬 빌더에 바인드 포즈 데이터를 추가합니다.
    /// 이미 바인드 포즈 데이터가 존재할 경우 데이터를 덮어씁니다.
    /// 
    /// # Panics
    /// 주어진 바인드 포즈 데이터가 비어있는 경우 `panic!`을 호출합니다.
    /// 
    #[must_use]
    pub fn with_bindposes<I>(mut self, bindposes: I) -> Self 
    where I: IntoIterator<Item = gmm::Float4x4>, I::IntoIter: ExactSizeIterator {
        let bindposes: Vec<_> = bindposes.into_iter().collect();
        assert!(!bindposes.is_empty(), "The given bindpose data is empty!");
        self.bindposes = bindposes;
        self
    }

    /// 메쉬 빌더로부터 메쉬를 생성합니다.
    #[must_use]
    pub fn build(self, device: &Arc<wgpu::Device>, queue: &Arc<wgpu::Queue>) -> MeshRenderer {
        let non_skinned = self.bindposes.is_empty()
            | self.attributes.get(&Attribute::BoneIndices).is_none()
            | self.attributes.get(&Attribute::BoneIndices).is_none();
        
        if non_skinned {
            log::info!("스키닝되지 않은 메쉬({})를 생성합니다.", &self.name);
            let mesh_uniform = MeshUniform::new(Some(&format!("MeshUniform({})", &self.name)), device);
            let bind_group = device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    label: Some(&format!("BindGroup({})", &self.name)), 
                    layout: &NonSkinnedMesh::bind_group_layout(device), 
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0, 
                            resource: mesh_uniform.as_entire_binding(), 
                        }, 
                    ]
                }
            );

            MeshRenderer::NonSkinnedMesh(
                NonSkinnedMesh {
                    model_mesh: MeshPool::get_or_init(device, queue, self), 
                    mesh_uniform, 
                    bind_group
                }.into()
            )
        } else {
            log::info!("스키닝된 메쉬({})를 생성합니다.", &self.name);
            let skinned_mesh_uniform = SkinnedMeshUniform::new(Some(&format!("SkinnedMeshUniform({})", &self.name)), device);
            let bindpose_uniform = BoneUniform::new(Some(&format!("BindposeUniform({})", &self.name)), device);
            let bone_transforms_uniform = BoneUniform::new(Some(&format!("BoneTransformsUniform({})", &self.name)), device);
            let bind_group = device.create_bind_group(
                &wgpu::BindGroupDescriptor {
                    label: Some(&format!("BindGroup({})", &self.name)), 
                    layout: &SkinnedMesh::bind_group_layout(device), 
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0, 
                            resource: skinned_mesh_uniform.as_entire_binding(), 
                        }, 
                        wgpu::BindGroupEntry {
                            binding: 1, 
                            resource: bindpose_uniform.as_entire_binding(), 
                        }, 
                        wgpu::BindGroupEntry {
                            binding: 2, 
                            resource: bone_transforms_uniform.as_entire_binding(), 
                        }, 
                    ]
                }
            );

            MeshRenderer::SkinnedMesh(
                SkinnedMesh {
                    model_mesh: MeshPool::get_or_init(device, queue, self), 
                    skinned_mesh_uniform,
                    bindpose_uniform, 
                    bone_transforms_uniform, 
                    bind_group
                }.into()
            )
        }
    }
}
