// mod finish;
// mod formation;
// mod in_game;
mod layer;
mod lobby;
mod room;
mod startup;
mod title;

use std::fmt;

pub use self::{layer::*, lobby::*, room::*, startup::*, title::*};

/// 인터페이스 배경 색상입니다.
pub const UI_BG_COLOR: egui::Color32 = egui::Color32::from_black_alpha(128);

/// 팀의 색상입니다.
pub const TEAM_COLOR: [egui::Color32; 2] = [
    egui::Color32::from_rgb(0, 150, 255), // 블루팀 색상
    egui::Color32::from_rgb(255, 68, 51), // 레드 팀 색상
];

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

impl fmt::Debug for GameLoginModalScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameLoginModalScene))
    }
}

impl fmt::Debug for GameExitModalScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(GameExitModalScene))
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

impl fmt::Debug for MainLobbyJoinModalScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(MainLobbyJoinModalScene))
    }
}

impl fmt::Debug for CustomGameRoomScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(CustomGameRoomScene))
    }
}

// impl fmt::Debug for CharacterFormationScene {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", stringify!(CharacterFormationScene))
//     }
// }

// impl fmt::Debug for InGameLoadScene {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", stringify!(InGameLoadScene))
//     }
// }

// impl fmt::Debug for InGameBuildScene {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", stringify!(InGameBuildScene))
//     }
// }

// impl fmt::Debug for InGameDominationModePrepareScene {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", stringify!(InGameDominationModePrepareScene))
//     }
// }

// impl fmt::Debug for InGameDominationModeScene {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", stringify!(InGameDominationModeScene))
//     }
// }

// impl fmt::Debug for InGamePauseLayer {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", stringify!(InGamePauseLayer))
//     }
// }

// impl fmt::Debug for InGameDominationModeStatusLayer {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", stringify!(InGameDominationModeStatusLayer))
//     }
// }

// impl fmt::Debug for InGameResultEnterScene {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", stringify!(InGameResultEnterScene))
//     }
// }

// impl fmt::Debug for InGameResultScene {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", stringify!(InGameResultScene))
//     }
// }

impl fmt::Debug for FatalErrorSceneLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(FatalErrorSceneLayer))
    }
}

impl fmt::Debug for MessageSceneLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", stringify!(MessageSceneLayer))
    }
}
