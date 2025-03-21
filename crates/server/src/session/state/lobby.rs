use std::sync::Arc;

use mod_network::{
    components::{JoinFailedReason, WorldId},
    protocol::{
        CustomGameJoinFailedPacket, CustomGameJoinRequestPacket, CustomGameJoinSuccessPacket,
        Packet, PacketType,
    },
};

use crate::{room::CustomGamePool, session::Session, token::UserTokenMap};

use super::{ControlFlow, SessionState, room::RoomState};

#[derive(Debug)]
pub struct LobbyState;

impl LobbyState {
    /// `CustomGameJoinRequestPacket`을 처리합니다.
    fn handle_custom_game_join_request_packet(
        &mut self,
        packet: CustomGameJoinRequestPacket,
        flow: &mut Option<ControlFlow>,
        session: &Arc<Session>,
    ) {
        // 수신한 패킷이 올바른지 검사합니다.
        let user_id = packet.user_id;
        let addr = session.addr;
        let token = packet.token;
        if !UserTokenMap::verify(&(user_id, addr), token) {
            log::warn!("{} invalid token (PACKET:{:?})", &session, &packet,);
            session.close();
            return;
        }

        let pool = CustomGamePool::get_instance();
        if packet.world_id == WorldId::NULL {
            // `WorldId::NULL`인 경우 새로운 커스텀 게임 대기실을 생성합니다.
            let (room, players) = pool.create(session);

            // `CustomGameJoinSuccessPacket`을 생성 후 전송합니다.
            let packet = CustomGameJoinSuccessPacket::new(room.id(), players);
            session.tcp_write(packet.as_raw());

            // 세션 상태를 변경합니다.
            let next_state = Box::new(RoomState::new(&room));
            *flow = Some(ControlFlow::Push(next_state));
        } else {
            // 주어진 `WorldId`에 해당하는 커스텀 게임 대기실을 찾습니다.
            match pool.get(&packet.world_id) {
                Some(room) => match room.join(session) {
                    Ok(players) => {
                        // `CustomGameJoinSuccessPacket`을 생성 후 전송합니다.
                        let packet = CustomGameJoinSuccessPacket::new(room.id(), players);
                        session.tcp_write(packet.as_raw());

                        // 세션 상태를 변경합니다.
                        let next_state = Box::new(RoomState::new(&room));
                        *flow = Some(ControlFlow::Push(next_state));
                    }
                    Err(reason) => {
                        // `CustomGameJoinFailedPacket`을 생성 후 전송합니다.
                        let packet = CustomGameJoinFailedPacket::new(reason);
                        session.tcp_write(packet.as_raw());
                    }
                },
                None => {
                    // `CustomGameJoinFailedPacket`을 생성 후 전송합니다.
                    let reason = JoinFailedReason::NotFound;
                    let packet = CustomGameJoinFailedPacket::new(reason);
                    session.tcp_write(packet.as_raw());
                }
            }
        }
    }
}

impl SessionState for LobbyState {
    fn handle_packets(&mut self, flow: &mut Option<ControlFlow>, session: &Arc<Session>) {
        while let Some(packet) = session.received_packets.pop() {
            let packet_type = packet.packet_type();
            match packet_type {
                PacketType::CustomGameJoinRequest => {
                    match CustomGameJoinRequestPacket::try_from_raw(packet) {
                        Some(packet) => {
                            self.handle_custom_game_join_request_packet(packet, flow, session)
                        }
                        None => {
                            log::warn!("{} failed to convert packet!", session);
                            session.close();
                            return;
                        }
                    }
                }
                _ => {
                    log::warn!("{} invalid packet received (PACKET:{:?})", session, packet);
                    session.close();
                    return;
                }
            }
        }
    }
}
