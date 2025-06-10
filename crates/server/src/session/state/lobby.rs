use std::sync::Arc;

use mod_network::{
    components::WorldId,
    protocol::{
        JoinFailedPacket, JoinFailedReason, JoinRequestPacket, LoginSuccessPacket, Packet,
        PacketType, QueryAvailableWorldsPacket, RawPacket, ResponseAvailableWorldsPacket,
    },
};

use crate::{account::Account, session::Session, token::UserTokenMap, world::GameWorldPool};

use super::{SessionState, SessionStateFlow};

/// 핑 측정 샘플의 최대 개수입니다.
const MAX_SAMPLES: usize = 30;

pub struct SessionLobbyState {
    /// 현재 시대 데이터
    epoch: u64,
    /// 핑 측정 샘플 데이터
    samples: [f32; MAX_SAMPLES],
    /// 샘플 개수입니다.
    num_samples: usize,

    /// 사용자 계정 데이터
    account: Account,
    /// 유효하지 않은 패킷 경고 횟수
    packet_warn_count: usize,
}

impl SessionLobbyState {
    /// 새로운 `LobbyState`를 생성합니다.
    pub fn new(account: Account) -> Self {
        Self {
            epoch: 0,
            samples: [0.0; MAX_SAMPLES],
            num_samples: 0,
            account,
            packet_warn_count: 0,
        }
    }

    /// 이용 가능 게임 월드 식별자 질의 패킷을 처리합니다.
    fn handle_query_available_worlds_packet(
        &mut self,
        session: &Arc<Session>,
        packet: QueryAvailableWorldsPacket,
    ) {
        // 사용자의 로그인 토큰을 검증합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token! (PACKET:{:?})",
                &session,
                &PacketType::QueryAvailableWorlds
            );
            session.close();
            return;
        }

        // 패킷을 생성하고 전송합니다.
        let worlds = GameWorldPool::get_available_world_ids();
        let packet = ResponseAvailableWorldsPacket::new(worlds);
        session.tcp_write(packet.as_raw());
    }

    /// 커스텀 게임 참여 요청 패킷을 처리합니다.
    fn handle_join_request_packet(&mut self, session: &Arc<Session>, packet: JoinRequestPacket) {
        // 사용자의 로그인 토큰을 검증합니다.
        if !UserTokenMap::is_valid(&(packet.uid, session.addr), packet.token) {
            log::error!(
                "{} invalid token! (PACKET:{:?})",
                &session,
                &PacketType::JoinRequest
            );
            session.close();
            return;
        }

        if packet.id == WorldId::NULL {
            // 커스텀 게임 생성을 시도합니다.
            let result =
                GameWorldPool::create_custom(self.account.uid, self.account.name, session.clone());
            match result {
                Some(world) => {
                    // 다음 세션 상태로 전환합니다.
                    // let state = Box::new(SessionRoomState::new(self.account.uid, world));
                    // let flow = SessionStateFlow::Push(state);
                    // session.flows.push(flow);
                }
                None => {
                    // 패킷을 생성하고 전송합니다.
                    let reason = JoinFailedReason::CreationLimited;
                    let packet = JoinFailedPacket::new(reason);
                    session.tcp_write(packet.as_raw());
                }
            }
        } else {
            // 커스텀 게임을 가져옵니다.
            let result = GameWorldPool::get(&packet.id);
            let world = match result {
                Some(world) => world,
                None => {
                    // 패킷을 생성하고 전송합니다.
                    let reason = JoinFailedReason::NotFound;
                    let packet = JoinFailedPacket::new(reason);
                    session.tcp_write(packet.as_raw());
                    return;
                }
            };

            // 커스텀 게임 참여를 시도합니다.
            // let result = world.try_join(self.account.uid, self.account.name, session.clone());
            // match result {
            //     Ok(_) => {
            //         // 다음 세션 상태로 전환합니다.
            //         // let state = Box::new(SessionRoomState::new(self.account.uid, world));
            //         // let flow = SessionStateFlow::Push(state);
            //         // session.flows.push(flow);
            //     }
            //     Err(reason) => {
            //         // 패킷을 생성하고 전송합니다.
            //         let packet = JoinFailedPacket::new(reason);
            //         session.tcp_write(packet.as_raw());
            //     }
            // };
        }
    }
}

impl SessionState for SessionLobbyState {
    fn on_enter(&mut self, session: &Arc<Session>) {
        // 로그인 토큰을 발행합니다.
        let token = UserTokenMap::alloc((self.account.uid, session.addr));

        // 패킷을 생성하고 전송합니다.
        let packet = LoginSuccessPacket::new(self.account.uid, self.account.name, token)
            .with_tier(self.account.tier);
        session.tcp_write(packet.as_raw());
    }

    fn on_exit(&mut self, session: &Arc<Session>) {
        // 발행한 로그인 토큰을 제거합니다.
        UserTokenMap::remove(&(self.account.uid, session.addr));
    }

    fn handle_packets(&mut self, session: &Arc<Session>, packet: RawPacket) {
        let packet_type = packet.packet_type();
        match packet_type {
            PacketType::QueryAvailableWorlds => {
                let packet = match QueryAvailableWorldsPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        log::error!(
                            "{} failed to convert packet! (PACKET:{:?})",
                            &session,
                            &packet_type,
                        );
                        session.close();
                        return;
                    }
                };

                self.handle_query_available_worlds_packet(session, packet);
            }
            PacketType::JoinRequest => {
                let packet = match JoinRequestPacket::try_from_raw(packet) {
                    Some(packet) => packet,
                    None => {
                        log::error!(
                            "{} failed to convert packet! (PACKET:{:?})",
                            &session,
                            &packet_type,
                        );
                        session.close();
                        return;
                    }
                };

                self.handle_join_request_packet(session, packet);
            }
            _ => {
                log::warn!(
                    "{} invalid packet received! (STATE:{:?}, PACKET:{:?})",
                    &session,
                    &self,
                    &packet_type,
                );

                // 유효하지 않은 패킷 경고 횟수를 증가시킵니다.
                self.packet_warn_count += 1;
                // 일정 횟수를 초과한 경우 세션을 종료시킵니다.
                const MAX_WARN_COUNT: usize = 0;
                if self.packet_warn_count > MAX_WARN_COUNT {
                    log::info!("{} closed after exceeding warning limit.", &session);
                    session.close();
                }
            }
        }
    }
}
