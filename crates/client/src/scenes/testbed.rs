use std::{collections::HashMap, error::Error, net::TcpStream, sync::Arc};

use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::Player;
use mod_parallelism::collections::{Queue, SkipMap};
use mod_world::{component::{AnimationSet, Camera, GameObject, IdGenerator, Perspective, ThirdPersonCamera, Transform, WorldID}, render::{camera::CameraDataLayout, mesh::{BoneDataLayout, Mesh, MeshDataLayout}, pipeline::mesh::MeshRenderer}};
use winit::window::Window;

const METER_PER_PIXEL: f32 = 1.0 / 1000.0;
const BACKGROUND_COLOR: wgpu::Color = wgpu::Color {
    r: 0.0, 
    g: 116.0 / 255.0, 
    b: 183.0 / 255.0, 
    a: 1.0
};



/// TestBed Game Scene
pub struct TestBedScene {
    stream: Arc<TcpStream>, 
    stage_data: Vec<Player>, 

    client_id: u32, 

    /// 게임 오브젝트 식별자 생성기입니다.
    id_generator: Arc<IdGenerator>, 

    /// 게임 오브젝트를 관리하는 풀 객체입니다.
    world: Arc<SkipMap<WorldID, GameObject>>, 

    /// 메인 카메라의 게임 오브젝트 식별자입니다.
    main_camera: WorldID, 

    /// 플레이어 목록입니다.
    players: HashMap<u32, WorldID>, 

    /// 메쉬 렌더러 오브젝트를 관리합니다.
    renderer: Arc<Queue<Arc<dyn MeshRenderer>>>, 
}

impl TestBedScene {
    /// 새로운 `TestBedScene` 장면을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new<I>(
        stream: Arc<TcpStream>, 
        client_id: u32, 
        players: I, 
    ) -> Self 
    where 
        I: IntoIterator<Item = Player>, 
        I::IntoIter: ExactSizeIterator
    {   
        Self { 
            stream, 
            stage_data: players.into_iter().collect(), 
            client_id, 
            id_generator: IdGenerator::new(), 
            world: Arc::new(SkipMap::new()), 
            main_camera: WorldID::default(), 
            players: HashMap::with_capacity(10),
            renderer: Arc::new(Queue::new()), 
        }
    }

    /// 플레이어를 추가합니다.
    fn insert_player(
        &mut self, 
        data: Player, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue
    ) {
        // 플레이어 게임 오브젝트를 생성합니다.
        let mut object = GameObject::new(
            &self.id_generator, 
            format!("Player({})", &data.id), 
            None
        );

        // 모델 파일을 로드합니다.
        let (root_id, clips) = crate::model::spawn_aris_original_model(
            &self.world, 
            &self.renderer, 
            &self.id_generator, 
            &device, 
            &queue
        );

        // 로컬 변환 행렬(부모로 부터 변환 행렬)을 설정합니다.
        object.set_local_transform(
            gmm::Matrix::from_rotation_translation(
                data.rotation.into(), 
                data.translation.into()
            )
        );

        // 하위 오브젝트를 설정합니다.
        object.set_child(Some(root_id));

        // 게임 오브젝트에 애니메이션을 추가합니다.
        object.insert(AnimationSet {
            clips, 
            index: 0, 
            timer: 0.0
        });

        // 플레이어를 게임 세상에 추가합니다.
        let world_id = object.id().clone();
        self.players.insert(data.id, world_id.clone());
        self.world.insert(object.id().clone(), object);
    }

    /// 메인 카메라를 생성합니다.
    fn create_main_camera(
        &mut self, 
        window: &Window, 
        device: &wgpu::Device, 
    ) {
        // 플레이어 게임 오브젝트를 가져옵니다.
        let player_id = self.players.get(&self.client_id).unwrap();
        let player = self.world.get(player_id).unwrap();
        
        // 플레이어 게임 오브젝트의 월드 변환 행렬을 가져옵니다.
        let player_world_transform = player.get_world_transform().clone();

        // 카메라의 초기 위치를 설정합니다.
        let mut world_transform = player_world_transform.clone();
        let right = world_transform.get_right_vector();
        let up = world_transform.get_up_vector();
        let dir = world_transform.get_look_vector();
        let distance = up * 1.25 - dir * 2.0;
        world_transform.translate(distance);
        world_transform.rotate(gmm::Quaternion::from_axis_angle(right, 10f32.to_radians()));

        // 카메라 오브젝트를 생성합니다.
        let mut camera_object = GameObject::new(
            &self.id_generator, 
            "Main_Camera".to_string(), 
            None
        );

        // 카메라 오브젝트의 월드 변환 행렬을 설정합니다.
        camera_object.set_world_transform(world_transform);

        // 카메라 오브젝트에 원근 투영 변환 행렬을 추가합니다.
        let (width, height): (u32, u32) = window.inner_size().into();
        camera_object.insert(Perspective::new(
            width as f32 * METER_PER_PIXEL, 
            height as f32 * METER_PER_PIXEL, 
            0.001, 
            1000.0
        ));

        // 카메라 오브젝트에 카메라 요소를 추가합니다.
        camera_object.insert(Arc::new(Camera::new(
            Some(camera_object.name()), 
            device
        )));

        // 카메라 오브젝트에 대상 오브젝트 식별자를 추가합니다.
        camera_object.insert(ThirdPersonCamera { target: player_id.clone() });

        let world_id = camera_object.id().clone();
        self.world.insert(world_id.clone(), camera_object);
        self.main_camera = world_id;
    }
}

