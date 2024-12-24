use std::{error::Error, fmt, sync::Arc};

use ahash::HashMap;
use hecs::{Entity, EntityBuilder, World};
use mod_app::{app::AppHandle, scene::GameScene};
use mod_render::{CameraResource, ScreenDescriptor, UiRenderer, DEPTH_FORMAT, SWAPCHAIN_FORMAT};
use winit::{
    event::Modifiers,
    keyboard::{KeyCode, KeyLocation},
    window::Window,
};

use crate::{
    component::{
        CameraState, CameraTag, CharacterInvMass, ControlDelayTime, Direction, MaxCharacterSpeed,
        MovementState, Projection, ToParentTrans, WorldTransform,
    },
    config::UserConfig,
    system::{
        assist_player_character_translation, draw_character, prepare_camera_resource, prepare_mesh_resource, update_character_animation, update_character_animation_system, update_entity_hierarchy, update_player_character_animation_state, update_player_character_direction, update_player_direction, update_third_person_camera
    },
};

const BACKGROUND_COLOR: wgpu::Color = wgpu::Color {
    r: 0.0,
    g: 116.0 / 255.0,
    b: 183.0 / 255.0,
    a: 1.0,
};

pub struct TestbedInGameScene {
    user_config: Option<Box<UserConfig>>,

    /// 메인 카메라의 `Entity`
    main_camera: Entity,

    /// 유저의 클라이언트 식별자입니다.
    client_id: u32,

    /// 엔티티 목록입니다.
    entities: HashMap<u32, Entity>,

    world: World,

    /// 플레이어 방향입니다.
    direction: Direction,

    /// 플레이어의 움직임 상태입니다.
    movement_state: MovementState,

    /// 플레이어가 키보드를 누른 시간입니다.
    keyboard_input_time: ControlDelayTime,

    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl TestbedInGameScene {
    pub fn new(
        client_id: u32,
        entities: HashMap<u32, Entity>,
        world: World,
        user_config: Box<UserConfig>,
    ) -> Self {
        Self {
            user_config: Some(user_config),
            main_camera: Entity::DANGLING,
            client_id,
            entities,
            world,
            direction: Direction(glam::Vec4::new(0.0, 0.0, 1.0, 0.0)),
            movement_state: MovementState::Idle,
            keyboard_input_time: ControlDelayTime(0.0),
            egui_clip_primitives: Vec::new(),
            egui_free_texture_ids: Vec::new(),
        }
    }

    #[allow(unused_variables)]
    pub fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let egui_ctx = app.egui_ctx();
        let timer = app.timer();
        let frame_rate = timer.frame_rate();

        let frame_rate_text = egui::RichText::new(format!("FPS:{}", frame_rate))
            .color(egui::Color32::WHITE)
            .background_color(egui::Color32::from_black_alpha(128))
            .size(18.0);

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(egui_ctx, |ui| {
                ui.label(frame_rate_text);
            });

        Ok(())
    }

    pub fn prepare_ui(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();
        let egui_ctx = app.egui_ctx();
        let egui_raw_input = app.egui_raw_input();
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: window.inner_size().into(),
            pixels_per_point: window.scale_factor() as f32,
        };

        egui_ctx.begin_pass(egui_raw_input);
        self.ui_callback(window, app)?;
        let egui_full_output = egui_ctx.end_pass();

        let egui_primitive =
            egui_ctx.tessellate(egui_full_output.shapes, egui_full_output.pixels_per_point);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut commands = egui_renderer.update_buffers(
            device,
            queue,
            &mut encoder,
            &egui_primitive,
            &screen_descriptor,
        );
        commands.push(encoder.finish());
        queue.submit(commands);

        for (id, image_delta) in &egui_full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, image_delta);
        }

        self.egui_clip_primitives = egui_primitive;
        self.egui_free_texture_ids = egui_full_output.textures_delta.free;

        Ok(())
    }

    /// 메인 카메라를 생성합니다.
    fn create_main_camera(&mut self, window: &Window, app: &dyn AppHandle) {
        let (width, height): (f32, f32) = window.inner_size().into();

        let mut builder = EntityBuilder::new();
        builder.add(CameraTag);
        builder.add(CameraState::Idle);
        builder.add(ToParentTrans(glam::Mat4::from_translation(glam::vec3(
            0.25, 0.75, -1.2,
        ))));
        builder.add(WorldTransform::default());
        builder.add(Projection(glam::Mat4::perspective_lh(
            75f32.to_radians(),
            width / height,
            0.0001,
            1000.0,
        )));
        builder.add(Arc::new(CameraResource::uninit(
            Some("main_camera"),
            app.render_device(),
        )));

        log::debug!("TestbedInGameScene :: 메인 카메라를 생성");
        self.main_camera = self.world.spawn(builder.build());
    }
}

