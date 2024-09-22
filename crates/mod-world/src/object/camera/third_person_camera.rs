use std::{fmt, sync::{Arc, Mutex, Weak}};

use crate::{object::{CameraObject, GameObject}, render::{camera::CameraUniform, light::LocalLightUniform}};



pub struct ThirdPersonCamera {
    /// 카메라의 이름입니다.
    name: String, 

    /// 카메라의 대상 오브젝트입니다.
    target: Arc<dyn GameObject>, 

    /// 카메라의 변환 행렬입니다.
    transform: Mutex<gmm::Matrix>, 

    /// 카메라 유니폼 버퍼
    camera_uniform: CameraUniform, 

    /// 지역 조명 유니폼 버퍼
    /// 
    /// ※ 차후 사용 예정
    /// 
    #[allow(dead_code)] local_light_uniform: LocalLightUniform, 

    /// 바인드 그룹
    bind_group: wgpu::BindGroup, 
}

impl GameObject for ThirdPersonCamera {
    #[inline]
    #[must_use]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    #[must_use]
    fn get_parent(&self) -> Option<&Weak<dyn GameObject>> {
        None
    }

    #[inline]
    #[must_use]
    fn get_sibling(&self) -> Option<&Arc<dyn GameObject>> {
        None
    }

    #[inline]
    #[must_use]
    fn get_child(&self) -> Option<&Arc<dyn GameObject>> {
        None
    }

    #[inline]
    #[must_use]
    fn to_parent_trans(&self) -> gmm::Matrix {
        gmm::Float4x4::IDENTITY.into()
    }

    #[inline]
    #[must_use]
    fn world_trans(&self) -> gmm::Matrix {
        self.transform.lock().unwrap().clone()
    }
}

impl CameraObject for ThirdPersonCamera {
    #[must_use]
    fn camera_trans(&self) -> gmm::Matrix {
        let world_trans: gmm::Float4x4 = self.world_trans().into();
        let eye: gmm::Vector = world_trans.w_axis.xyz().into();
        let dir: gmm::Vector = world_trans.z_axis.xyz().into();
        let dir = dir.vec3_normalize().unwrap();
        let up: gmm::Vector = world_trans.y_axis.xyz().into();
        let up = up.vec3_normalize().unwrap();
        gmm::Matrix::look_to_lh(eye, dir, up)
    }

    #[must_use]
    fn inv_camera_trans(&self) -> gmm::Matrix {
        let camera_trans = self.camera_trans();
        camera_trans.inverse().unwrap_or(gmm::Float4x4::IDENTITY.into())
    }

    #[inline]
    fn projection_trans(&self) -> gmm::Matrix {
        gmm::Matrix::perspective_lh(
            60f32.to_radians(), 
            16.0 / 9.0, 
            0.0001, 
            1000.0
        )
    }

    #[inline]
    fn inv_projection_trans(&self) -> gmm::Matrix {
        self.projection_trans().inverse().unwrap_or(gmm::Float4x4::IDENTITY.into())
    }

    #[inline]
    #[must_use]
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    #[inline]
    #[must_use]
    fn camera_uniform(&self) -> &CameraUniform {
        &self.camera_uniform
    }
}

impl fmt::Debug for ThirdPersonCamera {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(stringify!(ThirdPersonCamera))
            .field("name", &self.name)
            .field("target", &self.target)
            .finish()
    }
}
