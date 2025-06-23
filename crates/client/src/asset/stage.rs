use core::f32;
use std::{fs::OpenOptions, io::Read, path::PathBuf};

use hecs::Entity;
use mod_network::components::{StageLayoutAreaHeight, StageLayoutAttributes};
use mod_physics::{collision::ColliderTree, object3d::Sphere};

use super::AssetError;

/// 스테이지를 구성하는 지역의 속성 데이터입니다.
#[derive(Debug, Clone)]
struct AreaAttribute {
    translation: glam::Vec3A,
    inv_transform: glam::Mat4,
    height_map: StageLayoutAreaHeight,
}

/// 스테이지의 속성 데이터입니다.
#[derive(Debug, Clone)]
pub struct StageAttributes {
    /// x축 방향의 수입니다.
    num_width: usize,
    /// z축 방향의 수입니다.
    num_depth: usize,
    /// 전체 스테이지의 x축 방향 크기입니다.
    sx: f32,
    /// 전체 스테이지의 z축 방향 크기입니다.
    sz: f32,
    /// 스테이지를 구성하는 지역의 x축 방향 크기입니다.
    area_sx: f32,
    /// 스테이지를 구성하는 지역의 z축 방향 크기입니다.
    area_sz: f32,
    /// 스테이지를 구성하는 지역의 속성 데이터입니다.
    area: Vec<Vec<Option<AreaAttribute>>>,
    /// 게임 월드 스테이지를 구성하는 충돌체 데이터입니다.
    colliders: ColliderTree,
}

