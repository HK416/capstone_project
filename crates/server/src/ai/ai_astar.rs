use glam::Vec3A;
use pathfinding::prelude::*;
use std::collections::HashMap;

use mod_physics::collision::StaticCollision;

/// 2D 그리드 지도 시스템 (실제 맵 정보 기반)
/// - 스테이지의 모든 오브젝트를 2D 그리드로 사전 구축
/// - 경로탐색 시 빠른 충돌 검사 제공
/// - 메모리 효율적인 비트맵 기반 저장
#[derive(Clone, Debug)]
pub struct GridMap2D {
    /// 그리드 크기 (m)
    pub grid_size: f32,
    /// 맵 범위 (min_x, min_z, max_x, max_z)
    pub bounds: (f32, f32, f32, f32),
    /// 그리드 차원 (width, height)
    pub dimensions: (usize, usize),
    /// 장애물 그리드 (true = 막힘, false = 통행가능)
    pub obstacle_grid: Vec<bool>,
    /// 그리드 중심점들의 월드 좌표 캐시
    /// 지형 기본 높이
    pub base_height: f32,
}

impl GridMap2D {
    /// 스테이지 정보로부터 2D 그리드 지도 생성
    pub fn from_stage(
        stage: &mod_network::components::StageAttributes,
        character_attributes: &mod_network::components::CharacterAttributes,
        grid_size: f32,
        map_size: f32, // 맵 전체 크기 (±map_size)
    ) -> Self {
        log::info!(
            "[GRID MAP] Building 2D grid map: size={:.1}m, grid={:.1}m",
            map_size,
            grid_size
        );

        let bounds = (-map_size, -map_size, map_size, map_size);
        let width = ((bounds.2 - bounds.0) / grid_size).ceil() as usize;
        let height = ((bounds.3 - bounds.1) / grid_size).ceil() as usize;

        log::info!(
            "[GRID MAP] Grid dimensions: {}x{} = {} cells",
            width,
            height,
            width * height
        );

        // 안전거리 계산 (AI 시스템과 동일한 검증 로직 사용)
        let character_radius = character_attributes.collider.radius;

        // **중요**: 캐릭터 반지름 값 검증 및 안전한 범위로 제한 (AI 시스템과 일치)
        let safe_character_radius = if character_radius.is_finite() && character_radius > 0.0 {
            character_radius.min(5.0) // 최대 5m로 제한 (비정상적으로 큰 값 방지)
        } else {
            log::warn!(
                "[GRID MAP] Invalid character_radius: {}, using default 0.5m",
                character_radius
            );
            0.5 // 기본값
        };

        // **핵심**: 순수 그리드 마진 시스템 - 격자 기반 안전거리
        // 캐릭터 반지름은 그리드 시스템에서 고려하지 않음 (이미 그리드 크기에 반영됨)
        let safety_margin = grid_size; // 1그리드 마진

        log::info!(
            "[GRID MAP] Grid-based safety margin: 2 grids = {:.2}m (character radius {:.2}m ignored in grid system)",
            safety_margin,
            safe_character_radius
        );

        // 그리드 초기화 (기본적으로 모든 셀이 통행 가능)
        let mut obstacle_grid = vec![false; width * height];
        let mut grid_centers = HashMap::new();

        // 각 그리드 셀에 대해 충돌 검사 수행 (중앙 원점 기준)
        let mut obstacle_count = 0;
        let mut detailed_collision_count = 0;

        for grid_y in 0..height {
            for grid_x in 0..width {
                // **수정**: 중앙 원점 기준 월드 좌표 계산
                // 그리드 (0,0) = 좌상단이지만, 월드 좌표는 중앙 기준
                let signed_x = (grid_x as i32) - (width as i32 / 2);
                let signed_z = (grid_y as i32) - (height as i32 / 2);

                let world_x = (signed_x as f32 + 0.5) * grid_size;
                let world_z = (signed_z as f32 + 0.5) * grid_size;
                let world_pos = Vec3A::new(world_x, 1.0, world_z);

                // 그리드 중심점 저장 (중앙 원점 기준)
                let grid_coord = (signed_x, signed_z);
                grid_centers.insert(grid_coord, world_pos);

                // 확장된 캐릭터 캡슐로 충돌 검사
                let mut test_capsule = character_attributes.collider.clone();
                test_capsule.center = world_pos.into();
                test_capsule.radius += safety_margin;

                if test_capsule.radius <= 0.0 || test_capsule.height <= 0.0 {
                    continue;
                }

                let test_aabb = mod_physics::object3d::BoundingBox::from(&test_capsule);
                let test_collider = mod_physics::collision::Collider::Capsule(test_capsule);

                // 스테이지의 모든 오브젝트와 충돌 검사
                let mut is_blocked = false;

                let collisions = stage.collider.search_aabb_collision(test_aabb);
                for collider in collisions {
                    let collision_result = match std::panic::catch_unwind(|| {
                        test_collider.check_collision(collider)
                    }) {
                        Ok(result) => result,
                        Err(_) => true, // 패닉 시 막힌 것으로 처리
                    };

                    if collision_result {
                        is_blocked = true;
                        // 상세 충돌 정보 로깅 (처음 몇 개만)
                        if detailed_collision_count < 10 {
                            let collider_type = match collider {
                                mod_physics::collision::Collider::Aabb(_) => "AABB",
                                mod_physics::collision::Collider::Obb(_) => "OBB",
                                mod_physics::collision::Collider::Capsule(_) => "Capsule",
                                mod_physics::collision::Collider::OrientedCapsule(_) => {
                                    "OrientedCapsule"
                                }
                                mod_physics::collision::Collider::Sphere(_) => "Sphere",
                            };
                            log::debug!(
                                "[GRID MAP] Obstacle detected at grid({},{}) world({:.1},{:.1}) - type: {}",
                                grid_x,
                                grid_y,
                                world_x,
                                world_z,
                                collider_type
                            );
                            detailed_collision_count += 1;
                        }
                        break;
                    }
                }

                let grid_index = grid_y * width + grid_x;
                obstacle_grid[grid_index] = is_blocked;

                if is_blocked {
                    obstacle_count += 1;
                }
            }
        }

        let passable_percentage =
            ((width * height - obstacle_count) as f32 / (width * height) as f32) * 100.0;
        log::info!(
            "[GRID MAP] Map analysis: {}/{} passable ({:.1}%), {} obstacles",
            width * height - obstacle_count,
            width * height,
            passable_percentage,
            obstacle_count
        );

        log::info!(
            "[GRID MAP] Collision detection: {} detailed collisions logged (safety margin: {:.2}m)",
            detailed_collision_count,
            safety_margin
        );

        // 콜라이더 트리 통계
        let total_colliders = stage.collider.count_colliders();
        log::info!(
            "[GRID MAP] Stage has {} total colliders for obstacle detection",
            total_colliders
        );

        Self {
            grid_size,
            bounds,
            dimensions: (width, height),
            obstacle_grid,
            base_height: 1.0,
        }
    }

