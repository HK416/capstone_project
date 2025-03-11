use std::{error::Error, io::Cursor, sync::Arc};

use ddsfile::Dds;
use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_parallelism::collections::Queue;
use mod_render::{
    CameraDataLayout, CameraResource, SamplerPool, ScreenDescriptor, TexturePool, TextureViewPool,
    UiRenderer, DEPTH_FORMAT, SWAPCHAIN_FORMAT,
};
use rayon::ThreadPool;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{
    asset::{AssetError, NEXON_LV2_GOTHIC, NEXON_LV2_GOTHIC_BOLD},
    config::{Locale, UserConfig, NUM_LOCALE},
    render::{BackgroundDataLayout, BackgroundResource, LOGIN_PAD_BG, LOGIN_PAD_BG_DATA},
    scenes::{GameLoginTitleScene, BASE_WIDTH},
};

/// 장면 지속 시간(초)
const SCENE_DURATION: f32 = 5.0;
/// 장면 전환 지속 시간(초)
const FADE_IN_DURATION: f32 = 1.0;
/// 안내사항 텍스트가 사라지는 시간(초)
const DISAPPER_DURATION: f32 = 0.75;

/// 애플리케이션 표시 언어에 따른 Head 텍스트
const HEAD_TEXTS: [&'static str; NUM_LOCALE] = ["안내 사항"];
/// 애플리케이션 표시 언어에 따른 Main 텍스트
const MAIN_TEXTS: [&'static str; NUM_LOCALE] = ["이 게임은 Blue Archive의 2차 창작 게임이며,"];
/// 애플리케이션 표시 언어에 따른 Sub 텍스트
const SUB_TEXTS: [&'static str; NUM_LOCALE] =
    ["2025년 한국공학대학교 게임공학과 졸업 작품 목적으로 제작되었습니다."];

/// 게임 인트로 화면을 보여주는 장면입니다.  
/// 검은색 화면에서 하얀색 화면으로 전환되며(Fade in) 화면에 안내사항이 표시됩니다.
pub struct GameIntroNotifyScene {
    /// 애플리케이션 표시 언어
    locale: Locale,

    /// 작업 결과를 저장하는 대기열
    task_result: Arc<Queue<Result<Box<dyn GameScene>, Box<dyn Error + Send>>>>,
    /// 총 경과 시간입니다.
    total_time_sec: f32,

    //----- UI -----
    egui_clip_primitives: Vec<egui::ClippedPrimitive>,
    egui_free_texture_ids: Vec<egui::TextureId>,
}

impl GameIntroNotifyScene {
    /// 새로운 `GameIntroScene`을 생성합니다.
    pub fn new() -> Self {
        let config = UserConfig::get();
        Self {
            locale: config.locale,
            task_result: Arc::new(Queue::new()),
            total_time_sec: 0.0,
            egui_clip_primitives: Vec::default(),
            egui_free_texture_ids: Vec::default(),
        }
    }

    /// UI 콜백 함수
    fn ui_callback(&mut self, window: &Window, egui_ctx: &egui::Context) {
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NEXON_LV2_GOTHIC_BOLD.into());
        let main_font_family = egui::FontFamily::Name(NEXON_LV2_GOTHIC.into());
        let head_font_id = egui::FontId::new(64.0 * scale, head_font_family);
        let main_font_id = egui::FontId::new(48.0 * scale, main_font_family.clone());
        let sub_font_id = egui::FontId::new(24.0 * scale, main_font_family);
        let font_color = self.get_font_color();

        // 텍스트
        let i = self.locale as usize;
        let text = HEAD_TEXTS[i];
        let head_text = egui::RichText::new(text)
            .font(head_font_id)
            .color(font_color);
        let text = MAIN_TEXTS[i];
        let main_text = egui::RichText::new(text)
            .font(main_font_id)
            .color(font_color);
        let text = SUB_TEXTS[i];
        let sub_text = egui::RichText::new(text)
            .font(sub_font_id)
            .color(font_color);

        egui::Area::new(egui::Id::new("Layout"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(egui_ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_min_width(width);
                    ui.label(head_text);
                });
                ui.add_space(48.0 * scale);
                ui.vertical_centered(|ui| {
                    ui.set_min_width(width);
                    ui.label(main_text);
                });
                ui.add_space(12.0 * scale);
                ui.vertical_centered(|ui| {
                    ui.set_min_width(width);
                    ui.label(sub_text);
                });
            });
    }

    /// 배경 색상을 가져옵니다.
    fn get_background_color(&self) -> wgpu::Color {
        let s = self.total_time_sec.min(FADE_IN_DURATION) / FADE_IN_DURATION;
        let c = (s * s * (3.0 - 2.0 * s)) as f64; // Smooth Step
        wgpu::Color {
            r: c,
            g: c,
            b: c,
            a: 1.0,
        }
    }

    /// 폰트 색상을 가져옵니다.
    fn get_font_color(&self) -> egui::Color32 {
        let s = (self.total_time_sec - (SCENE_DURATION - DISAPPER_DURATION)).max(0.0)
            / DISAPPER_DURATION;
        let c = 1.0 - (s * s * (3.0 - 2.0 * s)) as f64; // Smooth Step
        egui::Color32::from_black_alpha((255.0 * c) as u8)
    }

    /// 다음 게임 장면을 생성합니다.
    fn build_next_scene(
        &self,
        thread_pool: &ThreadPool,
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
    ) {
        let task_result = self.task_result.clone();
        let device = device.clone();
        let queue = queue.clone();
        thread_pool.spawn(move || {
            let main_camera = Self::create_camera_resource(&device, &queue);
            let result = Self::create_background_resource(&device, &queue);
            match result {
                Ok(background) => {
                    let next_scene = Box::new(GameLoginTitleScene::new(main_camera, background));
                    task_result.push(Ok(next_scene));
                }
                Err(e) => {
                    task_result.push(Err(e));
                }
            }
        });
    }

    /// 카메라 쉐이더 리소스를 생성합니다.
    fn create_camera_resource(device: &wgpu::Device, queue: &wgpu::Queue) -> CameraResource {
        // 카메라 쉐이더 리소스를 생성합니다.
        let label = format!("MainCamera({})", stringify!(GameLoginTitleScene));
        let main_camera = CameraResource::uninit(Some(&label), device);

        // 카메라 쉐이더 리소스의 유니폼 버퍼를 갱신합니다.
        let proj_view = glam::Mat4::orthographic_lh(-1.0, 1.0, -1.0, 1.0, 0.0, 1.0);
        main_camera.camera_uniform.update(
            device,
            queue,
            CameraDataLayout {
                proj_view: proj_view.to_cols_array(),
                ..Default::default()
            },
        );

        main_camera
    }

    /// 배경 쉐이더 리소스를 생성합니다.
    fn create_background_resource(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<BackgroundResource, Box<dyn Error + Send>> {
        // 임베딩된 데이터로부터 텍스처를 생성합니다.
        let reader = Cursor::new(LOGIN_PAD_BG_DATA);
        let dds = Dds::read(reader).map_err(|e| {
            log::error!("failed to read texture file! (REASON:{e}");
            Box::new(e) as Box<dyn Error + Send>
        })?;

        // 텍스처를 생성합니다.
        let texture = TexturePool::get_or_init(LOGIN_PAD_BG, || {
            let texture = Arc::new(device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some(&format!("Texture({})", LOGIN_PAD_BG)),
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Bc7RgbaUnorm,
                    size: wgpu::Extent3d {
                        width: dds.get_width(),
                        height: dds.get_height(),
                        depth_or_array_layers: dds.get_depth(),
                    },
                    mip_level_count: dds.get_num_mipmap_levels(),
                    sample_count: 1,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &dds.data,
            ));
            Ok(texture)
        })
        .map_err(|e: AssetError| {
            log::error!("failed to create texture! (REASON:{e})");
            Box::new(e) as Box<dyn Error + Send>
        })?;

        // 텍스처 뷰와 텍스처 샘플러를 생성합니다.
        let texture_view =
            TextureViewPool::get_or_init(&texture, &wgpu::TextureViewDescriptor::default());
        let sampler = SamplerPool::get_or_init(&device, &wgpu::SamplerDescriptor::default());

        // 배경을 그리는 쉐이더 리소스를 생성합니다.
        let background = BackgroundResource::uninit(
            Some("LoginTitle"),
            &device,
            &texture_view,
            &sampler,
            SWAPCHAIN_FORMAT,
            DEPTH_FORMAT,
        );
        background.uniform_buffer.update(
            device,
            queue,
            BackgroundDataLayout {
                ratio: dds.get_width() as f32 / dds.get_height() as f32,
                ..Default::default()
            },
        );

        Ok(background)
    }
}

impl GameScene for GameIntroNotifyScene {
    fn on_enter(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 애플리케이션 창을 표시합니다.
        window.set_visible(true);
        self.build_next_scene(app.io_threads(), app.render_device(), app.render_queue());
        Ok(())
    }

    fn on_update(
        &mut self,
        elapsed_time_sec: f32,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 총 경과 시간을 갱신합니다.
        self.total_time_sec += elapsed_time_sec;

        if self.total_time_sec >= SCENE_DURATION {
            if let Some(next_scene) = self.task_result.pop() {
                // 다음 게임 장면으로 전환합니다.
                let next_scene = next_scene?;
                let scene_flow = GameSceneFlow::Change(next_scene);
                let event = AppEvent::SetGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
        }

        Ok(())
    }

    fn on_prepare_draw(
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
        self.ui_callback(window, egui_ctx);
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
        for (id, image_delta) in &egui_full_output.textures_delta.set {
            egui_renderer.update_texture(device, queue, *id, image_delta);
        }
        commands.push(encoder.finish());
        queue.submit(commands);

        self.egui_clip_primitives = egui_primitive;
        self.egui_free_texture_ids = egui_full_output.textures_delta.free;

        Ok(())
    }

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

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            ..Default::default()
        });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("RenderPass({})", stringify!(GameIntroNotifyScene))),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.get_background_color()),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_buffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

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

    fn on_finish_draw(
        &mut self,
        _window: &Window,
        egui_renderer: &mut UiRenderer,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.egui_clip_primitives.clear();
        while let Some(id) = self.egui_free_texture_ids.pop() {
            egui_renderer.free_texture(&id);
        }

        Ok(())
    }
}
