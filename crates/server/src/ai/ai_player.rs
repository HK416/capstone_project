// AI 이동 로직 (A* 경로탐색, 입력 생성)
use glam::Vec3A;
use uuid::Uuid;

use crate::session::Session;
use crate::world::GameWorld;
use mod_network::components::{
    CharacterKind, GameTier, MAX_IN_GAME_PLAYERS, Permission, ProfileIcon, StageKind, Team, UserId,
    UserName,
};
use std::sync::Arc;

use crate::ai::ai_astar::{GridMap2D, grid_based_astar_pathfind};
use crate::ai::ai_fsm::AIPlayerFSM;
use crate::data::get_stage_attributes;
use crate::entities::player::Player;
use mod_network::components::update_player_translation;
use mod_physics::collision::Collider;

// 전역 그리드 맵 저장소 (스테이지별로 관리)
use ahash::HashMap;
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref GLOBAL_GRID_MAP_CACHE: std::sync::Arc<Mutex<HashMap<StageKind, GridMap2D>>> =
        std::sync::Arc::new(Mutex::new(HashMap::default()));
}

// AI 상수들
const MIN_DISTANCE_THRESHOLD: f32 = 2.0;
const GRID_SCALE: f32 = 8.0;
const DETECTION_RANGE: f32 = 30.0; // 적 탐지 범위 (미터)
const ATTACK_RANGE: f32 = 25.0; // 공격 범위 (미터)
// 참고: CAPTURE_ZONE 관련 상수들은 stage.capture_zone에서 동적으로 가져옴

#[derive(Debug, Clone, PartialEq)]
pub enum AIBehaviorState {
    MovingToCapture, // 점령지역으로 이동 중
    InCaptureZone,   // 점령지역 내에서 활동 중
}

#[derive(Debug, Clone)]
pub struct AiPlayer {
    pub user_id: UserId,                                       // AI UserId
    pub fsm: AIPlayerFSM,                                      // AI FSM 상태
    pub move_counter: u32,                                     // 이동 횟수 카운터
    pub current_target: Vec3A,                                 // 현재 목표 위치
    pub visited_grids: std::collections::VecDeque<(i32, i32)>, // 최근 방문한 그리드 기록 (최대 20개)
    pub last_position: Vec3A,                                  // 이전 위치 (제자리 판정용)
    pub stuck_counter: u32,                                    // 제자리 카운터
    pub direction_history: std::collections::VecDeque<u8>,     // 최근 사용한 방향 기록 (8방향: 0~7)
    pub circle_penalty: f32,                                   // 맴돌기 페널티 누적값
    pub last_direction: Option<u8>,                            // 마지막 이동 방향
    pub behavior_state: AIBehaviorState,                       // AI 행동 상태
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
        let fsm = AIPlayerFSM::new(player.translation, player.translation);

        let ai_player = AiPlayer {
            user_id: ai_uid,
            fsm,
            move_counter: 0,
            current_target: Vec3A::ZERO, // 초기에는 중앙으로 설정됨
            visited_grids: std::collections::VecDeque::with_capacity(20),
            last_position: player.translation,
            stuck_counter: 0,
            direction_history: std::collections::VecDeque::with_capacity(16), // 최근 16번의 방향 기록
            circle_penalty: 0.0,
            last_direction: None,
            behavior_state: AIBehaviorState::MovingToCapture, // 초기에는 점령지역으로 이동
        };

        let ai_session = Arc::new(Session::ai(ai_uid));
        world.add_ai_player(ai_uid, player.clone(), Arc::clone(&ai_session));

        // HashMap에 AI 플레이어 관리
        world.ai_players.insert(ai_uuid, ai_player.clone());

        ai_index += 1;
    }
}

/// 간단한 AI 업데이트 함수 - 사전 로딩된 그리드 맵을 사용하여 빠른 AI 처리
pub fn update_ai_players(world: &mut GameWorld) {
    let mut blue_idx = 0;
    let mut red_idx = 0;

    // 스테이지 정보 가져오기
    let stage = get_current_stage_attributes();

    // **그리드 맵 사전 확인**: 캐시에서 빠르게 확인
    let current_stage = StageKind::City; // 현재 스테이지 (필요시 동적으로 변경)
    if get_cached_grid_map(current_stage).is_none() {
        log::warn!(
            "[AI UPDATE] Grid map not found in cache for stage {:?}, AI may use fallback movement",
            current_stage
        );
    }

    // 중앙 집결지 좌표 및 반지름 계산
    let (central_pos, capture_zone_radius) = if let Collider::Sphere(sphere) = &stage.capture_zone {
        (glam::Vec3A::from(sphere.center), sphere.radius)
    } else {
        (glam::Vec3A::ZERO, 10.0) // 기본값
    };

    // **적 정보 수집**: borrowing 문제 해결을 위해 먼저 모든 플레이어 정보 수집
    let mut player_info = Vec::new();
    for (user_id, player) in world.players.iter() {
        if player.health_data.remaining > 0 {
            // 살아있는 플레이어만
            player_info.push((*user_id, player.translation, player.team()));
        }
    }

    println!(
        "[AI SYSTEM] Updating {} AI players using mixed movement - Central target: {:?}",
        world.ai_players.len(),
        central_pos
    );

    for (_ai_uuid, ai_player) in world.ai_players.iter_mut() {
        // world.players에서 AI의 Player 객체를 직접 참조
        let player = match world.players.get_mut(&ai_player.user_id) {
            Some(p) => p,
            None => continue,
        };

        let team = player.team();
        let team_name = if team == Team::Red { "Red" } else { "Blue" };
        let index = match team {
            Team::Blue => {
                let idx = blue_idx;
                blue_idx += 1;
                idx
            }
            Team::Red => {
                let idx = red_idx;
                red_idx += 1;
                idx
            }
        };

        // 사망 시 리스폰 처리
        if player.health_data.remaining == 0 {
            let respawn_pos = get_respawn_position(team, index);
            ai_player.fsm.ctx.position = respawn_pos;
            player.translation = respawn_pos;
            player.velocity.0 = Vec3A::ZERO;
            let max_hp = player.character_attributes().max_health_point;
            player.health_data.remaining = max_hp;
            // 경로 초기화
            ai_player.fsm.ctx.path = None;
            ai_player.fsm.ctx.target = central_pos;
            // 새로운 목표 시스템 초기화
            ai_player.move_counter = 0;
            ai_player.current_target = Vec3A::ZERO; // 다음 업데이트에서 새 목표 설정됨
            // **방문 기록 초기화**: 리스폰 시 새로운 시작
            ai_player.visited_grids.clear();
            ai_player.last_position = respawn_pos;
            ai_player.stuck_counter = 0;
            // **새로운 필드 초기화**: 방향 기록과 맴돌기 페널티 리셋
            ai_player.direction_history.clear();
            ai_player.circle_penalty = 0.0;
            ai_player.last_direction = None;
            ai_player.behavior_state = AIBehaviorState::MovingToCapture; // 리스폰 시 다시 점령지역으로 이동
            println!(
                "[AI] {} team AI#{} respawned at {:?} (all histories cleared)",
                team_name, index, respawn_pos
            );
            continue;
        }

        // AI 플레이어 위치를 실제 플레이어 위치와 동기화
        ai_player.fsm.ctx.position = player.translation;
        let current_pos = player.translation;

        // **방문 그리드 추적**: 맴돌기 방지를 위한 그리드 기록 시스템
        if let Ok(grid_map_cache) = GLOBAL_GRID_MAP_CACHE.lock() {
            if let Some(grid_map) = grid_map_cache.get(&StageKind::City) {
                // 현재 위치를 그리드 좌표로 변환
                if let Some((grid_x, grid_z)) = grid_map.world_to_grid(current_pos) {
                    let current_grid = (grid_x as i32, grid_z as i32);

                    // 제자리 판정: 이전 위치와 비교
                    let movement_distance = (current_pos - ai_player.last_position).length();
                    let is_stuck = movement_distance < 0.5; // 0.5m 이하면 제자리로 판정

                    if is_stuck {
                        ai_player.stuck_counter += 1;
                    } else {
                        ai_player.stuck_counter = 0;

                        // 새로운 그리드에 들어왔을 때만 기록 추가 (맴돌기 방지)
                        let should_record = ai_player.visited_grids.is_empty()
                            || ai_player.visited_grids.back() != Some(&current_grid);

                        if should_record {
                            ai_player.visited_grids.push_back(current_grid);

                            // 최대 20개 그리드만 기록 유지
                            if ai_player.visited_grids.len() > 20 {
                                ai_player.visited_grids.pop_front();
                            }

                            println!(
                                "[AI TRACKING] {} team AI#{} visited grid {:?}, history: {} grids",
                                team_name,
                                index,
                                current_grid,
                                ai_player.visited_grids.len()
                            );
                        }
                    }

                    // 이전 위치 업데이트
                    ai_player.last_position = current_pos;
                }
            }
        }

        // **핵심 안전성 검사**: 현재 위치가 5그리드 마진을 유지하는지 확인
        if let Ok(grid_map_cache) = GLOBAL_GRID_MAP_CACHE.lock() {
            if let Some(grid_map) = grid_map_cache.get(&StageKind::City) {
                if !grid_map.is_walkable(current_pos) {
                    println!(
                        "[AI SAFETY] {} team AI#{} is in UNSAFE position: {:?}! Emergency action required.",
                        team_name, index, current_pos
                    );

                    // 긴급 안전 이동
                    let emergency_input = find_escape_movement(current_pos, grid_map);
                    player.held_input = emergency_input;
                    println!(
                        "[AI SAFETY] Emergency movement applied: {:?}",
                        emergency_input
                    );
                    continue; // 이번 프레임은 안전 이동만 수행
                } else {
                    println!(
                        "[AI SAFETY] {} team AI#{} position is SAFE (5-grid margin maintained)",
                        team_name, index
                    );
                }
            }
        }

        // 현재 목표까지의 거리 계산
        let dist_to_target = (current_pos - ai_player.current_target).length();

        println!(
            "[AI] {} team AI#{} - Pos: {:?}, Target: {:?}, Dist: {:.2}m, Move count: {}",
            team_name,
            index,
            current_pos,
            ai_player.current_target,
            dist_to_target,
            ai_player.move_counter
        );

        // **목표: 중앙 거점 중심으로 이동 후 정지**
        let target_point = central_pos; // stage에서 가져온 중앙 거점 위치
        let distance_to_target = (current_pos - target_point).length();

        // 중앙 거점의 반지름의 절반 내에 있는지 확인 (stage에서 가져온 실제 반지름 사용)
        let inner_zone_radius = capture_zone_radius / 2.0;
        let is_in_inner_zone = distance_to_target <= inner_zone_radius;

        if is_in_inner_zone {
            // 중앙 거점의 내부 구역에 있으면 정지
            ai_player.current_target = current_pos; // 현재 위치를 목표로 설정하여 정지
            ai_player.behavior_state = AIBehaviorState::InCaptureZone;
            println!(
                "[AI TARGET] {} team AI#{} REACHED INNER ZONE (radius {:.1}m) - STOPPING (distance: {:.2}m)",
                team_name, index, inner_zone_radius, distance_to_target
            );
        } else {
            // 내부 구역 밖에 있으면 중앙 거점으로 이동
            ai_player.current_target = target_point;
            ai_player.behavior_state = AIBehaviorState::MovingToCapture;
            ai_player.move_counter += 1;

            println!(
                "[AI TARGET] {} team AI#{} MOVING TO CAPTURE ZONE CENTER - distance: {:.2}m",
                team_name, index, distance_to_target
            );

            // 경로 초기화 - 새 목표 설정 시 기존 경로 무효화
            ai_player.fsm.ctx.path = None;
        }

        // **기본 이동: 장애물 회피 + 카메라 기반 이동 (목표에 도달하지 않은 경우만)**
        let mut move_input = if is_in_inner_zone {
            // 목표에 도달했으면 완전히 정지
            mod_network::components::HeldInput::empty()
        } else {
            // 목표에 도달하지 않았으면 방문 기록을 고려한 스마트 이동 사용
            calculate_smart_camera_movement_with_history(
                current_pos,
                ai_player.current_target,
                &stage,
                &ai_player.visited_grids,
                ai_player.stuck_counter,
            )
        };

        // **적 감지 및 자동 발포 시스템**: 수집된 플레이어 정보 사용
        const DETECTION_RANGE: f32 = 30.0; // 30m 감지 범위
        const ATTACK_RANGE: f32 = 25.0; // 25m 공격 범위

        let mut closest_enemy_distance = f32::MAX;
        let mut closest_enemy_position = None;

        // 적 탐지
        for &(other_user_id, other_pos, other_team) in &player_info {
            if other_user_id != ai_player.user_id && other_team != team {
                let distance = (other_pos - current_pos).length();
                if distance <= DETECTION_RANGE && distance < closest_enemy_distance {
                    closest_enemy_distance = distance;
                    closest_enemy_position = Some(other_pos);
                }
            }
        }

        // 적 발견시 공격
        if let Some(enemy_pos) = closest_enemy_position {
            if closest_enemy_distance <= ATTACK_RANGE {
                // 공격 입력 추가
                move_input |= mod_network::components::HeldInput::Attack;

                // 적 방향으로 조준
                let direction_to_enemy = (enemy_pos - current_pos).normalize_or_zero();
                let aim_input = convert_direction_to_aim_input(direction_to_enemy);
                move_input |= aim_input;

                println!(
                    "[AI COMBAT] {} team AI#{} opening fire on enemy at {:.1}m!",
                    team_name, index, closest_enemy_distance
                );
            }
        }

        // AI 입력을 실제 플레이어 입력으로 설정
        player.held_input = move_input;

        // **실제 플레이어와 동일한 회전 시스템 적용**
        let character_attributes = player.character_attributes();

        // Action State 업데이트 (회전 로직에 필요)
        mod_network::components::update_action_state(
            player.held_input,
            &mut player.action_state,
            &mut player.action_state_timer,
            &character_attributes,
            &mut player.bullet_data,
            &mut player.skill_cost_data,
            &mut Vec::new(),
        );

        // Movement State 업데이트 (회전 로직에 필요)
        mod_network::components::update_movement_state(
            player.held_input,
            player.action_state,
            &mut player.movement_state,
            &mut player.movement_state_timer,
        );

        // **LatLon을 이용한 점진적 카메라 방향 설정 (부드러운 회전)**
        if !is_in_inner_zone {
            // 목표 방향을 카메라 방향으로 설정
            let target_direction = (ai_player.current_target - current_pos).normalize_or_zero();
            if target_direction.length() > 0.1 {
                // 목표 방향을 각도로 변환
                let target_angle = target_direction.z.atan2(target_direction.x);
                let current_angle = player.latlon.lon;

                // 각도 차이 계산 (가장 짧은 회전 경로)
                let mut angle_diff = target_angle - current_angle;

                // 각도를 -π ~ π 범위로 정규화
                while angle_diff > std::f32::consts::PI {
                    angle_diff -= 2.0 * std::f32::consts::PI;
                }
                while angle_diff < -std::f32::consts::PI {
                    angle_diff += 2.0 * std::f32::consts::PI;
                }

                // 점진적 회전 (부드러운 곡선 이동을 위해)
                let max_rotation_speed = 0.15; // 라디안/프레임 (약 8.6도/프레임)
                let rotation_step = if angle_diff.abs() > max_rotation_speed {
                    max_rotation_speed * angle_diff.signum()
                } else {
                    angle_diff
                };

                // 새로운 각도 적용
                let new_angle = current_angle + rotation_step;
                player.latlon.lon = new_angle;

                println!(
                    "[AI CAMERA] {} team AI#{} smooth rotation: current={:.1}° -> target={:.1}° -> new={:.1}° (step={:.1}°)",
                    team_name,
                    index,
                    current_angle.to_degrees(),
                    target_angle.to_degrees(),
                    new_angle.to_degrees(),
                    rotation_step.to_degrees()
                );
            }
        } else {
            // 목표에 도달했으면 카메라 방향도 고정
            println!(
                "[AI CAMERA] {} team AI#{} target reached - camera direction locked",
                team_name, index
            );
        }

        // **실제 플레이어와 100% 동일한 MovingDirection 업데이트 로직 적용**
        player.direction.update(player.held_input, player.latlon);

        // **실제 플레이어와 100% 동일한 회전 로직 적용**: update_player_rotation 사용
        let current_look = player.direction.0;
        let action_state = player.action_state;
        let movement_state = player.movement_state;
        let latlon = player.latlon;

        // 실제 플레이어 회전 로직 호출 (MovingDirection은 이미 HeldInput으로부터 업데이트됨)
        let new_look = mod_network::components::update_player_rotation(
            current_look,
            action_state,
            movement_state,
            player.direction,
            latlon,
        );

        // 회전 결과 적용
        player.direction.0 = new_look;

        // 쿼터니언 회전도 업데이트 (렌더링용)
        let target_rotation = glam::Quat::from_rotation_y(new_look.z.atan2(new_look.x));
        player.rotation = target_rotation;

        println!(
            "[AI ROTATION] {} team AI#{} rotation updated: look={:?}, quat={:?}",
            team_name, index, new_look, target_rotation
        );

        // FSM 컨텍스트 업데이트
        ai_player.fsm.ctx.target = ai_player.current_target;

        let movement_type = if ai_player.move_counter % 5 == 0 {
            "CENTRAL"
        } else {
            "RANDOM"
        };
        println!(
            "[AI] {} team AI#{} moving to {} target with grid-based pathfinding",
            team_name, index, movement_type
        );

        // 실제 이동 처리
        let mut is_grounded = player.is_grounded();
        let mut is_invincible = player.is_invincible();
        let character_attributes = player.character_attributes();
        let action_state = player.action_state;
        let movement_state = &mut player.movement_state;
        let movement_state_timer = &mut player.movement_state_timer;
        let velocity = &mut player.velocity;
        let translation = &mut player.translation;
        let direction = player.direction;
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

        // 처리 후 상태 동기화
        ai_player.fsm.ctx.position = player.translation;
        player.set_grounded(is_grounded);
        player.set_invincible(is_invincible);
    }
}

