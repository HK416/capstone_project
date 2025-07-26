//! 스테이지 객체의 생성과 관련된 코드를 관리합니다.
//!
use std::{ops::Deref, sync::Arc};

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, World};
use mod_network::components::{AreaAttributes, PropAttributeData, StageAttributes};
use mod_physics::object3d::Sphere;
use parking_lot::Mutex;

use crate::{
    asset::{
        ModelNode, ModelPool, ModelRoot, StageBoundingVolumn, StageBoundingVolumnHierarchy,
        TextureDataPool,
    },
    component::{
        BoneCollection, BoneTransformUniform, Child, MaterialData, MaterialResource,
        MaterialUniform, MeshResource, Parent, Sibling, SkinnedMeshResource, Stage,
        StageBarrierMaterialResource, StageBarrierMaterialUniform, StageMaterialResource,
        StageMaterialUniform, ToParentTrans, TransformUniform, TreeMaterialResource,
        TreeMaterialUniform, WorldTransform, MAX_BONES,
    },
};

/// 스테이지를 구성하는 엔터티를 생성하고, Bounding Volumn Hierarchy를 반환합니다.
pub fn build_stage(
    label: Option<&str>,
    world: &World,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    model_pool: &ModelPool,
    texture_data_pool: &TextureDataPool,
    stage_attributes: &StageAttributes,
) -> (StageBoundingVolumnHierarchy, Vec<(Entity, EntityBuilder)>) {
    let mut batch_commands = Vec::default();
    let mut bvh = StageBoundingVolumnHierarchy::default();

    // 지역 데이터를 생성합니다.
    for v in stage_attributes.area.iter().flatten() {
        if let Some(area_data) = v {
            build_stage_area(
                label,
                world,
                model_pool,
                texture_data_pool,
                area_data,
                device,
                encoder,
                staging_buffers,
                &mut batch_commands,
                &mut bvh,
            );
        }
    }

    // 장식물 데이터를 생성합니다.
    bvh.root = stage_attributes.prop.as_ref().map(|prop_data| {
        build_stage_prop(
            label,
            world,
            device,
            encoder,
            staging_buffers,
            &mut batch_commands,
            model_pool,
            texture_data_pool,
            prop_data,
        )
    });

    (bvh, batch_commands)
}

/// 스테이지를 구성하는 지역 엔터티를 추가합니다.
fn build_stage_area(
    label: Option<&str>,
    world: &World,
    model_pool: &ModelPool,
    texture_data_pool: &TextureDataPool,
    area_data: &AreaAttributes,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    bvh: &mut StageBoundingVolumnHierarchy,
) {
    let (entity, mut batch_command) = spawn_stage_area(
        label,
        world,
        device,
        encoder,
        staging_buffers,
        model_pool,
        texture_data_pool,
        area_data,
    );
    bvh.area.push(entity);
    batch_commands.append(&mut batch_command);
}

fn build_stage_prop(
    label: Option<&str>,
    world: &World,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    model_pool: &ModelPool,
    texture_data_pool: &TextureDataPool,
    prop_data: &PropAttributeData,
) -> Box<StageBoundingVolumn> {
    let (entity, mut batch_command) = spawn_stage_prop(
        label,
        world,
        device,
        encoder,
        staging_buffers,
        model_pool,
        texture_data_pool,
        prop_data,
    );
    batch_commands.append(&mut batch_command);

    Box::new(StageBoundingVolumn {
        entity,
        sphere: Sphere {
            center: prop_data.center.into(),
            radius: prop_data.radius,
        },
        left: prop_data.left.as_ref().map(|prop_data| {
            build_stage_prop(
                label,
                world,
                device,
                encoder,
                staging_buffers,
                batch_commands,
                model_pool,
                texture_data_pool,
                prop_data,
            )
        }),
        right: prop_data.right.as_ref().map(|prop_data| {
            build_stage_prop(
                label,
                world,
                device,
                encoder,
                staging_buffers,
                batch_commands,
                model_pool,
                texture_data_pool,
                prop_data,
            )
        }),
    })
}

