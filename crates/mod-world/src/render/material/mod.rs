use std::fmt::Debug;

/// 재질 쉐이더 리소스 `trait`
pub trait MaterialResource : Sync + Send + Debug {
    fn bind_group(&self) -> &wgpu::BindGroup;
}





pub mod universal;