    /// CSV 파일로부터 그리드 맵 로드
    pub fn from_csv(csv_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        log::info!("[GRID MAP] Loading grid map from CSV: {}", csv_path);

        // CSV 파일 읽기
        let content = std::fs::read_to_string(csv_path)?;
        let lines: Vec<&str> = content.lines().collect();

        if lines.is_empty() {
            return Err("Empty CSV file".into());
        }

        // 첫 번째 줄에서 메타데이터 파싱 (grid_size, bounds)
        let first_line = lines[0];
        let metadata: Vec<&str> = first_line.split(',').collect();

        if metadata.len() < 6 {
            return Err("Invalid CSV format: missing metadata".into());
        }

        let grid_size: f32 = metadata[0].parse()?;
        let min_x: f32 = metadata[1].parse()?;
        let min_z: f32 = metadata[2].parse()?;
        let max_x: f32 = metadata[3].parse()?;
        let max_z: f32 = metadata[4].parse()?;
        let base_height: f32 = metadata[5].parse()?;

        let bounds = (min_x, min_z, max_x, max_z);

        // 그리드 데이터 파싱 (두 번째 줄부터)
        let mut obstacle_grid = Vec::new();
        let mut width = 0;
        let height = lines.len() - 1; // 첫 번째 줄은 메타데이터

        for (row_idx, line) in lines.iter().skip(1).enumerate() {
            let cells: Vec<&str> = line.split(',').collect();

            if row_idx == 0 {
                width = cells.len();
            } else if cells.len() != width {
                return Err(format!("Inconsistent row width at line {}", row_idx + 2).into());
            }

            for cell in cells {
                let is_obstacle: bool = cell.trim() == "1";
                obstacle_grid.push(is_obstacle);
            }
        }

        let dimensions = (width, height);

        log::info!(
            "[GRID MAP] Loaded CSV grid map: {}x{} cells, grid_size={:.1}m, bounds=({:.1},{:.1},{:.1},{:.1})",
            width,
            height,
            grid_size,
            min_x,
            min_z,
            max_x,
            max_z
        );

        Ok(Self {
            grid_size,
            bounds,
            dimensions,
            obstacle_grid,
            base_height,
        })
    }

    /// 월드 좌표를 그리드 좌표로 변환 (중앙 원점 기준)
    pub fn world_to_grid(&self, world_pos: Vec3A) -> Option<(usize, usize)> {
        // 입력값 안전성 검사
        if !world_pos.is_finite() {
            log::warn!(
                "[GRID MAP] Invalid world_pos in world_to_grid: {:?}",
                world_pos
            );
            return None;
        }

        if self.grid_size <= 0.0 || !self.grid_size.is_finite() {
            log::error!("[GRID MAP] Invalid grid_size: {}", self.grid_size);
            return None;
        }

        // **수정**: 중앙 원점 기준 좌표 변환
        // 월드 좌표 (0,0) = 그리드 중앙
        let grid_x_signed = (world_pos.x / self.grid_size).floor() as i32;
        let grid_z_signed = (world_pos.z / self.grid_size).floor() as i32;

        // 그리드 배열 인덱스로 변환 (중앙을 기준으로 오프셋)
        let half_width = (self.dimensions.0 / 2) as i32;
        let half_height = (self.dimensions.1 / 2) as i32;

        let grid_x = grid_x_signed + half_width;
        let grid_z = grid_z_signed + half_height;

        if grid_x >= 0
            && grid_x < self.dimensions.0 as i32
            && grid_z >= 0
            && grid_z < self.dimensions.1 as i32
        {
            Some((grid_x as usize, grid_z as usize))
        } else {
            // 경계 밖 디버깅 정보
            static mut LOGGED_BOUNDS_ERROR: bool = false;
            unsafe {
                if !LOGGED_BOUNDS_ERROR {
                    log::debug!(
                        "[GRID MAP] Out of bounds: pos={:?}, signed_grid=({},{}), array_grid=({},{}), dims={:?}",
                        world_pos,
                        grid_x_signed,
                        grid_z_signed,
                        grid_x,
                        grid_z,
                        self.dimensions
                    );
                    LOGGED_BOUNDS_ERROR = true;
                }
            }
            None
        }
    }

