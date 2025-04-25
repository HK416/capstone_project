use std::{fs::File, io::Read};

use ahash::HashMap;
use lazy_static::lazy_static;
use mod_network::components::{NUM_STAGES, StageHeight, StageKind, StageLayoutData};
use mod_physics::collision::ColliderTree;

use super::get_current_path;

const ROOT_WORKSPACE: &'static str = "server_data/stage";
const STAGE_WORKSPACES: [(StageKind, &'static str); NUM_STAGES] = [(StageKind::City, "city")];

lazy_static! {
    static ref STAGE_ATTRIBUTES: HashMap<StageKind, StageAttributes> = {
        let mut map = HashMap::default();
        let current_path = get_current_path().to_string_lossy().into_owned();
        for (kind, sub_workspace) in STAGE_WORKSPACES {
            let workspace = format!("{}/{}/{}", current_path, ROOT_WORKSPACE, sub_workspace);
            map.insert(kind, load_stage_layout(&workspace));
        }
        map
    };
}

/// 게임 월드 스테이지 데이터입니다.
/// - 게임 월드 스테이지의 중심은 월드 좌표계의 원점입니다.
#[derive(Debug, Clone)]
pub struct StageAttributes {
    /// 지역의 x축 방향 개수입니다.
    num_width: usize,
    /// 지역의 z축 방향 개수입니다.
    num_depth: usize,
    /// 게임 스테이지의 크기입니다.
    size: glam::Vec2,
    /// 지역의 크기입니다.
    area_size: glam::Vec2,
    /// 게임 월드 스테이지를 구성하는 각 지역 데이터입니다.   
    /// 인덱스 기반으로 접근하여 높이 값을 가져옵니다.
    area: Vec<Vec<Option<Area>>>,
    /// 게임 월드 스테이지를 구성하는 충돌체 데이터입니다.
    colliders: ColliderTree,
}

#[derive(Debug, Clone)]
pub struct Area {
    translation: glam::Vec3,
    inv_transform: glam::Mat4,
    height: StageHeight,
}

/// 스테이지 속성 데이터를 초기화합니다.
pub fn init_stage_attributes() {
    // `lazy_static` crate는 처음 전역 변수가 사용될 때 초기화를 시도합니다.
    log::info!("initializes stage attribute data.");
    STAGE_ATTRIBUTES.len();
}

/// 주어진 스테이지 종류에 대한 스테이지 속성 데이터를 가져옵니다.
fn get_stage_attributes(kind: StageKind) -> &'static StageAttributes {
    STAGE_ATTRIBUTES.get(&kind).unwrap()
}

/// 스테이지 속성 데이터 파일을 로드합니다.
///
/// # Panics
/// 스테이지 속성 데이터 파일을 찾지 못하거나, 읽기에 실패한 경우 `panic!`을 호출합니다.
///
fn load_stage_layout(workspace: &str) -> StageAttributes {
    let path = format!("{}/map.json", workspace);
    let mut file = File::open(&path)
        .map_err(|e| log::error!("failed to open file. (PATH:{}, REASON:{})", &path, &e))
        .expect("스테이지 속성 데이터 파일 열기에 실패했습니다!");

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| log::error!("failed to read file. (PATH:{}, REASON:{})", &path, &e))
        .expect("스테이지 속성 데이터 파일 읽기에 실패했습니다!");

    let stage_layout: StageLayoutData = serde_json::from_slice(&buf)
        .map_err(|e| log::error!("failed to parse file. (PATH:{}, REASON:{})", &path, &e))
        .expect("스테이지 속성 데이터 파일 구문 분석에 실패했습니다!");

    let n = stage_layout.num_area_width as usize;
    let m = stage_layout.num_area_depth as usize;
    let w = stage_layout.area_size.x * n as f32;
    let d = stage_layout.area_size.y * m as f32;
    let mut area = vec![vec![None; m]; n];
    for data in stage_layout.area.iter() {
        // 높이 데이터가 존재하는 경우만 지역을 추가합니다.
        if let Some(height_map) = &data.height {
            let i = ((data.translation.x + 0.5 * w) / stage_layout.area_size.x).floor() as usize;
            let j = ((data.translation.z + 0.5 * d) / stage_layout.area_size.y).floor() as usize;

            // 역행렬을 계산합니다.
            let transform = glam::Mat4::from_rotation_translation(
                data.rotation.into(),
                data.translation.into(),
            );
            let inv_transform = transform.inverse();

            // 지역의 높이 데이터를 가져옵니다.
            let path = format!("{}/{}.json", workspace, height_map);
            let mut file = File::open(&path)
                .map_err(|e| log::error!("failed to open file. (PATH:{}, REASON:{})", &path, &e))
                .expect("Height 데이터 파일 열기에 실패했습니다!");

            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| log::error!("failed to read file. (PATH:{}, REASON:{})", &path, &e))
                .expect("Height 데이터 파일 읽기에 실패했습니다!");

            let height: StageHeight = serde_json::from_slice(&buf)
                .map_err(|e| log::error!("failed to parse file. (PATH:{}, REASON:{})", &path, &e))
                .expect("Height 데이터 파일 구문 분석에 실패했습니다!");

            area[i][j] = Some(Area {
                translation: data.translation.into(),
                inv_transform,
                height,
            });
        }
    }

    let path = format!("{}/collider.json", workspace);
    let mut file = File::open(&path)
        .map_err(|e| log::error!("failed to open file. (PATH:{}, REASON:{})", &path, &e))
        .expect("스테이지 충돌체 데이터 파일 열기에 실패했습니다!");

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| log::error!("failed to read file. (PATH:{}, REASON:{})", &path, &e))
        .expect("스테이지 충돌체 데이터 파일 읽기에 실패했습니다!");

    let colliders: ColliderTree = serde_json::from_slice(&buf)
        .map_err(|e| log::error!("failed to parse file. (PATH:{}, REASON:{})", &path, &e))
        .expect("스테이지 충돌체 데이터 파일 구문 분석에 실패했습니다!");

    StageAttributes {
        num_width: n,
        num_depth: m,
        size: glam::vec2(w, d),
        area_size: stage_layout.area_size.into(),
        area,
        colliders,
    }
}

