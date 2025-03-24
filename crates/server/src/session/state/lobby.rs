use std::sync::Arc;

use mod_network::{
    components::{JoinFailedReason, UserAccount, WorldId},
    protocol::{
        CustomGameJoinFailedPacket, CustomGameJoinRequestPacket, CustomGameJoinSuccessPacket,
        Packet, PacketType, RawPacket,
    },
};

use crate::{game::GameWorldPool, session::Session, token::UserTokenMap};

use super::{ControlFlow, SessionState, room::RoomState};

#[derive(Debug)]
pub struct LobbyState {
    account: UserAccount,
}

impl LobbyState {
    /// 새로운 `LobbyState`를 생성합니다.
    pub fn new(user_info: UserAccount) -> Self {
        Self { account: user_info }
    }

    /// `CustomGameJoinRequestPacket`을 처리합니다.
    fn handle_custom_game_join_request_packet(
        &mut self,
        flow: &mut Option<ControlFlow>,
        session: &Arc<Session>,
        packet: RawPacket,
    ) {
        let packet = match CustomGameJoinRequestPacket::try_from_raw(packet) {
            Some(packet) => packet,
            None => {
                log::warn!("{} failed to convert packet!", session);
                session.close();
                return;
            }
        };

        // 수신한 패킷이 올바른지 검사합니다.
        let user_id = packet.user_id;
        let addr = session.addr;
        let token = packet.token;
        if !UserTokenMap::is_valid(&(user_id, addr), token) {
            log::warn!("{} invalid token (PACKET:{:?})", &session, &packet,);
            session.close();
            return;
        }

        if packet.world_id == WorldId::NULL {
            self.create_custom_game(flow, session, packet);
        } else {
            self.try_join_custom_game(flow, session, packet);
        }
    }

    /// 커스텀 게임 대기실을 생성합니다.
    fn create_custom_game(
        &mut self,
        flow: &mut Option<ControlFlow>,
        session: &Arc<Session>,
        _packet: CustomGameJoinRequestPacket,
    ) {
        // 새로운 커스텀 게임 대기실을 생성합니다.
        let (room, players) = GameWorldPool::create_custom(self.account, session);

        // 패킷을 생성합니다.
        let packet = CustomGameJoinSuccessPacket::new(room.id(), players);
        // 패킷을 전송합니다.
        session.tcp_write(packet.as_raw());

        // 다음 세션 상태로 전환합니다.
        let next_state = Box::new(RoomState::new(&room));
        *flow = Some(ControlFlow::Push(next_state));
    }

    /// 커스텀 게임 참여를 시도합니다.
    fn try_join_custom_game(
        &mut self,
        flow: &mut Option<ControlFlow>,
        session: &Arc<Session>,
        packet: CustomGameJoinRequestPacket,
    ) {
        // 해당 커스텀 게임 대기실을 가져옵니다.
        match GameWorldPool::get(&packet.world_id) {
            Some(world) => {
                // 커스텀 게임 대기실에 참가를 시도합니다.
                match world.try_join(self.account, session) {
                    Ok(players) => {
                        // 패킷을 생성합니다.
                        let packet = CustomGameJoinSuccessPacket::new(world.id(), players);
                        // 패킷을 전송합니다.
                        session.tcp_write(packet.as_raw());

                        // 다음 세션 상태로 전환합니다.
                        let next_state = Box::new(RoomState::new(&world));
                        *flow = Some(ControlFlow::Push(next_state));
                    }
                    Err(reason) => {
                        // 패킷을 생성합니다.
                        let packet = CustomGameJoinFailedPacket::new(reason);
                        // 패킷을 전송합니다.
                        session.tcp_write(packet.as_raw());
                    }
                };
            }
            None => {
                // 패킷을 생성합니다.
                let reason = JoinFailedReason::NotFound;
                let packet = CustomGameJoinFailedPacket::new(reason);
                // 패킷을 전송합니다.
                session.tcp_write(packet.as_raw());
            }
        };
    }
}

impl SessionState for LobbyState {
    fn handle_packets(&mut self, flow: &mut Option<ControlFlow>, session: &Arc<Session>) {
        while let Some(packet) = session.received_packets.pop() {
            let packet_type = packet.packet_type();
            match packet_type {
                PacketType::CustomGameJoinRequest => {
                    self.handle_custom_game_join_request_packet(flow, session, packet);
                }
                _ => {
                    log::warn!(
                        "{} invalid packet received! (STATE:{:?}, PACKET:{:?})",
                        &session,
                        &self,
                        &packet
                    );
                    session.close();
                    return;
                }
            }
        }
    }

    fn on_exit(&mut self, session: &Arc<Session>) {
        // 발행한 로그인 토큰을 제거합니다.
        let uid = self.account.uid;
        let addr = session.addr;
        UserTokenMap::remove(&(uid, addr));
    }
}
