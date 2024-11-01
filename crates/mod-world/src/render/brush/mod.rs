use std::{fmt::Debug, sync::Arc};

use super::{camera::CameraResource, mesh::Mesh};

/// 메쉬를 그리는 브러쉬
pub trait MeshBrush : Sync + Send + Debug {
    /// 이전에 실행되어야 하는 렌더링 파이프라인의 식별자를 가져옵니다.
    #[inline]
    #[must_use]
    fn prev(&self) -> Option<wgpu::Id<wgpu::RenderPipeline>> {
        None
    }

    /// 이후에 실행되어야 하는 렌더링 파이프라인의 식별자를 가져옵니다.
    #[inline]
    #[must_use]
    fn next(&self) -> Option<wgpu::Id<wgpu::RenderPipeline>> {
        None
    }

    /// 브러쉬로 그릴려는 대상 메쉬를 가져옵니다.
    fn mesh(&self) -> &Arc<Mesh>;

    /// 브러쉬의 렌더링 파이프라인을 가져옵니다.
    fn pipeline(&self) -> &'static wgpu::RenderPipeline;

    /// 렌더 패스에 브러쉬를 바인드합니다.
    /// 
    /// # Warnings
    /// 함상 브러쉬로 그리기 전에 렌더패스에 바인드해야 합니다.
    /// 
    fn bind<'a>(&'a self, camera: &CameraResource, rpass: &mut wgpu::RenderPass<'a>);

    /// 브러쉬를 사용하여 화면에 그립니다.
    /// 
    /// # Warnings
    /// 브러쉬가 렌더패스에 바인드되지 않았을 경우 미정의 동작을 수행합니다.
    /// 
    fn draw<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>);
}



pub mod bullet;
pub mod student;
pub mod terrain;
