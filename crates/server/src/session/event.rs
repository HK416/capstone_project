use crate::session::SessionStateFlow;

/// 세션 이벤트 목록입니다.
#[derive(Debug)]
pub enum SessionEvents {
    SetControlFlow(SessionStateFlow),
    /// 캐릭터 편성 상태에 진입합니다.
    EnterFormation,
    /// 캐릭터 편성 상태에서 빠져나옵니다.
    ExitFormation,
    /// 인게임 동기화 상태에 진입합니다.
    EnterInGameSync,
    /// 인게임 상태에 진입합니다.
    EnterInGame,
    /// 인게임 상태에서 빠져나옵니다.
    ExitInGame,
}
