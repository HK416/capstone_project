use hecs::{Entity, ViewBorrow, World};
use rayon::{iter::ParallelIterator, slice::ParallelSlice};

use crate::component::{BoneCollection, Child, Sibling, WorldTransform};

use super::{BoneTransformUniform, TransformDataLayout, TransformUniform};

/// 주어진 엔터티의 메쉬 리소스를 준비하는 재귀함수입니다.
///
/// 주어진 엔터티가 변환 행렬 유니폼 버퍼(`TransformUniform`), 월드 변환 행렬(`WorldTransform`)을
/// 갖고 있지 않는 경우 해당 엔터티를 생략합니다.
///
/// # Note
/// 이 시스템은 주어진 엔터티의 월드 변환 행렬이 먼저 갱신되어야 합니다.
///
/// # Panics
/// - 주어진 엔터티는 유효한 엔터티여야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn prepare_mesh_resource(
    world: &World,
    entities: &[Entity],
    device: &wgpu::Device,
    chunk_size: usize,
) -> Vec<(Vec<wgpu::Buffer>, wgpu::CommandBuffer)> {
    let child_view = &world.view::<&Child>();
    let sibling_view = &world.view::<&Sibling>();
    let resource_view = &world.view::<(&WorldTransform, &TransformUniform)>();
    entities
        .par_chunks(chunk_size)
        .map(|entities| {
            rayon::scope(|_| {
                let mut staging_buffers = Vec::new();
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                for &entity in entities {
                    prepare_mesh_resource_recursion(
                        child_view,
                        sibling_view,
                        resource_view,
                        entity,
                        device,
                        &mut encoder,
                        &mut staging_buffers,
                    );
                }

                (staging_buffers, encoder.finish())
            })
        })
        .collect()
}

/// 주어진 엔터티의 메쉬 리소스를 준비하는 재귀함수입니다.
///
/// 주어진 엔터티가 변환 행렬 유니폼 버퍼(`TransformUniform`), 월드 변환 행렬(`WorldTransform`)을
/// 갖고 있지 않는 경우 해당 엔터티를 생략합니다.
///
/// # Note
/// 이 시스템은 주어진 엔터티의 월드 변환 행렬이 먼저 갱신되어야 합니다.
///
/// # Panics
/// - 주어진 엔터티는 유효한 엔터티여야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn prepare_mesh_resource_recursion(
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    resource_view: &ViewBorrow<'_, (&WorldTransform, &TransformUniform)>,
    entity: Entity,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) {
    // 형제 엔터티가 존재하는 경우 형제 엔터티의 계층 구조를 탐색합니다.
    if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
        prepare_mesh_resource_recursion(
            child_view,
            sibling_view,
            resource_view,
            *sibling_entity,
            device,
            encoder,
            staging_buffers,
        );
    }

    // 자식 엔터티가 존재하는 경우 자식 엔터티의 계층 구조를 탐색합니다.
    if let Some(child_entity) = child_view.get(entity).cloned() {
        prepare_mesh_resource_recursion(
            child_view,
            sibling_view,
            resource_view,
            *child_entity,
            device,
            encoder,
            staging_buffers,
        );
    }

    // 현재 엔터티가 조건에 맞는지 확인합니다.
    if let Some((world_transform, transform_uniform)) = resource_view.get(entity) {
        // 변환 행렬 유니폼 버퍼를 갱신합니다.
        transform_uniform.update(
            device,
            encoder,
            staging_buffers,
            TransformDataLayout {
                trans: world_transform.0.to_cols_array(),
                ..Default::default()
            },
        );
    }
}

/// 주어진 엔터티의 스키닝된 메쉬 리소스를 준비합니다.
///
/// 주어진 엔터티가 월드 변환 행렬(`WorldTransform`),
/// 뼈 변환 행렬 유니폼 버퍼(`BoneTransUniform`), 뼈 집합(`BoneCollection`)을
/// 갖고 있지 않는 경우 해당 엔터티를 생략합니다.
///
/// # Note
/// 이 시스템은 주어진 엔터티의 월드 변환 행렬이 먼저 갱신되어야 합니다.
///
/// # Panics
/// - 주어진 엔터티는 유효한 엔터티여야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
pub fn prepare_skinned_mesh_resource(
    world: &World,
    entities: &[Entity],
    device: &wgpu::Device,
    chunk_size: usize,
) -> Vec<(Vec<wgpu::Buffer>, wgpu::CommandBuffer)> {
    let child_view = &world.view::<&Child>();
    let sibling_view = &world.view::<&Sibling>();
    let transform_view = &world.view::<&WorldTransform>();
    let resource_view = &world.view::<(&BoneCollection, &BoneTransformUniform)>();

    entities
        .par_chunks(chunk_size)
        .map(|entities| {
            rayon::scope(|_| {
                let mut staging_buffers = Vec::new();
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                for &entity in entities {
                    prepare_skinned_mesh_resource_recursion(
                        child_view,
                        sibling_view,
                        transform_view,
                        resource_view,
                        entity,
                        device,
                        &mut encoder,
                        &mut staging_buffers,
                    );
                }

                (staging_buffers, encoder.finish())
            })
        })
        .collect()
}

/// 주어진 엔터티의 스키닝된 메쉬 리소스를 준비하는 재귀함수입니다.
///
/// 주어진 엔터티가 월드 변환 행렬(`WorldTransform`),
/// 뼈 변환 행렬 유니폼 버퍼(`BoneTransUniform`), 뼈 집합(`BoneCollection`)을
/// 갖고 있지 않는 경우 해당 엔터티를 생략합니다.
///
/// # Note
/// 이 시스템은 주어진 엔터티의 월드 변환 행렬이 먼저 갱신되어야 합니다.
///
/// # Panics
/// - 주어진 엔터티는 유효한 엔터티여야합니다. 그렇지 않는 경우 [`panic!`]을 호출합니다.
/// - 엔터티의 컴포넌트 데이터가 스레드에 안전하지 않는 경우 [`panic!`]을 호출합니다.
///
fn prepare_skinned_mesh_resource_recursion(
    child_view: &ViewBorrow<'_, &Child>,
    sibling_view: &ViewBorrow<'_, &Sibling>,
    transform_view: &ViewBorrow<'_, &WorldTransform>,
    resource_view: &ViewBorrow<'_, (&BoneCollection, &BoneTransformUniform)>,
    entity: Entity,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
) {
    // 형제 엔터티가 존재하는 경우 형제 엔터티의 계층 구조를 탐색합니다.
    if let Some(sibling_entity) = sibling_view.get(entity).cloned() {
        prepare_skinned_mesh_resource_recursion(
            child_view,
            sibling_view,
            transform_view,
            resource_view,
            *sibling_entity,
            device,
            encoder,
            staging_buffers,
        );
    }

    // 자식 엔터티가 존재하는 경우 자식 엔터티의 계층 구조를 탐색합니다.
    if let Some(child_entity) = child_view.get(entity).cloned() {
        prepare_skinned_mesh_resource_recursion(
            child_view,
            sibling_view,
            transform_view,
            resource_view,
            *child_entity,
            device,
            encoder,
            staging_buffers,
        );
    }

    // 현재 엔터티가 조건에 맞는지 확인합니다.
    if let Some((collection, uniform_buffer)) = resource_view.get(entity) {
        let data = collection
            .bones
            .iter()
            .map(|&entity| {
                transform_view
                    .get(entity)
                    .expect("invalid entity or invalid entity component")
                    .0
                    .to_cols_array()
            })
            .collect();
        uniform_buffer.update(device, encoder, staging_buffers, data);
    }
}
