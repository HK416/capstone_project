use std::sync::Arc;

use mod_network::components::{CharacterKind, ObjectId, Team, UserId};

use crate::session::Session;

use super::GameWorldStateFlow;

/// 게임 월드 이벤트 목록입니다.
#[derive(Debug)]
pub enum GameWorldEvent {
    /// 게임 월드 상태를 변경합니다.
    SetControlFlow(GameWorldStateFlow),

    /// 커스텀 게임 대기실에서 사용되는
    /// 플레이어의 게임 준비 요청입니다.
    CustomRoomReady {
        session: Arc<Session>,
        uid: UserId,
        ready: bool,
    },

    /// 캐릭터 편성에서 사용되는
    /// 플레이어의 캐릭터 선택 요청입니다.
    SelectCharacter {
        session: Arc<Session>,
        uid: UserId,
        kind: CharacterKind,
    },

    /// 인게임 진입에서 사용되는
    /// 플레이어 인게임 로드 완료 요청입니다.
    GameLoadFinish { session: Arc<Session>, uid: UserId },

    /// 총알 오브젝트를 추가합니다.
    AddBullet { shooter_id: UserId, delay: f32 },
    /// 총알 오브젝트를 제거합니다.
    RemoveBullet(ObjectId),

    /// 플레이어 리스폰 요청
    RespawnPlayer { uid: UserId },

    /// 게임 종료
    GameOver { winner: Team },
}
