mod bullet;
mod camera;
mod capture_zone;
mod character;
mod control;
mod damage_font;
mod hierarchy;
mod light;
mod material;
mod mesh;
mod skybox;
mod stage;
mod transform;
mod weighted_blended_oit;

use std::sync::Arc;

use ahash::HashMap;

pub use self::{
    bullet::*, camera::*, capture_zone::*, character::*, control::*, damage_font::*, hierarchy::*,
    light::*, material::*, mesh::*, skybox::*, stage::*, transform::*, weighted_blended_oit::*,
};

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

pub type BakeList = Vec<ShadowResource>;
pub type ShadowMap = HashMap<(Arc<Mesh>, MaterialKind), HashMap<usize, Vec<MeshFilter>>>;
pub type OpaqueMap =
    HashMap<(Arc<Mesh>, MaterialKind), HashMap<(usize, MaterialResource), Vec<MeshFilter>>>;
pub type TransparentMap =
    HashMap<(Arc<Mesh>, MaterialKind), HashMap<(usize, MaterialResource), Vec<MeshFilter>>>;
pub type MeshRenderer<'a> = (
    &'a Arc<Mesh>,
    &'a MeshResource,
    &'a TransformUniform,
    &'a mut Vec<MaterialUniform>,
    &'a Vec<MaterialResource>,
);
pub type SkinnedMeshRenderer<'a> = (
    &'a Arc<Mesh>,
    &'a SkinnedMeshResource,
    &'a BoneCollection,
    &'a BoneTransformUniform,
    &'a mut Vec<MaterialUniform>,
    &'a Vec<MaterialResource>,
);