    /// 그리드 좌표를 월드 좌표로 변환
    /// 특정 위치가 통행 가능한지 확인 (중앙 원점 기준 그리드 조회)
    /// 기본 충돌 검사만 수행 (안전 마진 제거로 순간이동 방지)
    pub fn is_walkable(&self, world_pos: Vec3A) -> bool {
        // 입력값 안전성 검사
        if !world_pos.is_finite() {
            log::warn!(
                "[GRID MAP] Invalid world_pos in is_walkable: {:?}",
                world_pos
            );
            return false;
        }

        // **수정**: 중앙 원점 기준 맵 범위 검사
        // 그리드가 커버하는 실제 월드 범위 계산
        let half_width = (self.dimensions.0 as f32 * self.grid_size) / 2.0;
        let half_height = (self.dimensions.1 as f32 * self.grid_size) / 2.0;

        if world_pos.x < -half_width
            || world_pos.x > half_width
            || world_pos.z < -half_height
            || world_pos.z > half_height
        {
            // 범위 밖이면 false (안전하게 차단)
            static mut LOGGED_BOUNDS_WARNING: bool = false;
            unsafe {
                if !LOGGED_BOUNDS_WARNING {
                    log::debug!(
                        "[GRID MAP] Position outside map bounds: {:?}, bounds=({:.1},{:.1}) to ({:.1},{:.1})",
                        world_pos,
                        -half_width,
                        -half_height,
                        half_width,
                        half_height
                    );
                    LOGGED_BOUNDS_WARNING = true;
                }
            }
            return false;
        }

        if let Some((grid_x, grid_z)) = self.world_to_grid(world_pos) {
            // 이중 범위 검사 (안전성 강화)
            if grid_x >= self.dimensions.0 || grid_z >= self.dimensions.1 {
                log::warn!(
                    "[GRID MAP] Grid coordinates out of range: ({},{}) >= ({},{})",
                    grid_x,
                    grid_z,
                    self.dimensions.0,
                    self.dimensions.1
                );
                return false;
            }

            // grid_z를 y축으로 사용 (Z가 월드의 Y축에 해당)
            let index = grid_z * self.dimensions.0 + grid_x;
            if index < self.obstacle_grid.len() {
                let is_walkable = !self.obstacle_grid[index];

                // 디버깅용 로그 (첫 번째 검사만)
                static mut LOGGED_GRID_CHECK: bool = false;
                unsafe {
                    if !LOGGED_GRID_CHECK {
                        log::debug!(
                            "[GRID MAP] Basic walkability check: pos {:?} -> grid({},{}) index={} -> walkable={}",
                            world_pos,
                            grid_x,
                            grid_z,
                            index,
                            is_walkable
                        );
                        LOGGED_GRID_CHECK = true;
                    }
                }
                is_walkable
            } else {
                log::warn!(
                    "[GRID MAP] Index {} out of bounds for grid size {}",
                    index,
                    self.obstacle_grid.len()
                );
                false
            }
        } else {
            // 맵 경계 밖은 통행 불가
            static mut LOGGED_OUT_OF_BOUNDS: bool = false;
            unsafe {
                if !LOGGED_OUT_OF_BOUNDS {
                    log::debug!("[GRID MAP] Position {:?} out of grid bounds", world_pos);
                    LOGGED_OUT_OF_BOUNDS = true;
                }
            }
            false
        }
    }

    /// 안전한 위치인지 확인 (벽과의 거리 고려)
    /// 이동 시에만 사용하는 별도 함수
    pub fn is_safe_walkable(&self, world_pos: Vec3A) -> bool {
        if let Some((grid_x, grid_z)) = self.world_to_grid(world_pos) {
            let index = grid_z * self.dimensions.0 + grid_x;
            if index < self.obstacle_grid.len() {
                let is_walkable = !self.obstacle_grid[index];

                // 기본적으로 통행 가능한 경우에만 안전 검사
                if is_walkable {
                    // 인접한 4방향만 검사 (대각선 제외로 과도한 제한 방지)
                    let directions = [(0, 1), (0, -1), (1, 0), (-1, 0)];
                    let mut adjacent_obstacles = 0;

                    for (dx, dz) in directions.iter() {
                        let check_x = grid_x as i32 + dx;
                        let check_z = grid_z as i32 + dz;

                        if check_x >= 0
                            && check_x < self.dimensions.0 as i32
                            && check_z >= 0
                            && check_z < self.dimensions.1 as i32
                        {
                            let check_index =
                                (check_z as usize) * self.dimensions.0 + (check_x as usize);
                            if check_index < self.obstacle_grid.len()
                                && self.obstacle_grid[check_index]
                            {
                                adjacent_obstacles += 1;
                            }
                        }
                    }

                    // 3면 이상이 막혀있으면 위험 (코너나 좁은 통로)
                    if adjacent_obstacles >= 3 {
                        static mut LOGGED_CORNER_BLOCK: bool = false;
                        unsafe {
                            if !LOGGED_CORNER_BLOCK {
                                log::debug!(
                                    "[GRID SAFETY] Blocked corner/narrow passage at ({},{}) - {} adjacent obstacles",
                                    grid_x,
                                    grid_z,
                                    adjacent_obstacles
                                );
                                LOGGED_CORNER_BLOCK = true;
                            }
                        }
                        return false;
                    }
                }

                is_walkable
            } else {
                false
            }
        } else {
            false
        }
    }

    /// 그리드 맵 통계 출력 (디버깅용)
    pub fn print_stats(&self) {
        let total_cells = self.obstacle_grid.len();
        let obstacle_cells = self
            .obstacle_grid
            .iter()
            .filter(|&&blocked| blocked)
            .count();
        let passable_cells = total_cells - obstacle_cells;

        log::info!(
            "[GRID MAP STATS] Total: {}, Passable: {}, Obstacles: {}, Passable: {:.1}%",
            total_cells,
            passable_cells,
            obstacle_cells,
            (passable_cells as f32 / total_cells as f32) * 100.0
        );
    }

    // TXT 파일 출력은 제거됨 - CSV 파일만 사용

    /// Bounds 기반 좌표 변환을 사용한 정확한 그리드 변환
    pub fn world_to_grid_with_bounds(&self, world_pos: Vec3A) -> Option<(usize, usize)> {
        let grid_pos = GridPos::from_world_pos_with_bounds(world_pos, self.grid_size, self.bounds);

        // bounds 기반 변환 결과를 배열 인덱스로 변환
        let array_x = grid_pos.x;
        let array_z = grid_pos.z;

        if array_x >= 0
            && array_x < self.dimensions.0 as i32
            && array_z >= 0
            && array_z < self.dimensions.1 as i32
        {
            Some((array_x as usize, array_z as usize))
        } else {
            None
        }
    }

    /// Bounds 기반 역변환을 사용한 정확한 월드 좌표 계산
    pub fn grid_to_world_with_bounds(&self, grid_x: usize, grid_z: usize) -> Vec3A {
        let grid_pos = GridPos::new(grid_x as i32, grid_z as i32);
        grid_pos.to_world_pos_with_bounds(self.grid_size, self.base_height, self.bounds)
    }
}

/// 2D 격자 위치 (pathfinding 크레이트 호환) - GridMap2D와 완전 호환
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GridPos {
    pub x: i32,
    pub z: i32,
}

impl GridPos {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// GridMap2D의 bounds와 호환되는 좌표 변환
    pub fn from_world_pos_with_bounds(
        world_pos: Vec3A,
        grid_size: f32,
        bounds: (f32, f32, f32, f32),
    ) -> Self {
        let safe_grid_size = if grid_size > 0.0 && grid_size.is_finite() {
            grid_size
        } else {
            log::error!(
                "[GRID POS] Invalid grid_size: {}, using fallback 1.0",
                grid_size
            );
            1.0
        };

        let safe_x = if world_pos.x.is_finite() {
            world_pos.x
        } else {
            0.0
        };
        let safe_z = if world_pos.z.is_finite() {
            world_pos.z
        } else {
            0.0
        };

        // GridMap2D와 동일한 변환 로직 사용
        let grid_x = ((safe_x - bounds.0) / safe_grid_size).floor() as i32;
        let grid_z = ((safe_z - bounds.1) / safe_grid_size).floor() as i32;

        Self {
            x: grid_x,
            z: grid_z,
        }
    }

