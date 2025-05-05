//! 스테이지 객체의 생성과 관련된 코드를 관리합니다.
//!
use std::{fs::OpenOptions, io::Read, ops::Deref, path::Path};

use hecs::{Entity, EntityBuilder, World};
use mod_network::components::{
    DirectionalLight, StageAreaData, StageLayoutData, StageLightData, StagePropData,
};

use crate::{
    asset::{AssetError, ModelNode, ModelPool, ModelRoot, SamplerPool, TextureDataPool},
    component::{
        Child, LightResource, MaterialData, MaterialUniform, MeshResource, Parent, ShadowResource,
        Sibling, StageMaterialDataLayout, StageMaterialResource, StageMaterialUniform, StageTag,
        ToParentTrans, TransformUniform, WorldTransform, NUM_CASCADES,
    },
};

/// 파일에서 스테이지 레이아웃 데이터를 로드합니다.
pub fn load_stage_layout_from_file<Dir, Uri>(
    workspace: Dir,
    uri: Uri,
) -> Result<StageLayoutData, AssetError>
where
    Dir: AsRef<Path>,
    Uri: AsRef<str>,
{
    let mut path = workspace.as_ref().to_path_buf();
    path.push(format!("{}.json", uri.as_ref()));

    // 파일을 읽습니다.
    log::debug!("open stage data asset (PATH:{})", path.display());
    let mut file = OpenOptions::new()
        .read(true)
        .write(false)
        .open(&path)
        .map_err(|e| {
            log::error!(
                "failed to open stage data asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::IOError(e)
        })?;

    log::debug!("read stage data asset (PATH:{})", path.display());
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(|e| {
        log::error!(
            "failed to read stage data asset (PATH:{}, REASON:{})",
            path.display(),
            &e
        );
        AssetError::IOError(e)
    })?;

    log::debug!("close stage data asset (PATH:{})", path.display());
    drop(file);

    log::debug!("decode stage data asset (PATH:{})", path.display());
    serde_json::from_slice(&buf).map_err(|e| {
        log::error!(
            "failed to decode stage data asset (PATH:{}, REASON:{})",
            path.display(),
            &e
        );
        AssetError::ParsingFailed(e)
    })
}

/// 스테이지를 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 가집니다.
/// - 자식 엔터티(`Child`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
pub fn spawn_stage_area(
    world: &World,
    model_pool: &ModelPool,
    texture_data_pool: &TextureDataPool,
    data: &StageAreaData,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    // 모델 풀 객체에서 스테이지 모델 노드를 가져옵니다.
    log::debug!("spawn stage model (URI:{})", &data.model);
    let root = model_pool
        .get(&data.model)
        .expect("the stage model must exist!");

    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트 데이터를 준비합니다.
    let local_transform = ToParentTrans(glam::Mat4::from_rotation_translation(
        data.rotation.into(),
        data.translation.into(),
    ));
    let world_transform = WorldTransform::default();

    // 컴포넌트를 추가합니다.
    builder.add_bundle((local_transform, world_transform));

    // 스테이지 모델을 구성하는 엔터티를 생성합니다.
    let (child, mut batch_commands) = spawn_stage_model(
        texture_data_pool,
        device,
        encoder,
        staging_buffers,
        world,
        entity,
        &root,
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
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
///
pub fn spawn_stage_prop(
    world: &World,
    model_pool: &ModelPool,
    texture_data_pool: &TextureDataPool,
    data: &StagePropData,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    // 모델 풀 객체에서 스테이지 모델 노드를 가져옵니다.
    log::debug!("spawn stage model (URI:{})", &data.model);
    let root = model_pool
        .get(&data.model)
        .expect("the stage model must exist!");

    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트 데이터를 준비합니다.
    let local_transform = ToParentTrans(glam::Mat4::from_scale_rotation_translation(
        data.scale.into(),
        data.rotation.into(),
        data.translation.into(),
    ));
    let world_transform = WorldTransform::default();

    // 컴포넌트를 추가합니다.
    builder.add_bundle((local_transform, world_transform, StageTag));

    // 스테이지 모델을 구성하는 엔터티를 생성합니다.
    let (child, mut batch_commands) = spawn_stage_model(
        texture_data_pool,
        device,
        encoder,
        staging_buffers,
        world,
        entity,
        &root,
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
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
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
    texture_data_pool: &TextureDataPool,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    world: &World,
    parent: Entity,
    root: &ModelRoot,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_stage_model_recursive(
        texture_data_pool,
        device,
        encoder,
        staging_buffers,
        &mut batch_commands,
        world,
        parent,
        &root.node,
        &[],
    );

    (entity, batch_commands)
}

/// 스테이지 모델을 구성하는 엔터티를 생성하는 재귀함수입니다.
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
/// - 메쉬 쉐이더 리소스(`MeshResource`)
/// - 변환 행렬 유니폼 버퍼(`TransformUniform`)
/// - 재질 쉐이더 리소스(`Vec<MaterialResource>`)
/// - 재질 유니폼 버퍼(`Vec<MaterialUniform>`)
///
fn spawn_stage_model_recursive(
    texture_data_pool: &TextureDataPool,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
    world: &World,
    parent: Entity,
    node: &ModelNode,
    siblings: &[ModelNode],
) -> Entity {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 부모 엔터티, 로컬 변환 행렬, 월드 변환 행렬 컴포넌트를 추가합니다.
    builder.add_bundle((
        Parent(parent),
        ToParentTrans(node.transform),
        WorldTransform::default(),
    ));

    // 자식 노드가 존재하는 경우 자식 엔터티를 생성합니다.
    if let Some(node) = node.children.first() {
        // 자식 엔터티를 생성합니다.
        let child = spawn_stage_model_recursive(
            texture_data_pool,
            device,
            encoder,
            staging_buffers,
            batch_commands,
            world,
            entity,
            node,
            &node.children[1..],
        );

        // 자식 컴포넌트를 추가합니다.
        builder.add(Child(child));
    }

    // 형제 노드가 존재하는 경우 형제 엔터티를 추가합니다.
    if let Some(node) = siblings.first() {
        // 형제 엔터티를 생성합니다.
        let sibling = spawn_stage_model_recursive(
            texture_data_pool,
            device,
            encoder,
            staging_buffers,
            batch_commands,
            world,
            parent,
            node,
            &siblings[1..],
        );

        // 형제 엔터티 컴포넌트를 추가합니다.
        builder.add(Sibling(sibling));
    }

    // 노드에 메쉬 데이터가 존재하는 경우 메쉬 데이터를 추가합니다.
    if let Some(mesh) = node.mesh.clone() {
        // 메쉬 쉐이더 리소스를 생성합니다.
        let transform_uniform = TransformUniform::uninit(None, device);
        let mesh_resource = MeshResource::new(None, device, &transform_uniform);

        // 메쉬, 메쉬 쉐이더 리소스, 등 컴포넌트를 추가합니다.
        builder.add_bundle((mesh, transform_uniform, mesh_resource));
    }

    // 현제 노드에 재질 데이터가 존재하는 경우 재질 데이터를 추가합니다.
    if !node.materials.is_empty() {
        let (uniforms, materials): (Vec<_>, Vec<_>) = node
            .materials
            .iter()
            .map(|data| {
                match data.deref() {
                    MaterialData::Stage(data) => {
                        // 재질 쉐이더 리소스를 생성합니다.
                        let stage_uniform = StageMaterialUniform::new(
                            None,
                            device,
                            StageMaterialDataLayout {
                                smoothness: data.smoothness,
                                glossiness: data.glossiness,
                                metallic: data.metallic,
                                ..Default::default()
                            },
                        );

                        // 스테이지 메인 컬러 텍스처를 가져옵니다.
                        let (main_color_view, main_color_sampler) = texture_data_pool
                            .get(&data.main_color)
                            .expect("the texture data must exist!");

                        let material_resource = StageMaterialResource::new(
                            None,
                            device,
                            &stage_uniform,
                            &main_color_view,
                            &main_color_sampler,
                        );

                        (MaterialUniform::Stage(stage_uniform), material_resource)
                    }
                    _ => panic!("invalid material data!"),
                }
            })
            .unzip();

        builder.add_bundle((uniforms, materials));
    }

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    entity
}

/// 스테이지를 구성하는 조명 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 공통적으로 가집니다.
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
/// - 조명 쉐이더 리소스(`LightResource`)
/// - 그림자 쉐이더 리소스(`ShadowResource`)
///
pub fn spawn_stage_light(
    sampler_pool: &SamplerPool,
    device: &wgpu::Device,
    world: &World,
    data: &StageLightData,
) -> (Vec<Entity>, Vec<(Entity, EntityBuilder)>) {
    let mut entities = Vec::new();
    let mut batch_commands = Vec::new();

    match data {
        StageLightData::Directional(data) => spawn_stage_directional_light(
            sampler_pool,
            device,
            world,
            data.clone(),
            &mut entities,
            &mut batch_commands,
        ),
    };

    (entities, batch_commands)
}

/// 스테이지를 구성하는 조명 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 가집니다.
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
/// - Directional Light 데이터(`DirectionalLight`)
/// - 조명 쉐이더 리소스(`LightResource`)
/// - 그림자 쉐이더 리소스(`ShadowResource`)
///
fn spawn_stage_directional_light(
    sampler_pool: &SamplerPool,
    device: &wgpu::Device,
    world: &World,
    data: DirectionalLight,
    entities: &mut Vec<Entity>,
    batch_commands: &mut Vec<(Entity, EntityBuilder)>,
) {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트 데이터를 준비합니다.
    let local_transform = ToParentTrans::default();
    let world_transform = WorldTransform::default();
    let mut light_resources = Vec::with_capacity(NUM_CASCADES);
    let mut shadow_resources = Vec::with_capacity(NUM_CASCADES);
    for i in 0..NUM_CASCADES {
        let light_resource = LightResource::new(
            Some(&format!("{}_{}", &data.label, i)),
            device,
            wgpu::TextureFormat::Depth32Float,
            1024,
            sampler_pool,
        );
        let shadow_resource = ShadowResource::new(
            Some(&format!("{}_{}", data.label, i)),
            device,
            &light_resource,
        );

        light_resources.push(light_resource);
        shadow_resources.push(shadow_resource);
    }

    // 컴포넌트를 추가합니다.
    builder.add_bundle((
        local_transform,
        world_transform,
        data,
        light_resources,
        shadow_resources,
    ));

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.push((entity, builder));

    // 조명 엔터티를 추가합니다.
    entities.push(entity);
}
