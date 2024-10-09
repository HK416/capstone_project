use std::{
    io::{self, BufReader, BufWriter, ErrorKind, Read, Write}, 
    net::{SocketAddr, TcpStream, /* UdpSocket */}, 
    sync::{atomic::{AtomicBool, Ordering as MemOrdering}, Arc}, 
    time::Duration
};

use mod_network::{PacketParser, RawPacket};
use mod_parallelism::collections::{Queue, SkipMap};
use rayon::{ThreadPool, ThreadPoolBuildError, ThreadPoolBuilder};
use winit::event_loop::EventLoopProxy;

use crate::etc::AppEvent;





/// 네트워크 IP 주소입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IpAddress {
    Tcp(SocketAddr), 
    Udp(SocketAddr), 
}

impl IpAddress {
    /// IP 주소를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn ip_addr(&self) -> &SocketAddr {
        match self {
            IpAddress::Tcp(addr) => addr, 
            IpAddress::Udp(addr) => addr, 
        }
    }
}



/// 생성된 네트워크 소켓입니다.
#[derive(Debug)]
enum Socket {
    Tcp(TcpStream), 
    // Udp(UdpSocket)
}

impl Socket {
    /// 수신한 데이터를 읽습니다.
    pub fn recv_stream(&self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Socket::Tcp(stream) => {
                let mut reader = BufReader::new(stream);
                reader.read(buf)
            }, 
            // Socket::Udp(socket) => {
            //     socket.recv(buf)
            // }
        }
    }

    /// 데이터를 전송합니다.
    pub fn send_stream(&self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Socket::Tcp(stream) => {
                let mut writer = BufWriter::new(stream);
                writer.write(buf)
            }, 
            // Socket::Udp(socket) => {
            //     socket.send(buf)
            // }
        }
    }
}



/// 소켓의 상태 정보입니다.
#[derive(Debug)]
pub struct SocketStatus {
    /// 네트워크 IP 주소입니다.
    /// 
    /// 네트워크 재연결을 시도할 때 사용됩니다.
    /// 
    address: IpAddress, 

    /// 네트워크 소켓입니다.
    socket: Socket, 

    /// 전송할 패킷의 대기열입니다.
    queue: Queue<RawPacket>, 

    /// 현재 네트워크 연결 여부를 나타냅니다.
    is_connected: AtomicBool, 
}

impl SocketStatus {
    /// 소켓이 연결되었는지 여부를 반환합니다.
    #[inline]
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(MemOrdering::Relaxed)
    }

    /// 전송할 패킷을 추가합니다.
    pub fn push_packet(&self, packet: RawPacket) {
        self.queue.push(packet);
    }
}



#[derive(Debug)]
struct NetManagerInner {
    /// 애플리케이션 이벤트 루프 프록시입니다.
    /// 
    /// 수신된 네트워크 패킷을 이벤트 루프로 보낼 때 사용합니다.
    /// 
    event_loop_proxy: Arc<EventLoopProxy<AppEvent>>, 

    /// 네트워크 I/O 작업을 위한 풀 객체입니다.
    /// 
    /// 최소 2개 ~ 현재 시스템의 코어 개수 절반 만큼 스레드를 생성합니다.
    /// 
    thread_pool: ThreadPool, 

    /// 생성된 소켓 집합입니다.
    sockets: SkipMap<IpAddress, Arc<SocketStatus>>, 
}



#[derive(Debug, Clone)]
pub struct NetManager(Arc<NetManagerInner>);

impl NetManager {
    /// 새로운 네트워크 매니저를 생성합니다.
    #[must_use]
    pub fn new(
        num_threads: usize, 
        event_loop_proxy: Arc<EventLoopProxy<AppEvent>>
    ) -> Result<Self, ThreadPoolBuildError> {
        Ok(Self(NetManagerInner {
            event_loop_proxy, 
            thread_pool: ThreadPoolBuilder::new()
                .num_threads((num_threads / 2).max(2))
                .build()?, 
            sockets: SkipMap::new(), 
        }.into()))
    }


