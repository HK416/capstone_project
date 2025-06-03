use std::{fmt, sync::Arc};

use mod_network::{
    components::{JoinFailedReason, RecruitPhasePlayer, UserAccount, WorldId},
    protocol::{
        AvailableWorldsPacket, CustomGameJoinFailedPacket, CustomGameJoinRequestPacket,
        CustomGameJoinSuccessPacket, Packet, PacketType, RawPacket, RequestAvailableWorldsPacket,
    },
};

use crate::{
    session::{Session, SessionEvents},
    token::UserTokenMap,
    world::GameWorldPool,
};

use super::{SessionState, SessionStateFlow, room::SessionRoomState};

pub struct SessionLobbyState {
    account: UserAccount,
}

impl SessionLobbyState {
    /// 새로운 `LobbyState`를 생성합니다.
    pub fn new(account: UserAccount) -> Self {
        Self { account }
    }

    /// `CustomGameJoinRequestPacket`을 처리합니다.
    fn handle_custom_game_join_request_packet(
        &mut self,
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
            self.create_custom_game(session, packet);
        } else {
            self.try_join_custom_game(session, packet);
        }
    }

    /// 커스텀 게임 대기실을 생성합니다.
    fn create_custom_game(&mut self, session: &Arc<Session>, _packet: CustomGameJoinRequestPacket) {
        // 새로운 커스텀 게임 대기실을 생성합니다.
        let result =
            GameWorldPool::create_custom(self.account.uid, self.account.name, session.clone());
        if let Some(world) = result {
            // 플레이어 정보를 수집합니다.
            let players = world
                .iter_players()
                .map(|item| {
                    RecruitPhasePlayer::new(
                        item.account().clone(),
                        item.team(),
                        item.bool_flag(),
                        item.permission(),
                    )
                })
                .collect();

            // 패킷을 생성합니다.
            let packet = CustomGameJoinSuccessPacket::new(world.id(), players);
            // 패킷을 전송합니다.
            session.tcp_write(packet.as_raw());

            // 다음 세션 상태로 전환합니다.
            let next_state = Box::new(SessionRoomState::new(self.account.uid, &world));
            let control_flow = SessionStateFlow::Push(next_state);
            let event = SessionEvents::SetControlFlow(control_flow);
            session.push_event(event);
        } else {
        }
    }

    /// 커스텀 게임 참여를 시도합니다.
    fn try_join_custom_game(
        &mut self,
        session: &Arc<Session>,
        packet: CustomGameJoinRequestPacket,
    ) {
        // 해당 커스텀 게임 대기실을 가져옵니다.
        match GameWorldPool::get(&packet.world_id) {
            Some(world) => {
                // 커스텀 게임 대기실에 참가를 시도합니다.
                let result = world.try_join(self.account.uid, self.account.name, session.clone());
                match result {
                    Ok(()) => {
                        // 플레이어 정보를 수집합니다.
                        let players = world
                            .iter_players()
                            .map(|item| {
                                RecruitPhasePlayer::new(
                                    item.account().clone(),
                                    item.team(),
                                    item.bool_flag(),
                                    item.permission(),
                                )
                            })
                            .collect();

                        // 패킷을 생성합니다.
                        let packet = CustomGameJoinSuccessPacket::new(world.id(), players);
                        // 패킷을 전송합니다.
                        session.tcp_write(packet.as_raw());

                        // 다음 세션 상태로 전환합니다.
                        let next_state = Box::new(SessionRoomState::new(self.account.uid, &world));
                        let control_flow = SessionStateFlow::Push(next_state);
                        let event = SessionEvents::SetControlFlow(control_flow);
                        session.push_event(event);
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

impl SessionState for SessionLobbyState {
    fn handle_packets(&mut self, session: &Arc<Session>) {
        while let Some(packet) = session.received_packets.pop() {
            // 취소된 패킷의 경우 스킵합니다.
            if session.packet_canceled() {
                continue;
            }

            let packet_type = packet.packet_type();
            match packet_type {
                PacketType::CustomGameJoinRequest => {
                    self.handle_custom_game_join_request_packet(session, packet);
                }
                PacketType::RequestAvailableWorlds => {
                    let packet = RequestAvailableWorldsPacket::from_raw(packet);
                    if !UserTokenMap::is_valid(&(packet.user_id, session.addr), packet.token) {
                        log::warn!("{} invalid token (PACKET:{:?})", &session, &packet,);
                        session.close();
                        return;
                    }
                    let worlds = GameWorldPool::get_available_world_ids();
                    let packet = AvailableWorldsPacket::new(worlds);
                    session.tcp_write(packet.as_raw());
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

impl fmt::Debug for SessionLobbyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(SessionLobbyState))
    }
}
