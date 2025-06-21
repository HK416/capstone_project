use std::sync::Arc;

use mod_network::components::{CharacterKind, ObjectId, UserId};

use crate::session::Session;

/// 게임 월드 이벤트 목록입니다.
#[derive(Debug)]
pub enum GameWorldEvent {
    /// 게임 시스템 이벤트
    System {
        session: Arc<Session>,
        uid: UserId,
        event: GameWorldSystemEvent,
    },

    /// 커스텀 게임 대기실 이벤트
    RoomState {
        session: Arc<Session>,
        uid: UserId,
        event: GameWorldRoomStateEvent,
    },

    /// 캐릭터 편성 이벤트
    FormationState {
        session: Arc<Session>,
        uid: UserId,
        event: GameWorldFormationStateEvent,
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
}

/// 게임 월드의 시스템 이벤트 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameWorldSystemEvent {
    /// 플레이어가 게임 월드에 참여할 때 발생되는 이벤트입니다.
    PlayerJoin,
    /// 플레이어가 게임 월드에서 떠날 때 발생되는 이벤트입니다.
    PlayerLeave,
}

/// 커스텀 게임 대기실 상태의 이벤트 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameWorldRoomStateEvent {
    /// 플레이어가 커스텀 게임 대기실에서 준비를 요청할 때 발생되는 이벤트입니다.
    Ready,
    /// 플레이어가 커스텀 게임 대기실에서 팀을 변경할 때 발생되는 이벤트입니다.
    ChangeTeam(UserId),
    /// 방장이 캐릭터 중복 옵션을 변경할 때 발생되는 이벤트입니다.
    ChangeDuplicateOption,
    /// 방장이 팀 균형 옵션을 변경할 때 발생되는 이벤트입니다.
    ChangeUnbalanceOption,
    /// 방장이 커스텀 게임 대기실에서 플레이어를 차단할 때 발생되는 이벤트입니다.
    PlayerBan(UserId),
}

/// 캐릭터 편성 상태의 이벤트 목록입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameWorldFormationStateEvent {
    /// 플레이어가 캐릭터 편성 장면에서 캐릭터를 선택할 때 발생되는 이벤트입니다.
    CharacterSelect(CharacterKind),
}
