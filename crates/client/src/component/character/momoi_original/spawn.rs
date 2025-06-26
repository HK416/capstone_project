//! 캐릭터 모델 생성과 관련된 코드를 관리합니다.
//!

use std::{ops::Deref, sync::Arc};

use ahash::{HashMap, HashSet, RandomState};
use hecs::{Component, Entity, EntityBuilder, World};
use mod_network::components::{
    ActionStateTimer, BulletData, HealthData, InGamePlayerInitData, MovementStateTimer,
    PlayerStateData, SkillCostData, ViewStateTimer,
};
use parking_lot::Mutex;

use crate::{
    asset::{ModelNode, ModelPool, ModelRoot, TextureDataPool, CHARACTER_URIS},
    component::{
        BoneCollection, BoneTransformUniform, CharacterMaterialResource, CharacterMaterialUniform,
        Child, EyeMouthMaterialResource, EyeMouthMaterialUniform, HaloMaterialResource,
        MaterialData, MaterialResource, MaterialUniform, MeshResource, Parent, Sibling,
        SkinnedMeshResource, SkinningAnimation, ToParentTrans, TransformUniform, WorldTransform,
        MAX_BONES, MODEL_BONE_HEAD, MODEL_BONE_L_CALF, MODEL_BONE_L_FOOT, MODEL_BONE_L_THIGH,
        MODEL_BONE_ROOT, MODEL_BONE_R_CALF, MODEL_BONE_R_FOOT, MODEL_BONE_R_HAND,
        MODEL_BONE_R_THIGH, MODEL_BONE_SPINE, MODEL_BONE_SPINE_1, MODEL_BONE_WEAPON,
    },
};

/// 플레이어 엔터티를 생성합니다.
pub fn spawn_player<Tag: Copy + Component>(
    tag: Tag,
    world: &World,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    texture_data_pool: &TextureDataPool,
    model_pool: &ModelPool,
    data: InGamePlayerInitData,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트를 추가합니다.
    builder.add_bundle((
        data.name,
        data.permission(),
        data.character_kind,
        (data.team(), data.team_index()),
        (data.network_state(), data.is_connected()),
        HealthData::splat(data.maximum_health),
        BulletData::splat(data.maximum_bullet),
        SkillCostData::new(0, data.maximum_skill_cost),
    ));
    builder.add((
        tag,
        ToParentTrans(glam::Mat4::from_rotation_translation(
            glam::Quat::from_array(data.rotation),
            glam::Vec3::from_array(data.translation),
        )),
    ));
    builder.add((tag, WorldTransform::default()));
    builder.add_bundle((
        PlayerStateData::new(),
        ActionStateTimer::new(0),
        MovementStateTimer::new(0),
        ViewStateTimer::new(0),
        data.latlon,
    ));

    // 캐릭터 모델을 가져옵니다.
    let root = model_pool
        .get(CHARACTER_URIS[data.character_kind as usize])
        .expect("the character model must be preloaded!");

    // 캐릭터 모델의 엔터티를 생성합니다.
    let (skinning_animation, child_entity, mut batch_commands) = spawn_character_model(
        tag,
        Some(&format!("Player({})", data.uid)),
        &root,
        world,
        entity,
        device,
        encoder,
        staging_buffers,
        texture_data_pool,
    );

    // 자식 엔터티와 스키닝 애니메이션 컴포넌트를 추가합니다.
    builder.add_bundle((Child(child_entity), skinning_animation));

    // 엔터티 생성 명령어에 현재 엔터티를 추가합니다.
    batch_commands.push((entity, builder));

    (entity, batch_commands)
}

