// AI FSM, 상태 패턴, 컨텍스트, 상태 트레잇, 상태 구조체, 매니저 등 구현

use glam::Vec3A;

// AI FSM State/Context/Event
#[derive(Clone, Debug)]
pub struct AIPlayerContext {
    pub position: Vec3A,
    pub target: Vec3A,
    // 경로 캐싱 및 경로 계산 주기 관리
    pub path: Option<Vec<Vec3A>>,
    pub last_pathfind_time: Option<u32>,
    // 충돌 회피를 위한 추가 정보
    pub stuck_counter: u32,              // 연속 충돌 횟수
    pub alternative_targets: Vec<Vec3A>, // 우회 경로를 위한 중간 목표들
    // 충돌 학습을 위한 정보
    pub blocked_areas: Vec<(Vec3A, f32)>, // (중심점, 반경) - 충돌이 발생한 지역들
    pub last_collision_time: Option<u32>, // 마지막 충돌 시간
    pub exploration_mode: bool,           // 탐색 모드 여부
    // 이벤트 기반 충돌 처리
    pub collision_events: Vec<CollisionEvent>, // 충돌 이벤트 큐
    // 로봇청소기 스타일 학습 시스템
    pub pathfinding_memory: PathfindingMemory, // 경로탐색 메모리
    pub current_exploration_target: Option<Vec3A>, // 현재 탐색 목표
    // 위치 변화량 추적 (막힘 상태 감지)
    pub movement_history: Vec<(Vec3A, u32)>, // (위치, 타임스탬프) 기록
    pub last_significant_movement: Option<u32>, // 마지막으로 의미있는 이동이 있던 시간
    pub stuck_threshold_distance: f32,       // 막힘 판정을 위한 최소 이동 거리 (기본: 0.5m)
    pub stuck_threshold_time: u32,           // 막힘 판정을 위한 시간 (기본: 2000ms)
    // 방향 전환 시스템 (전진 전용 AI)
    pub current_direction: Vec3A,     // 현재 진행 방향 (정규화된 벡터)
    pub target_direction: Vec3A,      // 목표 진행 방향 (정규화된 벡터)
    pub rotation_speed: f32,          // 방향 전환 속도 (라디안/초)
    pub collision_rotation_bias: f32, // 충돌 시 방향 전환 편향 (-1.0 ~ 1.0, 음수=좌회전, 양수=우회전)
    pub last_direction_update: Option<u32>, // 마지막 방향 업데이트 시간
}

#[derive(Clone, Debug)]
pub struct CollisionEvent {
    pub position: Vec3A,
    pub normal: Vec3A,
    pub timestamp: u32,
}

// 로봇청소기 스타일 탐색을 위한 구조체들
#[derive(Clone, Debug)]
pub struct ExploredArea {
    pub center: Vec3A,
    pub radius: f32,
    pub is_passable: bool,
    pub last_updated: u32,
}

#[derive(Clone, Debug)]
pub struct PathfindingMemory {
    pub explored_areas: Vec<ExploredArea>, // 탐색한 지역들
    pub successful_paths: Vec<Vec<Vec3A>>, // 성공한 경로들
    pub wall_normals: Vec<(Vec3A, Vec3A)>, // (위치, 벽면 법선) - 벽의 방향 정보
}

impl PathfindingMemory {
    pub fn new() -> Self {
        Self {
            explored_areas: Vec::new(),
            successful_paths: Vec::new(),
            wall_normals: Vec::new(),
        }
    }

    /// 지역을 탐색된 것으로 표시
    pub fn mark_area_explored(&mut self, position: Vec3A, is_passable: bool, timestamp: u32) {
        // 기존 탐색 지역과 겹치는지 확인
        for area in &mut self.explored_areas {
            if (area.center - position).length() < area.radius {
                area.is_passable = is_passable;
                area.last_updated = timestamp;
                return;
            }
        }

        // 새로운 지역 추가
        self.explored_areas.push(ExploredArea {
            center: position,
            radius: 1.0,
            is_passable,
            last_updated: timestamp,
        });

        // 너무 많은 기록은 제거 (메모리 관리)
        if self.explored_areas.len() > 100 {
            self.explored_areas.remove(0);
        }
    }

