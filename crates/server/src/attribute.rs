use serde_json::Error;
use std::fs;
use serde::de::Error as SerdeError;
use mod_network::components::attributes::CharacterAttributes;

pub fn load_character_attribute(path: &str) -> Result<CharacterAttributes, Error> {
    // 파일 내용 읽기
    let file_content = fs::read_to_string(path).map_err(|err| {
        Error::custom(format!("파일 읽기 에러: {}", err)) // `serde::de::Error` 트레이트를 통해 custom 메서드 호출
    })?;

    // JSON 파싱
    let attributes: CharacterAttributes = serde_json::from_str(&file_content)?;
    Ok(attributes)
}

