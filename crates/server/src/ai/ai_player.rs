// AI 이동 로직 (A* 경로탐색, 입력 생성)
use glam::Vec3A;
use std::collections::HashMap;
use uuid::Uuid;

use std::sync::Arc;
use crate::world::GameWorld;
use crate::session::Session;
use mod_network::components::{UserName, ProfileIcon, Permission, GameTier, Team, CharacterKind, UserId, MAX_IN_GAME_PLAYERS, StageKind, HeldInput};

use crate::entities::player::Player;
use crate::data::get_stage_attributes;
use crate::ai::ai_fsm::AIPlayerFSM;
#[derive(Debug, Clone)]
pub struct AiPlayer {
    pub player: Player,         // 실제 게임 플레이어 데이터
    pub ai_id: Uuid,           // AI 식별자
    pub user_id: UserId,       // AI의 UserId (world.players 키)
    pub fsm: AIPlayerFSM,      // AI FSM 상태
    // 경로, 추가 상태 등 확장 가능
}

pub fn insert_ai_players(world: &mut GameWorld, num_blue_players: usize, num_red_players: usize) {
    let total_slots = MAX_IN_GAME_PLAYERS as usize;
    let mut blue_count = num_blue_players;
    let mut red_count = num_red_players;
    let mut ai_index = 1;
    for _ in 0..total_slots {
        let team = if blue_count < total_slots / 2 {
            blue_count += 1;
            Team::Blue
        } else if red_count < total_slots / 2 {
            red_count += 1;
            Team::Red
        } else {
            continue;
        };
        let mut player = Player::new(
            UserName::from_str(&format!("Player AI{}", ai_index)),
            ProfileIcon::default(),
            Permission::User,
            GameTier::Bronze,
        );
        player.user_id = Some(ai_uid);
        player.set_team(team);
        player.set_character_kind(CharacterKind::ArisOriginal);
        player.set_ready_to_play(true);
        let ai_uid = UserId::new(rand::random::<u32>()); 
        let ai_uuid = Uuid::new_v4();

        // AI FSM 초기화
        let fsm = AIPlayerFSM::new(
            ai_uuid,
            player.translation,
            player.translation,
        );

        let ai_player = AiPlayer {
            player,
            ai_id: ai_uuid,
            user_id: ai_uid,
            fsm,
        };

        let ai_session = Arc::new(Session::ai(ai_uid));
        world.add_ai_player(ai_uid, ai_player.player.clone(), Arc::clone(&ai_session));

        // HashMap에 AI 플레이어 관리
        world.ai_players.insert(ai_uuid, ai_player.clone());

        ai_index += 1;
    }
}