/// 모델을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 소유합니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`(Tag, ToParentTrans)`)
/// - 월드 변환 행렬(`(Tag, WorldTransform)`)
///
/// 일부 엔터티는 아래 컴포넌트를 소유합니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 스키닝된 메쉬 쉐이더 리소스(`SkinnedMeshResource`)
/// - 뼈 변환 행렬 유니폼 버퍼(`BoneTransUniform`)
/// - 뼈 엔터티 집합(`BoneCollection`)
/// - 메쉬 쉐이더 리소스(`MeshResource`)
/// - 월드 변환 행렬 유니폼 버퍼(`TransformUniform`)
/// - 재질 쉐이더 리소스(`Vec<MaterialResource>`)
/// - 재질 유니폼 버퍼(`Vec<MaterialUniform>`)
///
fn spawn_character_model<Tag: Copy + Component>(
    tag: Tag,
    label: Option<&str>,
    root: &ModelRoot,
    world: &World,
    parent: Entity,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    texture_data_pool: &TextureDataPool,
) -> (SkinningAnimation, Entity, Vec<(Entity, EntityBuilder)>) {
    log::debug!("ModelRoot:{:?}", &root);

    let num_entity_list = root.num_nodes;
    let mut batch_commands = Vec::with_capacity(num_entity_list);
    let mut entity_list = HashMap::with_capacity_and_hasher(num_entity_list, RandomState::new());
    let mut mesh_entity_list = HashMap::default();
    let mut mixing_bone_list = HashSet::default();
    let entity = spawn_character_model_recursive(
        tag,
        label,
        world,
        parent,
        &root.node,
        &[],
        device,
        encoder,
        staging_buffers,
        &mut batch_commands,
        &mut entity_list,
        &mut mesh_entity_list,
        &mut mixing_bone_list,
        texture_data_pool,
        false,
    );

    // 스키닝 애니메이션 컴포넌트를 생성합니다.
    let skinning_animation = SkinningAnimation {
        root: entity_list
            .get(MODEL_BONE_ROOT)
            .cloned()
            .expect("no such entity"),
        head: entity_list
            .get(MODEL_BONE_HEAD)
            .cloned()
            .expect("no such entity"),
        muzzle: entity_list.get("fire_01").cloned().expect("no such entity"),
        weapon: entity_list
            .get(MODEL_BONE_WEAPON)
            .cloned()
            .expect("no such entity"),
        lower_spine: entity_list
            .get(MODEL_BONE_SPINE)
            .cloned()
            .expect("no such entity"),
        uppper_spine: entity_list
            .get(MODEL_BONE_SPINE_1)
            .cloned()
            .expect("no such entity"),
        main_hand: entity_list
            .get(MODEL_BONE_R_HAND)
            .cloned()
            .expect("no such entity"),
        left_thigh: entity_list
            .get(MODEL_BONE_L_THIGH)
            .cloned()
            .expect("no such entity"),
        right_thigh: entity_list
            .get(MODEL_BONE_R_THIGH)
            .cloned()
            .expect("no such entity"),
        left_calf: entity_list
            .get(MODEL_BONE_L_CALF)
            .cloned()
            .expect("no such entity"),
        right_calf: entity_list
            .get(MODEL_BONE_R_CALF)
            .cloned()
            .expect("no such entity"),
        left_foot: entity_list
            .get(MODEL_BONE_L_FOOT)
            .cloned()
            .expect("no such entity"),
        right_foot: entity_list
            .get(MODEL_BONE_R_FOOT)
            .cloned()
            .expect("no such entity"),
        mesh_entity_list,
        mixing_bone_list,
    };

    (skinning_animation, entity, batch_commands)
}