    /// 성공한 경로를 기록
    pub fn record_successful_path(&mut self, path: Vec<Vec3A>) {
        self.successful_paths.push(path);

        // 너무 많은 경로 기록은 제거 (메모리 관리)
        if self.successful_paths.len() > 20 {
            self.successful_paths.remove(0);
        }
    }

    /// 벽면 정보 추가
    pub fn add_wall_info(&mut self, position: Vec3A, normal: Vec3A) {
        // 중복 방지: 기존 벽면과 가까우면 업데이트만
        for (wall_pos, wall_normal) in &mut self.wall_normals {
            if (*wall_pos - position).length() < 1.0 {
                *wall_normal = normal;
                return;
            }
        }

        // 새로운 벽면 정보 추가
        self.wall_normals.push((position, normal));

        // 벽면 정보 수 제한
        if self.wall_normals.len() > 50 {
            self.wall_normals.remove(0);
        }
    }
}

impl AIPlayerContext {
    /// 현재 위치를 이동 기록에 추가하고 막힘 상태를 검사
    pub fn update_movement_tracking(&mut self, current_position: Vec3A, current_time: u32) -> bool {
        // 새로운 위치 기록 추가
        self.movement_history.push((current_position, current_time));

        // 이동 기록이 너무 많으면 오래된 것 제거 (최근 10개만 유지)
        if self.movement_history.len() > 10 {
            self.movement_history.remove(0);
        }

        // 의미있는 이동이 있었는지 검사
        if self.movement_history.len() >= 2 {
            let recent_pos = self.movement_history[self.movement_history.len() - 1].0;
            let older_pos = self.movement_history[self.movement_history.len() - 2].0;
            let distance_moved = (recent_pos - older_pos).length();

            if distance_moved > self.stuck_threshold_distance {
                self.last_significant_movement = Some(current_time);
            }
        }

        // 막힘 상태 검사: 일정 시간 동안 의미있는 이동이 없었는지 확인
        let is_stuck = if let Some(last_movement_time) = self.last_significant_movement {
            current_time.saturating_sub(last_movement_time) > self.stuck_threshold_time
        } else {
            // 처음 시작하는 경우 충분한 시간이 지나면 막힘으로 간주
            self.movement_history.len() >= 5
        };

        is_stuck
    }

    /// 막힘 감지 상태를 초기화 (리스폰이나 경로 변경 시 호출)
    pub fn reset_movement_tracking(&mut self, current_time: u32) {
        self.movement_history.clear();
        self.last_significant_movement = Some(current_time);
    }

    /// 현재 위치에서 목표까지의 직선 거리 내에서의 평균 이동 속도 계산
    pub fn get_recent_movement_speed(&self, time_window_ms: u32) -> f32 {
        if self.movement_history.len() < 2 {
            return 0.0;
        }

        let current_time = self.movement_history.last().unwrap().1;
        let mut total_distance = 0.0;
        let mut time_span = 0u32;

        for i in (0..self.movement_history.len() - 1).rev() {
            let (pos1, time1) = self.movement_history[i];
            let (_pos2, _time2) = self.movement_history[i + 1];

            if current_time.saturating_sub(time1) > time_window_ms {
                break;
            }

            total_distance += (self.movement_history[i + 1].0 - pos1).length();
            time_span = current_time.saturating_sub(time1);
        }

        if time_span > 0 {
            total_distance / (time_span as f32 / 1000.0) // m/s
        } else {
            0.0
        }
    }

