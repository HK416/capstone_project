// AI 이동 로직 (A* 경로탐색, 입력 생성)
use glam::Vec3A;
use mod_network::components::GameInputBits;
pub use crate::ai::ai_astar::astar_pathfind_vec3a;
use std::collections::HashMap;
use uuid::Uuid;
use crate::entities::player::Player;


/// AI 전용 플레이어 구조체
#[derive(Debug, Clone)]
pub struct AiPlayer {
    pub player: Player,         // 실제 게임 플레이어 데이터
    pub ai_id: Uuid,           // AI 식별자
    // FSM, 경로, 내부 상태 등 필요시 확장 가능
}


pub fn update_ai_players(ai_players: &mut HashMap<Uuid, AiPlayer>) {
    for (_, ai_player) in ai_players.iter_mut() {

        if ai_player.player.health_data.remaining == 0 {
            continue;
        }
        let current_pos = ai_player.player.translation;
        let target_pos = Vec3A::new(0.0, 0.0, 0.0); // 목표 좌표 (예: 중심)
        let step_size = 0.5;
        let path_opt = astar_pathfind_vec3a(
            current_pos,
            target_pos,
            step_size,
            |pos| is_walkable_stub(pos), // 현재는 더미
        );
        if let Some(path) = path_opt {
            if path.len() > 1 {
                let next = path[1]; // path[0]은 자기 자신
                let dir = (next - current_pos).normalize_or_zero();
                let mut input = GameInputBits::default();
                if dir.x > 0.1 {
                    input |= GameInputBits::Right;
                } else if dir.x < -0.1 {
                    input |= GameInputBits::Left;
                }
                if dir.z > 0.1 {
                    input |= GameInputBits::Forward;
                } else if dir.z < -0.1 {
                    input |= GameInputBits::Backward;
                }
                ai_player.player.input_bits = input;
            }
        }
    }
}

/// 현재는 더미 함수: 전체 지형 또는 월드 충돌 정보와 연결 필요
fn is_walkable_stub(_pos: Vec3A) -> bool {
    true
}
