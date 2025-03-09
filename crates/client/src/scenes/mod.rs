mod lobby;
mod room;
mod startup;
mod title;

use std::fmt;

pub use self::{lobby::*, room::*, startup::*, title::*};

impl fmt::Debug for GameStartupScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameStartupScene))
    }
}

impl fmt::Debug for InitConfigScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(InitConfigScene))
    }
}

impl fmt::Debug for GameIntroScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameIntroScene))
    }
}

impl fmt::Debug for GameLoginScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameLoginScene))
    }
}

impl fmt::Debug for GameTitleScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameTitleScene))
    }
}

impl fmt::Debug for MainLobbyEnterScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(MainLobbyEnterScene))
    }
}

impl fmt::Debug for MainLobbyExitScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(MainLobbyExitScene))
    }
}

impl fmt::Debug for MainLobbyScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(MainLobbyScene))
    }
}

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