/// 방문 기록을 고려한 스마트 카메라 이동 (같은 그리드 왕복 방지 강화)
fn calculate_smart_camera_movement_with_history(
    current_pos: Vec3A,
    target_pos: Vec3A,
    _stage: &mod_network::components::StageAttributes,
    visited_grids: &std::collections::VecDeque<(i32, i32)>,
    stuck_counter: u32,
) -> mod_network::components::HeldInput {
    let distance = (target_pos - current_pos).length();

    // 목표가 매우 가까우면 정지
    if distance < 1.0 {
        println!(
            "[AI SMART HISTORY] Target very close ({:.2}m), stopping movement",
            distance
        );
        return mod_network::components::HeldInput::empty();
    }

    // 제자리에 너무 오래 있으면 강제 탈출
    if stuck_counter > 15 {
        // 더 빠른 탈출 (0.25초)
        println!("[AI SMART HISTORY] Stuck detected early! Forcing escape movement");
        return force_escape_movement_advanced(current_pos, visited_grids);
    }

    // 그리드 맵을 이용한 장애물 검사 및 방문 기록 고려
    if let Ok(grid_map_cache) = GLOBAL_GRID_MAP_CACHE.lock() {
        if let Some(grid_map) = grid_map_cache.get(&StageKind::City) {
            // 현재 위치가 안전한지 확인
            if !grid_map.is_walkable(current_pos) {
                println!("[AI SMART HISTORY] Current position unsafe, escaping");
                return find_escape_movement(current_pos, grid_map);
            }

            // 목표 방향 계산
            let target_direction = (target_pos - current_pos).normalize_or_zero();

            // 직진 경로의 여러 지점을 확인 (더 세밀한 검사)
            let check_distances = [2.0, 4.0, 6.0, 8.0]; // 2m, 4m, 6m, 8m 지점 확인
            let mut obstacle_detected = false;
            let mut visited_path_detected = false;

            for &check_dist in &check_distances {
                let check_pos = current_pos + target_direction * check_dist;

                // 장애물 확인
                if !grid_map.is_walkable(check_pos) {
                    obstacle_detected = true;
                    println!(
                        "[AI SMART HISTORY] Obstacle detected at {:.1}m ahead",
                        check_dist
                    );
                    break;
                }

                // 방문 기록 확인 (더 엄격하게)
                if let Some((grid_x, grid_z)) = grid_map.world_to_grid(check_pos) {
                    let check_grid = (grid_x as i32, grid_z as i32);

                    // 최근 방문한 그리드인지 확인 (최근 12개까지 확인)
                    if let Some(index) = visited_grids.iter().rposition(|&g| g == check_grid) {
                        let recency = visited_grids.len() - index;
                        if recency <= 12 {
                            // 더 긴 기간 동안 방문 기록 고려
                            visited_path_detected = true;
                            println!(
                                "[AI SMART HISTORY] Recently visited grid detected at {:.1}m ahead (recency: {})",
                                check_dist, recency
                            );
                            break;
                        }
                    }
                }
            }

            // 장애물이나 방문한 경로가 감지되면 우회 경로 탐색
            if obstacle_detected || visited_path_detected {
                println!(
                    "[AI SMART HISTORY] Path blocked or recently visited, finding alternative route"
                );

                // A* 경로탐색으로 우회 경로 찾기 (방문 기록 고려)
                if let Some(path) = smart_astar_pathfind_with_penalty(
                    current_pos,
                    target_pos,
                    grid_map,
                    visited_grids,
                ) {
                    if path.len() >= 2 {
                        let next_waypoint = path[1];
                        let waypoint_direction = (next_waypoint - current_pos).normalize_or_zero();

                        println!(
                            "[AI SMART HISTORY] Using A* path with visit penalty - next waypoint: {:?}",
                            next_waypoint
                        );
                        return calculate_camera_based_movement_with_direction(
                            current_pos,
                            next_waypoint,
                            waypoint_direction,
                        );
                    }
                }

                // A* 실패 시 방문 기록을 고려한 장애물 회피
                println!("[AI SMART HISTORY] A* failed, using advanced obstacle avoidance");
                return calculate_obstacle_avoidance_with_history(
                    current_pos,
                    target_pos,
                    grid_map,
                    visited_grids,
                );
            }
        }
    }

    // 경로가 깨끗하면 기본 카메라 기반 이동
    println!("[AI SMART HISTORY] Path clear, using basic camera movement");
    calculate_camera_based_movement(current_pos, target_pos)
}

