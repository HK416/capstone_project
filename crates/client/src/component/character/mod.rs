mod action_state;
mod animation;
mod camera;
mod movement_state;
mod pipeline;
mod pull;
mod render;
mod snapshot;
mod spawn;
mod transform;
mod view_state;

mod aris_original;
mod midori_original;
mod momoi_original;
mod yuuka_original;

use lazy_static::lazy_static;
use mod_network::components::{CharacterAttributes, NUM_CHARACTERS};

pub use self::{
    action_state::*, animation::*, camera::*, movement_state::*, pipeline::*, pull::*, render::*,
    snapshot::*, spawn::*, transform::*, view_state::*,
};

lazy_static! {
    pub static ref CHARACTER_ATTRIBUTES: [&'static CharacterAttributes; NUM_CHARACTERS] = [
        &aris_original::CHARACTER_ATTRIBUTE,
        &momoi_original::CHARACTER_ATTRIBUTE,
        &midori_original::CHARACTER_ATTRIBUTE,
        &yuuka_original::CHARACTER_ATTRIBUTE,
    ];
}
