// AI 이동 로직 (A* 경로탐색, 입력 생성)
use glam::Vec3A;
use uuid::Uuid;

use std::sync::Arc;
use crate::world::GameWorld;
use crate::session::Session;
use mod_network::components::{UserName, ProfileIcon, Permission, GameTier, Team, CharacterKind, UserId, MAX_IN_GAME_PLAYERS, StageKind};

use crate::entities::player::Player;
use crate::data::get_stage_attributes;
use crate::ai::ai_fsm::AIPlayerFSM;
use mod_physics::collision::Collider;
use mod_physics::object3d::BoundingBox;
use crate::ai::ai_astar::astar_pathfind_vec3a;
use mod_network::components::update_player_translation;
#[derive(Debug, Clone)]
pub struct AiPlayer {
    pub ai_id: Uuid,           // AI 식별자
    pub user_id: UserId,       // AI UserId
    pub fsm: AIPlayerFSM,      // AI FSM 상태
    // Player를 직접 소유하지 않음. user_id로 world.players의 Player를 참조
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
            ai_id: ai_uuid,
            user_id: ai_uid,
            fsm,
        };

        let ai_session = Arc::new(Session::ai(ai_uid));
        world.add_ai_player(ai_uid, player.clone(), Arc::clone(&ai_session));

        // HashMap에 AI 플레이어 관리
        world.ai_players.insert(ai_uuid, ai_player.clone());

        ai_index += 1;
    }
}