    /// 외부에서 발생한 충돌 이벤트를 AI 시스템에 전달
    pub fn handle_collision_event(
        &mut self,
        collision_pos: Vec3A,
        collision_normal: Vec3A,
        current_time: u32,
    ) {
        // 충돌 이벤트 큐에 추가
        let event = CollisionEvent {
            position: collision_pos,
            normal: collision_normal,
            timestamp: current_time,
        };
        self.collision_events.push(event);

        // 충돌 카운터 증가
        self.stuck_counter += 1;
        self.last_collision_time = Some(current_time);

        // 충돌 위치를 차단 구역으로 기록
        let blocked_radius = 1.0; // 1m 반경 차단
        self.blocked_areas.push((collision_pos, blocked_radius));

        // 차단 구역이 너무 많아지면 오래된 것 제거
        if self.blocked_areas.len() > 15 {
            self.blocked_areas.remove(0);
        }

        // 벽면 정보 기록
        self.pathfinding_memory
            .add_wall_info(collision_pos, collision_normal);

        log::info!(
            "[AI] Collision event handled at position: {:?}, normal: {:?}, counter: {}",
            collision_pos,
            collision_normal,
            self.stuck_counter
        );
    }

    /// 방향 전환 시스템 업데이트
    pub fn update_direction(&mut self, target_position: Vec3A, current_time: u32, delta_time: f32) {
        let to_target = (target_position - self.position).normalize_or_zero();

        // 목표 방향 설정 (XZ 평면에서만 계산)
        self.target_direction = Vec3A::new(to_target.x, 0.0, to_target.z).normalize_or_zero();

        // 유효한 목표 방향이 없으면 현재 방향 유지
        if self.target_direction.length() < 0.1 {
            return;
        }

        // 현재 방향과 목표 방향 사이의 각도 계산
        let current_2d =
            Vec3A::new(self.current_direction.x, 0.0, self.current_direction.z).normalize_or_zero();
        let target_2d = self.target_direction;

        // 각도 차이 계산 (외적을 이용한 회전 방향 결정)
        let cross_product = current_2d.x * target_2d.z - current_2d.z * target_2d.x;
        let dot_product = current_2d.x * target_2d.x + current_2d.z * target_2d.z;
        let angle_diff = cross_product.atan2(dot_product);

        // 방향 전환 속도 계산 (충돌 편향 고려)
        let base_rotation_rate = self.rotation_speed * delta_time;

        // 각도 차이가 클 때는 더 빠르게 회전 (적응적 회전 속도) - 성능 최적화
        let angle_diff_abs = angle_diff.abs();
        let speed_multiplier = if angle_diff_abs > std::f32::consts::PI / 2.0 {
            3.0 // 90도 이상 차이나면 3배 빠르게 (더 반응적)
        } else if angle_diff_abs > std::f32::consts::PI / 4.0 {
            2.0 // 45도 이상 차이나면 2배 빠르게
        } else {
            1.0 // 기본 속도
        };

        let rotation_rate = base_rotation_rate * speed_multiplier;
        let biased_angle_diff = angle_diff + self.collision_rotation_bias * 0.2; // 편향 강도 더 감소

        // 목표 방향과의 각도 차이가 작으면 즉시 목표 방향으로 설정 (더 반응적으로)
        if angle_diff_abs < std::f32::consts::PI / 12.0 {
            // 15도 이하면 즉시 맞춤 (더 관대하게)
            self.current_direction = self.target_direction;
        } else {
            // 점진적 방향 전환 - 더 빠르게 회전
            let rotation_amount = if biased_angle_diff > 0.0 {
                rotation_rate
            } else {
                -rotation_rate
            };
            let current_angle = self.current_direction.z.atan2(self.current_direction.x);
            let new_angle = current_angle + rotation_amount;

            self.current_direction = Vec3A::new(new_angle.cos(), 0.0, new_angle.sin()).normalize();
        }

        self.last_direction_update = Some(current_time);

        // 디버깅을 위한 로깅 (주기적으로) - 성능 최적화를 위해 빈도 감소
        if current_time % 5000 == 0 {
            // 5초마다로 변경
            log::debug!(
                "[AI Direction] Target: {:?}, Current: {:?}, Angle diff: {:.1}°",
                self.target_direction,
                self.current_direction,
                angle_diff_abs.to_degrees()
            );
        }
    }

