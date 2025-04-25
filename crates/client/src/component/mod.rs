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
mod shadow;
mod skybox;
mod stage;
mod transform;
mod weighted_blended_oit;

use std::sync::Arc;

use ahash::HashMap;

pub use self::{
    bullet::*, camera::*, capture_zone::*, character::*, control::*, damage_font::*, hierarchy::*,
    light::*, material::*, mesh::*, shadow::*, skybox::*, stage::*, transform::*,
    weighted_blended_oit::*,
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

pub type ShadowMap = HashMap<(Arc<Mesh>, MaterialKind), Vec<(usize, MeshFilter)>>;
pub type OpaqueMap = HashMap<(Arc<Mesh>, MaterialKind), Vec<(usize, MeshFilter, MaterialResource)>>;
pub type TransparentMap =
    HashMap<(Arc<Mesh>, MaterialKind), Vec<(usize, MeshFilter, MaterialResource)>>;
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