/// 주어진 좌표의 게임 스테이지의 높이를 가져옵니다.   
/// 주어진 좌표에 해당하는 게임 스테이지 높이가 없는 경우 `None`을 반환합니다.
pub fn get_stage_height(kind: StageKind, x: f32, z: f32) -> Option<f32> {
    let stage = get_stage_attributes(kind);
    let n = stage.num_width;
    let m = stage.num_depth;
    let translation = glam::Vec3A::new(x, 0.0, z);
    let x = (x + 0.5 * stage.size.x) / stage.area_size.x;
    let z = (z + 0.5 * stage.size.y) / stage.area_size.y;
    let i = x.floor();
    let j = z.floor();
    let mut idx = vec![(i, j)];
    // 정수이면
    if x == i {
        // i+1도 검사
        idx.push((i - 1.0, j));
        idx.push((i + 1.0, j));
    }
    if z == j {
        // j+1도 검사
        idx.push((i, j - 1.0));
        idx.push((i, j + 1.0));
    }
    if x == i && z == j {
        // i+1, j+1도 검사
        idx.push((i - 1.0, j - 1.0));
        idx.push((i + 1.0, j + 1.0));
        idx.push((i - 1.0, j + 1.0));
        idx.push((i + 1.0, j - 1.0));
    }

    let (area, height) = idx
        .iter()
        .filter(|(i, j)| *i >= 0.0 && *i < n as f32 && *j >= 0.0 && *j < m as f32)
        .filter_map(|(i, j)| stage.area[*i as usize][*j as usize].as_ref())
        .map(|area| (area, &area.height))
        .next()?;

    let hw = 0.5 * stage.area_size.x;
    let hh = 0.5 * stage.area_size.y;
    let translation = area.inv_transform.transform_point3a(translation);
    if translation.x < -hw || translation.x > hw || translation.z < -hh || translation.z > hh {
        return None;
    }

    let i = (translation.x + hw) / stage.area_size.x * (height.width - 1) as f32;
    let j = (translation.z + hh) / stage.area_size.y * (height.height - 1) as f32;

    let px = i.floor();
    let pz = j.floor();
    let index = (pz as usize) * (height.width as usize) + (px as usize);
    let height = height.data[index] + area.translation.y;

    Some(height)
}

/// 주어진 좌표가 유효한지 확인합니다.
pub fn is_valid_position(kind: StageKind, x: f32, z: f32) -> bool {
    let stage = get_stage_attributes(kind);
    let n = stage.num_width;
    let m = stage.num_depth;
    let x = (x + 0.5 * stage.size.x) / stage.area_size.x;
    let z = (z + 0.5 * stage.size.y) / stage.area_size.y;
    let i = x.floor() as usize;
    let j = z.floor() as usize;

    if x < 0.0 && i < n && z < 0.0 && j < m {
        stage.area[i][j].is_some()
    } else {
        false
    }
}

pub fn get_nearest_valid_position(kind: StageKind, x: f32, z: f32) -> (f32, f32) {
    let stage = get_stage_attributes(kind);
    let mut min_distance_position = (x, z);
    let mut min_distance = f32::MAX;
    for row in 0..stage.num_depth {
        for col in 0..stage.num_width {
            if let Some(area) = &stage.area[row][col] {
                let min_x = area.translation.x - 0.5 * stage.area_size.x;
                let max_x = area.translation.x + 0.5 * stage.area_size.x;
                let min_z = area.translation.z - 0.5 * stage.area_size.y;
                let max_z = area.translation.z + 0.5 * stage.area_size.y;
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

/// 주어진 스테이지 종류에 대한 충돌체 데이터를 가져옵니다.
pub fn get_stage_colliders(kind: StageKind) -> &'static ColliderTree {
    &get_stage_attributes(kind).colliders
}
