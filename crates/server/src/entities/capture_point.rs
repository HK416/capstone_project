use mod_network::components::{ActionState, CapturePoint, Team};
use mod_physics::collision::Collider;

use crate::entities::Player;

pub struct CapturePointObject {
    /// 점령지 데이터
    capture_point: CapturePoint,
    /// 점령지 충돌체
    collider: Collider,
}

impl CapturePointObject {
    pub fn new(collider: Collider) -> Self {
        Self {
            capture_point: CapturePoint::default(),
            collider,
        }
    }

    /// 점령도를 갱신합니다.
    pub fn update<'a, I>(&mut self, players: I, elapsed_time_ms: u16)
    where
        I: Iterator<Item = &'a Player>,
    {
        // 점령지 안에 있는 플레이어 수를 계산합니다.
        let mut num_blue_players = 0;
        let mut num_red_players = 0;
        for player in players {
            if player.action_state != ActionState::Death {
                if self.collider.check_point_collision(&player.translation) {
                    match player.team() {
                        Team::Blue => {
                            num_blue_players += 1;
                        }
                        Team::Red => {
                            num_red_players += 1;
                        }
                    }
                }
            }
        }

        // 레드 팀만 점령지 안에 존재하는 경우
        if num_red_players > 0 && num_blue_players == 0 {
            let offset = num_red_players as u16;
            self.capture_point.set_capture_team(Some(Team::Red));
            self.capture_point.update(elapsed_time_ms, offset);
        }
        // 블루 팀만 점령지 안에 존재하는 경우
        else if num_blue_players > 0 && num_red_players == 0 {
            let offset = num_blue_players as u16;
            self.capture_point.set_capture_team(Some(Team::Blue));
            self.capture_point.update(elapsed_time_ms, offset);
        }
        // 점령지 안에 두 팀 모두 존재하거나, 또는 아무도 존재하지 않은 경우
        else {
            self.capture_point.set_capture_team(None);
        }
    }

    pub fn as_ref(&self) -> &CapturePoint {
        &self.capture_point
    }
}