/// AI FSM 상태를 GameWorld의 Player에 동기화하는 통합 함수
pub fn update_ai_players(world: &mut GameWorld) {
    let mut blue_idx = 0;
    let mut red_idx = 0;
    for (_, ai_player) in world.ai_players.iter_mut() {
        // 경로 계산 주기(ms)
        const PATHFIND_INTERVAL_MS: u32 = 500;
        // world.players에서 AI의 Player 객체를 직접 참조
        let player = match world.players.get_mut(&ai_player.user_id) {
            Some(p) => p,
            None => continue, // Player가 없으면 스킵
        };
        let now_ms = player.input_state_timer.0;
        let last_pathfind = ai_player.fsm.ctx.last_pathfind_time.unwrap_or(0);
        let need_pathfind = (now_ms as u32).saturating_sub(last_pathfind) >= PATHFIND_INTERVAL_MS;
        let team = player.team();
        let index = match team {
            Team::Blue => { let idx = blue_idx; blue_idx += 1; idx }
            Team::Red => { let idx = red_idx; red_idx += 1; idx }
        };
        if player.health_data.remaining == 0 {
            let respawn_pos = get_respawn_position(team, index);
            ai_player.fsm.ctx.position = respawn_pos;
            player.translation = respawn_pos;
            player.velocity.0 = Vec3A::ZERO;
            let max_hp = player.character_attributes().max_health_point;
            player.health_data.remaining = max_hp;
            continue;
        }

        // 1. FSM 상태 업데이트 (AI 행동 결정)
        ai_player.fsm.update();

        // 목표 설정: 일정 확률로 중앙 또는 랜덤 위치
        let stage = get_current_stage_attributes();
        use rand::Rng;
        let mut rng = rand::rng();
        let mut central_target = None;
        if let Collider::Sphere(sphere) = &stage.capture_zone {
            central_target = Some(glam::Vec3A::from(sphere.center));
        }
        // 경로를 일정 주기마다만 갱신, 또는 목표가 바뀌었거나 이동 불가/경로 없음이면 강제 재탐색
        let central_pos = central_target.unwrap_or(glam::Vec3A::ZERO);
        let dist_to_central = (ai_player.fsm.ctx.position - central_pos).length();
        let central_radius = 2.0; // 집결지 도달 판정 반경 (m)
        let mut target_changed = false;
        if dist_to_central <= central_radius {
            // 중앙에 도달한 경우 목표를 중앙으로 고정, 이동 멈춤
            if ai_player.fsm.ctx.target != central_pos {
                ai_player.fsm.ctx.target = central_pos;
                target_changed = true;
            }
        } else {
            // 중앙에 도달하지 않은 경우 기존 로직대로 목표 설정
            let new_target = if rng.random_bool(0.7) {
                central_pos
            } else {
                let x = rng.random_range(-stage.area_width..stage.area_width);
                let z = rng.random_range(-stage.area_depth..stage.area_depth);
                let y = stage.get_area_height(x, z).unwrap_or(0.0) + 1.0;
                glam::Vec3A::new(x, y, z)
            };
            if ai_player.fsm.ctx.target != new_target {
                ai_player.fsm.ctx.target = new_target;
                target_changed = true;
            }
        }

        let mut is_grounded = player.is_grounded();
        let mut is_invincible = player.is_invincible();
        let goal = ai_player.fsm.ctx.target;
        let step = 1.0;
        let character_attributes = player.character_attributes();
        let is_walkable = |pos: Vec3A| is_walkable_real(pos, stage, team, character_attributes);
        // 경로를 일정 주기마다만 갱신, 또는 목표가 바뀌었거나 이동 불가/경로 없음이면 강제 재탐색
        let mut force_pathfind = false;
        if target_changed {
            force_pathfind = true;
        }
        let path_opt = ai_player.fsm.ctx.path.as_ref();
        let mut next_pos = ai_player.fsm.ctx.position;
        let prev_pos = ai_player.fsm.ctx.position;
        let mut can_move = false;
        let elapsed_time_sec = 0.016; // 실제 플레이어와 동일하게 적용
        if let Some(path) = path_opt {
            let move_speed = character_attributes.speed * elapsed_time_sec;
            let mut tried_next = false;
            let mut collision = false;
            if path.len() > 1 && path[0] == prev_pos {
                let target_pos = path[1];
                let dir = (target_pos - prev_pos).normalize_or_zero();
                let dist_to_target = (target_pos - prev_pos).length();
                if dist_to_target <= move_speed {
                    next_pos = target_pos;
                } else {
                    next_pos = prev_pos + dir * move_speed;
                }
                tried_next = true;
            } else if path.len() > 0 && path[0] != prev_pos {
                let target_pos = path[0];
                let dir = (target_pos - prev_pos).normalize_or_zero();
                let dist_to_target = (target_pos - prev_pos).length();
                if dist_to_target <= move_speed {
                    next_pos = target_pos;
                } else {
                    next_pos = prev_pos + dir * move_speed;
                }
                tried_next = true;
            }
            // 이동하려는 위치가 벽 등으로 인해 불가능하면 즉시 경로 재탐색
            if tried_next && !is_walkable_real(next_pos, stage, team, character_attributes) {
                collision = true;
            }
            if collision {
                // 즉시 경로 재탐색
                let path = astar_pathfind_vec3a(prev_pos, goal, step, is_walkable);
                ai_player.fsm.ctx.path = path;
                ai_player.fsm.ctx.last_pathfind_time = Some(now_ms as u32);
                // 재탐색된 경로로 다시 이동 시도
                if let Some(new_path) = ai_player.fsm.ctx.path.as_ref() {
                    if new_path.len() > 1 && new_path[0] == prev_pos {
                        let target_pos = new_path[1];
                        let dir = (target_pos - prev_pos).normalize_or_zero();
                        let dist_to_target = (target_pos - prev_pos).length();
                        if dist_to_target <= move_speed {
                            next_pos = target_pos;
                        } else {
                            next_pos = prev_pos + dir * move_speed;
                        }
                        can_move = true;
                    } else if new_path.len() > 0 && new_path[0] != prev_pos {
                        let target_pos = new_path[0];
                        let dir = (target_pos - prev_pos).normalize_or_zero();
                        let dist_to_target = (target_pos - prev_pos).length();
                        if dist_to_target <= move_speed {
                            next_pos = target_pos;
                        } else {
                            next_pos = prev_pos + dir * move_speed;
                        }
                        can_move = true;
                    } else {
                        can_move = false;
                    }
                } else {
                    can_move = false;
                }
            } else {
                can_move = true;
            }
        }
        // 경로가 없거나 이동 불가면 목표를 랜덤 위치로 재설정하고 멈춤 (단, 중앙에 도달한 경우는 멈춤만 수행)
        if !can_move {
            if dist_to_central <= central_radius {
                // 중앙에 도달한 경우 멈춤만 수행
                next_pos = prev_pos;
            } else {
                let mut rng = rand::rng();
                let x = rng.random_range(-stage.area_width..stage.area_width);
                let z = rng.random_range(-stage.area_depth..stage.area_depth);
                let y = stage.get_area_height(x, z).unwrap_or(0.0) + 1.0;
                ai_player.fsm.ctx.target = glam::Vec3A::new(x, y, z);
                next_pos = prev_pos; // 멈춤
            }
        }
        // 경로 보간 결과를 AI FSM 컨텍스트 위치에 반영
        ai_player.fsm.ctx.position = next_pos;
        // 목표 위치(next_pos)와 현재 위치(prev_pos)를 비교하여 이동 방향을 결정
        let move_dir = (next_pos - player.translation).normalize_or_zero();
        log::info!("[AI] MoveDir: next_pos={:?} player_pos={:?} move_dir={:?}", next_pos, player.translation, move_dir);
        let mut input = mod_network::components::HeldInput::empty();
        if move_dir.x > 0.1 { input |= mod_network::components::HeldInput::Right; } else if move_dir.x < -0.1 { input |= mod_network::components::HeldInput::Left; }
        if move_dir.z > 0.1 { input |= mod_network::components::HeldInput::Forward; } else if move_dir.z < -0.1 { input |= mod_network::components::HeldInput::Backward; }
        player.held_input = input;
        // 실제 플레이어 이동 로직과 동일하게 방향 및 상태 갱신
        player.direction = mod_network::components::MovingDirection(move_dir.with_y(0.0).normalize_or(glam::Vec3A::Z));
        let character_attributes = player.character_attributes();
        mod_network::components::update_action_state(
            player.held_input,
            &mut player.action_state,
            &mut player.action_state_timer,
            &character_attributes,
            &mut player.bullet_data,
            &mut player.skill_cost_data,
            &mut Vec::new(), // 이벤트 버퍼 (AI는 비워둠)
        );
        mod_network::components::update_movement_state(
            player.held_input,
            player.action_state,
            &mut player.movement_state,
            &mut player.movement_state_timer,
        );

        // 실제 이동은 플레이어와 동일하게 update_player_translation에만 맡김
        let character_attributes = player.character_attributes();
        let action_state = player.action_state;
        let movement_state = &mut player.movement_state;
        let movement_state_timer = &mut player.movement_state_timer;
        let velocity = &mut player.velocity;
        let translation = &mut player.translation;
        let direction = mod_network::components::MovingDirection(move_dir);
        let health_data = Some(&mut player.health_data);
        let input_state_timer = player.input_state_timer;
        let elapsed_time_sec = 0.016;
        update_player_translation(
            stage,
            character_attributes,
            action_state,
            movement_state,
            movement_state_timer,
            velocity,
            translation,
            direction,
            player.held_input,
            team,
            &mut is_grounded,
            &mut is_invincible,
            health_data,
            input_state_timer,
            elapsed_time_sec,
        );
        // 이동 처리 후 FSM 위치를 실제 위치로 동기화
        ai_player.fsm.ctx.position = player.translation;
        player.set_grounded(is_grounded);
        player.set_invincible(is_invincible);
    }
}



fn is_walkable_real(
    pos: Vec3A,
    stage: &mod_network::components::StageAttributes,
    team: mod_network::components::Team,
    character_attributes: &mod_network::components::CharacterAttributes
) -> bool {
    let mut capsule = character_attributes.collider.clone();
    capsule.center = pos.into();
    let player_aabb = BoundingBox::from(&capsule);
    let player_collider = Collider::Capsule(capsule);

    for collider in mod_physics::collision::ColliderTreeIterator::new(&stage.collider) {
        if !collider.check_aabb_collision(&player_aabb) { continue; }
        if player_collider.check_collision(collider) {
            return false;
        }
    }
    return stage.is_valid_position(team, pos.x, pos.z);
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

