mod animation;
mod camera;
mod pipeline;
mod render;
mod spawn;
mod states;

mod aris_original;
mod midori_original;
mod momoi_original;
mod yuuka_original;

use lazy_static::lazy_static;
use mod_network::components::{CharacterAttributes, NUM_CHARACTERS};

pub use self::{animation::*, camera::*, pipeline::*, render::*, spawn::*, states::*};

lazy_static! {
    pub static ref CHARACTER_ATTRIBUTES: [&'static CharacterAttributes; NUM_CHARACTERS] = [
        &aris_original::CHARACTER_ATTRIBUTE,
        &momoi_original::CHARACTER_ATTRIBUTE,
        &midori_original::CHARACTER_ATTRIBUTE,
        &yuuka_original::CHARACTER_ATTRIBUTE,
    ];
}
