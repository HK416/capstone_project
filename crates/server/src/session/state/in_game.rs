use std::{
    fmt,
    sync::{Arc, Weak},
};

use mod_network::{
    components::UserAccount,
    protocol::{Packet, PacketType, PushStatusPacket, RawPacket},
};

use crate::{session::Session, token::UserTokenMap, world::GameWorld};

use super::SessionState;

pub struct SessionInGameState {
    /// 세션 상태 실행 여부
    is_running: bool,

    /// 사용자 계정 데이터
    #[allow(dead_code)]
    account: UserAccount,
    /// 연결된 게임 월드
    world: Weak<GameWorld>,
}

impl SessionInGameState {
    /// 샤로운 인게임 상태를 생성합니다.
    pub fn new(account: UserAccount, world: &Weak<GameWorld>) -> Self {
        Self {
            is_running: true,
            account,
            world: world.clone(),
        }
    }

    /// `PushStatusPacket`을 처리합니다.
    fn handle_push_status_packet(&mut self, session: &Arc<Session>, packet: RawPacket) {
        let packet = match PushStatusPacket::try_from_raw(packet) {
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

        if let Some(world) = self.world.upgrade() {
            if !world.access_mut(session, |player| {
                player.set_rotation(packet.rotation);
                player.set_direction(packet.direction);
                player.update_state(packet.input_flags);
                player.set_view(
                    packet.view_state,
                    packet.view_state_timer,
                    packet.view_rotation,
                );
            }) {
                log::warn!("{} accesses an invalid player", session);
                session.close();
                return;
            }
        } else {
            log::warn!("{} accesses an invalid world", session);
            session.close();
            return;
        }
    }
}

impl SessionState for SessionInGameState {
    fn handle_packets(&mut self, session: &Arc<Session>) {
        while let Some(packet) = session.received_packets.pop() {
            // 세션 상태가 실행 중이 아닌 경우 스킵합니다
            if !self.is_running {
                continue;
            }

            let packet_type = packet.packet_type();
            match packet_type {
                PacketType::PushSync => {
                    /* empty */
                },
                PacketType::PushStatus => {
                    self.handle_push_status_packet(session, packet);
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
            };
        }
    }
}

impl fmt::Debug for SessionInGameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionInGameState))
    }
}
