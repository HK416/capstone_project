use std::fs;
use serde_json::Error;
use serde::de::Error as SerdeError;
use mod_network::components::CharacterAttributes;

pub fn load_character_attribute(path: &str) -> Result<CharacterAttributes, Error> {
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
        let aris_path = "../../assets/characters/aris_original/attribute.json";
        let momoi_path = "../../assets/characters/momoi_original/attribute.json";
    
        assert!(load_character_attribute(aris_path).is_ok());
        assert!(load_character_attribute(momoi_path).is_ok());
    }
}