/// 방문 기록에 페널티를 적용한 A* 경로탐색
fn smart_astar_pathfind_with_penalty(
    start: Vec3A,
    goal: Vec3A,
    grid_map: &GridMap2D,
    visited_grids: &std::collections::VecDeque<(i32, i32)>,
) -> Option<Vec<Vec3A>> {
    use crate::ai::ai_astar::grid_based_astar_pathfind;

    // 먼저 기본 A* 시도
    if let Some(path) = grid_based_astar_pathfind(start, goal, grid_map) {
        // 경로가 최근 방문한 그리드를 통과하는지 확인
        let mut visit_penalty = 0;
        let mut problem_waypoints = Vec::new();

        for (i, waypoint) in path.iter().enumerate() {
            if let Some((grid_x, grid_z)) = grid_map.world_to_grid(*waypoint) {
                let grid_coord = (grid_x as i32, grid_z as i32);

                // 최근 방문한 그리드인지 확인
                if let Some(index) = visited_grids.iter().rposition(|&g| g == grid_coord) {
                    let recency = visited_grids.len() - index;
                    if recency <= 15 {
                        // 최근 15개 그리드에 대해 페널티
                        let penalty = (16 - recency) * 3; // 더 높은 페널티
                        visit_penalty += penalty;
                        problem_waypoints.push((i, recency));
                    }
                }
            }
        }

        // 페널티가 너무 높으면 대안 경로 시도
        if visit_penalty > 20 {
            println!(
                "[AI SMART HISTORY] Path has high visit penalty ({}), trying alternatives",
                visit_penalty
            );
            println!(
                "[AI SMART HISTORY] Problem waypoints: {:?}",
                problem_waypoints
            );

            // 목표를 여러 방향으로 오프셋해서 대안 경로 시도
            let offset_goals = generate_advanced_offset_goals(goal, 3.0, 8); // 3m 오프셋, 8방향

            for (i, offset_goal) in offset_goals.iter().enumerate() {
                if let Some(alt_path) = grid_based_astar_pathfind(start, *offset_goal, grid_map) {
                    let mut alt_penalty = 0;

                    for waypoint in &alt_path {
                        if let Some((grid_x, grid_z)) = grid_map.world_to_grid(*waypoint) {
                            let grid_coord = (grid_x as i32, grid_z as i32);
                            if let Some(index) =
                                visited_grids.iter().rposition(|&g| g == grid_coord)
                            {
                                let recency = visited_grids.len() - index;
                                if recency <= 15 {
                                    alt_penalty += (16 - recency) * 2;
                                }
                            }
                        }
                    }

                    if alt_penalty < visit_penalty {
                        println!(
                            "[AI SMART HISTORY] Found better alternative path #{} (penalty: {} vs {})",
                            i, alt_penalty, visit_penalty
                        );
                        return Some(alt_path);
                    }
                }
            }
        }

        Some(path)
    } else {
        None
    }
}

/// 방문 기록을 고려한 고급 장애물 회피
fn calculate_obstacle_avoidance_with_history(
    current_pos: Vec3A,
    target_pos: Vec3A,
    grid_map: &GridMap2D,
    visited_grids: &std::collections::VecDeque<(i32, i32)>,
) -> mod_network::components::HeldInput {
    println!("[AI AVOIDANCE HISTORY] Calculating advanced obstacle avoidance with visit history");

    let target_direction = (target_pos - current_pos).normalize_or_zero();

    // 더 많은 방향 옵션 확인 (16방향)
    let directions = [
        Vec3A::new(1.0, 0.0, 0.0),               // 동
        Vec3A::new(-1.0, 0.0, 0.0),              // 서
        Vec3A::new(0.0, 0.0, 1.0),               // 북
        Vec3A::new(0.0, 0.0, -1.0),              // 남
        Vec3A::new(1.0, 0.0, 1.0).normalize(),   // 북동
        Vec3A::new(-1.0, 0.0, 1.0).normalize(),  // 북서
        Vec3A::new(1.0, 0.0, -1.0).normalize(),  // 남동
        Vec3A::new(-1.0, 0.0, -1.0).normalize(), // 남서
        Vec3A::new(0.5, 0.0, 1.0).normalize(),   // 북북동
        Vec3A::new(-0.5, 0.0, 1.0).normalize(),  // 북북서
        Vec3A::new(0.5, 0.0, -1.0).normalize(),  // 남남동
        Vec3A::new(-0.5, 0.0, -1.0).normalize(), // 남남서
        Vec3A::new(1.0, 0.0, 0.5).normalize(),   // 동동북
        Vec3A::new(-1.0, 0.0, 0.5).normalize(),  // 서서북
        Vec3A::new(1.0, 0.0, -0.5).normalize(),  // 동동남
        Vec3A::new(-1.0, 0.0, -0.5).normalize(), // 서서남
    ];

    let check_distance = 6.0; // 6m 거리까지 확인
    let mut best_direction = None;
    let mut best_score = f32::MIN;

    for &direction in &directions {
        let test_pos = current_pos + direction * check_distance;

        // 안전성 확인
        if !grid_map.is_walkable(test_pos) {
            continue; // 장애물이 있으면 제외
        }

        // 방문 기록 페널티 계산
        let mut visit_penalty = 0.0;
        if let Some((grid_x, grid_z)) = grid_map.world_to_grid(test_pos) {
            let test_grid = (grid_x as i32, grid_z as i32);

            if let Some(index) = visited_grids.iter().rposition(|&g| g == test_grid) {
                let recency = visited_grids.len() - index;
                if recency <= 10 {
                    visit_penalty = (11 - recency) as f32 * 10.0; // 높은 페널티
                }
            }
        }

        // 목표 방향과의 일치도 계산
        let alignment = direction.dot(target_direction);

        // 최종 목표까지의 거리 (더 가까울수록 좋음)
        let distance_to_goal = (target_pos - test_pos).length();
        let distance_score = 1.0 / (1.0 + distance_to_goal * 0.1);

        // 최종 점수 계산 (높을수록 좋음)
        let final_score = alignment * 100.0 + distance_score * 50.0 - visit_penalty;

        println!(
            "[AI AVOIDANCE HISTORY] Direction {:?}: alignment={:.2}, distance_score={:.2}, visit_penalty={:.1}, final_score={:.1}",
            direction, alignment, distance_score, visit_penalty, final_score
        );

        if final_score > best_score {
            best_score = final_score;
            best_direction = Some(direction);
        }
    }

    if let Some(chosen_direction) = best_direction {
        println!(
            "[AI AVOIDANCE HISTORY] Chose direction: {:?} (score: {:.1})",
            chosen_direction, best_score
        );
        return generate_movement_input(chosen_direction);
    }

    // 모든 방향이 막혀있으면 후진
    println!("[AI AVOIDANCE HISTORY] All directions problematic, backing up");
    mod_network::components::HeldInput::Backward
}

/// 고급 강제 탈출 이동 (방문 기록 고려)
fn force_escape_movement_advanced(
    current_pos: Vec3A,
    visited_grids: &std::collections::VecDeque<(i32, i32)>,
) -> mod_network::components::HeldInput {
    use rand::Rng;
    let mut rng = rand::rng();

    println!("[AI ESCAPE ADVANCED] Forcing advanced escape movement with visit history");

    // 12방향으로 확장된 탈출 시도
    let directions = [
        Vec3A::new(1.0, 0.0, 0.0),               // 동
        Vec3A::new(-1.0, 0.0, 0.0),              // 서
        Vec3A::new(0.0, 0.0, 1.0),               // 북
        Vec3A::new(0.0, 0.0, -1.0),              // 남
        Vec3A::new(1.0, 0.0, 1.0).normalize(),   // 북동
        Vec3A::new(-1.0, 0.0, 1.0).normalize(),  // 북서
        Vec3A::new(1.0, 0.0, -1.0).normalize(),  // 남동
        Vec3A::new(-1.0, 0.0, -1.0).normalize(), // 남서
        Vec3A::new(0.3, 0.0, 1.0).normalize(),   // 북북동 (미세 조정)
        Vec3A::new(-0.3, 0.0, 1.0).normalize(),  // 북북서 (미세 조정)
        Vec3A::new(0.3, 0.0, -1.0).normalize(),  // 남남동 (미세 조정)
        Vec3A::new(-0.3, 0.0, -1.0).normalize(), // 남남서 (미세 조정)
    ];

    // 최근 방문하지 않은 방향들 찾기
    let mut safe_directions = Vec::new();

    if let Ok(grid_map_cache) = GLOBAL_GRID_MAP_CACHE.lock() {
        if let Some(grid_map) = grid_map_cache.get(&StageKind::City) {
            for &dir in &directions {
                let test_pos = current_pos + dir * 4.0; // 4m 앞

                if grid_map.is_walkable(test_pos)
                    && !is_recently_visited_strict(test_pos, grid_map, visited_grids)
                {
                    safe_directions.push(dir);
                }
            }
        }
    }

    if !safe_directions.is_empty() {
        let chosen_dir = safe_directions[rng.random_range(0..safe_directions.len())];
        println!(
            "[AI ESCAPE ADVANCED] Chose unvisited direction: {:?} (from {} options)",
            chosen_dir,
            safe_directions.len()
        );
        return generate_movement_input(chosen_dir);
    }

    // 모든 방향이 최근 방문했다면 가장 오래된 방문 방향 선택
    println!("[AI ESCAPE ADVANCED] All directions recently visited, choosing oldest");
    let chosen_dir = directions[rng.random_range(0..directions.len())];
    generate_movement_input(chosen_dir)
}

/// 더 엄격한 방문 기록 확인
fn is_recently_visited_strict(
    pos: Vec3A,
    grid_map: &GridMap2D,
    visited_grids: &std::collections::VecDeque<(i32, i32)>,
) -> bool {
    if let Some((grid_x, grid_z)) = grid_map.world_to_grid(pos) {
        let grid_coord = (grid_x as i32, grid_z as i32);

        if let Some(index) = visited_grids.iter().rposition(|&g| g == grid_coord) {
            let recency = visited_grids.len() - index;
            return recency <= 15; // 최근 15개 그리드에 포함되면 최근 방문 (더 엄격)
        }
    }
    false
}

/// 고급 오프셋 목표들 생성 (더 정밀한 방향)
fn generate_advanced_offset_goals(
    goal: Vec3A,
    offset_distance: f32,
    num_directions: usize,
) -> Vec<Vec3A> {
    let mut goals = Vec::new();

    for i in 0..num_directions {
        let angle = (i as f32) * 2.0 * std::f32::consts::PI / (num_directions as f32);
        let offset = Vec3A::new(
            angle.cos() * offset_distance,
            0.0,
            angle.sin() * offset_distance,
        );
        goals.push(goal + offset);
    }

    goals
}

/// 장애물 회피 + 카메라 기반 스마트 이동 (벽 앞 왕복운동 방지)
fn calculate_smart_camera_movement(
    current_pos: Vec3A,
    target_pos: Vec3A,
    stage: &mod_network::components::StageAttributes,
) -> mod_network::components::HeldInput {
    let distance = (target_pos - current_pos).length();

    // 목표가 매우 가까우면 정지
    if distance < 1.0 {
        println!(
            "[AI SMART] Target very close ({:.2}m), stopping movement",
            distance
        );
        return mod_network::components::HeldInput::empty();
    }

    // 그리드 맵을 이용한 장애물 검사
    if let Ok(grid_map_cache) = GLOBAL_GRID_MAP_CACHE.lock() {
        if let Some(grid_map) = grid_map_cache.get(&StageKind::City) {
            // 현재 위치가 안전한지 확인
            if !grid_map.is_walkable(current_pos) {
                println!("[AI SMART] Current position unsafe, escaping");
                return find_escape_movement(current_pos, grid_map);
            }

            // 목표 방향으로 직진했을 때 장애물이 있는지 확인
            let target_direction = (target_pos - current_pos).normalize_or_zero();
            let check_distance = 5.0; // 5m 앞까지 확인
            let check_pos = current_pos + target_direction * check_distance;

            // 직진 경로에 장애물이 있으면 우회 경로 탐색
            if !grid_map.is_walkable(check_pos) || !grid_map.is_walkable(target_pos) {
                println!("[AI SMART] Obstacle detected in direct path, using A* pathfinding");

                // A* 경로탐색으로 우회 경로 찾기
                if let Some(path) = grid_based_astar_pathfind(current_pos, target_pos, grid_map) {
                    if path.len() >= 2 {
                        let next_waypoint = path[1];
                        let waypoint_direction = (next_waypoint - current_pos).normalize_or_zero();

                        // 웨이포인트 방향으로 카메라 기반 이동
                        return calculate_camera_based_movement_with_direction(
                            current_pos,
                            next_waypoint,
                            waypoint_direction,
                        );
                    }
                }

                // A* 실패 시 장애물 회피 이동
                println!("[AI SMART] A* failed, using obstacle avoidance");
                return calculate_obstacle_avoidance_movement(current_pos, target_pos, grid_map);
            }
        }
    }

    // 장애물이 없으면 기본 카메라 기반 이동
    calculate_camera_based_movement(current_pos, target_pos)
}

