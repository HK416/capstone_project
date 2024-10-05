use std::io;

use mod_network::RawPacket;

use crate::scene::GameSceneFlow;



/// 애플리케이션의 커스텀 이벤트 목록입니다.
#[derive(Debug)]
pub enum AppEvent {
    /// 게임 장면 흐름을 설정합니다.
    SetGameSceneFlow(GameSceneFlow), 

    /// 서버 연결이 끊어졌을 때 전달되는 이벤트입니다.
    ClosedSocket, 

    /// 패킷을 수신했을 때 전달되는 이벤트입니다.
    PacketReceived(RawPacket), 

    /// 네트워크 입/출력 오류가 발생했을 떄 전달되는 이벤트입니다.
    NetworkIOError(io::Error), 
}