impl StageAttributes {
    /// 파일로부터 스테이지 속성 데이터를 로드합니다.
    pub fn new<Dir, Uri>(workspace: Dir, uri: Uri) -> Result<StageAttributes, AssetError>
    where
        Dir: AsRef<PathBuf>,
        Uri: AsRef<str>,
    {
        let mut path = workspace.as_ref().clone();
        path.push(format!("{}.json", uri.as_ref()));

        log::debug!("open stage layout asset (PATH:{})", path.display());
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open stage layout asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                AssetError::IOError(e)
            })?;

        log::debug!("read stage layout asset, (PATH:{})", path.display());
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read stage layout asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::IOError(e)
        })?;

        log::debug!("close stage layout asset (PATH:{})", path.display());
        drop(file);

        log::debug!("encode stage layout asset (PATH:{})", path.display());
        let layout: Box<StageLayoutAttributes> = serde_json::from_slice(&buf).map_err(|e| {
            log::error!(
                "failed to encode stage layout asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::ParsingFailed(e)
        })?;

        log::debug!("generate stage attribute data (URI:{})", uri.as_ref());
        let n = layout.num_area_width as usize;
        let m = layout.num_area_depth as usize;
        let w = layout.area_size.x * n as f32;
        let d = layout.area_size.y * m as f32;
        let mut area = vec![vec![None; m]; n];
        for data in layout.area.iter() {
            // 높이 데이터가 존재하는 경우만 지역을 추가합니다.
            if let Some(height_map) = &data.height {
                let i = ((data.translation.x + 0.5 * w) / layout.area_size.x).floor() as usize;
                let j = ((data.translation.z + 0.5 * d) / layout.area_size.y).floor() as usize;

                // 역행렬을 계산합니다.
                let transform = glam::Mat4::from_rotation_translation(
                    data.rotation.into(),
                    data.translation.into(),
                );
                let inv_transform = transform.inverse();

                // 지역의 높이 데이터를 가져옵니다.
                let mut path = workspace.as_ref().clone();
                path.push(format!("{}.json", &height_map));

                log::debug!("open height map asset (PATH:{})", path.display());
                let mut file = OpenOptions::new()
                    .read(true)
                    .write(false)
                    .open(&path)
                    .map_err(|e| {
                        log::error!(
                            "failed to open height map asset (PATH:{}, REASON:{})",
                            path.display(),
                            &e
                        );
                        AssetError::IOError(e)
                    })?;

                log::debug!("read height map asset (PATH:{})", path.display());
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).map_err(|e| {
                    log::error!(
                        "failed to read height map asset (PATH:{}, REASON:{})",
                        path.display(),
                        &e
                    );
                    AssetError::IOError(e)
                })?;

                log::debug!("close height map asset (PATH:{})", path.display());
                drop(file);

                log::debug!("encode height map asset (PATH:{})", path.display());
                let height_map: StageLayoutAreaHeight =
                    serde_json::from_slice(&buf).map_err(|e| {
                        log::error!(
                            "failed to encode height map asset (PATH:{}, REASON:{})",
                            path.display(),
                            &e
                        );
                        AssetError::ParsingFailed(e)
                    })?;

                area[i][j] = Some(AreaAttribute {
                    translation: data.translation.into(),
                    inv_transform,
                    height_map,
                });
            }
        }

        let mut path = workspace.as_ref().clone();
        path.push("collider.json");

        log::debug!("open stage collider asset (PATH:{})", path.display());
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open stage collider asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                AssetError::IOError(e)
            })?;

        log::debug!("read stage collider asset (PATH:{})", path.display());
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read stage collider asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::IOError(e)
        })?;

        log::debug!("close stage collider asset (PATH:{})", path.display());
        drop(file);

        log::debug!("encode stage collider asset (PATH:{})", path.display());
        let colliders: ColliderTree = serde_json::from_slice(&buf).map_err(|e| {
            log::error!(
                "failed to encode stage collider data (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::ParsingFailed(e)
        })?;

        Ok(Self {
            num_width: n,
            num_depth: m,
            sx: w,
            sz: d,
            area_sx: layout.area_size.x,
            area_sz: layout.area_size.y,
            area,
            colliders,
        })
    }

    /// 주어진 x축, z축 좌표의 게임 스테이지 높이를 가져옵니다.  
    /// 해당 좌표의 게임 스테이지 높이 데이터가 없는 경우 `None`을 반환합니다.
    pub fn get_height(&self, x: f32, z: f32) -> Option<f32> {
        let n = self.num_width;
        let m = self.num_depth;
        let i = ((x + 0.5 * self.sx) / self.area_sx).floor();
        let j = ((z + 0.5 * self.sz) / self.area_sz).floor();

        let mut indices = vec![(i, j)];
        if x == i {
            // i±1을 검사합니다.
            indices.push((i - 1.0, j));
            indices.push((i + 1.0, j));
        }
        if z == j {
            // j±1을 검사합니다.
            indices.push((i, j - 1.0));
            indices.push((i, j + 1.0));
        }
        if x == i && z == j {
            // i±1, j±1을 검사합니다.
            indices.push((i - 1.0, j - 1.0));
            indices.push((i + 1.0, j + 1.0));
            indices.push((i - 1.0, j + 1.0));
            indices.push((i + 1.0, j - 1.0));
        }

        let (area, height_map) = indices
            .iter()
            .filter(|(i, j)| *i >= 0.0 && *i < n as f32 && *j >= 0.0 && *j < m as f32)
            .filter_map(|(i, j)| self.area[*i as usize][*j as usize].as_ref())
            .map(|area| (area, &area.height_map))
            .next()?;

        let hw = 0.5 * self.area_sx;
        let hh = 0.5 * self.area_sz;
        let translation = area.inv_transform.transform_point3a(glam::vec3a(x, 0.0, z));
        if translation.x < -hw || translation.x > hw || translation.z < -hh || translation.z > hh {
            return None;
        }

        let i = ((x + hw) / self.area_sx * (height_map.width - 1) as f32).floor();
        let j = ((z + hh) / self.area_sz * (height_map.height - 1) as f32).floor();
        let index = (j as usize) * (height_map.width as usize) + (i as usize);
        let height = height_map.data[index] + area.translation.y;

        Some(height)
    }

    /// 주어진 좌표가 유효한지 여부를 반환합니다.
    pub fn is_valid_position(&self, x: f32, z: f32) -> bool {
        let i = ((x + 0.5 * self.sx) / self.area_sx).floor() as usize;
        let j = ((z + 0.5 * self.sz) / self.area_sz).floor() as usize;
        self.area.get(i).is_some_and(|area| area.get(j).is_some())
    }

    /// 가까운 유효한 위치를 가져옵니다.
    pub fn get_nearest_valid_position(&self, x: f32, z: f32) -> (f32, f32) {
        let mut min_distance_position = (x, z);
        let mut min_distance = f32::MAX;
        for row in 0..self.num_depth {
            for col in 0..self.num_width {
                if let Some(area) = &self.area[row][col] {
                    let min_x = area.translation.x - 0.5 * self.area_sx;
                    let max_x = area.translation.x + 0.5 * self.area_sx;
                    let min_z = area.translation.z - 0.5 * self.area_sz;
                    let max_z = area.translation.z + 0.5 * self.area_sz;
                    let dx = x.clamp(min_x, max_x) - x;
                    let dz = z.clamp(min_z, max_z) - z;
                    let distance = dx * dx + dz * dz;
                    if distance < min_distance {
                        min_distance = distance;
                        min_distance_position = (x + dx, z + dz);
                    }
                }
            }
        }

        min_distance_position
    }

    /// 스테이지의 충돌체 데이터를 가져옵니다.
    pub fn get_collider_tree(&self) -> &ColliderTree {
        &self.colliders
    }
}

/// 스테이지의 BVH입니다.
#[derive(Debug, Clone)]
pub struct StageBoundingVolumnHierarchy {
    pub area: Vec<Entity>,
    pub root: Option<Box<StageBoundingVolumn>>,
}

impl Default for StageBoundingVolumnHierarchy {
    fn default() -> Self {
        Self {
            area: Vec::default(),
            root: None,
        }
    }
}

/// 스테이지를 구성하는 오브젝트의 Bounding Volumn입니다.
#[derive(Debug, Clone)]
pub struct StageBoundingVolumn {
    pub entity: Entity,
    pub sphere: Sphere,
    pub left: Option<Box<StageBoundingVolumn>>,
    pub right: Option<Box<StageBoundingVolumn>>,
}