/// 방향 지정 카메라 기반 이동 (A* 웨이포인트용)
fn calculate_camera_based_movement_with_direction(
    current_pos: Vec3A,
    target_pos: Vec3A,
    direction: Vec3A,
) -> mod_network::components::HeldInput {
    let distance = (target_pos - current_pos).length();

    if distance < 1.0 {
        return mod_network::components::HeldInput::empty();
    }

    // 부드러운 곡선 이동을 위한 조합 입력 생성
    let mut input = mod_network::components::HeldInput::empty();

    // 항상 전진하면서 좌우 조정으로 곡선 이동
    input |= mod_network::components::HeldInput::Forward;

    // 방향에 따라 좌우 조정 (부드러운 커브)
    let angle_threshold = 0.2; // 더 민감한 곡선 이동

    if direction.x > angle_threshold {
        input |= mod_network::components::HeldInput::Right;
        println!(
            "[AI SMART] A* path curve RIGHT (distance: {:.2}m)",
            distance
        );
    } else if direction.x < -angle_threshold {
        input |= mod_network::components::HeldInput::Left;
        println!("[AI SMART] A* path curve LEFT (distance: {:.2}m)", distance);
    } else {
        println!(
            "[AI SMART] A* path straight FORWARD (distance: {:.2}m)",
            distance
        );
    }

    // 후진이 필요한 경우
    if direction.z < -0.3 {
        input = mod_network::components::HeldInput::empty();
        input |= mod_network::components::HeldInput::Backward;

        if direction.x > angle_threshold {
            input |= mod_network::components::HeldInput::Left; // 후진 시 반전
        } else if direction.x < -angle_threshold {
            input |= mod_network::components::HeldInput::Right; // 후진 시 반전
        }

        println!(
            "[AI SMART] A* path backing up with curve (distance: {:.2}m)",
            distance
        );
    }

    input
}

/// 장애물 회피 이동 (벽 앞 왕복운동 방지)
fn calculate_obstacle_avoidance_movement(
    current_pos: Vec3A,
    target_pos: Vec3A,
    grid_map: &GridMap2D,
) -> mod_network::components::HeldInput {
    println!("[AI AVOIDANCE] Calculating obstacle avoidance movement");

    let target_direction = (target_pos - current_pos).normalize_or_zero();

    // 목표 방향의 좌우 수직 방향 계산
    let perpendicular_right = Vec3A::new(-target_direction.z, 0.0, target_direction.x);
    let perpendicular_left = Vec3A::new(target_direction.z, 0.0, -target_direction.x);

    let check_distance = 8.0; // 8m 거리까지 확인

    // 오른쪽 우회 경로 확인
    let right_pos = current_pos + perpendicular_right * check_distance;
    let right_safe = grid_map.is_walkable(right_pos);

    // 왼쪽 우회 경로 확인
    let left_pos = current_pos + perpendicular_left * check_distance;
    let left_safe = grid_map.is_walkable(left_pos);

    let mut input = mod_network::components::HeldInput::empty();

    if right_safe && left_safe {
        // 둘 다 안전하면 목표에 더 가까운 쪽 선택
        let right_to_target = (target_pos - right_pos).length();
        let left_to_target = (target_pos - left_pos).length();

        if right_to_target < left_to_target {
            input |= mod_network::components::HeldInput::Right;
            input |= mod_network::components::HeldInput::Forward;
            println!("[AI AVOIDANCE] Both sides safe, choosing right (closer to target)");
        } else {
            input |= mod_network::components::HeldInput::Left;
            input |= mod_network::components::HeldInput::Forward;
            println!("[AI AVOIDANCE] Both sides safe, choosing left (closer to target)");
        }
    } else if right_safe {
        input |= mod_network::components::HeldInput::Right;
        input |= mod_network::components::HeldInput::Forward;
        println!("[AI AVOIDANCE] Only right side safe, moving right");
    } else if left_safe {
        input |= mod_network::components::HeldInput::Left;
        input |= mod_network::components::HeldInput::Forward;
        println!("[AI AVOIDANCE] Only left side safe, moving left");
    } else {
        // 양쪽 모두 막혀있으면 후진
        input |= mod_network::components::HeldInput::Backward;
        println!("[AI AVOIDANCE] Both sides blocked, backing up");
    }

    input
}

/// 카메라 방향 기반 부드러운 곡선 이동 (점진적 회전 + Forward/Left/Right 조합)
fn calculate_camera_based_movement(
    current_pos: Vec3A,
    target_pos: Vec3A,
) -> mod_network::components::HeldInput {
    let distance = (target_pos - current_pos).length();

    // 목표가 매우 가까우면 정지
    if distance < 1.0 {
        println!(
            "[AI CAMERA] Target very close ({:.2}m), stopping movement",
            distance
        );
        return mod_network::components::HeldInput::empty();
    }

    // 목표 방향 계산
    let target_direction = (target_pos - current_pos).normalize_or_zero();

    // 현재 카메라 방향과 목표 방향 사이의 각도 차이 계산
    let target_angle = target_direction.z.atan2(target_direction.x);

    // 부드러운 곡선 이동을 위한 조합 입력 생성
    let mut input = mod_network::components::HeldInput::empty();

    // 항상 전진하면서 좌우 조정으로 곡선 이동
    input |= mod_network::components::HeldInput::Forward;

    // 목표 방향에 따라 좌우 조정 (부드러운 커브)
    let angle_threshold = 0.3; // 약 17도 - 부드러운 회전을 위한 임계값

    if target_direction.x > angle_threshold {
        // 오른쪽으로 부드럽게 커브
        input |= mod_network::components::HeldInput::Right;
        println!(
            "[AI CAMERA] Smooth curve RIGHT towards target (angle: {:.1}°)",
            target_angle.to_degrees()
        );
    } else if target_direction.x < -angle_threshold {
        // 왼쪽으로 부드럽게 커브
        input |= mod_network::components::HeldInput::Left;
        println!(
            "[AI CAMERA] Smooth curve LEFT towards target (angle: {:.1}°)",
            target_angle.to_degrees()
        );
    } else {
        // 거의 직진
        println!(
            "[AI CAMERA] Moving straight FORWARD towards target (angle: {:.1}°)",
            target_angle.to_degrees()
        );
    }

    // 후진이 필요한 경우 (목표가 뒤쪽에 있을 때)
    if target_direction.z < -0.5 {
        // Forward 대신 Backward 사용
        input = mod_network::components::HeldInput::empty();
        input |= mod_network::components::HeldInput::Backward;

        // 후진 시에는 좌우 반전
        if target_direction.x > angle_threshold {
            input |= mod_network::components::HeldInput::Left; // 후진 시 반전
        } else if target_direction.x < -angle_threshold {
            input |= mod_network::components::HeldInput::Right; // 후진 시 반전
        }

        println!(
            "[AI CAMERA] Backing up with curve (target behind, angle: {:.1}°)",
            target_angle.to_degrees()
        );
    }

    println!(
        "[AI CAMERA] Smooth movement input: {:?} (distance: {:.2}m)",
        input, distance
    );

    input
}

/// 집결 구역 도달을 위한 직선 이동 (랜덤 운동 없음)
fn calculate_direct_movement_to_target(
    current_pos: Vec3A,
    target_pos: Vec3A,
) -> mod_network::components::HeldInput {
    let direction = target_pos - current_pos;
    let distance = direction.length();

    // 목표가 매우 가까우면 정지 (랜덤 이동 대신)
    if distance < 1.0 {
        println!(
            "[AI DIRECT] Target very close ({:.2}m), stopping movement",
            distance
        );
        return mod_network::components::HeldInput::empty();
    }

    let normalized = direction.normalize_or_zero();
    let mut input = mod_network::components::HeldInput::empty();

    // 명확한 임계값으로 정확한 방향 설정
    let threshold = 0.1;

    if normalized.x > threshold {
        input |= mod_network::components::HeldInput::Right;
    } else if normalized.x < -threshold {
        input |= mod_network::components::HeldInput::Left;
    }

    if normalized.z > threshold {
        input |= mod_network::components::HeldInput::Forward;
    } else if normalized.z < -threshold {
        input |= mod_network::components::HeldInput::Backward;
    }

    // 입력이 비어있으면 가장 큰 성분 방향으로 강제 이동
    if input.is_empty() {
        if normalized.x.abs() > normalized.z.abs() {
            if normalized.x > 0.0 {
                input |= mod_network::components::HeldInput::Right;
            } else {
                input |= mod_network::components::HeldInput::Left;
            }
        } else {
            if normalized.z > 0.0 {
                input |= mod_network::components::HeldInput::Forward;
            } else {
                input |= mod_network::components::HeldInput::Backward;
            }
        }
    }

    println!(
        "[AI DIRECT] Moving directly to target: direction={:?}, distance={:.2}m, input={:?}",
        normalized, distance, input
    );

    input
}

/// 방문 기록을 고려한 스마트 이동 (맴돌기 방지)
fn calculate_smart_movement(
    current_pos: Vec3A,
    target_pos: Vec3A,
    _stage: &mod_network::components::StageAttributes,
    visited_grids: &std::collections::VecDeque<(i32, i32)>,
    stuck_counter: u32,
) -> mod_network::components::HeldInput {
    // **핵심**: 최근 방문한 그리드를 피하면서 목표로 이동

    // 제자리에 너무 오래 있으면 강제 탈출
    if stuck_counter > 30 {
        // 약 0.5초간 제자리
        println!("[AI SMART] Stuck detected! Forcing escape movement");
        return force_escape_movement(current_pos, visited_grids);
    }

    // 전역 그리드 맵 가져오기
    if let Ok(grid_map_cache) = GLOBAL_GRID_MAP_CACHE.lock() {
        if let Some(grid_map) = grid_map_cache.get(&StageKind::City) {
            println!("[AI SMART] Using smart pathfinding with visit avoidance");

            // 현재 위치가 막혀있으면 탈출
            if !grid_map.is_walkable(current_pos) {
                println!("[AI SMART] Current position blocked, escaping");
                return find_escape_movement(current_pos, grid_map);
            }

            // 목표가 막혀있으면 대안 찾기
            let effective_target = if !grid_map.is_walkable(target_pos) {
                println!("[AI SMART] Target blocked, finding alternative");
                find_safe_position_near(target_pos, grid_map).unwrap_or(target_pos)
            } else {
                target_pos
            };

            // **스마트 A* 경로탐색**: 방문 기록을 고려
            if let Some(path) =
                smart_astar_pathfind(current_pos, effective_target, grid_map, visited_grids)
            {
                if path.len() >= 2 {
                    let next_waypoint = path[1];
                    let direction = next_waypoint - current_pos;

                    if direction.length() > 0.1 {
                        let normalized_dir = direction.normalize();
                        let mut input = mod_network::components::HeldInput::empty();

                        // 매우 민감한 이동 판정
                        let move_threshold = 0.1;

                        if normalized_dir.x.abs() > move_threshold {
                            if normalized_dir.x > 0.0 {
                                input |= mod_network::components::HeldInput::Right;
                            } else {
                                input |= mod_network::components::HeldInput::Left;
                            }
                        }
                        if normalized_dir.z.abs() > move_threshold {
                            if normalized_dir.z > 0.0 {
                                input |= mod_network::components::HeldInput::Forward;
                            } else {
                                input |= mod_network::components::HeldInput::Backward;
                            }
                        }

                        if !input.is_empty() {
                            println!(
                                "[AI SMART] Smart path found: {} waypoints, avoiding {} visited grids",
                                path.len(),
                                visited_grids.len()
                            );
                            return input;
                        }
                    }
                }
            }

            // A* 실패 시: 직선 이동하되 방문한 그리드 피하기
            return calculate_avoidance_movement(
                current_pos,
                effective_target,
                grid_map,
                visited_grids,
            );
        }
    }

    // 폴백: 기본 적극적 이동
    calculate_aggressive_movement(current_pos, target_pos)
}

