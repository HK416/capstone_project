use std::{error::Error, io::Cursor, sync::Arc};

use image::{ImageFormat, ImageReader};
use mod_app::{
    app::AppHandle,
    asset::AssetManager,
    etc::AppEvent,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::components::{LoginToken, UserInfo};
use mod_parallelism::collections::Queue;
use mod_render::TexturePool;
use rayon::ThreadPool;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{
    asset::{BG_MAIN_LOBBY_URI, NOTOSANS_BOLD},
    config::Locale,
    scenes::BASE_WIDTH,
};

use super::MainLobbyScene;

pub struct MainLobbyEnterScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 사용자 계정 정보
    user_info: UserInfo,
    /// 로그인 토큰
    token: LoginToken,

    /// 작업 결과
    task_results: Arc<Queue<Result<(), Box<dyn Error + Send>>>>,
    /// 남은 작업의 수
    num_remaining_tasks: usize,
}

impl MainLobbyEnterScene {
    /// 새로운 `MainLobbyEnterScene`을 생성합니다.
    pub fn new(locale: Locale, user_info: UserInfo, token: LoginToken) -> Self {
        Self {
            locale,
            user_info,
            token,
            task_results: Arc::new(Queue::new()),
            num_remaining_tasks: 0,
        }
    }

    /// `MainLobby`의 배경 텍스처를 풀 객체에 등록합니다.
    fn regist_background_texture(
        &mut self,
        thread_pool: &ThreadPool,
        asset_manager: &AssetManager,
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
    ) {
        let task_results = self.task_results.clone();
        let asset_manager = asset_manager.clone();
        let device = device.clone();
        let queue = queue.clone();
        thread_pool.spawn(move || {
            // 에셋을 로드합니다.
            let result = asset_manager.get_or_init(BG_MAIN_LOBBY_URI);
            let asset = match result {
                Ok(asset) => asset,
                Err(e) => {
                    log::error!("failed to load asset! (PATH:{BG_MAIN_LOBBY_URI}, REASON:{e})");
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 이미지를 디코딩합니다.
            let reader = Cursor::new(asset.as_bytes());
            let mut reader = ImageReader::new(reader);
            reader.set_format(ImageFormat::Png);

            let image = match reader.decode() {
                Ok(image) => image,
                Err(e) => {
                    log::error!("failed to load texture! (PATH:{BG_MAIN_LOBBY_URI}, REASON:{e}");
                    task_results.push(Err(Box::new(e)));
                    return;
                }
            };

            // 캐싱된 에셋을 제거합니다.
            asset_manager.remove(BG_MAIN_LOBBY_URI);

            // 텍스처를 생성합니다.
            let texture = device.create_texture_with_data(
                &queue,
                &wgpu::TextureDescriptor {
                    label: Some(&format!("Texture({})", BG_MAIN_LOBBY_URI)),
                    size: wgpu::Extent3d {
                        width: image.width(),
                        height: image.height(),
                        depth_or_array_layers: 1,
                    },
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::default(),
                &image.to_rgba8(),
            );

            // 텍스처 풀 객체에 등록합니다.
            TexturePool::register(BG_MAIN_LOBBY_URI.into(), texture.into());

            // 결과를 전송합니다.
            task_results.push(Ok(()));
        });
        self.num_remaining_tasks += 1;
    }
}

impl GameScene for MainLobbyEnterScene {
    fn on_enter(
        &mut self,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        self.regist_background_texture(
            app.io_threads(),
            app.asset_manager(),
            app.render_device(),
            app.render_queue(),
        );
        Ok(())
    }

    fn on_update(
        &mut self,
        _elapsed_time_sec: f32,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 작업 결과를 확인합니다.
        if let Some(result) = self.task_results.pop() {
            self.num_remaining_tasks -= 1;
            result?;
        }

        // 다음 게임 장면으로 전환합니다.
        if self.num_remaining_tasks == 0 {
            let next_scene = Box::new(MainLobbyScene::new(self.locale, self.user_info, self.token));
            let scene_flow = GameSceneFlow::Change(next_scene);
            let event = AppEvent::SetGameSceneFlow(scene_flow);
            let event_loop_proxy = app.event_loop_proxy();
            event_loop_proxy.send_event(event).unwrap();
        }

        Ok(())
    }

    fn on_draw(
        &self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("RenderPass({})", stringify!(MainLobbyEnterScene))),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        Ok(())
    }

    fn ui_callback(
        &mut self,
        window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());

        // 텍스트
        let loading_text_id = egui::FontId::new(32.0 * scale, head_font_family);
        let loading_text = egui::RichText::new("Now Loading")
            .font(loading_text_id)
            .color(egui::Color32::WHITE);

        egui::Area::new(egui::Id::new("Layout"))
            .anchor(egui::Align2::RIGHT_BOTTOM, (-32.0 * scale, -32.0 * scale))
            .show(app.egui_ctx(), |ui| ui.label(loading_text));

        Ok(())
    }
}
