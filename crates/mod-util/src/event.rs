use std::io;

use mod_network::RawPacket;



/// 애플리케이션의 커스텀 이벤트 목록입니다.
#[derive(Debug)]
pub enum AppEvent {
    /// 서버 연결이 종료됬을 때 전달되는 이벤트입니다.
    ClosedConnection, 

    /// 패킷을 수신했을 때 전달되는 이벤트입니다.
    PacketReceived(RawPacket), 

    /// 네트워크 입/출력 오류가 발생했을 때 전달되는 이벤트입니다.
    NetworkIOError(io::Error), 
}
