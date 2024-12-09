use std::sync::Arc;

use hecs::World;
use mod_app::asset::AssetManager;
use mod_render::{AttributeKind, CameraResource, MaterialResource, Mesh, MeshResource};

use crate::{
    asset::MotionPool,
    component::{
        aris_original::ArisOriginalMesh, AnimationTimer, BoneCollection, MotionCollection,
        StudentBehaviorState, StudentTag, ToParentTrans,
    },
};

const MOTION_NAME: &'static str = "aris_original";
const WORKSPACE: &'static str = "characters/aris_original";
const IDLE_ANIMATION: &'static str = "aris_original_normal_idle";

pub fn sys_aris_original_animation(
    world: &mut World,
    asset_manager: &AssetManager,
    elapsed_time_sec: f32,
) {
    type Q<'a> = (
        &'a mut AnimationTimer,
        &'a mut StudentBehaviorState,
        &'a MotionCollection,
    );
    type R<'a> = &'a StudentTag;

    let mut query = world.query::<Q>().with::<R>();
    for (_, (timer, state, collection)) in query.iter() {
        aris_original_idle_animation(
            world,
            asset_manager,
            timer,
            state,
            collection,
            elapsed_time_sec,
        );
    }
}

fn aris_original_idle_animation(
    world: &World,
    asset_manager: &AssetManager,
    timer: &mut AnimationTimer,
    state: &mut StudentBehaviorState,
    motion_collection: &MotionCollection,
    elapsed_time_sec: f32,
) {
    let motions = MotionPool::get_or_init(&MOTION_NAME, &WORKSPACE, asset_manager).unwrap();
    let motion = motions.get(IDLE_ANIMATION).unwrap();

    let keyframe = motion.keyframes.first().unwrap();
    let mut transform = world
        .get::<&mut ToParentTrans>(motion_collection.root)
        .unwrap();
    *transform = ToParentTrans(keyframe.root_matrix.into());
    drop(transform);

    for keyframe_mesh in keyframe.meshes.iter() {
        let entity = motion_collection.meshes.get(&keyframe_mesh.name).unwrap();
        let bone_collection = world.get::<&BoneCollection>(*entity).unwrap();
        for (index, bone_transform) in keyframe_mesh.bone_trans.iter().enumerate() {
            let entity = bone_collection.bones[index];
            let mut transform = world.get::<&mut ToParentTrans>(entity).unwrap();
            *transform = ToParentTrans(bone_transform.into_mat4());
            drop(transform);
        }
    }
}

/// `Aris_Original` 모델을 그립니다.
pub fn sys_aris_original_draw<'a>(
    world: &'a World,
    camera: &'a CameraResource,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    type Q<'a> = (
        &'a Arc<Mesh>,
        &'a Arc<MeshResource>,
        &'a Vec<Arc<MaterialResource>>,
    );
    type R<'a> = &'a ArisOriginalMesh;

    let mut query = world.query::<Q>().with::<R>();
    for (_, (mesh, mesh_resource, materials)) in query.iter() {
        // 메쉬의 정점 속성을 바인드합니다.
        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Normal, ..).unwrap());
        rpass.set_vertex_buffer(2, mesh.attribute(&AttributeKind::Tangent, ..).unwrap());
        rpass.set_vertex_buffer(3, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());
        rpass.set_vertex_buffer(4, mesh.attribute(&AttributeKind::BoneIndex, ..).unwrap());
        rpass.set_vertex_buffer(5, mesh.attribute(&AttributeKind::BoneWeight, ..).unwrap());

        // 쉐이더 리소스를 바인드합니다.
        rpass.set_bind_group(0, &camera.bind_group, &[]);
        rpass.set_bind_group(1, &mesh_resource.bind_group, &[]);

        for (index, submesh) in mesh.submeshes().iter().enumerate() {
            // 메쉬의 인덱스 버퍼를 바인드합니다.
            rpass.set_index_buffer(submesh.slice(..), submesh.format());

            // 재질의 쉐이더 리소스를 바인드합니다.
            rpass.set_bind_group(2, &materials[index].bind_group, &[]);

            // 인덱스 버퍼를 사용하여 그립니다.
            rpass.draw_indexed(0..submesh.count(), 0, 0..1);
        }
    }
}
