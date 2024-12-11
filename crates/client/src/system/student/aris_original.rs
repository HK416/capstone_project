use std::sync::Arc;

use hecs::{QueryOneError, World};
use mod_app::asset::AssetManager;
use mod_render::{AttributeKind, CameraResource, MaterialResource, Mesh, MeshResource};

use crate::{
    asset::MotionPool,
    component::{
        aris_original::{ArisOriginal, ArisOriginalHalo},
        AnimationTimer, BoneCollection, MotionCollection, StudentBehaviorState, StudentTag,
        ToParentTrans,
    },
};

const MOTION_NAME: &'static str = "aris_original";
const WORKSPACE: &'static str = "characters/aris_original";
const IDLE_ANIMATION: &'static str = "aris_original_normal_idle";

type FuncType = fn(
    &World,
    &AssetManager,
    &mut AnimationTimer,
    &mut StudentBehaviorState,
    &MotionCollection,
    f32,
) -> Result<(), QueryOneError>;

const FUNC: [FuncType; 1] = [aris_original_idle_animation];

pub fn sys_aris_original_animation(
    world: &World,
    asset_manager: &AssetManager,
    elapsed_time_sec: f32,
    batch_size: u32,
) {
    type Q<'a> = (
        &'a mut AnimationTimer,
        &'a mut StudentBehaviorState,
        &'a MotionCollection,
    );
    type R<'a> = (&'a StudentTag, &'a ArisOriginal);

    let mut query = world.query::<Q>().with::<R>();
    let mut batched_iter = query.iter_batched(batch_size);
    rayon::scope(|scope| {
        while let Some(query) = batched_iter.next() {
            scope.spawn(move |_| {
                for (_, (timer, state, collection)) in query {
                    let index: usize = (*state).into();
                    FUNC[index](
                        world,
                        asset_manager,
                        timer,
                        state,
                        collection,
                        elapsed_time_sec,
                    )
                    .unwrap();
                }
            });
        }
    });
}

fn aris_original_idle_animation(
    world: &World,
    asset_manager: &AssetManager,
    timer: &mut AnimationTimer,
    state: &mut StudentBehaviorState,
    collection: &MotionCollection,
    elapsed_time_sec: f32,
) -> Result<(), QueryOneError> {
    let motions = MotionPool::get_or_init(&MOTION_NAME, &WORKSPACE, asset_manager).unwrap();
    let motion = motions.get(IDLE_ANIMATION).unwrap();

    // 애니메이션 타이머를 갱신합니다. (Loop)
    timer.0 = (timer.0 + elapsed_time_sec) % motion.length;
    let keyframe = motion.linear_sampling(timer.0);

    {
        let mut transform = world
            .query_one::<&mut ToParentTrans>(collection.root)
            .map_err(|_| QueryOneError::NoSuchEntity)?;
        let transform = transform.get().ok_or(QueryOneError::Unsatisfied)?;
        transform.0 = keyframe.root_matrix;
    }

    for keyframe_mesh in keyframe.meshes.iter() {
        let entity = collection
            .meshes
            .get(&keyframe_mesh.name)
            .cloned()
            .expect("no such entity");
        let mut collection = world
            .query_one::<&BoneCollection>(entity)
            .map_err(|_| QueryOneError::NoSuchEntity)?;
        let collection = collection.get().ok_or(QueryOneError::Unsatisfied)?;
        for (index, &bone_transform) in keyframe_mesh.bone_trans.iter().enumerate() {
            let entity = collection.bones[index];
            let mut transform = world
                .query_one::<&mut ToParentTrans>(entity)
                .map_err(|_| QueryOneError::NoSuchEntity)?;
            let transform = transform.get().ok_or(QueryOneError::Unsatisfied)?;
            transform.0 = bone_transform;
        }
    }

    Ok(())
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
    type R<'a> = &'a ArisOriginal;

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

/// `Aris_Original_Halo` 모델을 그립니다.
pub fn sys_aris_original_halo_draw<'a>(
    world: &'a World,
    camera: &'a CameraResource,
    rpass: &mut wgpu::RenderPass<'a>,
) {
    type Q<'a> = (
        &'a Arc<Mesh>,
        &'a Arc<MeshResource>,
        &'a Vec<Arc<MaterialResource>>,
    );
    type R<'a> = &'a ArisOriginalHalo;

    let mut query = world.query::<Q>().with::<R>();
    for (_, (mesh, mesh_resource, materials)) in query.iter() {
        // 메쉬의 정점 속성을 바인드합니다.
        rpass.set_vertex_buffer(0, mesh.vertex(..));
        rpass.set_vertex_buffer(1, mesh.attribute(&AttributeKind::Texcoord0, ..).unwrap());

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
