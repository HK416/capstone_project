//! 에너지 볼 형태의 총알 객체와 관련된 코드를 관리합니다.
//!

mod pipeline;

use std::ops::Deref;

use hecs::{Entity, EntityBuilder, World};
use mod_network::components::BulletKind;

use crate::{
    asset::{ModelNode, ModelRoot, TextureDataPool},
    component::{
        Child, EnergyBulletMaterialDataLayout, EnergyBulletMaterialResource,
        EnergyBulletMaterialUniform, MaterialData, MaterialUniform, MeshResource, Parent, Sibling,
        ToParentTrans, TransformUniform, WorldTransform,
    },
};

pub use self::pipeline::*;

/// 에너지 볼 형태의 총알 모델을 구성하는 엔터티를 생성합니다.
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
/// - 재질 유니폼 버퍼(`Vec<EnergyBulletMaterialUniform`)
/// - 총알 종류(`BulletKind`)
///
/// # Panics
/// - 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn spawn_energy_bullet_model(
    label: Option<&str>,
    texture_data_pool: &TextureDataPool,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    world: &World,
    parent: Entity,
    root: &ModelRoot,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    let mut batch_commands = Vec::with_capacity(root.num_nodes);
    let entity = spawn_energy_bullet_model_recursive(
        label,
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

/// 에너지 볼 형태의 총알 모델을 구성하는 엔터티를 생성하는 재귀함수입니다.
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
/// - 재질 유니폼 버퍼(`Vec<EnergyBulletMaterialUniform`)
/// - 총알 종류(`BulletKind`)
///
/// # Panics
/// - 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn spawn_energy_bullet_model_recursive(
    label: Option<&str>,
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
        let child = spawn_energy_bullet_model_recursive(
            label,
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
        let sibling = spawn_energy_bullet_model_recursive(
            label,
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
        let transform_uniform = TransformUniform::uninit(
            Some(&format!("Transform({})", label.unwrap_or("Unknown"))),
            device,
        );
        let mesh_resource = MeshResource::new(label, device, &transform_uniform);

        // 메쉬, 메쉬 쉐이더 리소스, 등 컴포넌트를 추가합니다.
        builder.add_bundle((
            mesh,
            transform_uniform,
            mesh_resource,
            BulletKind::EnergyBoll,
        ));
    }

    // 현제 노드에 재질 데이터가 존재하는 경우 재질 데이터를 추가합니다.
    if !node.materials.is_empty() {
        let (uniforms, materials): (Vec<_>, Vec<_>) = node
            .materials
            .iter()
            .map(|data| {
                match data.deref() {
                    MaterialData::EnergyBullet(data) => {
                        // 재질 쉐이더 리소스를 생성합니다.
                        let data_layout = EnergyBulletMaterialDataLayout {
                            emissive: data.emissive.into(),
                            main_color: data.main_color.into(),
                            ..Default::default()
                        };
                        let bullet_uniform =
                            EnergyBulletMaterialUniform::new(label, device, data_layout);
                        let material_resource =
                            EnergyBulletMaterialResource::new(label, device, &bullet_uniform);

                        (
                            MaterialUniform::EnergyBullet {
                                data: data_layout,
                                buffer: bullet_uniform,
                            },
                            material_resource,
                        )
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