impl GameScene for TestbedInGameScene {
    #[allow(unused_variables)]
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.create_main_camera(window, app);
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_keyboard_pressed(
        &mut self,
        keycode: KeyCode,
        location: KeyLocation,
        modifiers: Modifiers,
        repeat: bool,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        if repeat || self.user_config.is_none() {
            return Ok(());
        }

        let user_config = self
            .user_config
            .as_ref()
            .expect("user configuration must exist");
        self.movement_state
            .handle_keyboard_pressed(&user_config, keycode, location);
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_keyboard_released(
        &mut self,
        keycode: KeyCode,
        location: KeyLocation,
        modifiers: Modifiers,
        repeat: bool,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        if repeat || self.user_config.is_none() {
            return Ok(());
        }

        let user_config = self
            .user_config
            .as_ref()
            .expect("user configuration must exist");
        self.movement_state
            .handle_keyboard_released(&user_config, keycode, location);
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_update(
        &mut self,
        elapsed_time_sec: f32,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        update_character_animation_system(
            app.asset_manager(),
            &self.world,
            elapsed_time_sec,
            rayon::current_num_threads() as u32,
        );
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_fixed_update(
        &mut self,
        fixed_time_sec: f32,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let player_entity = self
            .entities
            .get(&self.client_id)
            .cloned()
            .expect("no such entity");

        // 키보드 입력에 따라 플레이어 방향을 갱신합니다.
        update_player_direction(
            &mut self.direction,
            &self.movement_state,
            &mut self.keyboard_input_time,
            fixed_time_sec,
        );

        // 플레이어 캐릭터의 위치를 보정합니다.
        assist_player_character_translation(
            &mut self.world,
            player_entity,
            &self.direction,
            &CharacterInvMass(1.0 / 43.0),
            &MaxCharacterSpeed(1.5),
            &self.keyboard_input_time,
            fixed_time_sec,
        );

        // 플레이어 캐릭터의 방향을 갱신합니다.
        update_player_character_direction(
            &mut self.world, 
            player_entity, 
            &self.direction
        );

        // 플레이어 캐릭터의 애니메이션 상태 머신을 갱신합니다.
        update_player_character_animation_state(
            &mut self.world, 
            player_entity, 
            &self.movement_state
        );

        Ok(())
    }

    #[allow(unused_variables)]
    fn on_prepare_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let entities: Vec<Entity> = self.entities.values().cloned().collect();

        // 캐릭터 애니메이션을 갱신합니다.
        update_character_animation(app.asset_manager(), &mut self.world, &entities);

        // 엔터티 계층 구조를 갱신합니다.
        for &entity in entities.iter() {
            update_entity_hierarchy(&mut self.world, entity, glam::Mat4::IDENTITY);
        }

        // 카메라 위치를 갱신합니다.
        let target_entity = self.entities.get(&self.client_id).cloned().unwrap();
        update_third_person_camera(&mut self.world, target_entity, self.main_camera);

        prepare_mesh_resource(
            &self.world,
            &entities,
            app.render_device(),
            app.render_queue(),
        );
        prepare_camera_resource(
            &self.world,
            &[self.main_camera],
            app.render_device(),
            app.render_queue(),
        );
        self.prepare_ui(window, egui_renderer, app)?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_draw(
        &self,
        window: &Window,
        render_target_view: &wgpu::TextureView,
        depth_buffer_view: &wgpu::TextureView,
        egui_renderer: &UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let device = app.render_device();
        let queue = app.render_queue();

        let mut query = self
            .world
            .query_one::<&Arc<CameraResource>>(self.main_camera)
            .expect("invalid entity");
        let camera_resource = query.get().expect("invalid entity component");

        let entities: Vec<Entity> = self.entities.values().cloned().collect();

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("RenderPass(TestbadEnterScene)"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(BACKGROUND_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_buffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            draw_character(
                &self.world,
                &entities,
                camera_resource,
                device,
                SWAPCHAIN_FORMAT,
                DEPTH_FORMAT,
                &mut rpass,
            );

            egui_renderer.render(
                &mut rpass,
                &self.egui_clip_primitives,
                &ScreenDescriptor {
                    size_in_pixels: window.inner_size().into(),
                    pixels_per_point: window.scale_factor() as f32,
                },
            );
        }

        queue.submit([encoder.finish()]);
        Ok(())
    }

    #[allow(unused_variables)]
    fn on_finish_draw(
        &mut self,
        window: &Window,
        egui_renderer: &mut UiRenderer,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.egui_clip_primitives.clear();
        while let Some(id) = self.egui_free_texture_ids.pop() {
            egui_renderer.free_texture(&id);
        }

        Ok(())
    }
}

impl fmt::Debug for TestbedInGameScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(TestbedInGameScene))
    }
}
