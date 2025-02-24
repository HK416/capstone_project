use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs};

lazy_static! {
    static ref GAMEMAP: HashMap<String, GameMap> = {
        let mut result = HashMap::new();

        for name in ["city"].iter() {
            if let Ok(game_map) = GameMap::new(name) {
                result.insert(name.to_string(), game_map);
            }
        }

        result
    };
    static ref HEIGHTMAP: HashMap<String, HeightMap> = {
        let mut result = HashMap::new();

        let base_path = match std::env::current_dir() {
            Ok(path) => path,
            Err(_) => {
                eprintln!("Failed to get current directory");
                return result;
            }
        };

        for (name, map) in GAMEMAP.iter() {
            let base_path = base_path.join(format!("assets/stage/{}", name));

            for area in &map.area {
                let height_path = base_path.join(format!("{}.json", area.height));
                if let Ok(height_map) = HeightMap::load(&height_path) {
                    result.insert(area.height.clone(), height_map);
                } else {
                    eprintln!("Failed to load height map: {:?}", height_path);
                }
            }
        }

        result
    };
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Float3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Float4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeightMap {
    pub width: u32,
    pub height: u32,
    pub data: Vec<f32>,
}

impl HeightMap {
    pub fn load(path: &std::path::PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let file_content = fs::read_to_string(path)?;
        let height_map: HeightMap = serde_json::from_str(&file_content)?;
        Ok(height_map)
    }

    /// 0~1 사이의 x, z 좌표에 해당하는 높이 값을 반환(사이값은 보간)
    pub fn get_height(&self, x: f32, z: f32) -> f32 {
        // 1. 0~1 사이의 좌표를 HeightMap의 크기에 맞게 변환
        let x = x * self.width as f32;
        let z = z * self.height as f32;

        // 2. 주위 4개 점의 높이 값을 찾는다
        let x0 = x.floor() as u32;
        let z0 = z.floor() as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let z1 = (z0 + 1).min(self.height - 1);

        let h00 = self.data[(z0 * self.width + x0) as usize];
        let h01 = self.data[(z0 * self.width + x1) as usize];
        let h10 = self.data[(z1 * self.width + x0) as usize];
        let h11 = self.data[(z1 * self.width + x1) as usize];

        let dx = x - x0 as f32;
        let dz = z - z0 as f32;

        // 3. 보간
        let h0 = h00 * (1.0 - dx) + h01 * dx;
        let h1 = h10 * (1.0 - dx) + h11 * dx;

        h0 * (1.0 - dz) + h1 * dz
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Area {
    /// Area를 구성하는 평면의 이름(종류)
    pub plane: String,
    /// HeightMap의 파일 이름
    pub height: String,
    /// HeightMap의 위치
    pub translation: Float3,
    /// HeightMap의 회전 각도
    pub rotation: Float4,
    /// Area의 x방향 크기
    #[serde(default = "Area::default_size_x")]
    pub size_x: f32,
    /// Area의 z방향 크기
    #[serde(default = "Area::default_size_z")]
    pub size_z: f32,
}

impl Area {
    pub fn get_height(&self, x: f32, z: f32) -> f32 {
        let (local_x, local_z) = self.get_local_xz(x, z);

        // HeightMap의 좌표계(0~1사이)로 변환
        let x = (local_x + self.size_x / 2.0) / self.size_x;
        let z = (local_z + self.size_z / 2.0) / self.size_z;

        // HeightMap에서 높이 값을 찾아 반환
        HEIGHTMAP.get(&self.height).unwrap().get_height(x, z) + self.translation.y
    }

    pub fn contains(&self, x: f32, z: f32) -> bool {
        let (local_x, local_z) = self.get_local_xz(x, z);

        // Area의 크기 내에 있는지 확인
        (-self.size_x / 2.0..=self.size_x / 2.0).contains(&local_x)
            && (-self.size_z / 2.0..=self.size_z / 2.0).contains(&local_z)
    }

    fn get_local_xz(&self, x: f32, z: f32) -> (f32, f32) {
        // 이동
        let x = x - self.translation.x;
        let z = z - self.translation.z;

        // 회전(y축 회전만 고려)
        let rot = glam::Quat::from_xyzw(
            self.rotation.x,
            self.rotation.y,
            self.rotation.z,
            self.rotation.w,
        );
        let local = glam::Vec3::new(x, 0.0, z);
        let local = rot * local;

        (local.x, local.z)
    }

    fn default_size_x() -> f32 {
        15.0
    }

    fn default_size_z() -> f32 {
        15.0
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameMap {
    /// 맵을 구성하는 평면 목록
    pub plane: Vec<String>,
    /// 맵을 구성하는 Area 목록
    pub area: Vec<Area>,
}

impl GameMap {
    pub fn get(name: &str) -> Option<GameMap> {
        GAMEMAP.get(name).cloned()
    }

    /// JSON 파일에서 `GameMap`을 생성하는 함수
    fn new(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Cargo 프로젝트 루트를 기준으로 상대 경로 설정
        let base_path = std::env::current_dir()?;
        let base_path = base_path.join(format!("assets/stage/{}", name));
        let map_path = base_path.join("map.json");

        println!("맵 파일 경로: {:?}", map_path);

        let map_info = fs::read_to_string(map_path)?;
        let game_map: GameMap = serde_json::from_str(&map_info)?;

        Ok(game_map)
    }

    /// 주어진 (x, z) 좌표가 유효한 지역 내에 있는지 확인
    pub fn is_position_valid(&self, x: f32, z: f32) -> bool {
        self.area.iter().any(|area| area.contains(x, z))
    }

    /// 주어진 (x, z) 좌표에 해당하는 지역을 찾아 y 좌표를 조정
    pub fn adjust_y_position(&self, x: f32, z: f32) -> Option<f32> {
        self.area
            .iter()
            .find(|area| area.contains(x, z))
            .map(|area| area.get_height(x, z))
    }
}
