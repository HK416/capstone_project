use std::{
    collections::VecDeque,
    io::{self, BufReader, BufWriter, ErrorKind, Read, Write},
    net::{SocketAddr, TcpStream, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering as MemOrdering},
        Arc,
    },
    time::{Duration, Instant},
};

use ahash::RandomState;
use dashmap::DashMap;
use mod_network::protocol::{PacketParser, PacketType, RawPacket};
use parking_lot::{Condvar, Mutex};
use winit::event_loop::EventLoopProxy;

use crate::etc::AppEvent;

/// 네트워크 오류 목록입니다.
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    /// 연결이 끊어졌을 때 발생하는 오류입니다.
    #[error("network connection was lost. (ADDR:{0:?})")]
    ClosedSocket(IpAddress),
    /// 입출력 오류입니다.
    #[error("socket I/O failed. (REASON:{0})")]
    IO(io::Error),
}

/// Socket Address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IpAddress {
    Tcp(SocketAddr),
    Udp { port: u16, remote: SocketAddr },
}

impl IpAddress {
    /// IP 주소를 가져옵니다.
    pub fn target_addr(&self) -> &SocketAddr {
        match self {
            IpAddress::Tcp(addr) => addr,
            IpAddress::Udp { remote, .. } => remote,
        }
    }
}

/// Network Socket
#[derive(Debug)]
enum Socket {
    Tcp(TcpStream, SocketAddr),
    Udp(UdpSocket, SocketAddr),
}

impl Socket {
    /// 수신한 데이터를 읽습니다.
    pub fn recv_stream(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        match self {
            Socket::Tcp(stream, addr) => {
                let mut reader = BufReader::new(stream);
                reader.read(buf).map(|n| (n, *addr))
            }
            Socket::Udp(socket, _) => socket.recv_from(buf),
        }
    }

    /// 데이터를 전송합니다.
    pub fn send_stream(&self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Socket::Tcp(stream, _) => {
                let mut writer = BufWriter::new(stream);
                writer.write(buf)
            }
            Socket::Udp(socket, addr) => socket.send_to(buf, addr),
        }
    }
}

/// ## Socket Status
#[derive(Debug)]
pub struct SocketStatus {
    address: IpAddress,
    socket: Socket,
    cvar: Condvar,
    queue: Mutex<VecDeque<RawPacket>>,
    is_connected: AtomicBool,
}

impl SocketStatus {
    /// 소켓이 연결되었는지 여부를 반환합니다.
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(MemOrdering::Relaxed)
    }

    /// 전송할 패킷을 추가합니다.
    pub fn push_packet(&self, packet: RawPacket) {
        let mut queue = self.queue.lock();
        queue.push_back(packet);
        self.cvar.notify_one();
    }
}

/// ## Network Manager (Inner)
#[derive(Debug)]
struct NetManagerInner {
    event_loop_proxy: Arc<EventLoopProxy<AppEvent>>,
    sockets: DashMap<IpAddress, Arc<SocketStatus>, RandomState>,
}

/// ## Network Manager
#[derive(Debug, Clone)]
pub struct NetManager(Arc<NetManagerInner>);

impl NetManager {
    /// 새로운 네트워크 매니저를 생성합니다.
    pub fn new(event_loop_proxy: Arc<EventLoopProxy<AppEvent>>) -> Self {
        Self(Arc::new(NetManagerInner {
            event_loop_proxy,
            sockets: DashMap::default(),
        }))
    }

    /// 주어진 IP 주소로 소켓을 생성하고 연결합니다.
    ///
    /// ※ 현재는 TCP 소켓만 구현됐습니다.
    ///
    /// 소켓을 연결하는 도중 오류가 발생한 경우 [`std::io::Error`]를 반환합니다.
    ///
    pub fn connect(&self, address: &IpAddress) -> io::Result<Arc<SocketStatus>> {
        match address {
            IpAddress::Tcp(addr) => {
                // TCP 스트림을 생성합니다.
                let stream = TcpStream::connect_timeout(addr, Duration::from_secs(5))?;
                stream.set_nonblocking(true)?;

                let status = Arc::new(SocketStatus {
                    address: address.clone(),
                    socket: Socket::Tcp(stream, *addr),
                    cvar: Condvar::new(),
                    queue: Mutex::new(VecDeque::new()),
                    is_connected: AtomicBool::new(true),
                });

                let status_cloned = status.clone();
                let event_loop_proxy_cloned = self.0.event_loop_proxy.clone();
                std::thread::spawn(|| {
                    tcp_packet_receive_loop(event_loop_proxy_cloned, status_cloned)
                });

                let status_cloned = status.clone();
                let event_loop_proxy_cloned = self.0.event_loop_proxy.clone();
                std::thread::spawn(|| tcp_packet_send_loop(event_loop_proxy_cloned, status_cloned));

                let status_cloned = status.clone();
                self.0.sockets.insert(address.clone(), status);

                Ok(status_cloned)
            }
            IpAddress::Udp { port, remote } => {
                // UDP 소켓을 생성합니다.
                let socket = if cfg!(feature = "dev") {
                    util::get_udp_socket(*port)
                } else {
                    UdpSocket::bind(remote)
                }?;
                let status = Arc::new(SocketStatus {
                    address: address.clone(),
                    socket: Socket::Udp(socket, *remote),
                    cvar: Condvar::new(),
                    queue: Mutex::new(VecDeque::new()),
                    is_connected: AtomicBool::new(true),
                });

                let status_cloned = status.clone();
                let event_loop_proxy_cloned = self.0.event_loop_proxy.clone();
                std::thread::spawn(|| {
                    udp_packet_receive_loop(event_loop_proxy_cloned, status_cloned)
                });

                let status_cloned = status.clone();
                self.0.sockets.insert(address.clone(), status);

                Ok(status_cloned)
            }
        }
    }