/// 방문 기록을 고려한 A* 경로탐색
fn smart_astar_pathfind(
    start: Vec3A,
    goal: Vec3A,
    grid_map: &GridMap2D,
    visited_grids: &std::collections::VecDeque<(i32, i32)>,
) -> Option<Vec<Vec3A>> {
    use crate::ai::ai_astar::grid_based_astar_pathfind;

    // 먼저 기본 A* 시도
    if let Some(path) = grid_based_astar_pathfind(start, goal, grid_map) {
        // 경로가 최근 방문한 그리드를 너무 많이 통과하는지 확인
        let mut visit_penalty = 0;

        for waypoint in &path {
            if let Some((grid_x, grid_z)) = grid_map.world_to_grid(*waypoint) {
                let grid_coord = (grid_x as i32, grid_z as i32);

                // 최근 10개 그리드에 있으면 페널티
                if let Some(index) = visited_grids.iter().rposition(|&g| g == grid_coord) {
                    let recency = visited_grids.len() - index;
                    if recency <= 10 {
                        visit_penalty += (11 - recency) * 2; // 최근일수록 높은 페널티
                    }
                }
            }
        }

        // 페널티가 너무 높으면 다른 경로 시도
        if visit_penalty > 15 {
            println!(
                "[AI SMART] Path has too many recently visited grids (penalty: {}), trying alternative",
                visit_penalty
            );

            // 목표를 약간 변경해서 다른 경로 시도
            let offset_goals = generate_offset_goals(goal, 5.0); // 5m 오프셋

            for offset_goal in offset_goals {
                if let Some(alt_path) = grid_based_astar_pathfind(start, offset_goal, grid_map) {
                    let mut alt_penalty = 0;

                    for waypoint in &alt_path {
                        if let Some((grid_x, grid_z)) = grid_map.world_to_grid(*waypoint) {
                            let grid_coord = (grid_x as i32, grid_z as i32);
                            if let Some(index) =
                                visited_grids.iter().rposition(|&g| g == grid_coord)
                            {
                                let recency = visited_grids.len() - index;
                                if recency <= 10 {
                                    alt_penalty += 11 - recency;
                                }
                            }
                        }
                    }

                    if alt_penalty < visit_penalty {
                        println!(
                            "[AI SMART] Found better alternative path (penalty: {} vs {})",
                            alt_penalty, visit_penalty
                        );
                        return Some(alt_path);
                    }
                }
            }
        }

        Some(path)
    } else {
        None
    }
}

/// 방문 기록을 피하는 직선 이동
fn calculate_avoidance_movement(
    current_pos: Vec3A,
    target_pos: Vec3A,
    grid_map: &GridMap2D,
    visited_grids: &std::collections::VecDeque<(i32, i32)>,
) -> mod_network::components::HeldInput {
    let direction = target_pos - current_pos;

    if direction.length() < 2.0 {
        return generate_random_movement();
    }

    let normalized = direction.normalize_or_zero();

    // 목표 방향으로 몇 그리드 앞을 확인
    let check_distance = grid_map.grid_size * 3.0; // 3그리드 앞까지
    let check_pos = current_pos + normalized * check_distance;

    if let Some((check_x, check_z)) = grid_map.world_to_grid(check_pos) {
        let check_grid = (check_x as i32, check_z as i32);

        // 확인할 위치가 최근 방문한 곳이면 회피
        if let Some(index) = visited_grids.iter().rposition(|&g| g == check_grid) {
            let recency = visited_grids.len() - index;
            if recency <= 5 {
                // 최근 5개 그리드에 포함되면
                println!(
                    "[AI AVOIDANCE] Target direction leads to recently visited grid, finding alternative"
                );

                // 수직 방향으로 회피 시도
                let perpendicular = Vec3A::new(-normalized.z, 0.0, normalized.x);

                // 양쪽 방향 중 더 안전한 쪽 선택
                let left_pos = current_pos + perpendicular * check_distance;
                let right_pos = current_pos - perpendicular * check_distance;

                let left_safe = grid_map.is_walkable(left_pos)
                    && !is_recently_visited(left_pos, grid_map, visited_grids);
                let right_safe = grid_map.is_walkable(right_pos)
                    && !is_recently_visited(right_pos, grid_map, visited_grids);

                if left_safe && !right_safe {
                    return generate_movement_input(perpendicular);
                } else if right_safe && !left_safe {
                    return generate_movement_input(-perpendicular);
                } else if left_safe && right_safe {
                    // 둘 다 안전하면 랜덤 선택
                    let choice_dir = if rand::random::<bool>() {
                        perpendicular
                    } else {
                        -perpendicular
                    };
                    return generate_movement_input(choice_dir);
                }
            }
        }
    }

    // 기본 목표 방향으로 이동
    generate_movement_input(normalized)
}

/// 강제 탈출 이동 (제자리에서 벗어나기)
fn force_escape_movement(
    current_pos: Vec3A,
    visited_grids: &std::collections::VecDeque<(i32, i32)>,
) -> mod_network::components::HeldInput {
    use rand::Rng;
    let mut rng = rand::rng();

    // 8방향 중 최근 방문하지 않은 방향 찾기
    let directions = [
        Vec3A::new(1.0, 0.0, 0.0),               // 동
        Vec3A::new(-1.0, 0.0, 0.0),              // 서
        Vec3A::new(0.0, 0.0, 1.0),               // 북
        Vec3A::new(0.0, 0.0, -1.0),              // 남
        Vec3A::new(1.0, 0.0, 1.0).normalize(),   // 북동
        Vec3A::new(-1.0, 0.0, 1.0).normalize(),  // 북서
        Vec3A::new(1.0, 0.0, -1.0).normalize(),  // 남동
        Vec3A::new(-1.0, 0.0, -1.0).normalize(), // 남서
    ];

    // 최근 방문하지 않은 방향들 필터링
    let mut safe_directions = Vec::new();

    for dir in directions.iter() {
        let test_pos = current_pos + *dir * 5.0; // 5m 앞

        if let Ok(grid_map_cache) = GLOBAL_GRID_MAP_CACHE.lock() {
            if let Some(grid_map) = grid_map_cache.get(&StageKind::City) {
                if grid_map.is_walkable(test_pos)
                    && !is_recently_visited(test_pos, grid_map, visited_grids)
                {
                    safe_directions.push(*dir);
                }
            }
        }
    }

    if !safe_directions.is_empty() {
        let chosen_dir = safe_directions[rng.random_range(0..safe_directions.len())];
        println!(
            "[AI ESCAPE] Forcing escape in unvisited direction: {:?}",
            chosen_dir
        );
        return generate_movement_input(chosen_dir);
    }

    // 모든 방향이 최근 방문했다면 완전 랜덤
    println!("[AI ESCAPE] All directions recently visited, using random escape");
    generate_random_movement()
}

/// 위치가 최근 방문한 곳인지 확인
fn is_recently_visited(
    pos: Vec3A,
    grid_map: &GridMap2D,
    visited_grids: &std::collections::VecDeque<(i32, i32)>,
) -> bool {
    if let Some((grid_x, grid_z)) = grid_map.world_to_grid(pos) {
        let grid_coord = (grid_x as i32, grid_z as i32);

        if let Some(index) = visited_grids.iter().rposition(|&g| g == grid_coord) {
            let recency = visited_grids.len() - index;
            return recency <= 8; // 최근 8개 그리드에 포함되면 최근 방문
        }
    }
    false
}

/// 목표 주변 오프셋 목표들 생성
fn generate_offset_goals(goal: Vec3A, offset_distance: f32) -> Vec<Vec3A> {
    let offsets = [
        Vec3A::new(offset_distance, 0.0, 0.0),
        Vec3A::new(-offset_distance, 0.0, 0.0),
        Vec3A::new(0.0, 0.0, offset_distance),
        Vec3A::new(0.0, 0.0, -offset_distance),
        Vec3A::new(offset_distance, 0.0, offset_distance),
        Vec3A::new(-offset_distance, 0.0, offset_distance),
        Vec3A::new(offset_distance, 0.0, -offset_distance),
        Vec3A::new(-offset_distance, 0.0, -offset_distance),
    ];

    offsets.iter().map(|offset| goal + *offset).collect()
}

/// 방향 벡터를 입력으로 변환
fn generate_movement_input(direction: Vec3A) -> mod_network::components::HeldInput {
    let normalized = direction.normalize_or_zero();
    let mut input = mod_network::components::HeldInput::empty();

    let threshold = 0.05;

    if normalized.x > threshold {
        input |= mod_network::components::HeldInput::Right;
    } else if normalized.x < -threshold {
        input |= mod_network::components::HeldInput::Left;
    }

    if normalized.z > threshold {
        input |= mod_network::components::HeldInput::Forward;
    } else if normalized.z < -threshold {
        input |= mod_network::components::HeldInput::Backward;
    }

    if input.is_empty() {
        // 방향이 애매하면 랜덤 선택
        return generate_random_movement();
    }

    input
}

/// 그리드 맵 기반 경로탐색 이동 (장애물 회피하면서 계속 이동)
fn calculate_grid_based_movement(
    current_pos: Vec3A,
    target_pos: Vec3A,
    _stage: &mod_network::components::StageAttributes,
) -> mod_network::components::HeldInput {
    // **핵심**: 장애물을 만나도 계속 움직이도록 개선된 로직

    // 전역 그리드 맵 가져오기
    if let Ok(grid_map_cache) = GLOBAL_GRID_MAP_CACHE.lock() {
        if let Some(grid_map) = grid_map_cache.get(&StageKind::City) {
            println!("[AI MOVEMENT] Grid map available, using flexible pathfinding");

            // **새로운 접근**: 현재 위치가 막혀있어도 근처 안전한 곳으로 이동 시도
            if !grid_map.is_walkable(current_pos) {
                println!(
                    "[AI MOVEMENT] Current position blocked, finding escape route: {:?}",
                    current_pos
                );
                return find_escape_movement(current_pos, grid_map);
            }

            // **유연한 목표 처리**: 목표가 막혀있으면 대안 목표 생성하되, 계속 움직임
            let effective_target = if !grid_map.is_walkable(target_pos) {
                println!(
                    "[AI MOVEMENT] Target blocked, finding alternative near: {:?}",
                    target_pos
                );
                // 목표 근처에서 안전한 위치 찾기 (실패해도 원래 목표 유지)
                find_safe_position_near(target_pos, grid_map).unwrap_or(target_pos)
            } else {
                target_pos
            };

            // **A* 경로탐색 시도**: 실패해도 직선 이동으로 폴백
            if let Some(path) = grid_based_astar_pathfind(current_pos, effective_target, grid_map) {
                if path.len() >= 2 {
                    let next_waypoint = path[1];
                    let direction = next_waypoint - current_pos;
                    let distance = direction.length();

                    if distance > 0.1 {
                        let normalized_dir = direction.normalize();
                        let mut input = mod_network::components::HeldInput::empty();

                        // **더 활발한 방향 판정**: 매우 작은 움직임도 허용하여 다양한 이동
                        let move_threshold = 0.1; // 기존 0.3에서 대폭 감소하여 더 민감한 반응

                        if normalized_dir.x.abs() > move_threshold {
                            if normalized_dir.x > 0.0 {
                                input |= mod_network::components::HeldInput::Right;
                            } else {
                                input |= mod_network::components::HeldInput::Left;
                            }
                        }
                        if normalized_dir.z.abs() > move_threshold {
                            if normalized_dir.z > 0.0 {
                                input |= mod_network::components::HeldInput::Forward;
                            } else {
                                input |= mod_network::components::HeldInput::Backward;
                            }
                        }

                        // 입력이 비어있으면 랜덤 이동
                        if input.is_empty() {
                            println!("[AI A*] No input generated, using random movement");
                            return generate_random_movement();
                        }

                        println!(
                            "[AI A*] Path found: {} waypoints, moving to {:?}, input: {:?}",
                            path.len(),
                            next_waypoint,
                            input
                        );

                        return input;
                    }
                } else {
                    println!("[AI A*] Path too short, using direct movement");
                }
            } else {
                println!("[AI A*] A* failed, using direct movement fallback");
            }
        } else {
            println!("[AI MOVEMENT] No grid map loaded, using direct movement");
        }
    } else {
        println!("[AI MOVEMENT] Failed to lock grid map, using direct movement");
    }

    // **폴백**: 항상 움직이는 직선 이동 (장애물 무시하고 계속 이동)
    calculate_aggressive_movement(current_pos, target_pos)
}

