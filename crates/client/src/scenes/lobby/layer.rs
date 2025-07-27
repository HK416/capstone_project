use std::time::Instant;

use mod_app::{
    app::AppHandle,
    etc::AppEvent,
    net::NetworkError,
    scene::{GameScene, GameSceneFlow},
};
use mod_network::{
    components::{LoginToken, UserId, WorldId},
    protocol::{
        JoinFailedReason, JoinRoomFailedPacket, JoinRoomRequestPacket, Packet, PacketType,
        RawPacket, RoomDataUpdatePacket,
    },
};
use mod_render::UiRenderer;
use rodio::Sink;
use winit::window::Window;

use crate::{
    asset::{SoundDataPool, TexturePool, TextureViewPool, UI_NOTICE},
    config::Locale,
    scenes::{
        lobby::{
            ERR_BANNED_TEXTS, ERR_FULL_CAPACITY_TEXTS, ERR_IN_PROGRASS_TEXTS, ERR_LIMITS_TEXTS,
            ERR_NOT_FOUND_TEXTS, MSG_MODAL_TEXTS,
        },
        CustomGameRoomScene, FatalErrorSceneLayer, MessageSceneLayer, ERR_CLOSED_MSG_TEXTS,
        ERR_IO_MSG_TEXTS, ERR_NETWORK_TITLE_TEXTS,
    },
    SERVER_TCP_ADDR,
};

/// 게임의 메인 로비 화면에서 서버 응답을 대기합니다.
pub struct MainLobbyWaitLayer {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 사용자 식별자
    uid: UserId,
    /// 로그인 토큰
    token: LoginToken,
    /// 배경음 음량
    background_volume: u8,
    /// 이펙트 음량
    effect_volume: u8,
    /// 목소리 음량
    voice_volume: u8,

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,

    /// 사운드 데이터 풀 객체
    sound_data_pool: SoundDataPool,
}

impl MainLobbyWaitLayer {
    /// 새로운 `MainLobbyWaitLayer`를 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        background_volume: u8,
        effect_volume: u8,
        voice_volume: u8,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
        sound_data_pool: SoundDataPool,
    ) -> Self {
        Self {
            locale,
            uid,
            token,
            background_volume,
            effect_volume,
            voice_volume,
            texture_pool,
            texture_view_pool,
            sound_data_pool,
        }
    }
}

impl GameScene for MainLobbyWaitLayer {
    fn transparents(&self) -> bool {
        true
    }

    fn on_enter(&mut self, _window: &Window, app: &dyn AppHandle, _ui_renderer: &mut UiRenderer) {
        // 커스텀 게임 생성 패킷을 생성합니다.
        let packet = JoinRoomRequestPacket::new(WorldId::NULL, self.uid, self.token);

        // 패킷을 전송합니다.
        let net_manager = app.net_manager();
        let socket = net_manager.get(&SERVER_TCP_ADDR).unwrap();
        socket.push_packet(packet.as_raw());
        return;
    }

    fn handle_network_error(&mut self, error: NetworkError, app: &dyn AppHandle) {
        let i = self.locale as usize;
        let title = ERR_NETWORK_TITLE_TEXTS[i];
        let message = match error {
            NetworkError::ClosedSocket(_) => ERR_CLOSED_MSG_TEXTS[i],
            NetworkError::IO(_) => ERR_IO_MSG_TEXTS[i],
        };

        // 다음 게임 장면으로 전환합니다.
        let next_scene = FatalErrorSceneLayer::new(
            self.locale,
            self.background_volume,
            self.effect_volume,
            self.voice_volume,
            title,
            message,
            self.sound_data_pool.clone(),
        );
        let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();

        // 효과음을 재생합니다.
        let decoded = self
            .sound_data_pool
            .get(UI_NOTICE)
            .expect("UI_Notice sound must be preloaded!");
        let source = decoded.as_source();
        let sink = Sink::connect_new(app.audio_mixer());
        sink.set_volume(self.effect_volume as f32 / 255.0);
        sink.append(source);
        sink.play();
        sink.detach();
    }

    fn on_received_packet(
        &mut self,
        _: Instant,
        packet: RawPacket,
        app: &dyn AppHandle,
    ) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::RoomDataUpdate => {
                let packet = RoomDataUpdatePacket::from_raw(packet);

                // 게임 장면을 변경합니다.
                let next_scene = CustomGameRoomScene::new(
                    self.locale,
                    self.uid,
                    self.token,
                    self.background_volume,
                    self.effect_volume,
                    self.voice_volume,
                    packet.id,
                    self.texture_pool.clone(),
                    self.texture_view_pool.clone(),
                    self.sound_data_pool.clone(),
                    packet.stage_kind(),
                    packet.allow_duplicates(),
                    packet.allow_unbalanced(),
                    packet.players,
                );
                let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
                let event: AppEvent = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();

                // 현재 재생 중인 배경음을 중단합니다.
                while let Some(sink) = app.sink_list().pop() {
                    sink.stop();
                }
            }
            PacketType::JoinRoomFailed => {
                // 패킷을 생성합니다
                let packet = JoinRoomFailedPacket::from_raw(packet);

                // 게임 장면을 변경합니다.
                let i = self.locale as usize;
                let next_scene = Box::new(MessageSceneLayer::new(
                    self.locale,
                    self.background_volume,
                    self.effect_volume,
                    self.voice_volume,
                    MSG_MODAL_TEXTS[i],
                    match packet.reason {
                        JoinFailedReason::NotFound => ERR_NOT_FOUND_TEXTS[i],
                        JoinFailedReason::FullCapacity => ERR_FULL_CAPACITY_TEXTS[i],
                        JoinFailedReason::InProgress => ERR_IN_PROGRASS_TEXTS[i],
                        JoinFailedReason::CreationLimited => ERR_LIMITS_TEXTS[i],
                        JoinFailedReason::Banned => ERR_BANNED_TEXTS[i],
                    },
                    None,
                    self.sound_data_pool.clone(),
                ));
                let scene_flow = GameSceneFlow::Change(next_scene);
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();

                // 효과음을 재생합니다.
                let decoded = self
                    .sound_data_pool
                    .get(UI_NOTICE)
                    .expect("UI_Notice sound must be preloaded!");
                let source = decoded.as_source();
                let sink = Sink::connect_new(app.audio_mixer());
                sink.set_volume(self.effect_volume as f32 / 255.0);
                sink.append(source);
                sink.play();
                sink.detach();
            }
            PacketType::LobbyDataUpdate => return Some(packet),
            _ => {
                log::warn!(
                    "packet ignored: invalid packet received! (TYPE:{:?})",
                    packet_type
                );
            }
        }

        None
    }
}