    /// 주어진 IP 주소에 해당하는 소켓 상태를 가져옵니다.  
    /// 해당 소켓 상태가 존재하지 않는 경우 `None`을 반환합니다.
    pub fn get(&self, address: &IpAddress) -> Option<Arc<SocketStatus>> {
        self.0.sockets.get(address).map(|status| status.clone())
    }

    /// 주어진 IP 주소에 해당하는 소켓을 연결해제 합니다.  
    /// 해당 소켓 상태가 존재하지 않는 경우 아무 동작도 하지 않습니다.
    pub fn disconnect(&self, address: &IpAddress) {
        if let Some((_, status)) = self.0.sockets.remove(address) {
            status.is_connected.store(false, MemOrdering::Release);
            status.cvar.notify_all();
        }
    }
}

/// TCP 패킷을 보내는 루프 함수입니다.
fn tcp_packet_send_loop(
    event_loop_proxy: Arc<EventLoopProxy<AppEvent>>,
    status: Arc<SocketStatus>,
) {
    loop {
        let mut queue = status.queue.lock();
        if status.is_connected() {
            status.cvar.wait(&mut queue);
        }

        if !status.is_connected() {
            break;
        }

        if let Some(raw_packet) = queue.pop_front() {
            let bytes = raw_packet.as_bytes();
            let mut buffer = bytes.as_slice();
            while !buffer.is_empty() && status.is_connected() {
                let result = status.socket.send_stream(&buffer);
                match result {
                    Ok(0) => {
                        status.is_connected.store(false, MemOrdering::Release);
                        let error = NetworkError::ClosedSocket(status.address);
                        let event = AppEvent::NetworkError(error);
                        event_loop_proxy.send_event(event).unwrap();
                        return;
                    }
                    Ok(n) => buffer = &buffer[n..],
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(ref e) if e.kind() == ErrorKind::ConnectionAborted => {
                        status.is_connected.store(false, MemOrdering::Release);
                        let error = NetworkError::ClosedSocket(status.address);
                        let event = AppEvent::NetworkError(error);
                        event_loop_proxy.send_event(event).unwrap();
                        return;
                    }
                    Err(ref e) if e.kind() == ErrorKind::ConnectionReset => {
                        status.is_connected.store(false, MemOrdering::Release);
                        let error = NetworkError::ClosedSocket(status.address);
                        let event = AppEvent::NetworkError(error);
                        event_loop_proxy.send_event(event).unwrap();
                        return;
                    }
                    Err(e) => {
                        status.is_connected.store(false, MemOrdering::Release);
                        let error = NetworkError::IO(e);
                        let event = AppEvent::NetworkError(error);
                        event_loop_proxy.send_event(event).unwrap();
                        return;
                    }
                }
            }
        }
    }
}

