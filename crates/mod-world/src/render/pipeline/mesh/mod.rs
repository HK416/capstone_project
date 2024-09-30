pub mod model;

use crate::{
    component::{ArenaID, CameraObject}, 
    render::{material::Material, mesh::Mesh}
};



/// 메쉬를 그리는 렌더러가 구현해야 하는 `trait`입니다.
pub trait MeshRenderer {
    /// 렌더러의 게임 오브젝트 식별자를 가져옵니다.
    fn game_object(&self) -> &ArenaID;

    /// 렌더러에 연결된 메쉬를 가져옵니다.
    fn mesh(&self) -> &Mesh;

    /// 렌더러의 재질을 가져옵니다.
    fn materials(&self) -> &[Material];

    /// 렌더러를 파이프라인 상태 머신에 바인드합니다.
    fn bind<'a>(&'a self, camera: &dyn CameraObject, rpass: &mut wgpu::RenderPass<'a>);

    /// 렌더러를 사용하여 그립니다.
    fn draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>);
}