    /// 주어진 IP 주소로 소켓을 생성하고 연결합니다.
    /// 
    /// ※ 현재는 TCP 소켓만 구현됐습니다.
    /// 
    /// 소켓을 연결하는 도중 오류가 발생한 경우 [`std::io::Error`]를 반환합니다.
    /// 
    #[must_use]
    pub fn connect(&self, address: &IpAddress) -> io::Result<Arc<SocketStatus>> {
        // 소켓을 생성합니다.
        let status = match address {
            IpAddress::Tcp(addr) => {
                // TCP 스트림을 생성하고 연결합니다.
                let stream = TcpStream::connect_timeout(addr, Duration::from_secs(5))?;
                stream.set_nodelay(true)?;
                stream.set_nonblocking(true)?;

                Arc::new(SocketStatus {
                    address: address.clone(), 
                    socket: Socket::Tcp(stream), 
                    queue: Queue::new(), 
                    is_connected: AtomicBool::new(true)
                })
            }, 
            _ => todo!(), 
        };

        let status_cloned = status.clone();
        let event_loop_proxy_cloned = self.0.event_loop_proxy.clone();
        self.0.thread_pool.spawn(|| packet_receive_loop(event_loop_proxy_cloned, status_cloned));

        let status_cloned = status.clone();
        let event_loop_proxy_cloned = self.0.event_loop_proxy.clone();
        self.0.thread_pool.spawn(|| packet_send_loop(event_loop_proxy_cloned, status_cloned));

        let status_cloned = status.clone();
        self.0.sockets.insert(address.clone(), status);

        Ok(status_cloned)
    }


    /// 주어진 IP 주소에 해당하는 소켓 상태를 가져옵니다.
    /// 
    /// 해당 소켓 상태가 존재하지 않는 경우 `None`을 반환합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn get(&self, address: &IpAddress) -> Option<Arc<SocketStatus>> {
        self.0.sockets.get(address)
            .map(|guard| guard.clone())
    }


    /// 주어진 IP 주소에 해당하는 소켓을 연결해제 합니다.
    /// 
    /// 해당 소켓 상태가 존재하지 않는 경우 아무 동작도 하지 않습니다.
    /// 
    pub fn disconnect(&self, address: &IpAddress) {
        if let Some(status) = self.0.sockets.remove(address) {
            status.is_connected.store(false, MemOrdering::Release);
        }
    }
}



/// 패킷을 보내는 루프 함수입니다.
fn packet_send_loop(
    event_loop_proxy: Arc<EventLoopProxy<AppEvent>>, 
    status: Arc<SocketStatus>
) {
    while status.is_connected() {
        if let Some(raw_packet) = status.queue.pop() {
            let bytes = raw_packet.as_bytes();
            let mut buffer = bytes.as_slice();
            while !buffer.is_empty() && status.is_connected() {
                let result = status.socket.send_stream(&buffer);
                match result {
                    Ok(0) => {
                        // Safe: 이벤트 루프가 종료된 경우는 애플리케이션이 종료되었을 때 이다.
                        unsafe { event_loop_proxy.send_event(AppEvent::ClosedSocket(status.address)).unwrap_unchecked() };
                        status.is_connected.store(false, MemOrdering::Release);
                        break;
                    }, 
                    Ok(n) => { buffer = &buffer[n..] }, 
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => { /* continue */}, 
                    Err(e) => {
                        // Safe: 이벤트 루프가 종료된 경우는 애플리케이션이 종료되었을 때 이다.
                        unsafe { event_loop_proxy.send_event(AppEvent::IOError(e)).unwrap_unchecked() };
                        break;
                    }
                }
            }
        }
    }
}



/// 패킷을 받는 루프 함수입니다.
/// 
/// 루프 함수에서 오류가 발생한 경우 오류 메시지를 이벤트 루프로 전송합니다.
/// 
fn packet_receive_loop(
    event_loop_proxy: Arc<EventLoopProxy<AppEvent>>, 
    status: Arc<SocketStatus>
) {
    // 받은 패킷을 구문 분석하는 구문 분석기입니다.
    // 구문 분석한 패킷은 EventLoopProxy를 통해 애플리케이션 이벤트 루프로 전송됩니다.
    //
    let mut parser = PacketParser::new();
    let mut buffer = [0; 1024];
    
    while status.is_connected() {
        // 버퍼를 초기화 합니다.
        buffer[..].fill(0);

        // 수신받은 데이터를 읽습니다.
        let result = status.socket.recv_stream(&mut buffer);
        match result {
            Ok(0) => {
                // Safe: 이벤트 루프가 종료된 경우는 애플리케이션이 종료되었을 때 이다.
                unsafe { event_loop_proxy.send_event(AppEvent::ClosedSocket(status.address)).unwrap_unchecked() };
                status.is_connected.store(false, MemOrdering::Release);
                break;
            }, 
            Ok(n) => { parser.push(&buffer[..n]) }, 
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => { /* continue */ }, 
            Err(e) => {
                // Safe: 이벤트 루프가 종료된 경우는 애플리케이션이 종료되었을 때 이다.
                unsafe { event_loop_proxy.send_event(AppEvent::IOError(e)).unwrap_unchecked() };
                break;
            }
        };

        while let Some(raw_packet) = parser.pop() {
            if event_loop_proxy.send_event(AppEvent::PacketReceived(raw_packet)).is_err() {
                break; // 애플리케이션 이벤트 루프가 종료되었을 경우(애플리케이션의 종료) 루프 함수를 빠져나옵니다.
            }
        }
    }
}