/// 현재 위치가 막혀있을 때 탈출 이동 (적극적으로 계속 움직임)
fn find_escape_movement(
    current_pos: Vec3A,
    grid_map: &GridMap2D,
) -> mod_network::components::HeldInput {
    println!("[AI ESCAPE] Finding escape route from: {:?}", current_pos);

    // 주변 8방향으로 안전한 위치 찾기 (더 적극적으로)
    let directions = [
        (1.0, 0.0),   // 동
        (-1.0, 0.0),  // 서
        (0.0, 1.0),   // 북
        (0.0, -1.0),  // 남
        (1.0, 1.0),   // 북동
        (-1.0, 1.0),  // 북서
        (1.0, -1.0),  // 남동
        (-1.0, -1.0), // 남서
    ];

    let step_size = grid_map.grid_size; // 1그리드씩 탐색 (더 공격적)

    for (dx, dz) in directions.iter() {
        let test_pos = current_pos + Vec3A::new(dx * step_size, 0.0, dz * step_size);
        if grid_map.is_walkable(test_pos) {
            println!("[AI ESCAPE] Found escape direction: {:?}", (dx, dz));
            let mut input = mod_network::components::HeldInput::empty();

            if *dx > 0.0 {
                input |= mod_network::components::HeldInput::Right;
            } else if *dx < 0.0 {
                input |= mod_network::components::HeldInput::Left;
            }

            if *dz > 0.0 {
                input |= mod_network::components::HeldInput::Forward;
            } else if *dz < 0.0 {
                input |= mod_network::components::HeldInput::Backward;
            }

            return input;
        }
    }

    // 탈출 방향을 못 찾아도 랜덤하게 계속 움직임 (완전 정지 방지)
    println!("[AI ESCAPE] No clear escape, moving randomly");
    generate_random_movement()
}

/// 적극적인 이동 (장애물 무시하고 계속 이동)
fn calculate_aggressive_movement(
    current_pos: Vec3A,
    target_pos: Vec3A,
) -> mod_network::components::HeldInput {
    let direction = target_pos - current_pos;
    let distance = direction.length();

    // 매우 가까우면 랜덤 이동
    if distance < 2.0 {
        return generate_random_movement();
    }

    let normalized = direction.normalize_or_zero();
    let mut input = mod_network::components::HeldInput::empty();

    // **매우 관대한 임계값**: 아주 작은 움직임도 허용하여 계속 이동
    let threshold = 0.05; // 기존 0.2에서 대폭 감소하여 더 활발한 움직임

    if normalized.x > threshold {
        input |= mod_network::components::HeldInput::Right;
    } else if normalized.x < -threshold {
        input |= mod_network::components::HeldInput::Left;
    }

    if normalized.z > threshold {
        input |= mod_network::components::HeldInput::Forward;
    } else if normalized.z < -threshold {
        input |= mod_network::components::HeldInput::Backward;
    }

    // 입력이 비어있으면 랜덤 이동으로 폴백
    if input.is_empty() {
        println!("[AI AGGRESSIVE] No clear direction, moving randomly");
        return generate_random_movement();
    }

    println!(
        "[AI AGGRESSIVE] Moving aggressively: direction={:?}, input={:?}",
        normalized, input
    );
    input
}

/// 완전 랜덤 이동 생성 (정지 방지)
fn generate_random_movement() -> mod_network::components::HeldInput {
    use rand::Rng;
    let mut rng = rand::rng();
    // 8방향 중 랜덤 선택
    let directions = [
        mod_network::components::HeldInput::Forward,
        mod_network::components::HeldInput::Backward,
        mod_network::components::HeldInput::Left,
        mod_network::components::HeldInput::Right,
        mod_network::components::HeldInput::Forward | mod_network::components::HeldInput::Right,
        mod_network::components::HeldInput::Forward | mod_network::components::HeldInput::Left,
        mod_network::components::HeldInput::Backward | mod_network::components::HeldInput::Right,
        mod_network::components::HeldInput::Backward | mod_network::components::HeldInput::Left,
    ];

    let random_index = rng.random_range(0..directions.len());
    let input = directions[random_index];

    println!("[AI RANDOM] Generated random movement: {:?}", input);
    input
}

/// 목표 근처의 안전한 위치 찾기
fn find_safe_position_near(target_pos: Vec3A, grid_map: &GridMap2D) -> Option<Vec3A> {
    let step = grid_map.grid_size;

    // 나선형으로 안전한 위치 검색
    for radius in 1..=8 {
        let search_dist = radius as f32 * step;
        let steps = (radius * 8) as i32; // 더 조밀한 검색

        for i in 0..steps {
            let angle = (i as f32) * 2.0 * std::f32::consts::PI / (steps as f32);
            let test_pos =
                target_pos + Vec3A::new(angle.cos() * search_dist, 0.0, angle.sin() * search_dist);

            if grid_map.is_walkable(test_pos) {
                println!(
                    "[SAFE SEARCH] Found safe alternative at radius {}: {:?}",
                    radius, test_pos
                );
                return Some(test_pos);
            }
        }
    }

    None
}

fn get_respawn_position(team: Team, index: usize) -> Vec3A {
    let stage = get_current_stage_attributes();
    match team {
        Team::Blue => stage.blue_team_positions[index],
        Team::Red => stage.red_team_positions[index],
    }
}

fn get_current_stage_attributes() -> &'static mod_network::components::StageAttributes {
    get_stage_attributes(StageKind::City)
}

/// 서버 시작 시 모든 스테이지의 그리드 맵을 미리 생성하여 캐시에 저장
pub fn preload_all_grid_maps() {
    log::info!("[AI GRID PRELOAD] Starting preload of all stage grid maps...");

    // 모든 스테이지 종류에 대해 그리드 맵 생성
    let all_stages = [StageKind::City]; // 필요시 다른 스테이지도 추가

    for &stage_kind in &all_stages {
        preload_grid_map_for_stage(stage_kind);
    }

    log::info!("[AI GRID PRELOAD] All grid maps preloaded successfully!");
}

/// 특정 스테이지의 그리드 맵을 미리 생성하여 캐시에 저장
fn preload_grid_map_for_stage(stage_kind: StageKind) {
    log::info!(
        "[AI GRID PRELOAD] Loading grid map for stage: {:?} from CSV file...",
        stage_kind
    );

    // CSV 파일 경로 결정 (프로젝트 루트 기준)
    let csv_path = match stage_kind {
        StageKind::City => "assets/stage/grid_map_City.csv",
    };

    // CSV 파일에서 그리드 맵 로드
    match GridMap2D::from_csv(csv_path) {
        Ok(grid_map) => {
            // 그리드 맵 통계 출력
            grid_map.print_stats();

            // 전역 캐시에 저장
            if let Ok(mut grid_map_cache) = GLOBAL_GRID_MAP_CACHE.lock() {
                grid_map_cache.insert(stage_kind, grid_map);
                log::info!(
                    "[AI GRID PRELOAD] Grid map for stage {:?} successfully loaded from CSV and cached",
                    stage_kind
                );
            } else {
                log::error!(
                    "[AI GRID PRELOAD] Failed to lock global grid map cache mutex for stage {:?}",
                    stage_kind
                );
            }
        }
        Err(e) => {
            log::error!(
                "[AI GRID PRELOAD] Failed to load CSV grid map for stage {:?}: {:?}",
                stage_kind,
                e
            );
            log::warn!(
                "[AI GRID PRELOAD] Falling back to runtime generation for stage {:?}",
                stage_kind
            );

            // 폴백: 런타임 생성
            preload_grid_map_for_stage_runtime(stage_kind);
        }
    }
}

/// 런타임에 그리드 맵 생성 (폴백 함수)
fn preload_grid_map_for_stage_runtime(stage_kind: StageKind) {
    let stage = get_stage_attributes(stage_kind);

    // 임시 플레이어 생성해서 캐릭터 속성 가져오기
    let mut temp_player = Player::new(
        UserName::from_str("TempPlayer"),
        ProfileIcon::default(),
        Permission::User,
        GameTier::Bronze,
    );
    temp_player.set_character_kind(CharacterKind::ArisOriginal);
    let character_attributes = temp_player.character_attributes();

    // 그리드 맵 생성 (더 넓은 탐험을 위한 설정)
    let grid_size = 2.0; // 2m x 2m 그리드 유지
    let map_size = 150.0; // ±150m (300m x 300m 맵)으로 확장

    log::info!(
        "[AI GRID PRELOAD] Creating grid map for stage: {:?} ({}m grid, {}m range)",
        stage_kind,
        grid_size,
        map_size
    );

    match std::panic::catch_unwind(|| {
        GridMap2D::from_stage(stage, &character_attributes, grid_size, map_size)
    }) {
        Ok(grid_map) => {
            // 그리드 맵 통계 출력
            grid_map.print_stats();

            // 전역 캐시에 저장
            if let Ok(mut grid_map_cache) = GLOBAL_GRID_MAP_CACHE.lock() {
                grid_map_cache.insert(stage_kind, grid_map);
                log::info!(
                    "[AI GRID PRELOAD] Grid map for stage {:?} successfully cached",
                    stage_kind
                );
            } else {
                log::error!(
                    "[AI GRID PRELOAD] Failed to lock global grid map cache mutex회저 for stage {:?}",
                    stage_kind
                );
            }
        }
        Err(e) => {
            log::error!(
                "[AI GRID PRELOAD] Failed to create grid map for stage {:?}: {:?}",
                stage_kind,
                e
            );
            log::warn!(
                "[AI GRID PRELOAD] Stage {:?} will use runtime obstacle detection",
                stage_kind
            );
        }
    }
}

/// 캐시된 그리드 맵을 빠르게 가져오는 함수
pub fn get_cached_grid_map(stage_kind: StageKind) -> Option<GridMap2D> {
    if let Ok(grid_map_cache) = GLOBAL_GRID_MAP_CACHE.lock() {
        grid_map_cache.get(&stage_kind).cloned()
    } else {
        log::error!(
            "[AI GRID CACHE] Failed to lock grid map cache for stage {:?}",
            stage_kind
        );
        None
    }
}