pub fn update_ai_players(world: &mut GameWorld) {
    let ai_players = &mut world.ai_players;
    // 팀별 인덱스 계산을 위해 분리
    let mut blue_idx = 0;
    let mut red_idx = 0;
    use crate::ai::ai_astar::astar_pathfind_vec3a;
    let stage = get_current_stage_attributes();
    // 중앙 점령지 위치: capture_zone의 bounding box 정점 평균 사용
    let vertices = match &stage.capture_zone {
        // Collider::Aabb(bbox) 등에서 get_vertices() 사용
        mod_physics::collision::Collider::Aabb(bbox) => bbox.get_vertices(),
        mod_physics::collision::Collider::Obb(obb) => obb.get_vertices(),
        mod_physics::collision::Collider::Capsule(capsule) => {
            // 캡슐의 중심 사용 (Vec3A 변환)
            [glam::Vec3A::from(capsule.center); 8]
        },
        mod_physics::collision::Collider::OrientedCapsule(ocapsule) => {
            [glam::Vec3A::from(ocapsule.center); 8]
        },
        mod_physics::collision::Collider::Sphere(sphere) => {
            [glam::Vec3A::from(sphere.center); 8]
        },
    };
    let mut sum = glam::Vec3A::ZERO;
    for v in &vertices {
        sum += *v;
    }
    let capture_center = sum / (vertices.len() as f32);
    for (ai_uuid, ai_player) in ai_players.iter_mut() {
        // 실제 게임 월드 Player 객체에 동기화 필요
        // UserId는 ai_player.player.user_id()
        // 실제 GameWorld의 players HashMap에 AI 입력/상태 동기화
        if let Some(world_player) = world.players.get_mut(&ai_player.user_id) {
            world_player.input_bits = ai_player.player.input_bits;
            world_player.translation = ai_player.player.translation;
            world_player.velocity = ai_player.player.velocity;
        }
        let team = ai_player.player.team();
        let index = match team {
            mod_network::components::Team::Blue => {
                let idx = blue_idx;
                blue_idx += 1;
                idx
            }
            mod_network::components::Team::Red => {
                let idx = red_idx;
                red_idx += 1;
                idx
            }
        };
        if ai_player.player.health_data.remaining == 0 {
            // 리스폰 처리: 팀별 리스폰 위치로 이동, 속도 초기화
            let respawn_pos = get_respawn_position(team, index);
            ai_player.fsm.ctx.position = respawn_pos;
            ai_player.player.translation = respawn_pos;
            ai_player.player.velocity.0 = Vec3A::ZERO;
            let max_hp = ai_player.player.character_attributes().max_health_point;
            ai_player.player.health_data.remaining = max_hp;
            continue;
        }

        // 목표를 중앙 점령지로 설정
        ai_player.fsm.ctx.target = capture_center;

        // A* 경로탐색으로 다음 위치 결정
        let start = ai_player.fsm.ctx.position;
        let goal = capture_center;
        let step = 0.5;
        let path = astar_pathfind_vec3a(start, goal, step, |pos| is_walkable_real(pos, stage, team));
        let next_pos = if let Some(mut waypoints) = path {
            if waypoints.len() > 1 {
                waypoints[1]
            } else {
                waypoints[0]
            }
        } else {
            start
        };

        let prev_pos = ai_player.fsm.ctx.position;
        ai_player.fsm.ctx.position = next_pos;

        // 충돌 및 이동 처리: 실제 플레이어와 동일하게
        if is_walkable_real(next_pos, stage, team) {
            ai_player.player.translation = next_pos;
            ai_player.player.velocity.0 = next_pos - prev_pos;
        } else {
            ai_player.fsm.ctx.position = prev_pos;
            ai_player.player.velocity.0 = Vec3A::ZERO;
        }

        // 입력 처리: 방향에 따라 HeldInput 비트플래그 설정
        let dir = (next_pos - prev_pos).normalize_or_zero();
        let mut input_bits = HeldInput::empty();
        if dir.x > 0.1 {
            input_bits |= HeldInput::Right;
        } else if dir.x < -0.1 {
            input_bits |= HeldInput::Left;
        }
        if dir.z > 0.1 {
            input_bits |= HeldInput::Forward;
        } else if dir.z < -0.1 {
            input_bits |= HeldInput::Backward;
        }
        ai_player.player.input_bits = input_bits;
    }
}

/// 현재는 더미 함수: 전체 지형 또는 월드 충돌 정보와 연결 필요
// 실제 월드/지형 정보와 연동 필요
fn is_walkable_real(pos: Vec3A, stage: &mod_network::components::StageAttributes, team: mod_network::components::Team) -> bool {
    stage.is_valid_position(team, pos.x, pos.z)
}

fn select_target_position(_player: &Player) -> Vec3A {
    // 예시: 항상 (0,0,0)으로 이동. 실제로는 적 플레이어 추적 등 구현 가능
    Vec3A::new(0.0, 0.0, 0.0)
}

fn get_respawn_position(team: mod_network::components::Team, index: usize) -> Vec3A {
    let stage = get_current_stage_attributes();
    // 일반 플레이어와 동일하게 팀별 positions 배열 사용
    match team {
        mod_network::components::Team::Blue => stage.blue_team_positions[index],
        mod_network::components::Team::Red => stage.red_team_positions[index],
    }
}

fn get_current_stage_attributes() -> &'static mod_network::components::StageAttributes {
    // 실제 스테이지 정보를 반환하도록 구현 필요
    // 예시: City 맵 사용
    get_stage_attributes(StageKind::City)
}
