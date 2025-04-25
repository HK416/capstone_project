//! 시가지 스테이지 지형과 관련된 코드를 관리합니다.
//!

use mod_network::components::MAX_IN_GAME_TEAM_PLAYERS;

/// 승리 팀의 회전 각도입니다.
pub const RESET_ROTATION: [glam::Quat; 2] = [
    glam::quat(0.0, 0.7071068, 0.0, 0.7071068),
    glam::quat(0.0, -0.7071068, 0.0, 0.7071068),
];

/// 승리 팀의 위치입니다.
pub const RESET_POSITIONS: [glam::Vec3; MAX_IN_GAME_TEAM_PLAYERS] = [
    glam::Vec3::new(0.0, 0.0, 0.0),
    glam::Vec3::new(0.0, 0.0, -1.0),
    glam::Vec3::new(0.0, 0.0, 1.0),
    glam::Vec3::new(0.0, 0.0, -2.0),
    glam::Vec3::new(0.0, 0.0, 2.0),
];
