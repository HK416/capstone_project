//! 각 팀의 캐릭터를 편성하는 단계와 관련된 코드를 관리합니다.
//!

use std::sync::Arc;

use ahash::{HashMap, RandomState};
use mod_network::{
    components::{CharacterKind, FormationPhasePlayer, MAX_IN_GAME_PLAYERS},
    protocol::{FormationPullPacket, Packet, PacketType, RawPacket},
};
use mod_parallelism::collections::Queue;
use tokio::time::Instant;

use crate::{
    game::{GameWorld, GameWorldState, StateControlFlow},
    session::Session,
};

/// 최대 상태 지속 시간(초)
const MAX_STATE_DURATION: f32 = 60.0;

/// 각 팀에서 사용할 캐릭터를 편성하는 단계입니다.
pub struct CharacterFormationState {
    /// 이전 시각 데이터입니다.
    previous_time_point: Instant,
    /// 편성 완료까지 남은 시간
    remaining_timer_sec: f32,
    /// 캐릭터 중복 허용 여부
    allow_duplicates: bool,
    /// 캐릭터 선택 명령어 큐
    select_commands: Arc<Queue<(Arc<Session>, CharacterKind)>>,
    /// 플레이어 정보
    players: HashMap<Arc<Session>, FormationPhasePlayer>,
}

impl CharacterFormationState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new(allow_duplicates: bool) -> Self {
        Self {
            previous_time_point: Instant::now(),
            remaining_timer_sec: MAX_STATE_DURATION,
            allow_duplicates,
            select_commands: Arc::new(Queue::new()),
            players: HashMap::with_capacity_and_hasher(MAX_IN_GAME_PLAYERS, RandomState::default()),
        }
    }
}

impl GameWorldState for CharacterFormationState {
    fn on_enter(&mut self, world: &GameWorld) {
        // 락을 획득합니다.
        let players = world.players.read();
        for (session, player) in players.iter() {
            // 게임 월드 내 세션에게 다음 상태로 전환하라고 알립니다.
            let ptr = Arc::into_raw(self.select_commands.clone()) as usize;
            let data = ptr.to_be_bytes();
            let packet = RawPacket::new(PacketType::EnterFormationState, &data);
            session.push_received_packet(packet);

            // 플레이어 데이터를 초기화합니다.
            self.players.insert(
                session.clone(),
                FormationPhasePlayer::new(
                    player.account.clone(),
                    None,
                    false,
                    player.game_play.lock().team(),
                ),
            );
        }
    }

    fn on_advanced(&mut self, _flow: &mut Option<StateControlFlow>, world: &GameWorld) {
        // 경과 시간을 계산합니다.
        let current_time_point = Instant::now();
        let elapsed_time_sec = current_time_point
            .saturating_duration_since(self.previous_time_point)
            .as_secs_f32();
        self.previous_time_point = current_time_point;

        // 편성 완료 시간을 감소시킵니다.
        self.remaining_timer_sec = (self.remaining_timer_sec - elapsed_time_sec).max(0.0);

        {
            // 패킷을 생성합니다.
            let packet = FormationPullPacket::from_iter(
                self.remaining_timer_sec,
                self.players.values().cloned(),
            );

            // 패킷을 각 세션에 전송합니다.
            let players = world.players.read();
            for session in players.keys() {
                session.tcp_write(packet.as_raw());
            }
        }
    }
}