    /// GridMap2D의 bounds와 호환되는 월드 좌표 변환
    pub fn to_world_pos_with_bounds(
        &self,
        grid_size: f32,
        base_height: f32,
        bounds: (f32, f32, f32, f32),
    ) -> Vec3A {
        let safe_grid_size = if grid_size > 0.0 && grid_size.is_finite() {
            grid_size
        } else {
            1.0
        };

        let safe_height = if base_height.is_finite() {
            base_height
        } else {
            1.0
        };

        // GridMap2D와 동일한 변환 로직: 그리드 중심점 계산
        Vec3A::new(
            bounds.0 + (self.x as f32 + 0.5) * safe_grid_size,
            safe_height,
            bounds.1 + (self.z as f32 + 0.5) * safe_grid_size,
        )
    }

    pub fn from_world_pos(world_pos: Vec3A, grid_size: f32) -> Self {
        // grid_size 안전성 검사
        let safe_grid_size = if grid_size > 0.0 && grid_size.is_finite() {
            grid_size
        } else {
            log::error!(
                "[GRID POS] Invalid grid_size: {}, using fallback 1.0",
                grid_size
            );
            1.0
        };

        // world_pos 안전성 검사
        let safe_x = if world_pos.x.is_finite() {
            world_pos.x
        } else {
            0.0
        };
        let safe_z = if world_pos.z.is_finite() {
            world_pos.z
        } else {
            0.0
        };

        // **수정**: 원점 기준 좌표계로 복원 (중앙(0,0) = GridPos(0,0))
        // A* 알고리즘이 올바른 방향을 계산할 수 있도록 함
        Self {
            x: (safe_x / safe_grid_size).round() as i32,
            z: (safe_z / safe_grid_size).round() as i32,
        }
    }

    pub fn to_world_pos(&self, grid_size: f32, base_height: f32) -> Vec3A {
        // grid_size와 base_height 안전성 검사
        let safe_grid_size = if grid_size > 0.0 && grid_size.is_finite() {
            grid_size
        } else {
            log::error!(
                "[GRID POS] Invalid grid_size in to_world_pos: {}, using fallback 1.0",
                grid_size
            );
            1.0
        };

        let safe_height = if base_height.is_finite() {
            base_height
        } else {
            1.0
        };

        // **수정**: 원점 기준 좌표계로 복원 (중앙(0,0) = GridPos(0,0))
        // A* 알고리즘이 올바른 방향을 계산할 수 있도록 함
        Vec3A::new(
            self.x as f32 * safe_grid_size,
            safe_height,
            self.z as f32 * safe_grid_size,
        )
    }

    /// 8방향 이웃 계산 (대각선 포함) - 우회 경로 최적화
    pub fn neighbors(&self) -> Vec<(GridPos, u32)> {
        let directions = [
            // 직선 이동과 대각선 이동 비용 동일화 (비용: 10)
            (-1, 0, 10),
            (1, 0, 10),
            (0, -1, 10),
            (0, 1, 10),
            // 대각선 이동도 동일한 비용 (비용: 10)
            (-1, -1, 10),
            (-1, 1, 10),
            (1, -1, 10),
            (1, 1, 10),
        ];

        directions
            .iter()
            .map(|(dx, dz, cost)| (GridPos::new(self.x + dx, self.z + dz), *cost))
            .collect()
    }

    /// 장애물 회피를 위한 추가 이웃 탐색 (더 넓은 범위)
    pub fn extended_neighbors(&self) -> Vec<(GridPos, u32)> {
        let mut neighbors = self.neighbors();

        // 2칸 점프 이동 추가 (큰 장애물 우회용)
        let jump_directions = [
            // 2칸 직선 점프 (비용: 20)
            (-2, 0, 20),
            (2, 0, 20),
            (0, -2, 20),
            (0, 2, 20),
            // 나이트 이동 패턴 (비용: 25)
            (-2, -1, 25),
            (-2, 1, 25),
            (2, -1, 25),
            (2, 1, 25),
            (-1, -2, 25),
            (1, -2, 25),
            (-1, 2, 25),
            (1, 2, 25),
        ];

        for (dx, dz, cost) in jump_directions.iter() {
            neighbors.push((GridPos::new(self.x + dx, self.z + dz), *cost));
        }

        neighbors
    }

    /// 맨하탄 거리 휴리스틱 (균등 비용) - 더 활발한 경로 탐색을 위해 조정
    pub fn distance_heuristic(&self, other: &GridPos) -> u32 {
        let dx = (self.x - other.x).abs() as u32;
        let dz = (self.z - other.z).abs() as u32;

        // 직선과 대각선 비용이 동일하므로 최대값 사용 (체비셰프 거리)
        let base_distance = std::cmp::max(dx, dz) * 10;

        // 휴리스틱 가중치를 더 낮춰서 훨씬 더 유연한 경로 탐색 허용
        // 0.5 = 매우 느슨한 탐색으로 다양한 우회 경로 적극 고려
        let heuristic_weight = 0.5; // 50% 낮춘 중요도로 활발한 우회 경로 허용

        (base_distance as f32 * heuristic_weight) as u32
    }

    /// 장애물 회피를 위한 매우 유연한 휴리스틱 (최대한 자유로운 이동)
    pub fn flexible_heuristic(&self, other: &GridPos) -> u32 {
        let dx = (self.x - other.x).abs() as u32;
        let dz = (self.z - other.z).abs() as u32;

        // 체비셰프 거리 (직선과 대각선 비용 동일)
        let base_distance = std::cmp::max(dx, dz) * 10;

        // 매우 유연한 탐색을 위해 휴리스틱 가중치를 대폭 낮춤
        let heuristic_weight = 0.3; // 70% 낮춘 중요도로 극도로 자유로운 이동 허용

        (base_distance as f32 * heuristic_weight) as u32
    }

