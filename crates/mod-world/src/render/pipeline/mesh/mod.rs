pub mod model;
pub mod shape;
pub mod terrain;

use std::sync::Arc;

use mod_parallelism::collections::SkipMap;

use crate::{
    component::{GameObject, WorldID}, 
    render::{camera::CameraResource, material::Material, mesh::Mesh}
};



/// 메쉬를 그리는 렌더러가 구현해야 하는 `trait`입니다.
pub trait MeshRenderer : Sync + Send {
    /// 렌더러의 게임 오브젝트 식별자를 가져옵니다.
    fn game_object(&self) -> &WorldID;

    /// 렌더러에 연결된 메쉬를 가져옵니다.
    fn mesh(&self) -> &Mesh;

    /// 렌더러의 재질을 가져옵니다.
    fn materials(&self) -> &[Arc<Material>];

    fn prepare(&self, device: &wgpu::Device, queue: &wgpu::Queue, world: &SkipMap<WorldID, GameObject>);

    /// 렌더러를 파이프라인 상태 머신에 바인드합니다.
    fn bind<'a>(&'a self, camera: &CameraResource, rpass: &mut wgpu::RenderPass<'a>);

    /// 렌더러를 사용하여 그립니다.
    fn draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>);
}
