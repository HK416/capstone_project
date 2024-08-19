use thiserror::Error;



/// `wgpu` 렌더러에서 발생하는 오류 목록입니다.
#[derive(Debug, Error)]
pub enum RenderError {
    /// 적절한 `wgpu` 장치 어뎁터를 찾지 못한 경우 발생하는 에러입니다.
    #[error("No suitable adapter!")]
    NoSuitableAdapter, 

    /// 적절한 `wgpu` 장치를 찾지 못한 경우 발생하는 에러입니다.
    #[error("No suitable device found for the following reasons: {0}")]
    NoSuitableDevice(#[from] wgpu::RequestDeviceError), 

    /// `wgpu` 창 표면을 생성에 실패할 경우 발생하는 에러입니다.
    #[error("Surface creation failed for the following reasons: {0}")]
    SurfaceCreationFailed(#[from] wgpu::CreateSurfaceError), 

    /// 현재 스왑체인 텍스처를 가져오지 못한 경우 발생하는 에러입니다.
    #[error("Failed to get current swapchain texture!")]
    SwapchainError(#[from] wgpu::SurfaceError), 
}