/// TCP 패킷을 받는 루프 함수입니다.
fn tcp_packet_receive_loop(
    event_loop_proxy: Arc<EventLoopProxy<AppEvent>>,
    status: Arc<SocketStatus>,
) {
    // 받은 패킷을 구문 분석하는 구문 분석기입니다.
    // 구문 분석한 패킷은 EventLoopProxy를 통해 애플리케이션 이벤트 루프로 전송됩니다.
    //
    let mut parser = PacketParser::new();
    let mut buffer = [0; 40960]; // 40KB

    while status.is_connected() {
        // 버퍼를 초기화 합니다.
        buffer[..].fill(0);

        // 수신받은 데이터를 읽습니다.
        let result = status.socket.recv_stream(&mut buffer);
        match result {
            Ok((0, _)) => {
                status.is_connected.store(false, MemOrdering::Release);
                let error = NetworkError::ClosedSocket(status.address);
                let event = AppEvent::NetworkError(error);
                event_loop_proxy.send_event(event).unwrap();
                return;
            }
            Ok((n, addr)) => {
                log::debug!("received tcp packet data (SIZE:{})", n);
                if addr == *status.address.target_addr() {
                    parser.push(&buffer[..n])
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(ref e) if e.kind() == ErrorKind::ConnectionAborted => {
                status.is_connected.store(false, MemOrdering::Release);
                let error = NetworkError::ClosedSocket(status.address);
                let event = AppEvent::NetworkError(error);
                event_loop_proxy.send_event(event).unwrap();
                return;
            }
            Err(ref e) if e.kind() == ErrorKind::ConnectionReset => {
                status.is_connected.store(false, MemOrdering::Release);
                let error = NetworkError::ClosedSocket(status.address);
                let event = AppEvent::NetworkError(error);
                event_loop_proxy.send_event(event).unwrap();
                return;
            }
            Err(e) => {
                status.is_connected.store(false, MemOrdering::Release);
                let error = NetworkError::IO(e);
                let event = AppEvent::NetworkError(error);
                event_loop_proxy.send_event(event).unwrap();
                return;
            }
        };

        while let Some(packet) = parser.pop() {
            if packet.packet_type() == PacketType::Ping {
                let mut guard = status.queue.lock();
                guard.push_back(packet);
                status.cvar.notify_one();
                continue;
            }

            if event_loop_proxy
                .send_event(AppEvent::PacketReceived(Instant::now(), packet))
                .is_err()
            {
                status.is_connected.store(false, MemOrdering::Release);
                return; // 애플리케이션 이벤트 루프가 종료되었을 경우(애플리케이션의 종료) 루프 함수를 빠져나옵니다.
            }
        }
    }
}

/// UDP 패킷을 수신하는 루프 함수입니다.
fn udp_packet_receive_loop(
    event_loop_proxy: Arc<EventLoopProxy<AppEvent>>,
    status: Arc<SocketStatus>,
) {
    // UDP 패킷의 크기는 1KB를 넘지 않습니다.
    let mut buffer = [0; 1024]; // 1KB

    while status.is_connected() {
        // 버퍼를 초기화 합니다.
        buffer[..].fill(0);

        // 수신받은 데이터를 읽습니다.
        let result = status.socket.recv_stream(&mut buffer);
        match result {
            Ok((0, _)) => {
                status.is_connected.store(false, MemOrdering::Release);
                let error = NetworkError::ClosedSocket(status.address);
                let event = AppEvent::NetworkError(error);
                event_loop_proxy.send_event(event).unwrap();
                return;
            }
            Ok((n, addr)) => {
                log::debug!("received udp packet data (SIZE:{})", n);
                let received_data = &buffer[0..n];
                if addr == *status.address.target_addr() {
                    // 바이트 배열을 RawPacket으로 변환한다.
                    // 이때, RawPacket으로 변환에 실패한 경우 해당 데이터를 버린다.
                    // (UDP로 보낸 패킷 데이터는 중요하지 않고, 1KB보다 작은 데이터이기 때문)
                    //
                    match RawPacket::try_from_bytes(received_data) {
                        Ok(packet) => {
                            event_loop_proxy
                                .send_event(AppEvent::PacketReceived(Instant::now(), packet))
                                .unwrap();
                        }
                        Err(e) => {
                            log::warn!("packet ignored >> failed to parse packet from {addr}: {e}");
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                continue;
            }
            Err(e) => {
                status.is_connected.store(false, MemOrdering::Release);
                let error = NetworkError::IO(e);
                let event = AppEvent::NetworkError(error);
                event_loop_proxy.send_event(event).unwrap();
                return;
            }
        };
    }
}

#[cfg(feature = "dev")]
mod util {
    use std::{
        io,
        net::{IpAddr, Ipv6Addr, SocketAddr, UdpSocket},
    };

    use socket2::{Domain, SockAddr, Socket, Type};

    /// 테스트를 위해 같은 포트를 재사용할 수 있는 옵션을 추가한 소켓을 생성합니다.
    ///
    pub fn get_udp_socket(port: u16) -> io::Result<UdpSocket> {
        let socket = Socket::new(Domain::IPV6, Type::DGRAM, None)?;
        socket.set_reuse_address(true)?;

        // 포트 번호 재사용 옵션
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        socket.set_reuse_port(true)?;

        // 소켓 주소 재사용 옵션
        let local_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port);
        socket.bind(&SockAddr::from(local_addr))?;

        Ok(socket.into())
    }
}
