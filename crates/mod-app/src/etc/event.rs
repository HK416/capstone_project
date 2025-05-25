use mod_network::protocol::RawPacket;

use crate::{error::Alert, net::NetworkError, scene::GameSceneFlow};

use super::WindowSize;

/// ## Application Custom Events
#[derive(Debug)]
pub enum AppEvent {
    /// 게임 장면 흐름을 설정합니다.
    AddGameSceneFlow(GameSceneFlow),

    /// 애플리케이션 창의 크기를 조절합니다.
    ResizeRequest(WindowSize),

    /// 애플리케이션 창을 전체화면으로 변경합니다.
    FullScreenRequest(bool),

    /// 시스템 API를 사용해 오류 메시지 Dialog를 화면에 출력합니다.
    Alert(Alert),

    /// 네트워크 오류가 발생했을 때 전달되는 이벤트입니다.
    NetworkError(NetworkError),

    /// 패킷을 수신했을 때 전달되는 이벤트입니다.
    PacketReceived(RawPacket),

    /// 마우스 커서를 비활성화 합니다.
    CursorDisable,

    /// 마우스 커서를 활성화 합니다.
    CursorEnable,
}
