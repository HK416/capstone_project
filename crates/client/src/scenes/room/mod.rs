//! 커스텀 게임 장면과 관련된 코드를 작성합니다.
//!
use std::error::Error;

use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::{
    components::{CustomGamePlayer, LoginToken, UserId, WorldId},
    protocol::{CustomGamePullPacket, Packet, PacketType, RawPacket},
};
use mod_render::{TexturePool, TextureViewPool};
use winit::window::Window;

use crate::{
    asset::{BG_MAIN_LOBBY_URI, NOTOSANS_BOLD},
    config::{Locale, NUM_LOCALE},
};

use super::BASE_WIDTH;

/// 애플리케이션 표시 언어에 따른 Head 텍스트
const HEAD_TEXTS: [&'static str; NUM_LOCALE] = ["커스텀 게임 대기실"];

/// 커스텀 게임 대기실 장면입니다.
pub struct CustomGameRoomScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 현재 클라이언트의 사용자 식별자입니다.
    user_id: UserId,
    /// 현재 클라이언트의 로그인 토큰입니다.
    token: LoginToken,

    /// 커스텀 게임 대기실의 월드 식별자입니다.
    world_id: WorldId,
    /// 현재 커스텀 게임에 참가한 플레이어 목록입니다.  
    /// 사용자 식별자의 오름차순으로 정렬됩니다.
    players: Vec<CustomGamePlayer>,

    /// 배경화면 텍스처의 식별자입니다.
    bg_texture_id: egui::load::SizedTexture,
}

impl CustomGameRoomScene {
    /// 새로운 `CustomGameRoomScene`을 생성합니다.
    ///
    /// # Panics
    /// `UserId` 또는 `LoginToken`이 NULL인 경우 `panic!`을 호출합니다.
    ///
    pub fn new<I>(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        world_id: WorldId,
        iter: I,
    ) -> Self
    where
        I: IntoIterator<Item = CustomGamePlayer>,
        I::IntoIter: ExactSizeIterator,
    {
        assert_ne!(user_id, UserId::NULL, "invalid user identifier");
        assert_ne!(token, LoginToken::NULL, "invalid login token");

        Self {
            locale,
            user_id,
            token,
            world_id,
            players: iter.into_iter().collect(),
            bg_texture_id: egui::load::SizedTexture {
                id: egui::TextureId::User(0),
                size: egui::Vec2::ZERO,
            },
        }
    }
}

impl GameScene for CustomGameRoomScene {
    fn on_enter(
        &mut self,
        _window: &Window,
        app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        // 메인 로비 배경화면 텍스처를 가져옵니다.
        let texture =
            TexturePool::get(BG_MAIN_LOBBY_URI).expect("BG_Main_Lobby texture must be preloaded!");
        let texture_size = egui::vec2(texture.width() as f32, texture.height() as f32);

        // 메인 로비 배경화면 텍스처의 텍스처 뷰를 생성합니다.
        let texture =
            TextureViewPool::get_or_init(&texture, &wgpu::TextureViewDescriptor::default());

        // egui 렌더러에 텍스처를 등록합니다.
        let mut egui_renderer = app.egui_renderer_mut();
        let texture_id = egui_renderer.register_native_texture(
            app.render_device(),
            &texture,
            wgpu::FilterMode::Linear,
        );

        // 등록된 텍스처 정보를 저장합니다.
        self.bg_texture_id = egui::load::SizedTexture {
            id: texture_id,
            size: texture_size,
        };

        Ok(())
    }

    fn on_received_packet(
        &mut self,
        packet: RawPacket,
        _app: &dyn AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::CustomGamePull => {
                let packet = CustomGamePullPacket::from_raw(packet);
                self.players = packet.players;
                self.players
                    .sort_by(|lhs, rhs| lhs.info.uid.cmp(&rhs.info.uid));
            }
            _ => {
                log::warn!("invalid packet received! (TYPE:{:?})", packet_type);
            }
        };

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
                label: Some(&format!("RenderPass({})", stringify!(CustomGameRoomScene))),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
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
        let (width, height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let head_font_id = egui::FontId::new(32.0 * scale, head_font_family);
        let head_font_color = egui::Color32::DARK_GRAY;

        // 텍스트
        let i = self.locale as usize;
        let head_text = format!("{} - {}", HEAD_TEXTS[i], self.world_id);
        let head_text = egui::RichText::new(head_text)
            .font(head_font_id)
            .color(head_font_color);

        // 배경화면
        let source = self.bg_texture_id;
        let ratio = source.size.x / source.size.y;
        let center_x = width * 0.5;
        let center_y = height * 0.5;
        let img_width = width;
        let img_height = img_width / ratio;
        let rect = egui::Rect {
            min: egui::pos2(
                (center_x - 0.5 * img_width) / scale_factor,
                (center_y - 0.5 * img_height) / scale_factor,
            ),
            max: egui::pos2(
                (center_x + 0.5 * img_width) / scale_factor,
                (center_y + 0.5 * img_height) / scale_factor,
            ),
        };

        egui::Area::new(egui::Id::new("World_Id"))
            .anchor(egui::Align2::LEFT_TOP, (32.0 * scale, 8.0 * scale))
            .show(app.egui_ctx(), |ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(head_text);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(app.egui_ctx(), |ui| {
                egui::Image::new(source).paint_at(ui, rect);
            });

        Ok(())
    }
}
