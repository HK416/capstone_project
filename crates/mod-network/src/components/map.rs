use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Quaternion {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Area {
    plane: String,
    height: String,
    translation: Vec3,
    rotation: Quaternion,
}

#[derive(Debug, Serialize, Deserialize)]
struct GameMap {
    plane: Vec<String>,
    area: Vec<Area>,
}


impl GameMap {
    /// path를 받아 JSON 파일을 읽어 GameMap 구조체로 변환하는 함수
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file_content = fs::read_to_string(path)?;
        let game_map: GameMap = serde_json::from_str(&file_content)?;
        Ok(game_map)
    }
}


