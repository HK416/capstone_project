//! `Yuuka_Original` 모델과 관련된 코드를 관리합니다.
//!

mod animation;
mod camera;
mod spawn;

use lazy_static::lazy_static;
use mod_network::components::CharacterAttributes;

pub use self::{animation::*, camera::*, spawn::*};

use super::look_to_camera_direction;

/// 캐릭터 모델의 이름입니다.
pub const MODEL_NAME: &'static str = "Yuuka_Original";

lazy_static! {
    pub static ref CHARACTER_ATTRIBUTE: CharacterAttributes = {
        let json = include_str!(concat!(
            env!("CARGO_WORKSPACE_DIR"),
            "/assets/characters/yuuka_original/attribute.json"
        ));
        serde_json::from_str(json).unwrap()
    };
}
