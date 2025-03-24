//! 함께할 플레이어를 모집하는 단계와 관련된 코드를 관리합니다.
//!

use mod_network::{
    components::RecruitPhasePlayer,
    protocol::{CustomGamePullPacket, Packet},
};
use tokio::time::{Duration, Instant};

use crate::game::{GameWorld, GameWorldState, StateControlFlow};

/// 함께 게임을 플레이할 플레이어를 모집하는 단계입니다.
pub struct PlayerRecruitState {
    /// 이전 시각 데이터입니다.
    previous_time_point: Instant,
}

impl PlayerRecruitState {
    /// 새로운 게임 월드 상태를 생성합니다.
    pub fn new() -> Self {
        Self {
            previous_time_point: Instant::now(),
        }
    }
}

impl GameWorldState for PlayerRecruitState {
    fn on_advanced(&mut self, _flow: &mut Option<StateControlFlow>, world: &GameWorld) {
        // 경과 시간을 계산합니다.
        let current_time_point = Instant::now();
        let _elapsed_time_sec = current_time_point
            .saturating_duration_since(self.previous_time_point)
            .as_secs_f32();
        self.previous_time_point = current_time_point;

        {
            // 락을 획득합니다.
            let players = world.players.read();

            // 패킷을 생성합니다.
            let packet = CustomGamePullPacket::from_iter(players.values().map(|player| {
                let game = player.game_play.lock();
                RecruitPhasePlayer::new(
                    player.account.clone(),
                    game.team(),
                    game.is_ready(),
                    game.permission(),
                )
            }));

            // 패킷을 각 세션에 전송합니다.
            for session in players.keys() {
                session.tcp_write(packet.as_raw());
            }
        }

        // 다른 작업이 실행될 수 있도록 현재 스레드를 양보합니다.
        std::thread::sleep(Duration::from_millis(4));
    }
}
