mod lobby;
mod room;
mod startup;
mod testbed;
mod title;

use std::fmt;

pub use self::{lobby::*, room::*, startup::*, title::*};

/// 기본 애플리케이션 창의 가로 길이 입니다.
const BASE_WIDTH: f32 = 1280.0;

impl fmt::Debug for GameStartupScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameStartupScene))
    }
}

impl fmt::Debug for InitLocaleScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(InitLocaleScene))
    }
}

impl fmt::Debug for InitWindowScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(InitWindowScene))
    }
}

impl fmt::Debug for InitFinishScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(InitFinishScene))
    }
}

impl fmt::Debug for GameIntroNotifyScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameIntroNotifyScene))
    }
}

impl fmt::Debug for GameIntroLogoScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameIntroLogoScene))
    }
}

impl fmt::Debug for GameIntroConnectScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameIntroConnectScene))
    }
}

impl fmt::Debug for GameIntroVerifyScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameIntroVerifyScene))
    }
}

impl fmt::Debug for GameLoginTitleScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameLoginTitleScene))
    }
}

impl fmt::Debug for MainLobbyEnterScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(MainLobbyEnterScene))
    }
}

impl fmt::Debug for MainLobbyScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(MainLobbyScene))
    }
}

impl fmt::Debug for CustomGameRoomScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(CustomGameRoomScene))
    }
}
