use std::{fs::File, io::Read, sync::OnceLock};

use ahash::HashMap;
use mod_network::{
    assets::{StageHeight, StageLayoutData},
    components::StageKind,
};

use super::get_current_path;

/// 전역 변수로 선언된 게임 스테이지 데이터입니다.
static GAME_MAP: OnceLock<HashMap<StageKind, Stage>> = OnceLock::new();

/// 시가지 스테이지의 맵 데이터 상대 경로입니다.
const STAGE_CITY_WORKSPACE: &'static str = "/server_data/stage/city";
/// 시가지 스테이지의 맵 데이터 파일 이름입니다.
const STAGE_CITY_LAYOUT: &'static str = "map.json";

/// 게임 월드 스테이지 데이터입니다.
/// - 게임 월드 스테이지의 중심은 월드 좌표계의 원점입니다.
#[derive(Debug, Clone)]
pub struct Stage {
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
}

#[derive(Debug, Clone)]
pub struct Area {
    translation: glam::Vec3,
    inv_transform: glam::Mat4,
    height: StageHeight,
}

/// 스테이지 정보를 읽고 서버에서 사용할 수 있도록 데이터를 가공합니다.
fn load_stage_layout(workspace: &str, path: &str) -> Stage {
    let path = format!("{}/{}", workspace, path);
    let mut file = File::open(&path)
        .map_err(|e| log::error!("{} (PATH:{})", &e, path))
        .expect("파일 열기에 실패했습니다!");

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| log::error!("{} (PATH:{})", &e, path))
        .expect("파일 읽기에 실패했습니다!");

    let stage_layout: StageLayoutData = serde_json::from_slice(&buf)
        .map_err(|e| log::error!("{} (PATH:{})", &e, path))
        .expect("JSON 포맷 구문 분석에 실패했습니다!");

    let n = stage_layout.num_area_width as usize;
    let m = stage_layout.num_area_depth as usize;
    let w = stage_layout.area_size.x * n as f32;
    let d = stage_layout.area_size.y * m as f32;
    let mut area = vec![vec![None; m]; n];
    for data in stage_layout.area.iter() {
        let i = ((data.translation.x + 0.5 * w) / stage_layout.area_size.x).floor() as usize;
        let j = ((data.translation.z + 0.5 * d) / stage_layout.area_size.y).floor() as usize;

        // 역행렬을 계산합니다.
        let transform =
            glam::Mat4::from_rotation_translation(data.rotation.into(), data.translation.into());
        let inv_transform = transform.inverse();

        // 지역의 높이 데이터를 가져옵니다.
        let path = format!("{}/{}.json", workspace, &data.height);
        let mut file = File::open(&path)
            .map_err(|e| log::error!("{} (PATH:{})", &e, &path))
            .expect("HeightMap 텍스처 열기에 실패했습니다!");
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| log::error!("{} (PATH:{})", &e, &path))
            .expect("HeightMap 텍스처 읽기에 실패했습니다!");
        let height: StageHeight = serde_json::from_slice(&buf)
            .map_err(|e| log::error!("{} (PATH:{})", &e, &path))
            .expect("JSON 포맷 구문 분석에 실패했습니다!");

        area[i][j] = Some(Area {
            translation: data.translation.into(),
            inv_transform,
            height,
        });
    }

    Stage {
        num_width: n,
        num_depth: m,
        size: glam::vec2(w, d),
        area_size: stage_layout.area_size.into(),
        area,
    }
}

/// 게임 스테이지 데이터를 가져옵니다.
fn get_game_map() -> &'static HashMap<StageKind, Stage> {
    GAME_MAP.get_or_init(|| {
        let current_dir = get_current_path().to_string_lossy().into_owned();
        let mut map = HashMap::default();
        map.insert(
            StageKind::Downtown,
            load_stage_layout(&(current_dir + STAGE_CITY_WORKSPACE), STAGE_CITY_LAYOUT),
        );
        map
    })
}

/// 주어진 좌표의 게임 스테이지의 높이를 가져옵니다.   
/// 주어진 좌표에 해당하는 게임 스테이지 높이가 없는 경우 `None`을 반환합니다.
pub fn get_stage_height(kind: StageKind, x: f32, z: f32) -> Option<f32> {
    let stage = get_game_map().get(&kind).unwrap();
    let n = stage.num_width;
    let m = stage.num_depth;
    let i = ((x + 0.5 * stage.size.x) / stage.area_size.x).floor();
    let j = ((z + 0.5 * stage.size.y) / stage.area_size.y).floor();
    if i < 0.0 || i >= n as f32 || j < 0.0 || j >= m as f32 {
        return None;
    }

    let (area, height) = match &stage.area[i as usize][j as usize] {
        Some(area) => (area, &area.height),
        None => return None,
    };

    let hw = 0.5 * stage.area_size.x;
    let hh = 0.5 * stage.area_size.y;
    let translation = glam::Vec3A::new(x, 0.0, z);
    let translation = area.inv_transform.transform_point3a(translation);
    if translation.x < -hw || translation.x >= hw || translation.z < -hh || translation.z >= hh {
        return None;
    }

    let i = (translation.x + hw) / stage.area_size.x * height.width as f32;
    let j = (translation.z + hh) / stage.area_size.y * height.height as f32;

    let px = i.floor();
    let pz = j.floor();
    let index = (pz as usize) * (height.width as usize) + (px as usize);
    let height = height.data[index] + area.translation.y;

    Some(height)
}

/// 주어진 좌표가 유효한지 확인합니다.
pub fn is_valid_position(kind: StageKind, x: f32, z: f32) -> bool {
    let stage = get_game_map().get(&kind).unwrap();
    let n = stage.num_width;
    let m = stage.num_depth;
    let x = (x + 0.5 * stage.size.x) / stage.area_size.x;
    let z = (z + 0.5 * stage.size.y) / stage.area_size.y;
    let i = x.floor();
    let j = z.floor();
    let mut idx = vec![(i, j)];
    // 정수이면
    if x == i {
        // i+1도 검사
        idx.push((i + 1.0, j));
    }
    if z == j {
        // j+1도 검사
        idx.push((i, j + 1.0));
    }
    if x == i && z == j {
        // i+1, j+1도 검사
        idx.push((i + 1.0, j + 1.0));
    }

    idx.iter()
        .filter(|(i, j)| *i >= 0.0 && *i < n as f32 && *j >= 0.0 && *j < m as f32)
        .any(|(i, j)| stage.area[*i as usize][*j as usize].is_some())
}

/// 두번째 좌표를 첫번째 좌표가 속한 영역 안으로 clamp합니다.  
pub fn clamp_x(kind: StageKind, x1: f32, x2: f32) -> f32 {
    let stage = get_game_map().get(&kind).unwrap();

    let x_min = ((x1 + 0.5 * stage.area_size.x) / stage.area_size.x).floor() * stage.area_size.x
        - 0.5 * stage.area_size.x;
    let x_max = x_min + stage.area_size.x;

    let x = x2.clamp(x_min, x_max);

    x
}

/// 두번째 좌표를 첫번째 좌표가 속한 영역 안으로 clamp합니다.  
pub fn clamp_z(kind: StageKind, z1: f32, z2: f32) -> f32 {
    let stage = get_game_map().get(&kind).unwrap();

    let z_min = ((z1 + 0.5 * stage.area_size.y) / stage.area_size.y).floor() * stage.area_size.y
        - 0.5 * stage.area_size.y;
    let z_max = z_min + stage.area_size.y;

    let z = z2.clamp(z_min, z_max);

    z
}
