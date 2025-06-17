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
use winit::window::Window;

use crate::{
    asset::{TexturePool, TextureViewPool},
    config::Locale,
    scenes::{
        lobby::{
            ERR_FULL_CAPACITY_TEXTS, ERR_IN_PROGRASS_TEXTS, ERR_LIMITS_TEXTS, ERR_NOT_FOUND_TEXTS,
            MSG_MODAL_TEXTS,
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

    /// 텍스처 풀 객체
    texture_pool: TexturePool,
    /// 텍스처 뷰 풀 객체
    texture_view_pool: TextureViewPool,
}

impl MainLobbyWaitLayer {
    /// 새로운 `MainLobbyWaitLayer`를 생성합니다.
    pub fn new(
        locale: Locale,
        uid: UserId,
        token: LoginToken,
        texture_pool: TexturePool,
        texture_view_pool: TextureViewPool,
    ) -> Self {
        Self {
            locale,
            uid,
            token,
            texture_pool,
            texture_view_pool,
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
        let next_scene = FatalErrorSceneLayer::new(self.locale, title, message);
        let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
        let event = AppEvent::AddGameSceneFlow(scene_flow);
        let event_loop_proxy = app.event_loop_proxy();
        event_loop_proxy.send_event(event).unwrap();
    }

    fn on_received_packet(&mut self, packet: RawPacket, app: &dyn AppHandle) -> Option<RawPacket> {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::RoomDataUpdate => {
                let packet = RoomDataUpdatePacket::from_raw(packet);

                // 게임 장면을 변경합니다.
                let next_scene = CustomGameRoomScene::new(
                    self.locale,
                    self.uid,
                    self.token,
                    packet.id,
                    self.texture_pool.clone(),
                    self.texture_view_pool.clone(),
                    packet.stage_kind(),
                    packet.allow_duplicates(),
                    packet.allow_unbalanced(),
                    packet.players,
                );
                let scene_flow = GameSceneFlow::Change(Box::new(next_scene));
                let event: AppEvent = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
            }
            PacketType::JoinRoomFailed => {
                // 패킷을 생성합니다
                let packet = JoinRoomFailedPacket::from_raw(packet);

                // 게임 장면을 변경합니다.
                let i = self.locale as usize;
                let next_scene = Box::new(MessageSceneLayer::new(
                    self.locale,
                    MSG_MODAL_TEXTS[i],
                    match packet.reason {
                        JoinFailedReason::NotFound => ERR_NOT_FOUND_TEXTS[i],
                        JoinFailedReason::FullCapacity => ERR_FULL_CAPACITY_TEXTS[i],
                        JoinFailedReason::InProgress => ERR_IN_PROGRASS_TEXTS[i],
                        JoinFailedReason::CreationLimited => ERR_LIMITS_TEXTS[i],
                    },
                    None,
                ));
                let scene_flow = GameSceneFlow::Change(next_scene);
                let event = AppEvent::AddGameSceneFlow(scene_flow);
                let event_loop_proxy = app.event_loop_proxy();
                event_loop_proxy.send_event(event).unwrap();
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