    /// 충돌 이벤트 발생 시 방향 편향 설정
    pub fn apply_collision_bias(&mut self, collision_normal: Vec3A) {
        // 충돌 법선을 기반으로 회전 편향 결정
        let current_2d =
            Vec3A::new(self.current_direction.x, 0.0, self.current_direction.z).normalize_or_zero();
        let normal_2d = Vec3A::new(collision_normal.x, 0.0, collision_normal.z).normalize_or_zero();

        // 외적을 이용해 좌회전 또는 우회전 결정
        let cross_product = current_2d.x * normal_2d.z - current_2d.z * normal_2d.x;

        // 편향 강도 조정 (충돌 횟수에 따라 증가)
        let bias_strength = (self.stuck_counter as f32 * 0.2).min(1.0);

        if cross_product > 0.0 {
            self.collision_rotation_bias = bias_strength; // 우회전
        } else {
            self.collision_rotation_bias = -bias_strength; // 좌회전
        }

        // 편향은 점진적으로 감소
        if self.stuck_counter == 0 {
            self.collision_rotation_bias *= 0.9;
        }

        log::info!(
            "[AI] Collision bias applied: {:.2}, stuck_counter: {}",
            self.collision_rotation_bias,
            self.stuck_counter
        );
    }

    /// 충돌 편향 초기화 (성공적인 이동 시)
    pub fn reset_collision_bias(&mut self) {
        self.collision_rotation_bias *= 0.95; // 점진적 감소
        if self.collision_rotation_bias.abs() < 0.1 {
            self.collision_rotation_bias = 0.0;
        }
    }
}

#[derive(Clone, Debug)]
pub enum AIStateEnum {
    Idle,
    Move,
}

#[derive(Debug, Clone)]
pub struct AIPlayerFSM {
    pub ctx: AIPlayerContext,
    pub state: AIStateEnum,
}

impl AIPlayerFSM {
    pub fn new(position: Vec3A, target: Vec3A) -> Self {
        Self {
            ctx: AIPlayerContext {
                position,
                target,
                path: None,
                last_pathfind_time: None,
                stuck_counter: 0,
                alternative_targets: Vec::new(),
                blocked_areas: Vec::new(),
                last_collision_time: None,
                exploration_mode: false,
                collision_events: Vec::new(),
                pathfinding_memory: PathfindingMemory::new(),
                current_exploration_target: None,
                // 위치 변화량 추적 초기화
                movement_history: Vec::new(),
                last_significant_movement: None,
                stuck_threshold_distance: 0.5, // 0.5m 이하의 이동은 막힘으로 판정
                stuck_threshold_time: 2000,    // 2초간 의미있는 이동이 없으면 막힘으로 판정
                // 방향 전환 시스템 초기화 (전진 전용 AI) - 성능 최적화
                current_direction: Vec3A::new(0.0, 0.0, 1.0), // 초기 방향: 북쪽(Z+)
                target_direction: Vec3A::new(0.0, 0.0, 1.0),  // 초기 목표 방향: 북쪽(Z+)
                rotation_speed: 6.0,                          // 6 라디안/초로 증가 (더 빠른 회전)
                collision_rotation_bias: 0.0,                 // 초기 편향 없음
                last_direction_update: None,
            },
            state: AIStateEnum::Idle,
        }
    }

    /// AI FSM 업데이트: 일정 주기마다만 호출되도록 설계
    pub fn update(&mut self) {
        match self.state {
            AIStateEnum::Idle => {
                // Idle 상태: 타겟 위치로 이동 명령
                self.state = AIStateEnum::Move;
            }
            AIStateEnum::Move => {
                // Move 상태: 이동 중
                // 목표에 도달했는지 확인
                let distance_to_target = (self.ctx.position - self.ctx.target).length();
                if distance_to_target < 2.0 {
                    self.state = AIStateEnum::Idle;
                }
            }
        }
    }
}
