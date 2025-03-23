use std::{error::Error, sync::Arc};

use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::{
    components::{LoginToken, UserId},
    protocol::{Packet, SyncDraftPacket},
};
use mod_parallelism::collections::Queue;
use winit::window::Window;

use crate::{
    asset::NOTOSANS_BOLD,
    config::{Locale, NUM_LOCALE},
    scenes::BASE_WIDTH,
    SERVER_TCP_ADDR,
};

/// 애플리케이션 표시 언어에 따른 `에셋을 로드할 때` 표시하는 텍스트
const LOAD_TEXTS: [&'static str; NUM_LOCALE] = ["Now Loading"];
/// 애플리케이션 표시 언어에 따른 `동기화할 때` 표시하는 텍스트
const WAIT_TEXTS: [&'static str; NUM_LOCALE] = ["다른 플레이어를 기다리는 중"];

/// 동기화 패킷을 전송하는 시각(초)
const SYNC_PACKET_TICK: f32 = 0.16;

/// 인 게임 장면에 진입하기 전 캐릭터를 편성하는 장면입니다.  
/// `InGameDraftScene`에서 사용할 에셋을 로드하고 다른 플레이어와 동기화합니다.
pub struct EnterCharacterFormationScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 현재 사용자 식별자
    user_id: UserId,
    /// 로그인 토큰
    token: LoginToken,

    /// 게임 장면의 경과 시간(초)
    elapsed_time_sec: f32,

    /// 작업 결과를 저장합니다.
    task_results: Arc<Queue<Result<(), Box<dyn Error + Send>>>>,
    /// 남은 작업의 수입니다.
    num_remaining_task: usize,
}

impl EnterCharacterFormationScene {
    /// 새로운 `InGameDraftEnterScene`을 생성합니다.
    pub fn new(locale: Locale, user_id: UserId, token: LoginToken) -> Self {
        Self {
            locale,
            user_id,
            token,
            elapsed_time_sec: 0.0,
            task_results: Arc::new(Queue::new()),
            num_remaining_task: 0,
        }
    }
}

impl GameScene for EnterCharacterFormationScene {
    fn on_update(
        &mut self,
        elapsed_time_sec: f32,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 작업 결과를 확인합니다.
        if let Some(result) = self.task_results.pop() {
            self.num_remaining_task -= 1;
            result?;
        }

        // 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed_time_sec;

        // 동기화 패킷을 게임 서버로 전송합니다.
        if self.elapsed_time_sec >= SYNC_PACKET_TICK {
            self.elapsed_time_sec %= SYNC_PACKET_TICK;

            // 패킷을 생성합니다.
            let packet =
                SyncDraftPacket::new(self.num_remaining_task as u16, self.user_id, self.token);

            // 패킷을 전송합니다.
            let net_manager = app.net_manager();
            let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
            socket.push_packet(packet.as_raw());
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
                label: Some(&format!(
                    "RenderPass({})",
                    stringify!(InGameDraftEnterScene)
                )),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
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
        let i = self.locale as usize;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());

        // 텍스트
        let text = match self.num_remaining_task == 0 {
            false => LOAD_TEXTS[i],
            true => WAIT_TEXTS[i],
        };
        let font_id = egui::FontId::new(32.0 * scale, head_font_family);
        let text = egui::RichText::new(text)
            .font(font_id)
            .color(egui::Color32::WHITE);

        egui::Area::new(egui::Id::new("Layout"))
            .anchor(egui::Align2::RIGHT_BOTTOM, (16.0 * scale, 16.0 * scale))
            .show(app.egui_ctx(), |ui| {
                ui.label(text);
            });

        Ok(())
    }
}
