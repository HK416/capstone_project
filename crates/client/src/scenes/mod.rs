mod intro;
mod room;
mod startup;
mod testbed;

use std::fmt;

pub use self::{intro::*, room::*, startup::*, testbed::*};

impl fmt::Debug for CustomGameEnterScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(CustomGameEnterScene))
    }
}

impl fmt::Debug for CustomGameExitScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(CustomGameExitScene))
    }
}

impl fmt::Debug for CustomGameRoomScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(CustomGameRoomScene))
    }
}