/// 스테이지용 그리드 맵 로딩 함수 (기존 함수 - 이제 캐시에서 빠르게 가져옴)
pub fn load_grid_map_for_stage(_world: &mut GameWorld, stage_kind: StageKind) {
    log::info!(
        "[AI GRID] Loading grid map for stage: {:?} from cache...",
        stage_kind
    );

    // 캐시에서 그리드 맵 확인
    if let Some(_grid_map) = get_cached_grid_map(stage_kind) {
        log::info!(
            "[AI GRID] Grid map for stage {:?} loaded successfully from cache!",
            stage_kind
        );
    } else {
        log::warn!(
            "[AI GRID] Grid map for stage {:?} not found in cache, generating at runtime...",
            stage_kind
        );

        // 캐시에 없으면 런타임에 생성 (폴백)
        preload_grid_map_for_stage(stage_kind);

        if get_cached_grid_map(stage_kind).is_some() {
            log::info!(
                "[AI GRID] Grid map for stage {:?} generated successfully at runtime",
                stage_kind
            );
        } else {
            log::error!(
                "[AI GRID] Failed to generate grid map for stage {:?} even at runtime",
                stage_kind
            );
        }
    }
}

// 다양한 목표 위치 생성 함수 (AI 개성과 넓은 범위)
fn generate_diverse_target(
    stage_kind: StageKind,
    personality_seed: u32,
    move_count: u32,
    current_pos: Vec3A,
) -> Vec3A {
    use rand::Rng;
    // AI마다 고유한 랜덤 시드 생성
    use rand::SeedableRng;
    let seed = personality_seed.wrapping_add(move_count);
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);

    // 안전한 랜덤 범위 함수
    let safe_random_range = |rng: &mut rand::rngs::StdRng, min: f32, max: f32| -> f32 {
        if min >= max {
            return (min + max) * 0.5; // 중간값 반환
        }
        rng.random_range(min..max)
    };

    // StageKind에 따라 스테이지 크기 결정 (훨씬 넓은 범위)
    let stage_attributes = get_stage_attributes(stage_kind);
    let bound_x = (stage_attributes.total_width * 0.9).abs().max(10.0); // 최소 10m 보장
    let bound_z = (stage_attributes.total_depth * 0.9).abs().max(10.0); // 최소 10m 보장

    println!(
        "[AI DIVERSE] Stage bounds: bound_x={:.2}, bound_z={:.2}",
        bound_x, bound_z
    );

    // AI 성격 분류 (personality_seed 기반)
    let personality_type = personality_seed % 2; // 2가지 성격 유형으로 축소

    // 이동 패턴 변화 (move_count 기반)
    let behavior_phase = move_count % 8; // 8단계 행동 변화

    let target = match personality_type {
        0 => {
            //  완전 랜덤 탐험
            Vec3A::new(
                safe_random_range(&mut rng, -bound_x, bound_x),
                0.0,
                safe_random_range(&mut rng, -bound_z, bound_z),
            )
        }
        _ => {
            // 다른 AI나 중요 지점을 향해 이동
            match behavior_phase {
                0..=2 => {
                    // 중앙 지역 추격
                    Vec3A::ZERO
                }
                3..=5 => {
                    // 현재 위치 기준 근처 탐색
                    let search_radius = 30.0; // 30m 반경 탐색
                    let angle = safe_random_range(&mut rng, 0.0, std::f32::consts::PI * 2.0);
                    let radius = safe_random_range(&mut rng, 10.0, search_radius);
                    current_pos + Vec3A::new(angle.cos() * radius, 0.0, angle.sin() * radius)
                }
                _ => {
                    // 원거리 목표 추격
                    Vec3A::new(
                        safe_random_range(&mut rng, -bound_x * 0.8, bound_x * 0.8),
                        0.0,
                        safe_random_range(&mut rng, -bound_z * 0.8, bound_z * 0.8),
                    )
                }
            }
        }
    };

    // 맵 경계 안전 확인 (경계를 벗어나면 조정)
    let safe_target = Vec3A::new(
        target.x.clamp(-bound_x * 0.95, bound_x * 0.95),
        0.0,
        target.z.clamp(-bound_z * 0.95, bound_z * 0.95),
    );

    println!(
        "[AI DIVERSE] Generated target for personality {}, phase {}: {:?}",
        personality_type, behavior_phase, safe_target
    );

    safe_target
}

// 기존 단순한 랜덤 목표 위치 생성 함수 (폴백용)
fn generate_random_target(stage_kind: StageKind) -> Vec3A {
    use rand::Rng;
    let mut rng = rand::rng();

    // StageKind에 따라 스테이지 크기 결정 (넓은 범위로 확장)
    let stage_attributes = get_stage_attributes(stage_kind);
    let bound_x = (stage_attributes.total_width * 0.8).abs().max(10.0); // 최소 10m 보장
    let bound_z = (stage_attributes.total_depth * 0.8).abs().max(10.0); // 최소 10m 보장

    let x = if bound_x > 0.0 {
        rng.random_range(-bound_x..bound_x)
    } else {
        0.0
    };
    let z = if bound_z > 0.0 {
        rng.random_range(-bound_z..bound_z)
    } else {
        0.0
    };

    Vec3A::new(x, 0.0, z)
}

/// 8방향을 나타내는 열거형 (0~7)
/// 0: 북(+Z), 1: 북동, 2: 동(+X), 3: 남동, 4: 남(-Z), 5: 남서, 6: 서(-X), 7: 북서
fn direction_to_index(direction: Vec3A) -> u8 {
    let angle = direction.z.atan2(direction.x);
    let normalized_angle = if angle < 0.0 {
        angle + 2.0 * std::f32::consts::PI
    } else {
        angle
    };
    let sector = (normalized_angle / (std::f32::consts::PI / 4.0)) as u8;
    (sector + 2) % 8 // 0을 북쪽(+Z)으로 맞추기 위한 오프셋
}

/// 방향 인덱스를 벡터로 변환
fn index_to_direction(index: u8) -> Vec3A {
    let angle = (index as f32) * std::f32::consts::PI / 4.0;
    Vec3A::new(angle.cos(), 0.0, angle.sin())
}

/// 8방향 사용 빈도를 계산
fn calculate_direction_usage(direction_history: &std::collections::VecDeque<u8>) -> [u32; 8] {
    let mut usage = [0u32; 8];
    for &dir in direction_history {
        if dir < 8 {
            usage[dir as usize] += 1;
        }
    }
    usage
}

/// 가장 적게 사용된 방향들을 찾기
fn find_least_used_directions(usage: &[u32; 8]) -> Vec<u8> {
    let min_usage = *usage.iter().min().unwrap_or(&0);
    usage
        .iter()
        .enumerate()
        .filter(|(_, count)| **count == min_usage)
        .map(|(index, _)| index as u8)
        .collect()
}

/// 맴돌기 패턴 감지 (연속된 방향 변화 패턴)
fn detect_circling_pattern(direction_history: &std::collections::VecDeque<u8>) -> f32 {
    if direction_history.len() < 8 {
        return 0.0;
    }

    let mut circle_penalty = 0.0;
    let recent_directions: Vec<u8> = direction_history.iter().rev().take(8).cloned().collect();

    // 시계방향/반시계방향 연속 패턴 감지
    let mut clockwise_sequence = 0;
    let mut counter_clockwise_sequence = 0;

    for i in 1..recent_directions.len() {
        let prev = recent_directions[i - 1];
        let curr = recent_directions[i];

        // 시계방향 체크 (방향 인덱스가 증가)
        if (curr + 8 - prev) % 8 == 1 {
            clockwise_sequence += 1;
        } else {
            clockwise_sequence = 0;
        }

        // 반시계방향 체크 (방향 인덱스가 감소)
        if (prev + 8 - curr) % 8 == 1 {
            counter_clockwise_sequence += 1;
        } else {
            counter_clockwise_sequence = 0;
        }

        // 연속된 회전이 3번 이상이면 큰 페널티
        if clockwise_sequence >= 3 || counter_clockwise_sequence >= 3 {
            circle_penalty += 50.0; // 매우 큰 페널티
        }
    }

    // 같은 방향 반복 패턴 감지
    for i in 1..recent_directions.len() {
        if recent_directions[i] == recent_directions[i - 1] {
            circle_penalty += 5.0; // 연속 같은 방향 페널티
        }
    }

    // 왕복 패턴 감지 (A-B-A-B 패턴)
    if recent_directions.len() >= 4 {
        for i in 0..recent_directions.len() - 3 {
            let pattern = &recent_directions[i..i + 4];
            if pattern[0] == pattern[2]
                && pattern[1] == pattern[3]
                && (pattern[0] + 4) % 8 == pattern[1]
            {
                // 정반대 방향 왕복
                circle_penalty += 30.0; // 왕복 패턴 큰 페널티
            }
        }
    }

    circle_penalty
}

/// 고급 스마트 이동 (집결 구역 내에서만 맴돌기 방지 + 8방향 골고른 사용)
fn calculate_smart_movement_advanced(
    current_pos: Vec3A,
    target_pos: Vec3A,
    _stage: &mod_network::components::StageAttributes,
    ai_player: &mut AiPlayer,
) -> mod_network::components::HeldInput {
    println!("[AI ADVANCED] Starting advanced smart movement calculation (capture zone only)");

    // 집결 구역 내에서만 복잡한 로직 실행
    // 임시로 하드코딩된 값 사용 (나중에 stage에서 가져오도록 수정 필요)
    let capture_center = Vec3A::ZERO;
    let capture_radius = 10.0;
    if !is_in_capture_zone_with_params(current_pos, capture_center, capture_radius) {
        println!("[AI ADVANCED] Outside capture zone, using direct movement");
        return calculate_direct_movement_to_target(current_pos, target_pos);
    }

    // 1. 맴돌기 패턴 감지 및 페널티 계산
    let circle_penalty = detect_circling_pattern(&ai_player.direction_history);
    ai_player.circle_penalty = (ai_player.circle_penalty * 0.9) + circle_penalty; // 페널티 감쇄

    println!(
        "[AI ADVANCED] Circle penalty: current={:.1}, accumulated={:.1}",
        circle_penalty, ai_player.circle_penalty
    );

    // 2. 8방향 사용 빈도 분석
    let direction_usage = calculate_direction_usage(&ai_player.direction_history);
    let least_used_directions = find_least_used_directions(&direction_usage);

    println!("[AI ADVANCED] Direction usage: {:?}", direction_usage);
    println!(
        "[AI ADVANCED] Least used directions: {:?}",
        least_used_directions
    );

    // 3. 제자리에 너무 오래 있거나 큰 맴돌기 페널티가 있으면 강제 탈출 (집결 구역 내에서만)
    if ai_player.stuck_counter > 20 || ai_player.circle_penalty > 100.0 {
        println!(
            "[AI ADVANCED] EMERGENCY: Breaking out of stuck/circle pattern! stuck={}, penalty={:.1}",
            ai_player.stuck_counter, ai_player.circle_penalty
        );

        return force_escape_movement_advanced(current_pos, &ai_player.visited_grids);
    }

    // 4. 목표 방향 계산
    let target_direction = (target_pos - current_pos).normalize_or_zero();
    let ideal_direction_index = direction_to_index(target_direction);

    // 5. 방향 선택 전략: 목표 방향 + 8방향 균등 사용 + 맴돌기 방지
    let chosen_direction = choose_optimal_direction(
        ideal_direction_index,
        &direction_usage,
        &least_used_directions,
        ai_player.circle_penalty,
        ai_player.last_direction,
    );

    // 6. 선택된 방향을 입력으로 변환
    let direction_vector = index_to_direction(chosen_direction);
    let input = convert_direction_to_input(direction_vector);

    // 7. 방향 기록 업데이트
    ai_player.direction_history.push_back(chosen_direction);
    if ai_player.direction_history.len() > 16 {
        ai_player.direction_history.pop_front();
    }
    ai_player.last_direction = Some(chosen_direction);

    println!(
        "[AI ADVANCED] Chosen direction: {} (vector: {:?}), input: {:?}",
        chosen_direction, direction_vector, input
    );

    input
}