    /// 중앙 지향 휴리스틱 (중앙으로 갈수록 높은 점수)
    /// 맵 중앙(0,0)에 가까워질수록 낮은 비용을 반환하여 중앙 집결을 유도
    pub fn center_bias_heuristic(&self, other: &GridPos, center_attraction: f32) -> u32 {
        // 기본 목표까지의 거리
        let dx = (self.x - other.x).abs() as u32;
        let dz = (self.z - other.z).abs() as u32;
        let base_distance = std::cmp::max(dx, dz) * 10;

        // 현재 위치에서 중앙(0,0)까지의 거리
        let center_distance = ((self.x * self.x + self.z * self.z) as f32).sqrt();

        // 중앙 매력도: 중앙에서 멀수록 페널티, 가까울수록 보너스
        let center_penalty = center_distance * center_attraction;

        // 목표까지의 거리에 중앙 매력도를 적용
        let adjusted_distance = base_distance as f32 + center_penalty;

        // 최소값 보장 (음수 방지)
        adjusted_distance.max(1.0) as u32
    }

    /// 적응형 중앙 지향 휴리스틱 (거리에 따라 중앙 매력도 조절)
    pub fn adaptive_center_heuristic(&self, other: &GridPos) -> u32 {
        let dx = (self.x - other.x).abs() as u32;
        let dz = (self.z - other.z).abs() as u32;
        let distance_to_goal = std::cmp::max(dx, dz);

        // 목표와의 거리에 따라 중앙 매력도 조절
        let center_attraction = if distance_to_goal > 20 {
            // 목표가 멀면 중앙 매력도 높임 (중앙 집결 강화)
            15.0
        } else if distance_to_goal > 10 {
            // 중간 거리에서는 적당한 중앙 매력도
            8.0
        } else {
            // 목표가 가까우면 중앙 매력도 낮춤 (목표 우선)
            3.0
        };

        self.center_bias_heuristic(other, center_attraction)
    }
}

/// 그리드 맵 기반 고속 2D A* 경로탐색 (장애물 회피 최적화)
/// - 사전 구축된 그리드 맵을 사용하여 초고속 경로탐색
/// - 실시간 충돌 검사 없이 그리드 조회만으로 처리
/// - 유연한 휴리스틱으로 우회 경로 허용
/// - 대규모 AI 시뮬레이션에 최적화
pub fn grid_based_astar_pathfind(
    start: Vec3A,
    goal: Vec3A,
    grid_map: &GridMap2D,
) -> Option<Vec<Vec3A>> {
    // 거리에 따라 계층적 경로탐색 또는 일반 경로탐색 선택
    let distance = (goal - start).length();

    if distance > 50.0 {
        // 50m 이상의 장거리는 계층적 경로탐색 사용
        log::debug!(
            "[GRID A*] Using hierarchical pathfinding for long distance: {:.1}m",
            distance
        );

        let is_walkable = |pos: Vec3A| grid_map.is_walkable(pos);
        hierarchical_pathfind(start, goal, 8.0, 2.0, is_walkable)
    } else {
        // 단거리는 기존 방식 사용
        grid_based_astar_pathfind_standard(start, goal, grid_map)
    }
}

