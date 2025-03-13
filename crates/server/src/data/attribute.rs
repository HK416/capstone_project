use std::{fs::File, io::Read};

use ahash::HashMap;
use lazy_static::lazy_static;
use mod_network::components::{CharacterAttributes, CharacterKind, NUM_CHARACTERS};

use crate::data::get_current_path;

const ROOT_WORKSPACE: &'static str = "server_data/characters";
const CHARACTERS: [(CharacterKind, &'static str); NUM_CHARACTERS] = [
    (CharacterKind::ArisOriginal, "aris_original"),
    (CharacterKind::MomoiOriginal, "momoi_original"),
    (CharacterKind::MidoriOriginal, "midori_original"),
    (CharacterKind::YuukaOriginal, "yuuka_original"),
];
const FILENAME: &'static str = "attribute.json";

lazy_static! {
    static ref CHARACTER_ATTRIBUTES: HashMap<CharacterKind, CharacterAttributes> = {
        let mut map = HashMap::default();
        let current_path = get_current_path().to_string_lossy().into_owned();
        for (kind, sub_workspace) in CHARACTERS {
            let path = format!(
                "{}/{}/{}/{}",
                current_path, ROOT_WORKSPACE, sub_workspace, FILENAME
            );
            map.insert(kind, load_character_attribute(&path));
        }
        map
    };
}

/// 캐릭터 속성 정보를 초기화합니다.
pub fn init_character_attributes() {
    // `lazy_static` crate는 처음 전역 변수가 사용될 때 초기화를 시도합니다.
    log::info!("initializes character attribute data.");
    CHARACTER_ATTRIBUTES.len();
}

/// 주어진 캐릭터 종류에 대한 속성 데이터를 가져옵니다.
pub fn get_character_attributes(kind: CharacterKind) -> &'static CharacterAttributes {
    CHARACTER_ATTRIBUTES.get(&kind).unwrap()
}

/// 캐릭터 속성 데이터 파일을 로드합니다.
///
/// # Panics
/// 캐릭터 속성 데이터 파일을 찾지 못하거나, 읽기에 실패한 경우 `panic!`을 호출합니다.
///
fn load_character_attribute(path: &str) -> CharacterAttributes {
    let mut file = File::open(path)
        .map_err(|e| log::error!("failed to open file. (PATH:{}, REASON:{})", &path, &e))
        .expect("캐릭터 속성 데이터 파일 열기에 실패했습니다!");

    let mut buf = Vec::new();
    file.read_to_end(&mut buf)
        .map_err(|e| log::error!("failed to read file. (PATH:{}, REASON:{})", &path, &e))
        .expect("캐릭터 속성 데이터 파일 읽기에 실패했습니다!");

    let attribute = serde_json::from_slice(&buf)
        .map_err(|e| log::error!("failed to parse file. (PATH:{}, REASON:{})", &path, &e))
        .expect("캐릭터 속성 데이터 파일 구문 분석에 실패했습니다!");

    attribute
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_character_attribute() {
        let current_path = env!("CARGO_WORKSPACE_DIR");
        let root_workspace = "assets/characters";
        for (_, sub_workspace) in CHARACTERS {
            let path = format!(
                "{}/{}/{}/{}",
                current_path, root_workspace, sub_workspace, FILENAME
            );
            load_character_attribute(&path);
        }
    }
}