/// 안전지대 페널티 계산 (팀별 리스폰 지역을 회피)
fn calculate_safe_zone_penalty(direction: u8) -> f32 {
    // 8방향을 각도로 변환
    let angle = (direction as f32) * std::f32::consts::PI / 4.0;
    let direction_vector = Vec3A::new(angle.cos(), 0.0, angle.sin());

    // 스테이지 정보 가져오기
    let stage = get_current_stage_attributes();

    let mut penalty = 0.0;

    // 블루팀 안전지대 체크
    for blue_pos in &stage.blue_team_positions {
        let blue_direction = blue_pos.normalize_or_zero();
        let dot_product = direction_vector.dot(blue_direction);

        // 방향이 블루팀 안전지대를 향하면 페널티 부여
        if dot_product > 0.7 {
            // 약 45도 이내
            penalty += 30.0; // 큰 페널티
            println!(
                "[AI SAFE ZONE] Direction {} leads to Blue safe zone, penalty: +30.0",
                direction
            );
        } else if dot_product > 0.3 {
            // 약 70도 이내
            penalty += 15.0; // 중간 페널티
            println!(
                "[AI SAFE ZONE] Direction {} near Blue safe zone, penalty: +15.0",
                direction
            );
        }
    }

    // 레드팀 안전지대 체크
    for red_pos in &stage.red_team_positions {
        let red_direction = red_pos.normalize_or_zero();
        let dot_product = direction_vector.dot(red_direction);

        // 방향이 레드팀 안전지대를 향하면 페널티 부여
        if dot_product > 0.7 {
            // 약 45도 이내
            penalty += 30.0; // 큰 페널티
            println!(
                "[AI SAFE ZONE] Direction {} leads to Red safe zone, penalty: +30.0",
                direction
            );
        } else if dot_product > 0.3 {
            // 약 70도 이내
            penalty += 15.0; // 중간 페널티
            println!(
                "[AI SAFE ZONE] Direction {} near Red safe zone, penalty: +15.0",
                direction
            );
        }
    }

    penalty
}

/// 최적 방향 선택 (목표 방향 + 균등 사용 + 맴돌기 방지 + 안전지대 회피)
fn choose_optimal_direction(
    ideal_direction: u8,
    usage: &[u32; 8],
    least_used: &[u8],
    circle_penalty: f32,
    last_direction: Option<u8>,
) -> u8 {
    // 각 방향에 대한 점수 계산
    let mut scores = [0.0f32; 8];

    for i in 0..8 {
        let direction = i as u8;

        // 1. 목표 방향과의 일치도 (높을수록 좋음)
        let angle_diff = ((direction as i16 - ideal_direction as i16)
            .abs()
            .min(8 - (direction as i16 - ideal_direction as i16).abs()))
            as f32;
        let target_score = 100.0 - (angle_diff * 12.5); // 0~100점

        // 2. 사용 빈도 역보정 (적게 사용할수록 좋음)
        let usage_score = 50.0 - (usage[i] as f32 * 3.0); // 많이 사용하면 페널티

        // 3. 맴돌기 방지: 이전 방향과 연속성 체크
        let mut continuity_penalty = 0.0;
        if let Some(last_dir) = last_direction {
            // 같은 방향 연속 사용 페널티
            if direction == last_dir {
                continuity_penalty += 10.0;
            }
            // 정반대 방향 (왕복) 큰 페널티
            if (direction + 4) % 8 == last_dir {
                continuity_penalty += 25.0;
            }
            // 연속 회전 패턴 페널티
            if (direction + 8 - last_dir) % 8 == 1 || (last_dir + 8 - direction) % 8 == 1 {
                continuity_penalty += circle_penalty * 0.5; // 맴돌기 페널티 반영
            }
        }

        // 4. 안전지대 회피 페널티 추가
        let safe_zone_penalty = calculate_safe_zone_penalty(direction);

        // 5. 최종 점수 계산
        scores[i] = target_score + usage_score - continuity_penalty - safe_zone_penalty;

        // 6. 가장 적게 사용된 방향에 보너스
        if least_used.contains(&direction) {
            scores[i] += 20.0;
        }
    }

    // 최고 점수 방향 선택
    let best_direction = scores
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(index, _)| index as u8)
        .unwrap_or(ideal_direction);

    println!(
        "[AI DIRECTION] Scores: {:?}, chosen: {}",
        scores, best_direction
    );

    best_direction
}

/// 방향 벡터를 입력으로 변환 (8방향 정밀 변환 + 랜덤 이동량)
fn convert_direction_to_input(direction: Vec3A) -> mod_network::components::HeldInput {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut input = mod_network::components::HeldInput::empty();

    // 랜덤 임계값 조절 (0.1 ~ 0.7 사이에서 랜덤)
    let threshold = rng.random_range(0.1..0.7);

    println!(
        "[AI CONVERT] Using random threshold: {:.2} for direction conversion",
        threshold
    );

    // X축 방향 (동서) - 랜덤 임계값 적용
    if direction.x > threshold {
        input |= mod_network::components::HeldInput::Right;
    } else if direction.x < -threshold {
        input |= mod_network::components::HeldInput::Left;
    }

    // Z축 방향 (남북) - 랜덤 임계값 적용
    if direction.z > threshold {
        input |= mod_network::components::HeldInput::Forward;
    } else if direction.z < -threshold {
        input |= mod_network::components::HeldInput::Backward;
    }

    // 입력이 없으면 전진으로 기본 설정
    if input.is_empty() {
        input = mod_network::components::HeldInput::Forward;
    }

    input
}

/// 적 감지 및 자동 발포 시스템
/// 상대 팀 플레이어를 감지하고 사정거리 내에 있으면 자동으로 발포
fn detect_and_attack_enemy(
    players: &HashMap<UserId, Player>,
    ai_user_id: &UserId,
    ai_position: Vec3A,
    ai_team: Team,
    move_input: &mut mod_network::components::HeldInput,
) -> bool {
    // AI 감지 범위 설정
    const DETECTION_RANGE: f32 = 30.0; // 30m 감지 범위
    const ATTACK_RANGE: f32 = 25.0; // 25m 공격 범위
    const FOV_ANGLE: f32 = 90.0; // 90도 시야각 (라디안으로 변환됨)

    let mut enemy_detected = false;
    let mut closest_enemy_distance = f32::MAX;
    let mut closest_enemy_position = Vec3A::ZERO;

    // 모든 플레이어를 검사하여 적 찾기
    for (user_id, player) in players {
        // 자기 자신은 제외
        if user_id == ai_user_id {
            continue;
        }

        // 같은 팀은 제외
        if player.team() == ai_team {
            continue;
        }

        // 죽은 플레이어는 제외
        if player.health_data.remaining == 0 {
            continue;
        }

        let enemy_position = player.translation;
        let distance = (enemy_position - ai_position).length();

        // 감지 범위 내에 있는지 확인
        if distance <= DETECTION_RANGE {
            // 시야각 내에 있는지 확인 (필요시 구현 가능)

            // 장애물 체크 (간단한 직선 거리 체크)
            let is_visible = check_line_of_sight(ai_position, enemy_position);

            if is_visible && distance < closest_enemy_distance {
                closest_enemy_distance = distance;
                closest_enemy_position = enemy_position;
                enemy_detected = true;

                println!(
                    "[AI COMBAT] Enemy detected at distance {:.1}m, position: {:?}",
                    distance, enemy_position
                );
            }
        }
    }

    // 적이 감지되고 공격 범위 내에 있으면 발포
    if enemy_detected && closest_enemy_distance <= ATTACK_RANGE {
        // 공격 입력 추가
        *move_input |= mod_network::components::HeldInput::Attack;

        println!(
            "[AI COMBAT] Opening fire on enemy at {:.1}m!",
            closest_enemy_distance
        );

        // 적 방향으로 조준 (이동하면서 동시에 공격)
        let direction_to_enemy = (closest_enemy_position - ai_position).normalize_or_zero();
        let aim_input = convert_direction_to_aim_input(direction_to_enemy);

        // 조준 입력을 이동 입력과 결합
        *move_input |= aim_input;

        return true;
    }

    enemy_detected
}

/// 시야선 체크 (간단한 장애물 감지)
fn check_line_of_sight(start: Vec3A, end: Vec3A) -> bool {
    // 간단한 구현: 그리드 맵을 이용한 시야선 체크
    if let Ok(grid_map_cache) = GLOBAL_GRID_MAP_CACHE.lock() {
        if let Some(grid_map) = grid_map_cache.get(&StageKind::City) {
            return check_line_of_sight_with_grid(start, end, grid_map);
        }
    }

    // 폴백: 거리만 체크 (장애물 무시)
    let distance = (end - start).length();
    distance <= 30.0 // 30m 이하면 보인다고 가정
}

/// 그리드 맵을 이용한 시야선 체크
fn check_line_of_sight_with_grid(start: Vec3A, end: Vec3A, grid_map: &GridMap2D) -> bool {
    let direction = (end - start).normalize_or_zero();
    let distance = (end - start).length();
    let step_size = grid_map.grid_size * 0.5; // 0.5 그리드씩 체크
    let num_steps = (distance / step_size).ceil() as i32;

    // 시작점부터 끝점까지 그리드 단위로 장애물 체크
    for i in 0..num_steps {
        let check_pos = start + direction * (i as f32 * step_size);

        if !grid_map.is_walkable(check_pos) {
            println!("[AI SIGHT] Line of sight blocked at {:?}", check_pos);
            return false; // 장애물 발견
        }
    }

    true // 시야선 깨끗함
}

/// 적 방향으로 조준하는 입력 생성
fn convert_direction_to_aim_input(direction: Vec3A) -> mod_network::components::HeldInput {
    let mut input = mod_network::components::HeldInput::empty();

    // 조준 정확도를 위한 더 민감한 임계값
    let aim_threshold = 0.3;

    // X축 방향 (좌우 조준)
    if direction.x > aim_threshold {
        input |= mod_network::components::HeldInput::Right;
    } else if direction.x < -aim_threshold {
        input |= mod_network::components::HeldInput::Left;
    }

    // Z축 방향 (전후 조준)
    if direction.z > aim_threshold {
        input |= mod_network::components::HeldInput::Forward;
    } else if direction.z < -aim_threshold {
        input |= mod_network::components::HeldInput::Backward;
    }

    input
}

// 안전한 범위 내 랜덤 생성 함수
fn safe_random_range(min: f32, max: f32) -> f32 {
    if min >= max {
        println!(
            "[WARN] Invalid range: min={}, max={} - using min value",
            min, max
        );
        return min;
    }
    rand::random_range(min..=max)
}

// 점령지역 내부에 있는지 확인
fn is_in_capture_zone_with_params(position: Vec3A, center: Vec3A, radius: f32) -> bool {
    let distance = (position - center).length();
    distance <= radius
}

// 점령지역 내에서 랜덤 위치 생성 (중앙에서 너무 멀어지지 않도록)
fn generate_random_position_in_capture_zone_with_params(center: Vec3A, radius: f32) -> Vec3A {
    use rand::Rng;
    let mut rng = rand::rng();

    // 중앙에서 적당한 거리 내에서만 이동 (반지름의 60% 이내)
    let max_radius = radius * 0.6;
    let angle = rng.random_range(0.0..2.0 * std::f32::consts::PI);
    let min_radius = (5.0_f32).min(max_radius * 0.3); // 최소 거리는 5m 또는 최대 반지름의 30%
    let random_radius = rng.random_range(min_radius..max_radius);

    let x = center.x + random_radius * angle.cos();
    let z = center.z + random_radius * angle.sin();

    let target = Vec3A::new(x, center.y, z);

    println!(
        "[AI CAPTURE] Generated random position in capture zone: {:?} (radius: {:.1}m)",
        target, random_radius
    );

    target
}
