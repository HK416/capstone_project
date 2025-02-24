use std::fs;

use ahash::HashMap;
use lazy_static::lazy_static;
use mod_network::components::{CharacterAttributes, CharacterKind};
use serde::de::Error as SerdeError;
use serde_json::Error;

use crate::data::get_current_path;

lazy_static! {
    static ref CHARACTER_INFO: HashMap<CharacterKind, CharacterAttributes> = {
        // # Errors
        // 프로그램을 실행하고 있는 도중 프로그램 디렉토리를 삭제할 경우 정의되지 않은 동작을 수행합니다.
        let path = get_current_path().to_str().unwrap();
        HashMap::from_iter([
            (
                CharacterKind::ArisOriginal,
                load_character_attribute(
                    &format!("{}/server_data/characters/aris_original/attribute.json", path)
                ).unwrap()
            ),
            (
                CharacterKind::MomoiOriginal,
                load_character_attribute(
                    &format!("{}/server_data/characters/momoi_original/attribute.json", path)
                ).unwrap()
            ),
            (
                CharacterKind::MidoriOriginal,
                load_character_attribute(
                    &format!("{}/server_data/characters/midori_original/attribute.json", path)
                ).unwrap()
            ),
            (
                CharacterKind::YuukaOriginal,
                load_character_attribute(
                    &format!("{}/server_data/characters/yuuka_original/attribute.json", path)
                ).unwrap()
            )
        ])
    };
}

/// server_data 디렉토리가 실행파일과 같은 위치에 있다고 가정
pub fn get_character_attributes(kind: CharacterKind) -> CharacterAttributes {
    CHARACTER_INFO.get(&kind).unwrap().clone()
}

fn load_character_attribute(path: &str) -> Result<CharacterAttributes, Error> {
    // 파일 내용 읽기
    let file_content = fs::read_to_string(path).map_err(|err| {
        Error::custom(format!("파일 읽기 에러: {}", err)) // `serde::de::Error` 트레이트를 통해 custom 메서드 호출
    })?;

    // JSON 파싱
    let attributes: CharacterAttributes = serde_json::from_str(&file_content)?;
    Ok(attributes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_character_attribute() {
        let aris_path = concat!(
            env!("CARGO_WORKSPACE_DIR"),
            "assets/characters/aris_original/attribute.json"
        );
        let momoi_path = concat!(
            env!("CARGO_WORKSPACE_DIR"),
            "assets/characters/momoi_original/attribute.json"
        );

        assert!(load_character_attribute(aris_path).is_ok());
        assert!(load_character_attribute(momoi_path).is_ok());
    }
}