/// 스테이지를 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 가집니다.
/// - 자식 엔터티(`Child`)
/// - 로컬 변환 행렬(`(Stage, ToParentTrans)`)
/// - 월드 변환 행렬(`(Stage, WorldTransform)`)
///
pub fn spawn_stage_area(
    label: Option<&str>,
    world: &World,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    model_pool: &ModelPool,
    texture_data_pool: &TextureDataPool,
    data: &AreaAttributes,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    // 모델 풀 객체에서 스테이지 모델 노드를 가져옵니다.
    log::debug!("spawn stage model (URI:{})", &data.model);
    let root = model_pool
        .get(&data.model)
        .expect("the stage model must exist!");

    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트를 추가합니다.
    builder.add((
        Stage,
        ToParentTrans(glam::Mat4::from_rotation_translation(
            data.rotation.to_quat(),
            data.translation.into(),
        )),
    ));
    builder.add((Stage, WorldTransform::default()));

    // 스테이지 모델을 구성하는 엔터티를 생성합니다.
    let (child, mut batch_commands) = spawn_stage_model(
        label,
        world,
        entity,
        &root,
        device,
        encoder,
        staging_buffers,
        texture_data_pool,
    );

    // 스테이지 모델 루트 노드를 추가합니다.
    builder.add(Child(child));

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    (entity, batch_commands)
}

/// 스테이지를 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 가집니다.
/// - 자식 엔터티(`Child`)
/// - 로컬 변환 행렬(`(Stage, ToParentTrans)`)
/// - 월드 변환 행렬(`(Stage, WorldTransform)`)
///
fn spawn_stage_prop(
    label: Option<&str>,
    world: &World,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    model_pool: &ModelPool,
    texture_data_pool: &TextureDataPool,
    data: &PropAttributeData,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    // 모델 풀 객체에서 스테이지 모델 노드를 가져옵니다.
    log::debug!("spawn stage model (URI:{})", &data.model);
    let root = model_pool
        .get(&data.model)
        .expect("the stage model must exist!");

    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트를 추가합니다.
    builder.add((
        Stage,
        ToParentTrans(glam::Mat4::from_scale_rotation_translation(
            data.scale.into(),
            data.rotation.into(),
            data.translation.into(),
        )),
    ));
    builder.add((Stage, WorldTransform::default()));

    // 스테이지 모델을 구성하는 엔터티를 생성합니다.
    let (child, mut batch_commands) = spawn_stage_model(
        label,
        world,
        entity,
        &root,
        device,
        encoder,
        staging_buffers,
        texture_data_pool,
    );

    // 스테이지 모델 루트 노드를 추가합니다.
    builder.add(Child(child));

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    (entity, batch_commands)
}

/// 스테이지 모델을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 가집니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`(Stage, ToParentTrans)`)
/// - 월드 변환 행렬(`(Stage, WorldTransform)`)
///
/// 일부 엔터티는 아래 컴포넌트를 선택적으로 가집니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 메쉬 쉐이더 리소스(`MeshResource`)
/// - 변환 행렬 유니폼 버퍼(`TransformUniform`)
/// - 재질 쉐이더 리소스(`Vec<MaterialResource>`)
/// - 재질 유니폼 버퍼(`Vec<MaterialUniform>`)
///
fn spawn_stage_model(
    label: Option<&str>,
    world: &World,
    parent: Entity,
    root: &ModelRoot,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    texture_data_pool: &TextureDataPool,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    let mut entity_list = HashMap::default();
    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_stage_model_recursive(
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
        texture_data_pool,
    );

    (entity, batch_commands)
}