/// 모델을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 소유합니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`(Tag, ToParentTrans)`)
/// - 월드 변환 행렬(`(Tag, WorldTransform)`)
///
/// 일부 엔터티는 아래 컴포넌트를 소유합니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 스키닝된 메쉬 쉐이더 리소스(`SkinnedMeshResource`)
/// - 뼈 변환 행렬 유니폼 버퍼(`BoneTransUniform`)
/// - 뼈 엔터티 집합(`BoneCollection`)
/// - 메쉬 쉐이더 리소스(`MeshResource`)
/// - 월드 변환 행렬 유니폼 버퍼(`TransformUniform`)
/// - 재질 쉐이더 리소스(`Vec<MaterialResource>`)
/// - 재질 유니폼 버퍼(`Vec<MaterialUniform>`)
///
fn spawn_character_model_recursive<Tag: Copy + Component>(
    tag: Tag,
    label: Option<&str>,
    world: &World,
    parent: Entity,
    current: &ModelNode,
    siblings: &[ModelNode],
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    entity_list: &mut HashMap<String, Entity>,
    mesh_entity_list: &mut HashMap<String, Entity>,
    mixing_bone_list: &mut HashSet<Entity>,
    texture_data_pool: &TextureDataPool,
    is_animation_mixing_bone: bool,
) -> Entity {
    // log::debug!("travel character model node:{}", &current.name);

    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 엔터티 목록에 현재 엔터티를 추가합니다.
    let entity_name = current.name.clone();
    entity_list.insert(entity_name.clone(), entity);

    // 부모 엔터티, 로컬 변환 행렬, 월드 변환 행렬 컴포넌트를 추가합니다.
    builder.add(Parent(parent));
    builder.add((tag, ToParentTrans(current.transform.clone())));
    builder.add((tag, WorldTransform::default()));

    // 자식 노드가 존재할 경우 자식 노드 엔터티를 생성합니다.
    if let Some(child) = current.children.first() {
        // 현재 엔터티가 애니메이션 믹싱 엔터티에 해당하는지 여부를 확인합니다.
        let is_animation_mixing_bone = is_animation_mixing_bone
            || entity_name == MODEL_BONE_L_THIGH
            || entity_name == MODEL_BONE_R_THIGH
            || entity_name.contains("skirt");

        // 자식 노드 엔터티를 생성합니다.
        let child_entity = spawn_character_model_recursive(
            tag,
            label,
            world,
            entity,
            child,
            &current.children[1..],
            device,
            encoder,
            staging_buffers,
            batch_commands,
            entity_list,
            mesh_entity_list,
            mixing_bone_list,
            texture_data_pool,
            is_animation_mixing_bone,
        );

        // 자식 노드 엔터티를 컴포넌트에 추가합니다.
        builder.add(Child(child_entity));
    };

    // 형제 노드가 존재하는 경우 형제 노드 엔터티를 생성합니다.
    if let Some(sibling) = siblings.first() {
        // 형제 노드 엔터티를 생성합니다.
        let sibling_entity = spawn_character_model_recursive(
            tag,
            label,
            world,
            parent,
            sibling,
            &siblings[1..],
            device,
            encoder,
            staging_buffers,
            batch_commands,
            entity_list,
            mesh_entity_list,
            mixing_bone_list,
            texture_data_pool,
            is_animation_mixing_bone,
        );

        // 형제 노드 엔터티를 컴포넌트에 추가합니다.
        builder.add(Sibling(sibling_entity));
    };

    // 메쉬 데이터가 존재하는 경우 엔터티에 메쉬 데이터를 추가합니다.
    if let Some(mesh) = current.mesh.clone() {
        match current.skinning.clone() {
            // 스키닝된 메쉬의 쉐이더 리소스를 생성합니다.
            Some(skinning) => {
                // 바인드 포즈(기본 자세 뼈 변환 행렬) 유니폼 버퍼를 복사합니다.
                let bindpose_uniform = skinning.bindpose_uniform.clone();

                // 뼈 변환 행렬 유니폼 버퍼를 생성합니다.
                let bone_trans_uniform = BoneTransformUniform::uninit(
                    Some(&format!("BoneTransform({})", label.unwrap_or("Unknown"))),
                    device,
                );

                // 스키닝 메쉬 쉐이더 리소스를 생성합니다.
                let resource =
                    SkinnedMeshResource::new(label, device, &bindpose_uniform, &bone_trans_uniform);

                // 스키닝 메쉬를 구성하는 뼈 엔터티 집합을 생성합니다.
                let root = entity_list
                    .get(&skinning.root_bone)
                    .cloned()
                    .expect("no such entity!");
                let mut bones = Vec::with_capacity(MAX_BONES);
                for entity_name in skinning.bones.iter() {
                    let entity = entity_list
                        .get(entity_name)
                        .cloned()
                        .expect("no such entity!");
                    bones.push(entity);
                }
                let collection = BoneCollection { root, bones };

                // 엔터티에 컴포넌트를 추가합니다.
                builder.add_bundle((mesh, collection, bone_trans_uniform, resource));
            }
            None => {
                // 월드 변환 행렬 유니폼 버퍼를 생성합니다.
                let transform_uniform = TransformUniform::uninit(
                    Some(&format!("Transform({})", label.unwrap_or("Unknown"))),
                    device,
                );

                // 메쉬 리소스를 생성합니다.
                let resource = MeshResource::new(label, device, &transform_uniform);

                // 엔터티에 컴포넌트를 추가합니다.
                builder.add_bundle((mesh, transform_uniform, resource));
            }
        };
    }

    // 재질 데이터가 존재하는 경우 엔터티에 재질 데이터를 추가합니다.
    let result = create_material_resources(label, device, texture_data_pool, &current.materials);
    if let Some((uniforms, resources)) = result {
        builder.add_bundle((uniforms, resources));
    }

    // 애니메이션 믹싱 엔터티 집합에 포함되는 경우 엔터티를 추가합니다.
    if is_animation_mixing_bone
        || entity_name == MODEL_BONE_L_THIGH
        || entity_name == MODEL_BONE_R_THIGH
        || entity_name.contains("skirt")
    {
        mixing_bone_list.insert(entity);
    }

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    entity
}

