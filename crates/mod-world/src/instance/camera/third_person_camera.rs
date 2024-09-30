use std::sync::Arc;

use crate::{
    component::{
        camera_bind_group_layout, 
        ArenaID, 
        CameraObject, 
        GameObject, 
        IdGenerator, 
        Perspective, 
        Transform
    }, 
    render::{
        camera::CameraUniform, 
        light::{GlobalLightUniform, LocalLightUniform}
    }
};



pub struct ThirdPersonCamera {
    /// 게임 오브젝트의 식별자입니다.
    id: ArenaID, 

    /// 게임 오브젝트의 이름입니다.
    name: String, 


    /// 부모 게임 오브젝트입니다.
    parent: Option<ArenaID>, 

    /// 형제 게임 오브젝트입니다.
    sibling: Option<ArenaID>, 

    /// 자식 게임 오브젝트입니다.
    child: Option<ArenaID>, 


    /// 로컬 변환 행렬(부모로 부터 변환 행렬)입니다.
    local_transform: Transform, 

    /// 월드 변환 행렬입니다.
    world_transform: Transform, 


    /// 원근 투영 변환 행렬입니다.
    projection_matrix: gmm::Matrix, 

    /// 원근 투영 변환 행렬의 역행렬입니다.
    inv_projection_matrix: gmm::Matrix,


    /// 카메라 유니폼 버퍼입니다.
    camera_uniform: CameraUniform, 

    /// 지역 조명 유니폼 버퍼입니다.
    local_light_uniform: LocalLightUniform, 

    /// 카메라 바인드 그룹입니다.
    bind_group: wgpu::BindGroup, 

    
    /// 대상 게임 오브젝트의 식별자입니다.
    target: ArenaID, 
}

impl ThirdPersonCamera {
    /// 새로운 삼인칭 카메라를 생성합니다.
    #[must_use]
    pub fn new(
        device: &wgpu::Device, 
        id_generator: &Arc<IdGenerator>,
        projection: Perspective, 
        parent: Option<ArenaID>, 
        target: ArenaID, 
        name: String
    ) -> Self {
        let camera_uniform = CameraUniform::new(Some(&format!("CameraUniform({})", &name)), device);
        let local_light_uniform = LocalLightUniform::new(Some(&format!("LocalLightUniform({})", &name)), device);
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("BindGroup({})", &name)), 
                layout: &camera_bind_group_layout(device), 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0, 
                        resource: camera_uniform.as_entire_binding(), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: GlobalLightUniform::get(device).as_entire_binding(), 
                    }, 
                    wgpu::BindGroupEntry {
                        binding: 2, 
                        resource: local_light_uniform.as_entire_binding(), 
                    }, 
                ]
            }
        );

        Self { 
            id: id_generator.alloc(), 
            name, 
            parent, 
            sibling: None, 
            child: None, 
            local_transform: Transform::new(), 
            world_transform: Transform::new(), 
            projection_matrix: projection.to_projection_matrix(), 
            inv_projection_matrix: projection.to_projection_matrix().inverse(), 
            camera_uniform, 
            local_light_uniform, 
            bind_group, 
            target 
        }
    }

    /// 대상 게임 오브젝트의 식별자를 가져옵니다.
    #[inline]
    pub fn target(&self) -> &ArenaID {
        &self.target
    }
}

impl GameObject for ThirdPersonCamera {
    fn id(&self) -> &ArenaID {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_parent(&self) -> Option<&ArenaID> {
        self.parent.as_ref()
    }

    fn set_parent(&mut self, id: Option<ArenaID>) {
        self.parent = id;
    }

    fn get_sibling(&self) -> Option<&ArenaID> {
        self.sibling.as_ref()
    }

    fn set_sibling(&mut self, id: Option<ArenaID>) {
        self.sibling = id;
    }

    fn get_child(&self) -> Option<&ArenaID> {
        self.child.as_ref()
    }

    fn set_child(&mut self, id: Option<ArenaID>) {
        self.child = id;
    }

    fn get_local_transform(&self) -> &Transform {
        &self.local_transform
    }

    fn set_local_transform(&mut self, transform: Transform) {
        self.local_transform = transform;
    }

    fn get_world_transform(&self) -> &Transform {
        &self.world_transform
    }

    fn set_world_transform(&mut self, transform: Transform) {
        self.world_transform = transform;
    }
}

impl CameraObject for ThirdPersonCamera {
    fn camera_uniform(&self) -> &CameraUniform {
        &self.camera_uniform
    }

    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    fn camera_transform(&self) -> gmm::Matrix {
        let eye = self.world_transform.get_translation();
        let dir = self.world_transform.get_look_vector();
        let up = self.world_transform.get_up_vector();
        gmm::Matrix::look_to_lh(eye, dir, up)
    }

    fn inv_camera_transform(&self) -> gmm::Matrix {
        self.camera_transform().inverse()
    }

    fn projection_transform(&self) -> gmm::Matrix {
        self.projection_matrix.clone()
    }

    fn inv_projection_transform(&self) -> gmm::Matrix {
        self.inv_projection_matrix.clone()
    }
}
