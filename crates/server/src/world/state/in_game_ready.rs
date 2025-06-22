use std::sync::Arc;

use ahash::HashSet;
use mod_network::{
    components::{MAX_IN_GAME_PLAYERS, NetworkState, Permission, Team, UserId},
    protocol::{InGameReadyStatusPacket, Packet, PlayerReadyStatus},
};
use rand::seq::SliceRandom;

use crate::{
    session::Session,
    world::{
        GameWorld, GameWorldEvent, GameWorldInGameReadyStateEvent, GameWorldState,
        GameWorldSystemEvent,
    },
};

/// 최대 게임 로드 시간 (초)
pub const MAX_LOAD_TIME: f32 = 60.0;

/// 인게임 상태 게임 월드입니다.
/// 모든 플레이어의 로딩이 완료될 때 까지 대기합니다.
pub struct GameWorldInGameReadyState {
    /// 게임 로드 완료까지 남은 시간
    remaining_time_sec: f32,

    /// 패킷을 보낸 후 경과 시간
    elapsed_time_sec: f32,

    /// 블루 팀 플레이어 수
    num_blue_players: usize,
    /// 레드 팀 플레이어 수
    num_red_players: usize,
    /// 떠난 플레이어 식별자입니다.
    leaved_players: HashSet<UserId>,
}

impl GameWorldInGameReadyState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(
        num_blue_players: usize,
        num_red_players: usize,
        leaved_players: HashSet<UserId>,
    ) -> Self {
        Self {
            remaining_time_sec: MAX_LOAD_TIME,
            elapsed_time_sec: 0.0,
            num_blue_players,
            num_red_players,
            leaved_players,
        }
    }

    /// [`GameWorldSystemEvent::PlayerJoin`] 이벤트를 처리합니다.
    fn handle_player_join_event(&mut self, world: &GameWorld, session: Arc<Session>, _uid: UserId) {
        log::error!("{} attempted unauthorized access in {}", &session, &world,);
        eprintln!("{} attempted unauthorized access in {}", &session, &world,);
        session.close();
    }

    /// [`GameWorldSystemEvent::PlayerLeave`] 이벤트를 처리합니다.
    fn handle_player_leave_event(&mut self, world: &GameWorld, session: Arc<Session>, uid: UserId) {
        // 플레이어 데이터를 가져옵니다.
        // 현재 상태에서 플레이어 데이터를 제거하지 않습니다.
        let mut data = match world.players.get_mut(&uid) {
            Some(data) => data,
            None => {
                log::error!("Player({}) not found in {}!", &uid, &world);
                eprintln!("Player({}) not found in {}!", &uid, &world);
                session.close();
                return;
            }
        };

        // 플레이어 네트워크 상태를 변경합니다.
        data.set_network_state(NetworkState::Critical);

        // 플레이어의 권한을 해제합니다.
        let permission = data.permission();
        data.set_permission(Permission::User);

        // 플레이어가 속한 팀의 인원 수를 감소시킵니다.
        let team = data.team();
        match team {
            Team::Blue => {
                self.num_blue_players -= 1;
            }
            Team::Red => {
                self.num_red_players -= 1;
            }
        }

        // 떠난 플레이어 식별자를 추가합니다.
        let uid = data.key().clone();
        self.leaved_players.insert(uid);
        drop(data);

        // 제거된 플레이어의 권한이 관리자인 경우
        // 남은 플레이어 중 무작위로 한 명을 선정하여 권한을 넘겨줍니다.
        if permission == Permission::Admin {
            let mut remainings: Vec<_> = world
                .sessions
                .iter()
                .map(|data| data.value().clone())
                .collect();
            remainings.shuffle(&mut rand::rng());

            if let Some(uid) = remainings.pop() {
                match world.players.get_mut(&uid) {
                    Some(mut data) => {
                        world.set_admin(uid);
                        data.set_permission(Permission::Admin);
                    }
                    None => {
                        log::error!("Player({}) not found in {}!", &uid, &world);
                        eprintln!("Player({}) not found in {}!", &uid, &world);
                    }
                }
            }
        }
    }

    /// [`GameWorldInGameReadyStateEvent::ReadyToPlay`] 이벤트를 처리합니다.
    fn handle_ready_to_play_event(
        &mut self,
        world: &GameWorld,
        session: Arc<Session>,
        uid: UserId,
    ) {
        // 플레이어 데이터를 가져옵니다.
        let mut player = match world.players.get_mut(&uid) {
            Some(guard) => guard,
            None => {
                log::error!("Player({}) not found in {}!", &uid, &world);
                eprintln!("Player({}) not found in {}!", &uid, &world);
                session.close();
                return;
            }
        };

        // 플레이어를 준비 상태로 전환합니다.
        player.set_ready_to_play(true);
    }

    /// 다음 게임 월드 상태로 전환을 시도합니다.
    fn try_enter_next_state(&mut self, world: &Arc<GameWorld>) {
        // 락을 획득합니다. 락은 함수 종료 시점에 해제됩니다.
        // 주의: tokio에서 호출될 경우 스케쥴링 과정에서 데드락이 발생할 수 있습니다.
        let num_players = world.num_players.lock();

        // 남은 시간이 없는 경우
        if self.remaining_time_sec <= 0.0 {
            // 준비되지 않은 플레이어의 서버 연결을 해제합니다.
            for data in world.sessions.iter() {
                let session = data.key();
                let uid = data.value().clone();
                match world.players.get_mut(&uid) {
                    Some(data) => {
                        if !data.is_ready_to_play() {
                            self.leaved_players.insert(uid);
                            session.close();
                        }
                    }
                    None => {
                        log::error!("player data for the {} not found in {}!", &session, &world);
                        eprintln!("player data for the {} not found in {}!", &session, &world);
                        session.close();
                        continue;
                    }
                };
            }
        }

        // 모든 플레이어가 준비되었는지 확인합니다.
        let all_player_readys: bool = world
            .players
            .iter()
            .filter(|data| {
                let uid = data.key().clone();
                !self.leaved_players.contains(&uid)
            })
            .all(|data| data.is_ready_to_play());
        if all_player_readys {
            todo!()
        }

        drop(num_players);
    }

    /// 모든 세션에 패킷 데이터를 전송합니다.
    fn broadcast(&self, world: &GameWorld) {
        let mut players = Vec::with_capacity(MAX_IN_GAME_PLAYERS);
        for data in world.players.iter() {
            let uid = data.key().clone();
            let connected = !self.leaved_players.contains(&uid);
            players.push(PlayerReadyStatus::new(
                uid,
                connected,
                data.network_state(),
                data.is_ready_to_play(),
            ));
        }

        // 플레이어가 비어있는 경우 실행을 생략합니다.
        if players.is_empty() {
            return;
        }

        let packet = InGameReadyStatusPacket::new(self.remaining_time_sec, players);
        for data in world.sessions.iter() {
            let session = data.key();
            session.tcp_write(packet.as_raw());
        }
    }
}

