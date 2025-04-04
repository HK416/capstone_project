use mod_network::components::{CapturePoint, Team};
use mod_physics::collision::Collider;


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

    pub fn capture_point_mut(&mut self) -> &mut CapturePoint {
        &mut self.capture_point
    }

    pub fn capture_point(&self) -> &CapturePoint {
        &self.capture_point
    }

    pub fn capture_progress(&mut self) -> &mut f32 {
        &mut self.capture_point.capture_progress
    }

    pub fn capture_score(&mut self) -> &mut [f32; 2] {
        &mut self.capture_point.capture_score
    }

    pub fn capture_team(&mut self) -> &mut Option<Team> {
        &mut self.capture_point.capture_team
    }

    pub fn collider(&self) -> &Collider {
        &self.collider
    }
}