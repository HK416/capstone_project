use std::fs;

use ahash::HashMap;
use lazy_static::lazy_static;
use mod_network::components::{CharacterAttributes, CharacterKind};
use serde::de::Error as SerdeError;
use serde_json::Error;

lazy_static! {
    static ref CHARACTER_INFO: HashMap<CharacterKind, CharacterAttributes> = {
        let mut map = HashMap::default();

        map.insert(
            CharacterKind::ArisOriginal,
            load_character_attribute("assets/characters/aris_original/attribute.json").unwrap(),
        );
        map.insert(
            CharacterKind::MomoiOriginal,
            load_character_attribute("assets/characters/momoi_original/attribute.json").unwrap(),
        );

        map
    };
}

/// assets폴더가 실행파일과 같은 위치에 있다고 가정
pub fn get_character_info(kind: CharacterKind) -> CharacterAttributes {
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