/// 재질 쉐이더 리소스를 생성합니다.
fn create_material_resources(
    label: Option<&str>,
    device: &wgpu::Device,
    texture_data_pool: &TextureDataPool,
    materials: &[Arc<MaterialData>],
) -> Option<(Vec<MaterialUniform>, Vec<MaterialResource>)> {
    let num_materials = materials.len();
    if num_materials == 0 {
        return None;
    }

    let mut material_uniforms = Vec::with_capacity(num_materials);
    let mut material_resources = Vec::with_capacity(num_materials);
    for material in materials.iter() {
        match material.deref() {
            MaterialData::Character(character_material_data) => {
                // 재질 유니폼 버퍼를 생성합니다.
                let data = character_material_data.as_layout();
                let material_uniform = CharacterMaterialUniform::new(
                    Some(&format!(
                        "CharacterMaterial({})",
                        label.unwrap_or("unknown")
                    )),
                    device,
                    data,
                );

                // 캐릭터 메인 컬러 텍스처를 가져옵니다.
                let (main_color_view, main_color_sampler) = texture_data_pool
                    .get(&character_material_data.main_color)
                    .expect("the texture data must be preloaded!");
                // 캐릭터 마스킹 텍스처를 가져옵니다.
                let (detail_mask_view, detail_mask_sampler) = texture_data_pool
                    .get(&character_material_data.detail_mask)
                    .expect("the texture data must be preloaded!");

                // 재질 쉐이더 리소스를 생성합니다.
                let resource = CharacterMaterialResource::new(
                    label,
                    device,
                    &material_uniform,
                    &main_color_view,
                    &main_color_sampler,
                    &detail_mask_view,
                    &detail_mask_sampler,
                );

                material_uniforms.push(MaterialUniform::Character {
                    data: Mutex::new(data),
                    material_uniform,
                });
                material_resources.push(resource);
            }
            MaterialData::CharacterEyeMouth(eye_mouth_material_data) => {
                // 재질 유니폼 버퍼를 생성합니다.
                let data = eye_mouth_material_data.as_layout();
                let material_uniform = EyeMouthMaterialUniform::new(
                    Some(&format!("EyeMouthMaterial({})", label.unwrap_or("Unknown"))),
                    device,
                    data,
                );

                // 캐릭터 메인 컬러 텍스처를 가져옵니다.
                let (main_color_view, main_color_sampler) = texture_data_pool
                    .get(&eye_mouth_material_data.main_color)
                    .expect("the texture data must be preloaded!");
                // 캐릭터 입 텍스처를 가져옵니다.
                let (eye_mouth_view, eye_mouth_sampler) = texture_data_pool
                    .get(&eye_mouth_material_data.eye_mouth)
                    .expect("the texture data must be preloaded!");

                // 재질 쉐이더 리소스를 생성합니다.
                let resource = EyeMouthMaterialResource::new(
                    label,
                    device,
                    &material_uniform,
                    &main_color_view,
                    &main_color_sampler,
                    &eye_mouth_view,
                    &eye_mouth_sampler,
                );

                material_uniforms.push(MaterialUniform::CharacterEyeMouth {
                    data: Mutex::new(data),
                    material_uniform,
                });
                material_resources.push(resource);
            }
            MaterialData::CharacterHalo(halo_material_data) => {
                // 캐릭터 메인 컬러 텍스처를 가져옵니다.
                let (main_color_view, main_color_sampler) = texture_data_pool
                    .get(&halo_material_data.main_color)
                    .expect("the texture data must be preloaded!");

                // 재질 쉐이더 리소스를 생성합니다.
                let resource =
                    HaloMaterialResource::new(label, device, &main_color_view, &main_color_sampler);

                material_uniforms.push(MaterialUniform::CharacterHalo);
                material_resources.push(resource);
            }
            _ => panic!("invalid material data!"),
        }
    }

    Some((material_uniforms, material_resources))
}