impl GameScene for TestBedScene {
    fn on_enter(
        &mut self, 
        window: &Window, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        // 플레이어들을 생성합니다.
        while let Some(data) = self.stage_data.pop() {
            self.insert_player(data, app.render_device(), app.render_queue());
        }

        // 메인 카메라를 생성합니다.
        self.create_main_camera(window, app.render_device());

        Ok(())
    }

    fn on_update(
        &mut self, 
        elapsed_time_sec: f32, 
        window: &Window, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let frame_rate = app.timer().frame_rate();
        window.set_title(&format!("Hello to Halo! (FPS: {})", frame_rate));

        // 애니메이션을 갱신합니다.
        for id in self.players.values() {
            let object = self.world.get(id).unwrap();
            let animations = object.get::<AnimationSet>().unwrap();
            
            let current = animations.clips.get(animations.index).unwrap();
            let keyframe = current.sample_animation(animations.timer);
            for skinning in keyframe.meshes() {
                for (index, id) in skinning.skinned_mesh.bones().iter().enumerate() {
                    self.world.get_mut(id).unwrap()
                        .set_local_transform(Transform(skinning.transforms[index].into()));
                }
            }

            update_hierarchy(&self.world, Transform::new(), id.clone());
        }


        Ok(())
    }

    fn on_prepare_draw(
        &self, 
        _window: &Window, 
        _surface: &wgpu::Surface, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        // 카메라를 준비합니다.
        if let Some(camera_object) = self.world.get(&self.main_camera) {
            let world_transform = camera_object.get_world_transform();
            let eye = world_transform.get_translation();
            let dir = world_transform.get_look_vector();
            let up = world_transform.get_up_vector();
            let camera_matrix = gmm::Matrix::look_to_lh(eye, dir, up);
            let projection_matrix = match camera_object.get::<Perspective>() {
                Some(perspective) => perspective.to_projection_matrix(), 
                None => gmm::Matrix::IDENTITY
            };
            let projection_matrix = gmm::Matrix::perspective_lh(
                60f32.to_radians(), 16.0 / 9.0, 0.001, 100.0
            );

            if let Some(camera) = camera_object.get::<Arc<Camera>>() {
                camera.camera_uniform().update(device, queue, CameraDataLayout {
                    proj_view: (projection_matrix * camera_matrix).store_float4x4(), 
                    position: eye.store_float3(), 
                    direction: dir.store_float3(), 
                    ..Default::default()
                });
            }
        }

        // 렌더러를 준비합니다.
        let mut render_list = Vec::new();
        while let Some(renderer) = self.renderer.pop() {
            if self.world.contains_key(renderer.game_object()) {
                match renderer.mesh() {
                    Mesh::NonSkinnedMesh(mesh) => {
                        let world_transform = self.world.get(renderer.game_object())
                            .unwrap()
                            .get_world_transform()
                            .clone();

                        mesh.mesh_uniform().update(device, queue, MeshDataLayout {
                            trans: world_transform.0.store_float4x4()
                        });
                    }, 
                    Mesh::SkinnedMesh(mesh) => {
                        let mut data = BoneDataLayout::default();
                        for (index, id) in mesh.bones().iter().enumerate() {
                            let bone_transform = self.world.get(id)
                                .expect("스키닝된 메쉬를 구성하는 게임 오브젝트를 찾을 수 없습니다!")
                                .get_world_transform()
                                .clone();
                            data[index] = bone_transform.0.store_float4x4();
                        }

                        mesh.bone_transforms_uniform().update(device, queue, data);
                    }
                }

                render_list.push(renderer);
            }
        }


        while let Some(renderer) = render_list.pop() {
            self.renderer.push(renderer);
        }


        Ok(())
    }

