use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path,PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Float3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Area {
    pub plane: String,
    pub height: String,
    pub translation: Float3,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GameMap {
    pub plane: Vec<String>,
    pub area: Vec<Area>,
}

impl GameMap {
    /// JSON 파일에서 `GameMap`을 생성하는 함수
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Cargo 프로젝트 루트를 기준으로 상대 경로 설정 
        let base_path = std::env::current_dir()?;
        let path = base_path.join("assets/stage/city/map.json");

        println!("맵 파일 경로: {:?}", path); 

        let file_content = fs::read_to_string(path)?;
        let game_map: GameMap = serde_json::from_str(&file_content)?;
        Ok(game_map)
    }

    /// 주어진 (x, z) 좌표가 유효한 지역 내에 있는지 확인
    pub fn is_position_valid(&self, x: f32, z: f32) -> bool {
        self.area.iter().any(|area| {
            (area.translation.x - 15.0..=area.translation.x + 15.0).contains(&x)
                && (area.translation.z - 15.0..=area.translation.z + 15.0).contains(&z)
        })
    }

    /// 주어진 (x, z) 좌표에 해당하는 지역을 찾아 y 좌표를 조정
    pub fn adjust_y_position(&self, x: f32, z: f32) -> Option<f32> {
        self.area
            .iter()
            .find(|area| {
                (area.translation.x - 15.0..=area.translation.x + 15.0).contains(&x)
                    && (area.translation.z - 15.0..=area.translation.z + 15.0).contains(&z)
            })
            .map(|area| area.translation.y)
    }
}
