//! 지형의 속성 데이터와 관련된 코드를 관리합니다.
//!

use core::f32;
use std::{
    collections::VecDeque,
    f32::EPSILON,
    fs::OpenOptions,
    io::{self, Read},
    path::Path,
};

use mod_physics::collision::{Collider, ColliderTree, ColliderTreeIterator};

use crate::components::{
    GlobalLightData, HeightData, PropAttributeData, StageAttributesData, Team,
};

#[derive(Debug, thiserror::Error)]
pub enum StageLoadError {
    #[error("invalid data")]
    InvalidData,

    /// 에셋 파일을 구문 분석하는데 실패한 경우 발생하는 오류입니다.
    #[error("failed to parse asset for the following reason:{0})")]
    ParsingFailed(#[from] serde_json::Error),

    /// 파일을 열거나 읽을 때 발생하는 오류입니다.
    #[error("failed to read asset for the following reason:{0})")]
    IOError(#[from] io::Error),
}

/// 스테이지 데이터입니다.
#[derive(Debug, Clone)]
pub struct StageAttributes {
    /// 지역의 x축 방향 개수입니다.
    pub num_area_width: usize,
    /// 지역의 z축 방향 개수입니다.
    pub num_area_depth: usize,

    /// 지역의 x축 방향 길이입니다.
    pub area_width: f32,
    /// 지역의 z축 방향 길이입니다.
    pub area_depth: f32,

    /// 스테이지의 x축 방향 전체 크기입니다.
    pub total_width: f32,
    /// 스테이지의 z축 방향 전체 크기입니다.
    pub total_depth: f32,

    /// 게임 월드 스테이지에서 사용되는 모델의 목록
    pub model_list: Vec<String>,

    /// 전역 조명 데이터입니다.
    pub global_light: Option<GlobalLightData>,

    /// 게임 월드 스테이지를 구성하는 각 지역 데이터입니다.
    /// 인덱스 기반으로 접근하여 높이 값을 가져옵니다.
    pub area: Vec<Vec<Option<AreaAttributes>>>,
    /// 소품 데이터입니다.
    pub prop: Option<Box<PropAttributeData>>,
    /// 게임 월드 스테이지를 구성하는 충돌체 데이터입니다.
    pub collider: ColliderTree,
    /// 점령 지역의 충돌체입니다.
    pub capture_zone: Collider,

    /// 블루 팀 스폰 위치입니다.
    pub blue_team_positions: Vec<glam::Vec3A>,
    /// 블루 팀 스폰 방향입니다.
    pub blue_team_rotation: glam::Quat,
    /// 블루 팀 안전 지역 충돌체입니다.
    pub blue_team_collider: ColliderTree,

    /// 레드 팀 스폰 위치입니다.
    pub red_team_positions: Vec<glam::Vec3A>,
    /// 레드 팀 스폰 방향입니다.
    pub red_team_rotation: glam::Quat,
    /// 레드 팀 안전 지역 충돌체입니다.
    pub red_team_collider: ColliderTree,
}

#[derive(Debug, Clone)]
pub struct AreaAttributes {
    pub model: String,
    /// 지역의 월드 변환 행렬입니다.
    pub translation: glam::Vec3A,
    /// 지역의 월드 변환 행렬의 역행렬입니다.
    pub inv_transform: glam::Mat4,
    /// 높이 데이터입니다.
    pub height: HeightData,
}

impl StageAttributes {
    /// 파일에서 스테이지 속성 데이터를 생성합니다.
    ///
    /// 스테이지 속성 데이터 생성에 실패한 경우 [`Err`]를 반환합니다.
    ///
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, StageLoadError> {
        // 경로를 가져옵니다.
        let path = path.as_ref();
        let workspace = match path.parent() {
            Some(workspace) => workspace,
            None => {
                log::error!("path not found!");
                return Err(StageLoadError::IOError(io::ErrorKind::NotFound.into()));
            }
        };

        // 파일을 엽니다.
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open file. (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                StageLoadError::IOError(e)
            })?;

        // 파일을 읽습니다.
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read file. (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            StageLoadError::IOError(e)
        })?;
        drop(file);

        // 데이터를 구문 분석합니다.
        let attributes: StageAttributesData = serde_json::from_slice(&buf).map_err(|e| {
            log::error!(
                "failed to parse file. (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            StageLoadError::ParsingFailed(e)
        })?;

        // 데이터를 생성합니다.
        let n = attributes.num_area_width as usize;
        let m = attributes.num_area_depth as usize;
        let w = attributes.area_width * n as f32;
        let d = attributes.area_depth * m as f32;
        let mut area = vec![vec![None; m]; n];
        for data in attributes.area.iter() {
            // 높이 데이터가 존재하는 경우만 지역을 추가합니다.
            let height = match &data.height_map {
                Some(filename) => {
                    let mut path = workspace.to_path_buf();
                    path.push(format!("{}.json", filename));

                    // 지역의 높이 데이터 파일을 엽니다.
                    let mut file = OpenOptions::new()
                        .read(true)
                        .write(false)
                        .open(&path)
                        .map_err(|e| {
                            log::error!(
                                "failed to open file. (PATH:{}, REASON:{})",
                                path.display(),
                                &e
                            );
                            StageLoadError::IOError(e)
                        })?;

                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf).map_err(|e| {
                        log::error!(
                            "failed to read file. (PATH:{}, REASON:{})",
                            path.display(),
                            &e
                        );
                        StageLoadError::IOError(e)
                    })?;
                    drop(file);

                    let height_map: HeightData = serde_json::from_slice(&buf).map_err(|e| {
                        log::error!(
                            "failed to parse file. (PATH:{}, REASON:{})",
                            path.display(),
                            &e
                        );
                        StageLoadError::ParsingFailed(e)
                    })?;

                    height_map
                }
                None => continue,
            };

            let i = ((data.translation.x + 0.5 * w) / attributes.area_width).floor() as usize;
            let j = ((data.translation.z + 0.5 * d) / attributes.area_depth).floor() as usize;
            let transform = glam::Mat4::from_translation(data.translation.into());
            let inv_transform = transform.inverse();
            area[i][j] = Some(AreaAttributes {
                model: data.model.clone(),
                translation: data.translation.into(),
                inv_transform,
                height,
            });
        }

        let blue_team_positions = attributes
            .blue_team_positions
            .iter()
            .copied()
            .map(|v| v.into())
            .collect();
        let blue_team_rotation = attributes.blue_team_rotation.into();
        let red_team_positions = attributes
            .red_team_positions
            .iter()
            .copied()
            .map(|v| v.into())
            .collect();
        let red_team_rotation = attributes.red_team_rotation.into();

        // 충돌체 데이터 파일을 엽니다.
        let mut path = workspace.to_path_buf();
        path.push(&attributes.collider);

        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open file. (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                StageLoadError::IOError(e)
            })?;

