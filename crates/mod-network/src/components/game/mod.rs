mod bullet_kind;
mod character;
mod formation;
mod health;
mod icon;
mod in_game;
mod input;
mod latlon;
mod name;
mod network;
mod permission;
mod room;
mod skill;
mod stage;
mod state;
mod team;
mod tier;
mod timer;
mod weapon;

pub use self::{
    bullet_kind::*, character::*, formation::*, health::*, icon::*, in_game::*, input::*,
    latlon::*, name::*, network::*, permission::*, room::*, skill::*, stage::*, state::*, team::*,
    tier::*, timer::*, weapon::*,
};

/// 최대 카메라 Fov-Y 값 (단위: 라디안)
pub const MAX_CAMERA_FOV_Y: f32 = 90f32.to_radians();

/// 최소 위도 (단위: 라디안)
pub const MIN_LATITUDE: f32 = -30f32.to_radians();
/// 최대 위도 (단위: 라디안)
pub const MAX_LATITUDE: f32 = 30f32.to_radians();
