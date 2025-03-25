use std::sync::Arc;

use hecs::{Entity, EntityBuilder, World};
use mod_app::asset::AssetManager;
use mod_render::{MaterialResource, MeshResource};

use crate::{
    asset::{AssetError, ModelHierarchyPool, Node, Root},
    component::{Child, Parent, Sibling, ToParentTrans, WorldTransform},
};

/// ## Stage Prop Tag
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StageProp;

/// 게임 스테이지 소품 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 가집니다.
/// - 자식 엔터티(`Child`)
/// - 소품 태그(`StageProp`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
pub fn spawn_stage_prop(
    name: &str,
    workspace: &str,
    scale: glam::Vec3,
    rotation: glam::Quat,
    translation: glam::Vec3,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    world: &World,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), AssetError> {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트 데이터를 준비합니다.
    let tag = StageProp;
    let local_transform = ToParentTrans(glam::Mat4::from_scale_rotation_translation(
        scale,
        rotation,
        translation,
    ));
    let world_transform = WorldTransform::default();

    // 컴포넌트를 추가합니다.
    builder.add_bundle((tag, local_transform, world_transform));

    // 소품 모델을 구성하는 엔터티를 생성합니다.

    let (model_root_entity, mut batch_commands) =
        spawn_stage_prop_model(name, workspace, asset_manager, device, queue, world, entity)?;

    // 자식 엔터티를 추가합니다.
    builder.add(Child(model_root_entity));

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    Ok((entity, batch_commands))
}

/// 게임 스테이지 소품 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 가집니다.
/// - 자식 엔터티(`Child`)
/// - 소품 태그(`StageProp`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
pub fn spawn_stage_prop_from_root(
    root: Arc<Root>,
    scale: glam::Vec3,
    rotation: glam::Quat,
    translation: glam::Vec3,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    world: &World,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), AssetError> {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트 데이터를 준비합니다.
    let tag = StageProp;
    let local_transform = ToParentTrans(glam::Mat4::from_scale_rotation_translation(
        scale,
        rotation,
        translation,
    ));
    let world_transform = WorldTransform::default();

    // 컴포넌트를 추가합니다.
    builder.add_bundle((tag, local_transform, world_transform));

    // 소품 모델을 구성하는 엔터티를 생성합니다.

    let (model_root_entity, mut batch_commands) =
        spawn_stage_prop_model_from_root(root, device, queue, world, entity)?;

    // 자식 엔터티를 추가합니다.
    builder.add(Child(model_root_entity));

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    Ok((entity, batch_commands))
}

/// 스테이지 소품을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 가집니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
/// 일부 엔터티는 아래 컴포넌트를 선택적으로 가집니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 메쉬 쉐이더 리소스(`Arc<MeshResource>`)
/// - 소품 태크(`StageProp`)
/// - 재질 쉐이더 리소스(`Vec<Arc<MaterialResource>>`)
///
fn spawn_stage_prop_model(
    name: &str,
    workspace: &str,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    world: &World,
    parent: Entity,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), AssetError> {
    let root = ModelHierarchyPool::get_or_init(name, workspace, asset_manager, device, queue)?;

    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_stage_prop_model_recursion(
        world,
        device,
        queue,
        &mut batch_commands,
        parent,
        &root.node,
        &[],
    );

    Ok((entity, batch_commands))
}

/// 스테이지 소품을 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 가집니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
/// 일부 엔터티는 아래 컴포넌트를 선택적으로 가집니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 메쉬 쉐이더 리소스(`Arc<MeshResource>`)
/// - 소품 태크(`StageProp`)
/// - 재질 쉐이더 리소스(`Vec<Arc<MaterialResource>>`)
///
fn spawn_stage_prop_model_from_root(
    root: Arc<Root>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    world: &World,
    parent: Entity,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), AssetError> {
    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_stage_prop_model_recursion(
        world,
        device,
        queue,
        &mut batch_commands,
        parent,
        &root.node,
        &[],
    );

    Ok((entity, batch_commands))
}

/// 스테이지 소품 모델을 구성하는 엔터티를 생성하는 재귀함수입니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 기본으로 가잡니다.
/// - 부모 엔터티(`Parent`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
/// 일부 엔터티는 아래 컴포넌트를 선택적으로 가집니다.
/// - 자식 엔터티(`Child`)
/// - 형제 엔터티(`Sibling`)
/// - 모델 메쉬(`Arc<Mesh>`)
/// - 메쉬 쉐이더 리소스(`Arc<MeshResource>`)
/// - 소품 태그(`StageProp`)
/// - 재질 쉐이더 리소스(`Vec<Arc<MaterialResource>>`)
///
fn spawn_stage_prop_model_recursion(
    world: &World,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    parent: Entity,
    current: &Node,
    siblings: &[Node],
) -> Entity {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 부모 엔터티, 로컬 변환 행렬, 월드 변환 행렬 컴포넌트를 추가합니다.
    builder.add(Parent(parent));
    builder.add(ToParentTrans(current.transform));
    builder.add(WorldTransform::default());

    // 자식 노드가 존재하는 경우 자식 엔터티를 생성합니다.
    if let Some(child) = current.children.first() {
        // 자식 엔터티를 생성하기 위한 매개변수를 준비합니다.
        let parent = entity;
        let current = child;
        let siblings = &current.children[1..];

        // 자식 엔터티를 생성합니다.
        let entity = spawn_stage_prop_model_recursion(
            world,
            device,
            queue,
            batch_commands,
            parent,
            current,
            siblings,
        );

        // 자식 컴포넌트를 추가합니다.
        builder.add(Child(entity));
    }

    // 형제 노드가 존재하는 경우 형제 엔터티를 추가합니다.
    if let Some(sibling) = siblings.first() {
        // 형제 엔터티를 생성하기 위한 매개변수를 준비합니다.
        let current = sibling;
        let siblings = &siblings[1..];

        // 형제 엔터티를 생성합니다.
        let entity = spawn_stage_prop_model_recursion(
            world,
            device,
            queue,
            batch_commands,
            parent,
            current,
            siblings,
        );

        // 형제 엔터티 컴포넌트를 추가합니다.
        builder.add(Sibling(entity));
    }

    // 노드에 메쉬 데이터가 존재하는 경우 메쉬 데이터를 추가합니다.
    if let Some(mesh) = current.mesh.clone() {
        // 메쉬 쉐이더 리소스를 생성합니다.
        let mesh_name = mesh.name().to_string();
        let mesh_resource = Arc::new(MeshResource::uninit(Some(&mesh_name), device));

        // 메쉬, 메쉬 쉐이더 리소스, 캐릭터 종류 컴포넌트를 추가합니다.
        builder.add_bundle((mesh, mesh_resource, StageProp));
    }

    // 현제 노드에 재질 데이터가 존재하는 경우 재질 데이터를 추가합니다.
    if !current.materials.is_empty() {
        let materials: Vec<Arc<MaterialResource>> = current.materials.iter().cloned().collect();
        builder.add(materials);
    }

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    entity
}