/// 표준 그리드 기반 A* 경로탐색 (기존 구현)
pub fn grid_based_astar_pathfind_standard(
    start: Vec3A,
    goal: Vec3A,
    grid_map: &GridMap2D,
) -> Option<Vec<Vec3A>> {
    log::debug!(
        "[GRID A*] Flexible pathfinding from {:?} to {:?}",
        start,
        goal
    );

    // **수정**: 간단한 중앙 원점 기준 좌표 변환 사용
    let start_grid = GridPos::from_world_pos(start, grid_map.grid_size);
    let goal_grid = GridPos::from_world_pos(goal, grid_map.grid_size);

    log::debug!(
        "[GRID A*] Grid coordinates: start_grid={:?}, goal_grid={:?}",
        start_grid,
        goal_grid
    );
    log::debug!(
        "[GRID A*] World coordinates: start={:?}, goal={:?}",
        start,
        goal
    );

    // 좌표 변환 검증
    let start_converted_back = start_grid.to_world_pos(grid_map.grid_size, grid_map.base_height);
    let goal_converted_back = goal_grid.to_world_pos(grid_map.grid_size, grid_map.base_height);
    log::debug!(
        "[GRID A*] Coordinate verification: start {} -> grid {:?} -> {}",
        start,
        start_grid,
        start_converted_back
    );
    log::debug!(
        "[GRID A*] Coordinate verification: goal {} -> grid {:?} -> {}",
        goal,
        goal_grid,
        goal_converted_back
    );

    // 기본 방향성 검증: 목표가 실제로 올바른 방향에 있는지 확인
    let direct_direction = (goal - start).normalize_or_zero();
    let grid_direction = (goal_converted_back - start_converted_back).normalize_or_zero();
    let direction_similarity = direct_direction.dot(grid_direction);

    log::debug!(
        "[GRID A*] Direction verification: direct={:?}, grid={:?}, similarity={:.3}",
        direct_direction,
        grid_direction,
        direction_similarity
    );

    if direction_similarity < 0.7 {
        log::warn!(
            "[GRID A*] WARNING: Grid direction differs significantly from direct direction!"
        );
        log::warn!("[GRID A*] This may indicate coordinate system issues!");
    }

    // 그리드 범위 검증 (중앙 원점 기준)
    let half_width = (grid_map.dimensions.0 / 2) as i32;
    let half_height = (grid_map.dimensions.1 / 2) as i32;

    if start_grid.x < -half_width
        || start_grid.x >= half_width
        || start_grid.z < -half_height
        || start_grid.z >= half_height
    {
        log::warn!(
            "[GRID A*] Start position out of grid bounds: {:?}, grid range: ({},{}) to ({},{})",
            start_grid,
            -half_width,
            -half_height,
            half_width - 1,
            half_height - 1
        );
        return None;
    }

    if goal_grid.x < -half_width
        || goal_grid.x >= half_width
        || goal_grid.z < -half_height
        || goal_grid.z >= half_height
    {
        log::warn!(
            "[GRID A*] Goal position out of grid bounds: {:?}, grid range: ({},{}) to ({},{})",
            goal_grid,
            -half_width,
            -half_height,
            half_width - 1,
            half_height - 1
        );
        return None;
    }

    // pathfinding 크레이트의 A* 사용 (확장된 이웃 탐색으로 장애물 회피 강화)
    let result = astar(
        &start_grid,
        |pos| {
            // 확장된 이웃 노드들 생성 (장애물 회피용 점프 이동 포함)
            pos.extended_neighbors()
                .into_iter()
                .filter_map(|(neighbor_pos, base_cost)| {
                    // Bounds 기반 정확한 좌표 변환 사용
                    let world_pos = neighbor_pos.to_world_pos_with_bounds(
                        grid_map.grid_size,
                        grid_map.base_height,
                        grid_map.bounds,
                    );

                    // 중앙 원점 기준 그리드 범위 검증
                    if neighbor_pos.x < -half_width
                        || neighbor_pos.x >= half_width
                        || neighbor_pos.z < -half_height
                        || neighbor_pos.z >= half_height
                    {
                        return None;
                    }

                    // 안전한 위치인지 확인 (is_safe_walkable 함수 활용)
                    if grid_map.is_safe_walkable(world_pos) {
                        // 중앙 지향 비용 계산에 center_bias_heuristic 활용
                        let center_attraction = 2.0; // 적당한 중앙 매력도
                        let center_cost = neighbor_pos
                            .center_bias_heuristic(&GridPos::new(0, 0), center_attraction);

                        // 기본 비용과 중앙 지향 비용을 조합
                        let adjusted_cost =
                            ((base_cost as f32 * 0.9) + (center_cost as f32 * 0.1)).max(1.0) as u32;

                        Some((neighbor_pos, adjusted_cost))
                    } else {
                        // 디버깅을 위한 로그 (처음 몇 개만)
                        static mut DEBUG_COUNT: u32 = 0;
                        unsafe {
                            if DEBUG_COUNT < 5 {
                                log::debug!(
                                    "[GRID A*] Blocked neighbor: grid={:?}, world={:?}",
                                    neighbor_pos,
                                    world_pos
                                );
                                DEBUG_COUNT += 1;
                            }
                        }
                        None
                    }
                })
                .collect::<Vec<_>>()
        },
        |pos| {
            // 중앙 지향 적응형 휴리스틱 사용
            // 목표까지의 거리와 중앙 매력도를 모두 고려하여 최적 경로 탐색
            pos.adaptive_center_heuristic(&goal_grid)
        },
        |pos| *pos == goal_grid,
    );

    match result {
        Some((path, total_cost)) => {
            log::info!(
                "[GRID A*] SUCCESS! Flexible path with {} nodes, cost: {}",
                path.len(),
                total_cost
            );

            let world_path: Vec<Vec3A> = path
                .into_iter()
                .map(|grid_pos| grid_pos.to_world_pos(grid_map.grid_size, grid_map.base_height))
                .collect();

            // 첫 번째 이동 방향 검증
            if world_path.len() >= 2 {
                let first_move = world_path[1] - world_path[0];
                let toward_goal = (goal - start).normalize_or_zero();
                let first_move_normalized = first_move.normalize_or_zero();
                let direction_dot = first_move_normalized.dot(toward_goal);

                log::debug!("[GRID A*] First move analysis:");
                log::debug!("  From: {:?} -> To: {:?}", world_path[0], world_path[1]);
                log::debug!("  Move vector: {:?}", first_move);
                log::debug!("  Toward goal: {:?}", toward_goal);
                log::debug!(
                    "  Direction alignment: {:.3} (1.0=perfect, -1.0=opposite)",
                    direction_dot
                );

                if direction_dot < -0.1 {
                    log::warn!(
                        "[GRID A*] WARNING: First move goes AWAY from goal! (alignment: {:.3})",
                        direction_dot
                    );
                    log::warn!("[GRID A*] This indicates a potential pathfinding error!");

                    // 디버그를 위해 처음 5개 웨이포인트 출력
                    let preview_count = std::cmp::min(5, world_path.len());
                    for i in 0..preview_count {
                        log::warn!("[GRID A*]   Waypoint[{}]: {:?}", i, world_path[i]);
                    }
                } else {
                    log::debug!(
                        "[GRID A*] First move direction looks correct (alignment: {:.3})",
                        direction_dot
                    );
                }
            }

            // 경로 품질 분석
            if world_path.len() >= 2 {
                let mut path_length = 0.0;
                for i in 1..world_path.len() {
                    path_length += (world_path[i] - world_path[i - 1]).length();
                }
                let direct_distance = (goal - start).length();
                let detour_ratio = path_length / direct_distance.max(0.1);

                log::info!(
                    "[GRID A*] Path analysis: length={:.1}m, direct={:.1}m, detour_ratio={:.2}x",
                    path_length,
                    direct_distance,
                    detour_ratio
                );

                if detour_ratio > 1.5 {
                    log::info!(
                        "[GRID A*] Significant detour detected - successfully avoiding obstacles"
                    );
                }
            }

            Some(world_path)
        }
        None => {
            log::warn!("[GRID A*] FAILED to find flexible path - trying fallback");

            // 폴백: 더욱 유연한 설정으로 재시도 (확장된 이웃 탐색 사용)
            let fallback_result = astar(
                &start_grid,
                |pos| {
                    pos.extended_neighbors()
                        .into_iter()
                        .filter_map(|(neighbor_pos, base_cost)| {
                            // Bounds 기반 정확한 좌표 변환 사용
                            let world_pos = neighbor_pos.to_world_pos_with_bounds(
                                grid_map.grid_size,
                                grid_map.base_height,
                                grid_map.bounds,
                            );

                            // 중앙 원점 기준 그리드 범위 검증
                            if neighbor_pos.x < -half_width
                                || neighbor_pos.x >= half_width
                                || neighbor_pos.z < -half_height
                                || neighbor_pos.z >= half_height
                            {
                                return None;
                            }

                            if grid_map.is_safe_walkable(world_pos) {
                                // 폴백에서는 중앙 지향을 더 강화
                                let center_attraction = 4.0; // 더 강한 중앙 매력도
                                let center_cost = neighbor_pos
                                    .center_bias_heuristic(&GridPos::new(0, 0), center_attraction);
                                let adjusted_cost = ((base_cost as f32 * 0.5)
                                    + (center_cost as f32 * 0.2))
                                    .max(1.0)
                                    as u32;

                                Some((neighbor_pos, adjusted_cost))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                },
                |pos| {
                    // 폴백에서는 중앙 지향 휴리스틱과 유연한 휴리스틱을 혼합 사용
                    let center_heuristic = pos.adaptive_center_heuristic(&goal_grid);
                    let flexible_heuristic = pos.flexible_heuristic(&goal_grid);

                    // 두 휴리스틱의 평균으로 중앙 지향과 유연성을 모두 확보
                    (center_heuristic + flexible_heuristic) / 2
                },
                |pos| *pos == goal_grid,
            );

            match fallback_result {
                Some((path, total_cost)) => {
                    log::info!(
                        "[GRID A*] FALLBACK SUCCESS! Ultra-flexible path with {} nodes, cost: {}",
                        path.len(),
                        total_cost
                    );

                    let world_path: Vec<Vec3A> = path
                        .into_iter()
                        .map(|grid_pos| {
                            grid_pos.to_world_pos(grid_map.grid_size, grid_map.base_height)
                        })
                        .collect();

                    Some(world_path)
                }
                None => {
                    log::error!(
                        "[GRID A*] COMPLETE FAILURE - no path found even with ultra-flexible settings"
                    );
                    None
                }
            }
        }
    }
}

/// pathfinding 크레이트를 사용한 2D 평면 A* 경로탐색 (실시간 충돌 검사)
/// - 오브젝트 유무만 체크하는 단순한 2D 평면 탐색
/// - 지형 높이 무시, 장애물 충돌만 확인  
/// - 실시간 충돌 검사를 통한 정확한 경로 탐색
pub fn advanced_astar_pathfind<F>(
    start: Vec3A,
    goal: Vec3A,
    grid_size: f32,
    mut is_walkable: F,
) -> Option<Vec<Vec3A>>
where
    F: FnMut(Vec3A) -> bool,
{
    log::debug!(
        "[2D A*] Starting 2D pathfinding from {:?} to {:?} (grid: {:.1}m)",
        start,
        goal,
        grid_size
    );

    let start_grid = GridPos::from_world_pos(start, grid_size);
    let goal_grid = GridPos::from_world_pos(goal, grid_size);

    log::debug!(
        "[2D A*] Grid coordinates: start={:?}, goal={:?}",
        start_grid,
        goal_grid
    );

    // 기본 높이 설정 (지형 높이 무시)
    let base_height = start.y;

    // pathfinding 크레이트의 A* 사용
    let result = astar(
        &start_grid,
        |pos| {
            // 확장된 이웃 노드들과 비용 계산 (장애물 회피 강화)
            pos.extended_neighbors()
                .into_iter()
                .filter_map(|(neighbor_pos, cost)| {
                    // 2D 평면 위치로 변환 (고정 높이 사용)
                    let world_pos = neighbor_pos.to_world_pos(grid_size, base_height);

                    // 오브젝트 충돌만 체크 (지형 높이 무시)
                    if is_walkable(world_pos) {
                        Some((neighbor_pos, cost))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        },
        |pos| {
            // 거리에 따라 적응적 휴리스틱 사용 (중앙 지향 포함)
            let dx = (pos.x - goal_grid.x).abs();
            let dz = (pos.z - goal_grid.z).abs();
            let distance_to_goal = std::cmp::max(dx, dz);

            if distance_to_goal > 15 {
                // 멀리 떨어진 목표: 중앙 지향과 유연한 휴리스틱 혼합
                let center_heuristic = pos.center_bias_heuristic(&goal_grid, 5.0);
                let flexible_heuristic = pos.flexible_heuristic(&goal_grid);
                (center_heuristic + flexible_heuristic) / 2
            } else {
                // 가까운 목표: 적응형 중앙 지향 휴리스틱 사용
                pos.adaptive_center_heuristic(&goal_grid)
            }
        }, // 2D 거리 휴리스틱 (적응적)
        |pos| *pos == goal_grid, // 목표 도달 조건
    );

    match result {
        Some((path, total_cost)) => {
            log::info!(
                "[2D A*] SUCCESS! Found 2D path with {} nodes, total cost: {}",
                path.len(),
                total_cost
            );

            // 격자 좌표를 월드 좌표로 변환 (고정 높이 사용)
            let world_path: Vec<Vec3A> = path
                .into_iter()
                .map(|grid_pos| grid_pos.to_world_pos(grid_size, base_height))
                .collect();

            // 2D 경로 품질 검증
            if world_path.len() >= 2 {
                let path_length = calculate_path_length_2d(&world_path);
                let direct_distance =
                    ((goal.x - start.x).powi(2) + (goal.z - start.z).powi(2)).sqrt();
                let efficiency = (direct_distance / path_length.max(0.1)) * 100.0;

                log::debug!(
                    "[2D A*] Path quality: 2D Length={:.1}m, Direct={:.1}m, Efficiency={:.1}%",
                    path_length,
                    direct_distance,
                    efficiency
                );
            }

            Some(world_path)
        }
        None => {
            log::warn!(
                "[2D A*] FAILED to find 2D path from {:?} to {:?}",
                start_grid,
                goal_grid
            );
            None
        }
    }
}

/// 계층적 경로탐색 (Hierarchical Pathfinding)
/// - 먼저 큰 그리드로 거시적 경로 탐색
/// - 그 다음 세밀한 그리드로 미시적 경로 세밀화
/// - 대규모 맵에서 매우 효율적
pub fn hierarchical_pathfind<F>(
    start: Vec3A,
    goal: Vec3A,
    coarse_grid: f32,
    fine_grid: f32,
    is_walkable: F,
) -> Option<Vec<Vec3A>>
where
    F: FnMut(Vec3A) -> bool + Clone,
{
    log::debug!("[HIERARCHICAL] Starting hierarchical pathfinding");
    log::debug!(
        "[HIERARCHICAL] Coarse grid: {:.1}m, Fine grid: {:.1}m",
        coarse_grid,
        fine_grid
    );

    // 1단계: 거시적 경로 (큰 그리드)
    let coarse_path = advanced_astar_pathfind(start, goal, coarse_grid, is_walkable.clone())?;

    if coarse_path.len() <= 2 {
        // 거시적 경로가 너무 단순하면 그대로 반환
        return Some(coarse_path);
    }

    log::debug!(
        "[HIERARCHICAL] Coarse path found with {} waypoints",
        coarse_path.len()
    );

    // 2단계: 미시적 경로 세밀화 (작은 그리드)
    let mut refined_path = Vec::new();

    for i in 0..coarse_path.len() - 1 {
        let segment_start = coarse_path[i];
        let segment_end = coarse_path[i + 1];

        // 각 세그먼트를 세밀한 그리드로 재탐색
        if let Some(segment_path) =
            advanced_astar_pathfind(segment_start, segment_end, fine_grid, is_walkable.clone())
        {
            // 중복 제거하면서 경로 추가
            if i == 0 {
                refined_path.extend(segment_path);
            } else {
                refined_path.extend(segment_path.into_iter().skip(1));
            }
        } else {
            // 세밀화 실패 시 직선으로 연결
            if i > 0 {
                refined_path.push(segment_end);
            }
        }
    }

    log::info!(
        "[HIERARCHICAL] SUCCESS! Refined path with {} waypoints",
        refined_path.len()
    );

    if refined_path.is_empty() {
        Some(coarse_path) // 폴백
    } else {
        Some(refined_path)
    }
}

/// 다중 목표 최적화 경로탐색 (중앙 지향 포함)
/// - 여러 목표 중 가장 가까운 것 찾기
/// - 동적 목표 선택 및 중앙 지향 평가
pub fn multi_target_pathfind_with_center_bias<F>(
    start: Vec3A,
    targets: &[Vec3A],
    grid_size: f32,
    is_walkable: F,
) -> Option<(Vec<Vec3A>, usize)>
// (경로, 선택된 목표 인덱스)
where
    F: FnMut(Vec3A) -> bool + Clone,
{
    if targets.is_empty() {
        return None;
    }

    log::debug!(
        "[MULTI-TARGET] Searching paths to {} targets with center bias",
        targets.len()
    );

    let mut best_path = None;
    let mut best_target_idx = 0;
    let mut best_score = f32::MAX;

    // 시작점의 그리드 좌표 계산
    let start_grid = GridPos::from_world_pos(start, grid_size);

    for (idx, &target) in targets.iter().enumerate() {
        // 각 목표에 대해 경로 비용과 중앙 지향 점수를 계산
        let target_grid = GridPos::from_world_pos(target, grid_size);

        // 중앙 지향 점수 계산 (목표가 중앙에 가까울수록 좋은 점수)
        let center_bias_score = target_grid.center_bias_heuristic(&GridPos::new(0, 0), 3.0) as f32;

        // 직선 거리 계산
        let direct_distance = (target - start).length();
        let distance_score = start_grid.distance_heuristic(&target_grid) as f32;

        // 종합 점수 계산 (거리 + 중앙 지향)
        let combined_score = distance_score + center_bias_score * 0.3; // 중앙 지향 30% 가중치

        if combined_score < best_score {
            // 실제 경로 탐색 시도
            if let Some(path) =
                advanced_astar_pathfind(start, target, grid_size, is_walkable.clone())
            {
                best_score = combined_score;
                best_path = Some(path);
                best_target_idx = idx;

                log::debug!(
                    "[MULTI-TARGET] Target {} - distance: {:.1}m, center_bias: {:.1}, score: {:.1}",
                    idx,
                    direct_distance,
                    center_bias_score,
                    combined_score
                );
            }
        }
    }

    if let Some(path) = best_path {
        log::info!(
            "[MULTI-TARGET] Best target found: index {}, score: {:.1}",
            best_target_idx,
            best_score
        );
        Some((path, best_target_idx))
    } else {
        None
    }
}

/// 다중 목표 최적화 경로탐색
/// - 여러 목표 중 가장 가까운 것 찾기
/// - 동적 목표 선택
// 사용되지 않는 다중 목표 경로탐색 함수 (향후 확장용)
#[allow(dead_code)]
pub fn multi_target_pathfind<F>(
    start: Vec3A,
    targets: &[Vec3A],
    grid_size: f32,
    is_walkable: F,
) -> Option<(Vec<Vec3A>, usize)>
// (경로, 선택된 목표 인덱스)
where
    F: FnMut(Vec3A) -> bool + Clone,
{
    if targets.is_empty() {
        return None;
    }

    log::debug!(
        "[MULTI-TARGET] Searching paths to {} targets",
        targets.len()
    );

    let mut best_path = None;
    let mut best_target_idx = 0;
    let mut shortest_cost = f32::MAX;

    for (idx, &target) in targets.iter().enumerate() {
        if let Some(path) = advanced_astar_pathfind(start, target, grid_size, is_walkable.clone()) {
            let path_cost = calculate_path_length(&path);

            if path_cost < shortest_cost {
                shortest_cost = path_cost;
                best_path = Some(path);
                best_target_idx = idx;
            }
        }
    }

    if let Some(path) = best_path {
        log::info!(
            "[MULTI-TARGET] Best target found: index {}, cost: {:.1}m",
            best_target_idx,
            shortest_cost
        );
        Some((path, best_target_idx))
    } else {
        None
    }
}

/// 경로의 총 길이 계산 (3D)
// 사용되지 않는 경로 길이 계산 함수 (디버깅용)
#[allow(dead_code)]
fn calculate_path_length(path: &[Vec3A]) -> f32 {
    let mut total_length = 0.0;
    for i in 1..path.len() {
        total_length += (path[i] - path[i - 1]).length();
    }
    total_length
}

/// 경로의 2D 평면 길이 계산 (Y축 무시)
fn calculate_path_length_2d(path: &[Vec3A]) -> f32 {
    let mut total_length = 0.0;
    for i in 1..path.len() {
        let dx = path[i].x - path[i - 1].x;
        let dz = path[i].z - path[i - 1].z;
        total_length += (dx * dx + dz * dz).sqrt();
    }
    total_length
}

/// 평면 8방향 이동 벡터 (Y축 제외)
// 사용되지 않는 평면 이웃 방향 계산 함수 (레거시)
#[allow(dead_code)]
fn planar_neighbor_directions() -> Vec<Vec3A> {
    vec![
        // 주요 4방향
        Vec3A::new(1.0, 0.0, 0.0),  // 동
        Vec3A::new(-1.0, 0.0, 0.0), // 서
        Vec3A::new(0.0, 0.0, 1.0),  // 북
        Vec3A::new(0.0, 0.0, -1.0), // 남
        // 대각선 4방향
        Vec3A::new(1.0, 0.0, 1.0).normalize(),   // 북동
        Vec3A::new(-1.0, 0.0, 1.0).normalize(),  // 북서
        Vec3A::new(1.0, 0.0, -1.0).normalize(),  // 남동
        Vec3A::new(-1.0, 0.0, -1.0).normalize(), // 남서
    ]
}