impl GameWorldState for GameWorldInGameReadyState {
    fn on_enter(&mut self, world: &Arc<GameWorld>) {
        // 모든 플레이어의 준비 상태를 `false`로 설정합니다.
        for mut player in world.players.iter_mut() {
            player.set_ready_to_play(false);
        }
    }

    fn on_exit(&mut self, world: &Arc<GameWorld>) {
        // 떠난 플레이어 데이터를 정리합니다.
        for uid in self.leaved_players.iter() {
            world.players.remove(uid);
        }
    }

    fn handle_event(&mut self, world: &Arc<GameWorld>, event: GameWorldEvent) {
        match event {
            GameWorldEvent::System {
                session,
                uid,
                event,
            } => match event {
                GameWorldSystemEvent::PlayerJoin => {
                    self.handle_player_join_event(world, session, uid);
                }
                GameWorldSystemEvent::PlayerLeave => {
                    self.handle_player_leave_event(world, session, uid);
                }
            },
            GameWorldEvent::FormationState { .. } => { /* empty */ }
            GameWorldEvent::InGameReadyState {
                session,
                uid,
                event,
            } => match event {
                GameWorldInGameReadyStateEvent::ReadyToPlay => {
                    self.handle_ready_to_play_event(world, session, uid);
                }
            },
            _ => {
                log::warn!(
                    "ignored >> unused world event (EVENT:{:?}, STATE:{:?})",
                    &event,
                    &self
                );
            }
        }
    }

    fn on_advanced(&mut self, world: &Arc<GameWorld>, elapsed_time_sec: f32) {
        // 남은 시간을 갱신합니다.
        self.remaining_time_sec = (self.remaining_time_sec - elapsed_time_sec).max(0.0);
        // 경과 시간을 갱신합니다.
        self.elapsed_time_sec += elapsed_time_sec;

        // 일전 시각마다 패킷을 전송합니다.
        const TICK: f32 = 1.0 / 30.0;
        if self.elapsed_time_sec >= TICK {
            self.elapsed_time_sec = 0.0;
            self.broadcast(world);
        }

        self.try_enter_next_state(world);
    }
}
