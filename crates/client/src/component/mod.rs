mod bullet;
mod camera;
mod character;
mod control;
mod damage_font;
mod deferred;
mod hierarchy;
mod light;
mod material;
mod mesh;
mod skybox;
mod stage;
mod transform;
mod ui;

use std::sync::Arc;

use ahash::HashMap;
use hecs::{Entity, World};

pub use self::{
    bullet::*, camera::*, character::*, control::*, damage_font::*, deferred::*, hierarchy::*,
    light::*, material::*, mesh::*, skybox::*, stage::*, transform::*, ui::*,
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

pub type BakeList = Vec<(Arc<ShadowResource>, ShadowMap)>;
pub type ShadowMap = HashMap<(Arc<Mesh>, MaterialKind), HashMap<usize, Vec<MeshFilter>>>;
pub type OpaqueMap =
    HashMap<(Arc<Mesh>, MaterialKind), HashMap<(usize, MaterialResource), Vec<MeshFilter>>>;
pub type TransparentMap =
    HashMap<(Arc<Mesh>, MaterialKind), HashMap<(usize, MaterialResource), Vec<MeshFilter>>>;
pub type MeshRenderer<'a> = (
    &'a Arc<Mesh>,
    &'a MeshResource,
    &'a TransformUniform,
    &'a Vec<MaterialUniform>,
    &'a Vec<MaterialResource>,
);
pub type SkinnedMeshRenderer<'a> = (
    &'a Arc<Mesh>,
    &'a SkinnedMeshResource,
    &'a BoneCollection,
    &'a BoneTransformUniform,
    &'a Vec<MaterialUniform>,
    &'a Vec<MaterialResource>,
);

macro_rules! define_tags {
    ( $( $name:ident ),* ) => {
        $(
            #[derive(Debug, Clone, Copy)]
            pub struct $name;

            impl $name {
                pub fn name() -> &'static str {
                    stringify!($name)
                }
            }
        )*
    }
}

define_tags!(
    Player0, Player1, Player2, Player3, Player4, Player5, Player6, Player7, Player8, Player9,
    Bullet, Camera, Stage
);

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlayerArchetype {
    Player0,
    Player1,
    Player2,
    Player3,
    Player4,
    Player5,
    Player6,
    Player7,
    Player8,
    Player9,
}

/// PlayerArchetype과 함께 엔터티의 컴포넌트를 질의합니다.
pub fn local_transform_query_mut<'a>(
    world: &'a mut World,
    entity: Entity,
    archetype: PlayerArchetype,
) -> &'a ToParentTrans {
    match archetype {
        PlayerArchetype::Player0 => {
            let (_, q) = world
                .query_one_mut::<&(Player0, ToParentTrans)>(entity)
                .expect("invalid entity or invalid entity component!");
            q
        }
        PlayerArchetype::Player1 => {
            let (_, q) = world
                .query_one_mut::<&(Player1, ToParentTrans)>(entity)
                .expect("invalid entity or invalid entity component!");
            q
        }
        PlayerArchetype::Player2 => {
            let (_, q) = world
                .query_one_mut::<&(Player2, ToParentTrans)>(entity)
                .expect("invalid entity or invalid entity component!");
            q
        }
        PlayerArchetype::Player3 => {
            let (_, q) = world
                .query_one_mut::<&(Player3, ToParentTrans)>(entity)
                .expect("invalid entity or invalid entity component!");
            q
        }
        PlayerArchetype::Player4 => {
            let (_, q) = world
                .query_one_mut::<&(Player4, ToParentTrans)>(entity)
                .expect("invalid entity or invalid entity component!");
            q
        }
        PlayerArchetype::Player5 => {
            let (_, q) = world
                .query_one_mut::<&(Player5, ToParentTrans)>(entity)
                .expect("invalid entity or invalid entity component!");
            q
        }
        PlayerArchetype::Player6 => {
            let (_, q) = world
                .query_one_mut::<&(Player6, ToParentTrans)>(entity)
                .expect("invalid entity or invalid entity component!");
            q
        }
        PlayerArchetype::Player7 => {
            let (_, q) = world
                .query_one_mut::<&(Player7, ToParentTrans)>(entity)
                .expect("invalid entity or invalid entity component!");
            q
        }
        PlayerArchetype::Player8 => {
            let (_, q) = world
                .query_one_mut::<&(Player8, ToParentTrans)>(entity)
                .expect("invalid entity or invalid entity component!");
            q
        }
        PlayerArchetype::Player9 => {
            let (_, q) = world
                .query_one_mut::<&(Player9, ToParentTrans)>(entity)
                .expect("invalid entity or invalid entity component!");
            q
        }
    }
}
