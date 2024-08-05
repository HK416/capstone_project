pub mod forward;

use std::fmt;
use hecs::World;
use hecs::Entity;
use winit::window::Window;

use crate::render::scale::RenderScale;



pub trait GraphicsPipeline : fmt::Debug {
    /// 3차원 모델 메쉬의 속성을 가져옵니다.
    fn attributes(&self) -> &'static [u32];

    /// 버퍼의 크기를 재조정 합니다.
    /// 버퍼 크기 재조정에 실패한 경우 `false`를 반환합니다.
    fn resize_buffer(
        &mut self, 
        scale: RenderScale, 
        window: &Window, 
        device: &wgpu::Device, 
    );

    /// 그리기를 실행합니다.
    fn process(
        &self, 
        world: &World, 
        camera: Entity, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        render_target: &wgpu::TextureView, 
        clear_color: wgpu::Color
    );
}
