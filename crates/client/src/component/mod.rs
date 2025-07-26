mod bullet;
mod camera;
mod character;
mod damage_font;
mod deferred;
mod fx;
mod hierarchy;
mod light;
mod material;
mod mesh;
mod query;
mod skybox;
mod stage;
mod transform;
mod ui;

use std::sync::Arc;

use ahash::HashMap;

pub use self::{
    bullet::*, camera::*, character::*, damage_font::*, deferred::*, fx::*, hierarchy::*, light::*,
    material::*, mesh::*, query::*, skybox::*, stage::*, transform::*, ui::*,
};

#[derive(Debug, Clone)]
pub enum MeshFilter {
    Mesh(MeshResource),
    SkinnedMesh(SkinnedMeshResource),
}

impl MeshFilter {
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        match self {
            MeshFilter::Mesh(resource) => resource.bind_group(),
            MeshFilter::SkinnedMesh(resource) => resource.bind_group(),
        }
    }
}

/// 그리기 작업
pub struct RenderTask {
    pub mesh: Arc<Mesh>,
    pub mesh_resource: MeshFilter,
    pub material_index: usize,
    pub material_resource: MaterialResource,
}

pub type BakeList = Vec<(Arc<ShadowResource>, ShadowMap)>;
pub type TransformMap = HashMap<usize, Vec<MeshFilter>>;
pub type ShadowMap = HashMap<(Arc<Mesh>, MaterialKind), TransformMap>;
pub type MaterialMap = HashMap<(usize, MaterialResource), Vec<MeshFilter>>;
pub type OpaqueMap = HashMap<(Arc<Mesh>, MaterialKind), MaterialMap>;
pub type TransparentMap = HashMap<(Arc<Mesh>, MaterialKind), MaterialMap>;
pub type MeshRenderer<'a> = (
    &'a Arc<Mesh>,
    &'a MeshResource,
    &'a TransformUniform,
    &'a Vec<MaterialResource>,
);
pub type SkinnedMeshRenderer<'a> = (
    &'a Arc<Mesh>,
    &'a SkinnedMeshResource,
    &'a BoneCollection,
    &'a BoneTransformUniform,
    &'a Vec<MaterialResource>,
);