/// 스테이지 모델을 구성하는 엔터티를 생성하는 재귀함수입니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 가집니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`(Stage, ToParentTrans)`)
/// - 월드 변환 행렬(`(Stage, WorldTransform)`)
///
/// 일부 엔터티는 아래 컴포넌트를 선택적으로 가집니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 메쉬 쉐이더 리소스(`MeshResource`)
/// - 변환 행렬 유니폼 버퍼(`TransformUniform`)
/// - 재질 쉐이더 리소스(`Vec<MaterialResource>`)
/// - 재질 유니폼 버퍼(`Vec<MaterialUniform>`)
///
fn spawn_stage_model_recursive(
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
    texture_data_pool: &TextureDataPool,
) -> Entity {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 부모 엔터티, 로컬 변환 행렬, 월드 변환 행렬 컴포넌트를 추가합니다.
    builder.add(Parent(parent));
    builder.add((Stage, ToParentTrans(current.transform)));
    builder.add((Stage, WorldTransform::default()));

    // 자식 노드가 존재하는 경우 자식 엔터티를 생성합니다.
    if let Some(child) = current.children.first() {
        // 자식 엔터티를 생성합니다.
        let child = spawn_stage_model_recursive(
            label,
            world,
            entity,
            child,
            &child.children[1..],
            device,
            encoder,
            staging_buffers,
            batch_commands,
            entity_list,
            texture_data_pool,
        );

        // 자식 컴포넌트를 추가합니다.
        builder.add(Child(child));
    }

    // 형제 노드가 존재하는 경우 형제 엔터티를 추가합니다.
    if let Some(sibling) = siblings.first() {
        // 형제 엔터티를 생성합니다.
        let sibling = spawn_stage_model_recursive(
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
            texture_data_pool,
        );

        // 형제 엔터티 컴포넌트를 추가합니다.
        builder.add(Sibling(sibling));
    }

    // 노드에 메쉬 데이터가 존재하는 경우 메쉬 데이터를 추가합니다.
    if let Some(mesh) = current.mesh.clone() {
        match current.skinning.clone() {
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
        }
    }

    // 재질 데이터가 존재하는 경우 엔터티에 재질 데이터를 추가합니다.
    let result = create_material_resources(label, device, texture_data_pool, &current.materials);
    if let Some((uniforms, resources)) = result {
        builder.add_bundle((uniforms, resources));
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
            MaterialData::StageBarrier(stage_material_data) => {
                // 재질 유니폼 버퍼를 생성합니다.
                let data = stage_material_data.as_layout();
                let material_uniform = StageBarrierMaterialUniform::new(
                    Some(&format!(
                        "StageBarrierUniform({})",
                        label.unwrap_or("unknown")
                    )),
                    device,
                    data,
                );

                // // 캐릭터 메인 컬러 텍스처를 가져옵니다.
                // let (main_color_view, main_color_sampler) = texture_data_pool
                //     .get(&stage_material_data.main_color)
                //     .expect("the texture data must be preloaded!");

                // 재질 쉐이더 리소스를 생성합니다.
                let resource = StageBarrierMaterialResource::new(
                    label,
                    device,
                    &material_uniform,
                    // &main_color_view,
                    // &main_color_sampler,
                );

                material_uniforms.push(MaterialUniform::StageBarrier {
                    data: Mutex::new(data),
                    material_uniform,
                });
                material_resources.push(resource);
            }
            MaterialData::Stage(stage_material_data) => {
                // 재질 유니폼 버퍼를 생성합니다.
                let data = stage_material_data.as_layout();
                let material_uniform = StageMaterialUniform::new(
                    Some(&format!(
                        "StageMaterialUniform({})",
                        label.unwrap_or("unknown")
                    )),
                    device,
                    data,
                );

                // 캐릭터 메인 컬러 텍스처를 가져옵니다.
                let (main_color_view, main_color_sampler) = texture_data_pool
                    .get(&stage_material_data.main_color)
                    .expect("the texture data must be preloaded!");

                // 재질 쉐이더 리소스를 생성합니다.
                let resource = StageMaterialResource::new(
                    label,
                    device,
                    &material_uniform,
                    &main_color_view,
                    &main_color_sampler,
                );

                material_uniforms.push(MaterialUniform::Stage {
                    data: Mutex::new(data),
                    material_uniform,
                });
                material_resources.push(resource);
            }
            MaterialData::Tree(tree_material_data) => {
                // 재질 유니폼 버퍼를 생성합니다.
                let data = tree_material_data.as_layout();
                let material_uniform = TreeMaterialUniform::new(
                    Some(&format!(
                        "TreeMaterialUniform({})",
                        label.unwrap_or("unknown")
                    )),
                    device,
                    data,
                );

                // 캐릭터 메인 컬러 텍스처를 가져옵니다.
                let (main_color_view, main_color_sampler) = texture_data_pool
                    .get(&tree_material_data.main_color)
                    .expect("the texture data must be preloaded!");

                // 재질 쉐이더 리소스를 생성합니다.
                let resource = TreeMaterialResource::new(
                    label,
                    device,
                    &material_uniform,
                    &main_color_view,
                    &main_color_sampler,
                );

                material_uniforms.push(MaterialUniform::Tree {
                    data: Mutex::new(data),
                    material_uniform,
                });
                material_resources.push(resource);
            }
            _ => panic!("invalid material data!"),
        }
    }

    Some((material_uniforms, material_resources))
}
