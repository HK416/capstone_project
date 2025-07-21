use ahash::HashMap;
use lazy_static::lazy_static;
use mod_network::components::{NUM_STAGES, StageAttributes, StageKind};

use crate::data::get_current_path;

const ROOT_WORKSPACE: &'static str = "server_data/stage";
const STAGE_WORKSPACES: [(StageKind, &'static str); NUM_STAGES] = [(StageKind::City, "city")];
const STAGE_URI: &'static str = "map.json";

lazy_static! {
    static ref STAGE_ATTRIBUTES: HashMap<StageKind, StageAttributes> = {
        let mut map = HashMap::default();
        let current_path = get_current_path().to_string_lossy().into_owned();
        for (kind, sub_workspace) in STAGE_WORKSPACES {
            let path = format!(
                "{}/{}/{}/{}",
                current_path, ROOT_WORKSPACE, sub_workspace, STAGE_URI
            );
            map.insert(kind, StageAttributes::load_from_file(path).unwrap());
        }
        map
    };
}

/// 스테이지 속성 데이터를 초기화합니다.
pub fn init_stage_attributes() {
    // `lazy_static` crate는 처음 전역 변수가 사용될 때 초기화를 시도합니다.
    log::info!("initializes stage attribute data.");
    STAGE_ATTRIBUTES.len();
}

/// 주어진 스테이지 종류에 대한 스테이지 속성 데이터를 가져옵니다.
pub fn get_stage_attributes(kind: StageKind) -> &'static StageAttributes {
    STAGE_ATTRIBUTES.get(&kind).unwrap()
}