    fn on_draw(
        &self, 
        render_target_view: &wgpu::TextureView, 
        depth_stencil_view: &wgpu::TextureView, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();


        // 카메라 오브젝트의 카메라 요소를 가져옵니다.
        let camera = match self.world.get(&self.main_camera) {
            Some(camera) => {
                match camera.get::<Arc<Camera>>() {
                    Some(camera) => camera.clone(), 
                    None => {
                        log::warn!("카메라 오브젝트에 카메라 요소가 없습니다!");
                        return Ok(())
                    }
                }
            }, 
            None => {
                log::warn!("카메라가 게임 월드에 존재하지 않습니다!");
                return Ok(())
            }
        };


        // 렌더러를 준비합니다.
        let mut render_list = Vec::new();
        while let Some(renderer) = self.renderer.pop() {
            if self.world.contains_key(renderer.game_object()) {
                render_list.push(renderer);
            }
        }


        let mut encoder = device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor::default()
        );

        {
            let mut rpass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("RenderPass(TestBedScene)"), 
                    color_attachments: &[
                        Some(wgpu::RenderPassColorAttachment { 
                            view: render_target_view, 
                            resolve_target: None, 
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(BACKGROUND_COLOR), 
                                store: wgpu::StoreOp::Store
                            } 
                        }), 
                    ], 
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { 
                        view: depth_stencil_view, 
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0), 
                            store: wgpu::StoreOp::Store
                        }), 
                        stencil_ops: None 
                    }), 
                    timestamp_writes: None, 
                    occlusion_query_set: None
                }
            );

            for renderer in render_list.iter() {
                renderer.bind(&camera, &mut rpass);
                renderer.draw(&mut rpass);
            }
        }

        // 그리기 명령(Draw Call)을 명령 대기열에 제출합니다.
        queue.submit([encoder.finish()]);


        while let Some(renderer) = render_list.pop() {
            self.renderer.push(renderer);
        }

        Ok(())
    }
}

impl std::fmt::Debug for TestBedScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", stringify!(TestBedScene))
    }
}

fn update_hierarchy(
    world: &Arc<SkipMap<WorldID, GameObject>>, 
    parent: Transform, 
    id: WorldID
) {
    let mut object = world.get_mut(&id).unwrap();
    let local_transform = object.get_local_transform().clone();
    let world_transform = parent * local_transform;
    object.set_world_transform(world_transform);

    let sibling_id = object.get_sibling().cloned();
    let child_id = object.get_child().cloned();

    if let Some(sibling_id) = sibling_id {
        update_hierarchy(world, parent, sibling_id);
    }

    if let Some(child_id) = child_id {
        update_hierarchy(world, world_transform, child_id);
    }
}
