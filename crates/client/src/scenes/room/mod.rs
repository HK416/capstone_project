//! 커스텀 게임 장면과 관련된 코드를 작성합니다.
//!
use std::error::Error;

use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::{
    components::{CustomGamePlayer, LoginToken, UserId, WorldId},
    protocol::{CustomGamePullPacket, Packet, PacketType, RawPacket},
};
use mod_render::{ScreenDescriptor, UiRenderer};
use winit::window::Window;

use crate::{
    asset::{NOTOSANS_BOLD, NOTOSANS_REGULAR},
    config::{Locale, UserConfig, NUM_LOCALE},
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
}

impl CustomGameRoomScene {
    /// 새로운 `CustomGameRoomScene`을 생성합니다.
    ///
    /// # Panics
    /// `UserId` 또는 `LoginToken`이 NULL인 경우 `panic!`을 호출합니다.
    ///
    pub fn new<I>(world_id: WorldId, iter: I) -> Self
    where
        I: IntoIterator<Item = CustomGamePlayer>,
        I::IntoIter: ExactSizeIterator,
    {
        let config = UserConfig::get();
        let locale = config.locale;
        let user_id = config.info.uid;
        let token = config.token;
        drop(config);

        assert_ne!(user_id, UserId::NULL, "invalid user identifier");
        assert_ne!(token, LoginToken::NULL, "invalid login token");

        Self {
            locale,
            user_id,
            token,
            world_id,
            players: iter.into_iter().collect(),
        }
    }

    fn ui_callback(&mut self, window: &Window, egui_ctx: &egui::Context) {
        let (width, _height): (f32, f32) = window.inner_size().into();
        let scale_factor = window.scale_factor() as f32;
        let scale = width / scale_factor / BASE_WIDTH;

        // 폰트 속성
        let head_font_family = egui::FontFamily::Name(NOTOSANS_BOLD.into());
        let head_font_id = egui::FontId::new(32.0 * scale, head_font_family);
        let head_font_color = egui::Color32::WHITE;
        let main_font_family = egui::FontFamily::Name(NOTOSANS_REGULAR.into());
        let main_font_id = egui::FontId::new(24.0 * scale, main_font_family);
        let main_font_color = egui::Color32::WHITE;

        // 텍스트
        let i = self.locale as usize;
        let head_text = format!("{} - {}", HEAD_TEXTS[i], self.world_id);
        let head_text = egui::RichText::new(head_text)
            .font(head_font_id)
            .color(head_font_color);

        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(egui_ctx, |ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.vertical(|ui| {
                    ui.label(head_text);
                });
                ui.separator();
            });
    }
}

impl GameScene for CustomGameRoomScene {
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
}
