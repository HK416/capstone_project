use mod_network::components::{CapturePoint, Team};
use mod_physics::collision::Collider;


pub struct CapturePointObject {
    /// 점령지 데이터
    capture_point: CapturePoint,
    /// 점령지 충돌체
    collider: Collider,
}

impl CapturePointObject {
    /// 최대 점령점수. capture_score가 이 값에 도달하면 게임이 종료됩니다.
    const MAX_CAPTURE_SCORE: f32 = 60.0;

    pub fn new(collider: Collider) -> Self {
        Self {
            capture_point: CapturePoint::default(),
            collider,
        }
    }

    pub fn capture_point(&self) -> &CapturePoint {
        &self.capture_point
    }

    pub fn capture_progress(&self) -> &f32 {
        &self.capture_point.capture_progress
    }

    pub fn capture_score(&self) -> &[f32; 2] {
        &self.capture_point.capture_score
    }

    pub fn capture_team(&self) -> &Option<Team> {
        &self.capture_point.capture_team
    }

    pub fn collider(&self) -> &Collider {
        &self.collider
    }

    pub fn capture(
        &mut self, 
        new_capture_team: Option<Team>,
        elapsed_time_sec: f32, 
        capturing_count: usize
    ) -> Option<Team> {
        if new_capture_team.is_none() {
            // 아무도 없는 경우
            if capturing_count == 0 {
                // 현재 점령완료한 팀의 점령시간 증가
                return self.checked_increase_capture_score(elapsed_time_sec);
            }

            // 두 팀 모두 있는 경우
            return None;
        }

        if new_capture_team == self.capture_point.capture_team {
            if self.capture_point.capture_progress == 100.0 {
                let team = new_capture_team.unwrap();
                self.capture_point.capture_score[team as usize] += elapsed_time_sec;
                if self.capture_point.capture_score[team as usize] >= Self::MAX_CAPTURE_SCORE {
                    self.capture_point.capture_score[team as usize] = Self::MAX_CAPTURE_SCORE;
                    return self.capture_point.capture_team;
                }
            } else {
                self.capture_point.capture_progress += 10.0 * capturing_count as f32 * elapsed_time_sec;
                self.capture_point.capture_progress = self.capture_point.capture_progress.min(100.0);
            }
        } else {
            // 인원수에 비례해서 점령도 증가
            self.capture_point.capture_progress -= 10.0 * capturing_count as f32 * elapsed_time_sec;
            if self.capture_point.capture_progress <= 0.0 {
                self.capture_point.capture_team = new_capture_team;
                self.capture_point.capture_progress = self.capture_point.capture_progress.abs();
            }
        }

        None
    }

    /// 점령지의 점수를 증가시키고 점령이 완료되었는지 확인합니다.
    fn checked_increase_capture_score(&mut self, elapsed_time_sec: f32) -> Option<Team> {
        if let Some(team) = self.capture_point.capture_team {
            if self.capture_point.capture_progress == 100.0 {
                self.capture_point.capture_score[team as usize] += elapsed_time_sec;
                if self.capture_point.capture_score[team as usize] >= Self::MAX_CAPTURE_SCORE {
                    self.capture_point.capture_score[team as usize] = Self::MAX_CAPTURE_SCORE;
                    return Some(team);
                }
            }
        }
        None
    }
}