        // 충돌체 데이터를 읽습니다.
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read file. (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            StageLoadError::IOError(e)
        })?;
        drop(file);

        // 충돌체 데이터를 구문 분석합니다.
        let collider: ColliderTree = serde_json::from_slice(&buf).map_err(|e| {
            log::error!(
                "failed to parse file. (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            StageLoadError::ParsingFailed(e)
        })?;

        // 블루 팀 안전 지역 충돌체 데이터 파일을 엽니다.
        let mut path = workspace.to_path_buf();
        path.push(&attributes.blue_team_collider);

        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open file. (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                StageLoadError::IOError(e)
            })?;

        // 충돌체 데이터를 읽습니다.
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read file. (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            StageLoadError::IOError(e)
        })?;
        drop(file);

        // 충돌체 데이터를 구문 분석합니다.
        let blue_team_collider: ColliderTree = serde_json::from_slice(&buf).map_err(|e| {
            log::error!(
                "failed to parse file. (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            StageLoadError::ParsingFailed(e)
        })?;

        // 레드 팀 안전 지역 충돌체 데이터 파일을 엽니다.
        let mut path = workspace.to_path_buf();
        path.push(&attributes.blue_team_collider);

        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open file. (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                StageLoadError::IOError(e)
            })?;

        // 충돌체 데이터를 읽습니다.
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read file. (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            StageLoadError::IOError(e)
        })?;
        drop(file);

        // 충돌체 데이터를 구문 분석합니다.
        let red_team_collider: ColliderTree = serde_json::from_slice(&buf).map_err(|e| {
            log::error!(
                "failed to parse file. (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            StageLoadError::ParsingFailed(e)
        })?;

        Ok(StageAttributes {
            num_area_width: n,
            num_area_depth: m,
            area_width: attributes.area_width,
            area_depth: attributes.area_depth,
            total_depth: d,
            total_width: w,
            model_list: attributes.model_list,
            global_light: attributes.global_light,
            area,
            prop: attributes.prop,
            collider,
            capture_zone: attributes.capture_zone,
            blue_team_positions,
            blue_team_rotation,
            blue_team_collider,
            red_team_positions,
            red_team_rotation,
            red_team_collider,
        })
    }

    /// 주어진 좌표에 해당하는 지역의 높이를 가져옵니다.  
    /// 해당 지역이 비어있는 경우 `None`을 반환합니다.
    pub fn get_area_height(&self, x: f32, z: f32) -> Option<f32> {
        let translation = glam::vec3a(x, 0.0, z);

        let n = self.num_area_width;
        let m = self.num_area_depth;

        let x = (x + 0.5 * self.total_width) / self.area_width;
        let z = (z + 0.5 * self.total_depth) / self.area_depth;

        let i = x.floor();
        let j = z.floor();

        let mut indices = vec![(i, j)];
        // 정수이면
        if (x - i).abs() <= EPSILON {
            // i+1도 검사
            indices.push((i - 1.0, j));
            indices.push((i + 1.0, j));
        }
        if (z - j).abs() <= EPSILON {
            // j+1도 검사
            indices.push((i, j - 1.0));
            indices.push((i, j + 1.0));
        }
        if (x - i) <= EPSILON && (z - j) <= EPSILON {
            // i+1, j+1도 검사
            indices.push((i - 1.0, j - 1.0));
            indices.push((i + 1.0, j + 1.0));
            indices.push((i - 1.0, j + 1.0));
            indices.push((i + 1.0, j - 1.0));
        }

        let (area, height) = indices
            .iter()
            .filter(|(i, j)| *i >= 0.0 && *i < n as f32 && *j >= 0.0 && *j < m as f32)
            .filter_map(|(i, j)| self.area[*i as usize][*j as usize].as_ref())
            .map(|area| (area, &area.height))
            .next()?;

        let hw = 0.5 * self.area_width;
        let hd = 0.5 * self.area_depth;
        let translation = area.inv_transform.transform_point3a(translation);
        if translation.x < -hw || translation.x > hw || translation.z < -hd || translation.z > hd {
            return None;
        }

        let i = (translation.x + hw) / self.area_width * (height.width - 1) as f32;
        let j = (translation.z + hd) / self.area_depth * (height.height - 1) as f32;

        let px = i.floor();
        let pz = j.floor();
        let index = (pz as usize) * (height.width as usize) + (px as usize);
        let height = height.data[index] + area.translation.y;

        Some(height)
    }

    /// 가까운 유효한 위치를 가져옵니다.
    pub fn get_nearest_valid_position(&self, x: f32, z: f32) -> (f32, f32) {
        let n = self.num_area_width;
        let m = self.num_area_depth;

        let i = ((x + 0.5 * self.total_width) / self.area_width).floor() as usize;
        let i = i.clamp(0, n - 1);

        let j = ((z + 0.5 * self.total_depth) / self.area_depth).floor() as usize;
        let j = j.clamp(0, m - 1);

        let mut min_distance = f32::MAX;
        let mut min_distance_position = (x, z);
        let mut queue = VecDeque::new();
        queue.push_back((i, j));

        while let Some((i, j)) = queue.pop_front() {
            if let Some(area) = &self.area[i][j] {
                let min_x = area.translation.x - 0.5 * self.area_width;
                let max_x = area.translation.x + 0.5 * self.area_width;
                let min_z = area.translation.z - 0.5 * self.area_depth;
                let max_z = area.translation.z + 0.5 * self.area_depth;
                let dx = x.clamp(min_x, max_x) - x;
                let dz = z.clamp(min_z, max_z) - z;
                let distance = dx * dx + dz * dz;
                if min_distance <= distance {
                    continue;
                }

                min_distance = min_distance;
                min_distance_position = (x + dx, z + dz);

                if i > 0 {
                    queue.push_back((i - 1, j));
                }

                if j > 0 {
                    queue.push_back((i, j - 1));
                }

                if i + 1 < n {
                    queue.push_back((i + 1, j));
                }

                if j + 1 < m {
                    queue.push_back((i, j + 1));
                }
            }
        }

        min_distance_position
    }

    /// 주어진 좌표가 유효한지 확인합니다.
    pub fn is_valid_point(&self, team: Team, point: &glam::Vec3A) -> bool {
        let n = self.num_area_width;
        let m = self.num_area_depth;

        let i = ((point.x + 0.5 * self.total_width) / self.area_width).floor() as usize;
        let j = ((point.z + 0.5 * self.total_depth) / self.area_depth).floor() as usize;

        if i < n && j < m {
            // 다른 팀의 안전구연인 경우 invalid
            let iterator = match team {
                Team::Blue => ColliderTreeIterator::new(&self.red_team_collider),
                Team::Red => ColliderTreeIterator::new(&self.blue_team_collider),
            };

            for collider in iterator {
                if collider.check_point_collision(point) {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }

    pub fn is_safe_area(&self, team: Team, point: &glam::Vec3A) -> bool {
        // 다른 팀의 안전구연인 경우 invalid
        let iterator = match team {
            Team::Blue => ColliderTreeIterator::new(&self.blue_team_collider),
            Team::Red => ColliderTreeIterator::new(&self.red_team_collider),
        };

        for collider in iterator {
            if collider.check_point_collision(point) {
                return true;
            }
        }

        false
    }
}
