mod intro;
mod lobby;
mod room;
mod startup;
mod testbed;
mod title;

use std::fmt;

pub use self::{intro::*, lobby::*, room::*, startup::*, testbed::*, title::*};

impl fmt::Debug for GameIntroScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: 게임 인트로 화면을 보여주는 장면입니다.",
            stringify!(GameIntroScene)
        )
    }
}

impl fmt::Debug for GameLoginScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: 게임 타이틀 화면에서 로그인창을 보여주는 장면입니다.",
            stringify!(GameLoginScene)
        )
    }
}

impl fmt::Debug for GameTitleScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: 게임 타이틀 화면을 보여주는 장면입니다.",
            stringify!(GameTitleScene)
        )
    }
}

impl fmt::Debug for MainLobbyEnterScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: 메인 로비 장면에 필요한 에셋을 로드하는 장면입니다.",
            stringify!(MainLobbyEnterScene)
        )
    }
}

impl fmt::Debug for MainLobbyExitScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: 메인 로비에 사용된 에셋을 정리하는 장면입니다.",
            stringify!(MainLobbyExitScene)
        )
    }
}

impl fmt::Debug for MainLobbyScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: 현재 플레이어의 정보를 보거나, 게임에 입장할 수 있는 메인 로비 장면입니다.",
            stringify!(MainLobbyScene)
        )
    }
}

impl fmt::Debug for CustomGameEnterScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: 커스텀 게임 대기실 장면에 필요한 에셋을 로드하는 장면입니다.",
            stringify!(CustomGameEnterScene)
        )
    }
}

impl fmt::Debug for CustomGameExitScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: 커스텀 게임 대기실 장면에 사용한 에셋을 정리하는 장면입니다.",
            stringify!(CustomGameExitScene)
        )
    }
}

impl fmt::Debug for CustomGameRoomScene {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: 타 플레이어와 사용자 정의 게임을 시작할 수 있는 커스텀 게임 대기실 장면입니다.",
            stringify!(CustomGameRoomScene)
        )
    }
}
