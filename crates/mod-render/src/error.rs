/// ## Wgpu Initialization Error
/// `wgpu` 렌더링 객체를 생성하는 도중 발생하는 오류 목록입니다.
#[derive(Debug, thiserror::Error)]
pub enum WgpuInitError {
    /// 적절한 장치 어뎁터를 찾지 못한 경우 발생하는 오류입니다.
    #[error("no suitable adapter")]
    NoSuitableAdapter,

    /// 적절한 장치를 찾지 못하 경우 발생하는 오류입니다.
    #[error("no suitable device found for the following reasons: {0}")]
    NoSuitableDevice(#[from] wgpu::RequestDeviceError),
}

/// ## Surface Initialization Error
/// `wgpu` 창 표면 객체를 생성하는 도중 발생하는 오류 목록입니다.
#[derive(Debug, thiserror::Error)]
pub enum SurfaceInitError {
    /// `wgpu` 창 표면 객체를 생성에 실패할 경우 발생하는 오류입니다.
    #[error("wgpu surface creation failed for the following reasons: {0}")]
    CreationFailed(#[from] wgpu::CreateSurfaceError),

    /// 생성된 `wgpu` 창 표면 객체가 `wgpu` 장치 어뎁터와 호환되지 않습니다.
    #[error("the surface is not compatible with the device adapter")]
    NotCompatible,
}
