use std::{
    collections::HashMap,
    env,
    io::{ErrorKind, Write},
    process::exit,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering}, Arc
    },
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt}, 
    net::TcpStream, 
    time::Duration, 
    select, 
};

use mod_network::{
    addr::Addr,
    components::{
        InputEvent, InputKind, InputSnapshot, LoginToken, SelectResult, UserId,
    },
    protocol::{
        CharacterSelectRequestPacket, CharacterSelectResponsePacket, InGameInputPacket, InGamePullPacket, InGameReadyNotifyPacket, InGameReadyStatusPacket, LoginFailedPacket, LoginRequestPacket, LoginSuccessPacket, MatchRequestPacket, MatchRequestRejectedPacket, Packet, PacketParser, PacketType
    },
};

lazy_static::lazy_static! {
    /// milliseconds
    static ref GLOBAL_DELAY: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));

    /// 서버에 접속한 클라이언트 수
    static ref NUM_CLIENTS: Arc<AtomicU16> = Arc::new(AtomicU16::new(0));

}

struct ReadyClient {
    uid: UserId,
    token: LoginToken,

    reader: tokio::net::tcp::OwnedReadHalf,
    writer: tokio::net::tcp::OwnedWriteHalf,
    packet_parser: PacketParser,
}

impl ReadyClient {
    fn new(uid: UserId, stream: tokio::net::TcpStream) -> Self {
        let (reader, writer) = stream.into_split();

        Self {
            uid,
            token: LoginToken::default(),

            reader,
            writer,
            packet_parser: PacketParser::new(),
        }
    }

    async fn run(mut self) -> Result<Self, std::io::Error> {
        // 일단 기본값으로 로그인. 나중에 DB에서 불러온 정보로 로그인하도록 해야함
        self.login().await?;

        // 방 접속 시도
        loop {
            // 방 접속
            match self.queue().await {
                Ok(_) => break,
                Err(ref e) if e.kind() == ErrorKind::NotConnected => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        // 게임 시작 준비
        loop {
            // 캐릭터 선택
            match self.select_character().await {
                Ok(_) => break,
                Err(e) if e.to_string() == "Game stopped" => continue,
                Err(e) => return Err(e),
            }
        }

        // sync
        self.sync().await?;

        Ok(self)
    }

    async fn read(&mut self) -> Result<(), std::io::Error> {
        let mut buf = [0; 32];

        match self.reader.read(&mut buf).await {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    format!(
                        "ReadyClient({})::read - Connection closed by server",
                        self.uid,
                    ),
                ));
            }
            Ok(n) => {
                self.packet_parser.push(&buf[..n]);
                return Ok(());
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    async fn login(&mut self) -> Result<(), std::io::Error> {
        let packet = LoginRequestPacket::new(self.uid).as_raw();
        self.writer.write_all(&packet.as_bytes()).await?;

        'readloop: loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                match packet.packet_type() {
                    PacketType::LoginSuccess => {
                        let p = LoginSuccessPacket::from_raw(packet);
                        // self.uid = p.uid;
                        self.token = p.token;
                        break 'readloop;
                    }
                    PacketType::LoginFailed => {
                        eprintln!("\n\nLogin failed: {:?}", LoginFailedPacket::from_raw(packet));
                        exit(1);
                        // return Err(std::io::Error::new(
                        //     std::io::ErrorKind::NotFound,
                        //     format!("Login failed - {:?} not found", self.uid),
                        // ));
                    }
                    PacketType::Ping => {
                        self.writer.write_all(&packet.as_bytes()).await?;
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "Login failed - Invalid packet received: {:?}",
                                packet.packet_type()
                            ),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    async fn queue(&mut self) -> Result<(), std::io::Error> {
        let packet = MatchRequestPacket::new(self.uid, self.token).as_raw();
        self.writer.write_all(&packet.as_bytes()).await?;

        'readloop: loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                match packet.packet_type() {
                    PacketType::MatchRequestRejected => {
                        let p = MatchRequestRejectedPacket::from_raw(packet);
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::NotConnected,
                            format!("Match request rejected: {:?}", p.reason),
                        ));
                    }
                    PacketType::FormationDataInit => {
                        break 'readloop;
                    }
                    PacketType::Ping => {
                        self.writer.write_all(&packet.as_bytes()).await?;
                    }
                    PacketType::LobbyDataUpdate => { }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "Match request failed - Invalid packet received: {:?}",
                                packet.packet_type()
                            ),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    async fn select_character(&mut self) -> Result<(), std::io::Error> {
        let mut select_packet_sent = false;

        // 캐릭터 선택 완료 대기
        'readloop: loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                match packet.packet_type() {
                    // 캐릭터 선택 완료 패킷
                    PacketType::FormationDataUpdate => {
                        // let _p = FormationPullPacket::from_raw(packet);
                        if !select_packet_sent {
                            // 캐릭터 선택 패킷 전송
                            let select_packet = CharacterSelectRequestPacket::new(
                                self.uid, self.token, rand::random()
                            ).as_raw().as_bytes();
                            self.writer.write_all(&select_packet).await?;
                            select_packet_sent = true;
                            continue 'readloop;
                        }
                    }
                    PacketType::CharacterSelectResponse => {
                        let p = CharacterSelectResponsePacket::from_raw(packet);
                        if p.result == SelectResult::Success {
                            // println!("Character selected successfully");
                        } else {
                            println!("Character selection failed: {:?}", p.result);
                            select_packet_sent = false;
                        }
                    }
                    PacketType::InGameDataInit => {
                        // let _p = InitStagePacket::from_raw(packet);
                        // println!("Stage initialized");
                        break 'readloop;
                    }
                    PacketType::EnterGameFailed => {
                        // let _p = CustomGamePullPacket::from_raw(packet);
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "Game stopped",
                        ));
                    }
                    PacketType::Ping => {
                        self.writer.write_all(&packet.as_bytes()).await?;
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "Character selection failed - Invalid packet received: {:?}",
                                packet.packet_type()
                            ),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    async fn sync(&mut self) -> Result<(), std::io::Error> {
        let mut ready_packet_sent = false;

        'readloop: loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                match packet.packet_type() {
                    PacketType::InGameEnterNotify => {
                        // let _p = PrepareStagePacket::from_raw(packet);
                        // println!("Stage prepared");
                        // break 'readloop;
                    }
                    PacketType::InGamePull => {
                        // let _p = PullStagePacket::from_raw(packet);
                        // println!("Stage pulled");
                        break 'readloop;
                    }
                    PacketType::Ping => {
                        self.writer.write_all(&packet.as_bytes()).await?;
                    }
                    PacketType::InGameReadyStatus => { 
                        if !ready_packet_sent {
                            let packet = InGameReadyNotifyPacket::new(self.uid, self.token).as_raw();
                            self.writer.write_all(&packet.as_bytes()).await?;
                            ready_packet_sent = true;
                        }
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "Sync failed - Invalid packet received: {:?}",
                                packet.packet_type()
                            ),
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

struct InGameClient {
    uid: UserId, 
    token: LoginToken, 
    
    game_time: u32,
    input: InputKind, 
    
    reader: tokio::net::tcp::OwnedReadHalf, 
    writer: tokio::net::tcp::OwnedWriteHalf, 
}

impl InGameClient {
    fn new(
        uid: UserId,
        token: LoginToken,
        reader: tokio::net::tcp::OwnedReadHalf,
        writer: tokio::net::tcp::OwnedWriteHalf,
    ) -> Self {
        Self { 
            uid, 
            token, 
            game_time: 0, 
            input: InputKind::Backward,
            reader, 
            writer, 
        }
    }

    async fn run(&mut self) -> Result<(), std::io::Error> {
        let mut parser = PacketParser::new();
        let mut buf = [0; 1024];
        let mut attack_count = 0;
        let mut interval = tokio::time::interval(Duration::from_millis(200));
        interval.tick().await;

        loop {
            select! {
                // 패킷 수신 및 처리
                read_result = self.reader.read(&mut buf) => {
                    match read_result {
                        Ok(0) => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                "Connection closed by server",
                            ));
                        }
                        Ok(n) => {
                            parser.push(&buf[..n]);
                            buf[..].fill(0);
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }

                    self.process_packet(&mut parser, &mut attack_count).await?;
                }
                // 일정 시간마다 이동 패킷 전송
                _ = interval.tick() => {
                    if self.game_time == 0 {
                        continue;
                    }
                    let snapshot = InputSnapshot::KeyEvent {
                        play_elapsed_time_ms: self.game_time,
                        events: if self.input == InputKind::Forward {
                            self.input = InputKind::Backward;
                            vec![
                                InputEvent::KeyRelease(InputKind::Forward),
                                InputEvent::KeyPress(InputKind::Backward),
                            ]
                        } else {
                            self.input = InputKind::Forward;
                            vec![
                                InputEvent::KeyRelease(InputKind::Backward),
                                InputEvent::KeyPress(InputKind::Forward),
                            ]
                        },
                    };
                    let packet = InGameInputPacket::new(
                        self.uid,
                        self.token,
                        self.game_time,
                        vec![snapshot],
                    );
                    self.writer.write_all(&packet.as_raw().as_bytes()).await?;
                }
            }
        }
    }

    async fn process_packet(
        &mut self, 
        parser: &mut PacketParser, 
        attack_count: &mut u32
    ) -> Result<(), std::io::Error> {
        while let Some(packet) = parser.pop() {
            if packet.packet_type() == PacketType::Ping {
                // println!("Received ping packet");
                self.writer.write_all(&packet.as_bytes()).await?;
                continue;
            }
            
            // 여기서 Client의 상태에 따라 패킷 처리를 다르게 해야함
            match packet.packet_type() {
                PacketType::InGamePull => {
                    let p = InGamePullPacket::from_raw(packet);
                    let global_delay = GLOBAL_DELAY.load(Ordering::Relaxed) as u16;
                    if p.ping > global_delay {
                        GLOBAL_DELAY.fetch_add(1, Ordering::Relaxed);
                    } else if p.ping < global_delay {
                        GLOBAL_DELAY.fetch_sub(1, Ordering::Relaxed);
                    }
                    self.game_time = p.play_elapsed_time_ms;

                    let snapshot = if *attack_count != 0 {
                        InputSnapshot::KeyEvent {
                            play_elapsed_time_ms: self.game_time,
                            events: vec![InputEvent::KeyPress(InputKind::Attack)],
                        }
                    } else {
                        InputSnapshot::KeyEvent {
                            play_elapsed_time_ms: self.game_time,
                            events: vec![
                                InputEvent::KeyRelease(InputKind::Attack),
                                InputEvent::KeyPress(InputKind::Reload),
                                InputEvent::KeyRelease(InputKind::Reload),
                            ],
                        }
                    };
                    let packet = InGameInputPacket::new(
                        self.uid,
                        self.token,
                        self.game_time,
                        vec![snapshot],
                    );
                    self.writer.write_all(&packet.as_raw().as_bytes()).await?;
                    *attack_count += 1;
                }
                PacketType::InGameStatus => {}
                PacketType::InGameFinish => {
                    return Ok(());
                }
                _ => {
                    // println!("InGame - Invalid packet received: {:?}", packet.packet_type());
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "InGame - Invalid packet received: {:?}",
                            packet
                        ),
                    ));
                }
            }
        }

        Ok(())
    }
}

struct Client {
    connected: AtomicBool,
}

impl Client {
    async fn new() -> Result<Client, std::io::Error> {
        Ok(Self {
            connected: AtomicBool::new(true),
        })
    }

    async fn run(&self, uid: UserId, addr: &str) {
        let stream = TcpStream::connect(addr).await.unwrap();
        NUM_CLIENTS.fetch_add(1, Ordering::Relaxed);

        let ready_client = ReadyClient::new(uid, stream);
        let ready_client = match ready_client.run().await {
            Ok(client) => client,
            Err(e) => {
                self.connected.store(false, Ordering::Release);
                println!("Client ready failed: {:?}", e);
                return;
            }
        };

        // 게임 시작
        let mut client = InGameClient::new(
            ready_client.uid,
            ready_client.token,
            ready_client.reader,
            ready_client.writer,
        );
        match client.run().await {
            Ok(()) => {}
            Err(e) => {
                println!("Client run failed: {:?}", e);
            }
        }

        self.connected.store(false, Ordering::Release);
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        NUM_CLIENTS.fetch_sub(1, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
enum AcceptState {
    Accept,
    Stable,
    Disconnect,
}

impl AcceptState {
    fn from(value: u32) -> Self {
        match value {
            0 => AcceptState::Accept,
            1 => AcceptState::Stable,
            2 => AcceptState::Disconnect,
            _ => panic!("Invalid AcceptState value"),
        }
    }
}

const IDLE_DELAY: u32 = 10; // ms
const STABLE_DELAY: u32 = 50; // ms
const DELAY_LIMIT1: u32 = 100; // ms
const DELAY_LIMIT2: u32 = 150; // ms

fn get_next_uid() -> UserId {
    static NEXT_UID: AtomicU32 = AtomicU32::new(1);
    UserId::new(NEXT_UID.fetch_add(1, Ordering::Relaxed))
}

async fn stress_test(addr: Addr, max_clients: usize) {
    let accept_state = Arc::new(AtomicU32::new(AcceptState::Accept as u32));
    let state = Arc::clone(&accept_state);

    let _printer_handle = tokio::spawn(async move {
        loop {
            print!(
                "\rDelay: {:?}ms  \tClients: {}  \t\t{:?}          ",
                GLOBAL_DELAY.load(Ordering::Relaxed),
                NUM_CLIENTS.load(Ordering::Relaxed),
                AcceptState::from(state.load(Ordering::Relaxed))
            );
            std::io::stdout().flush().unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    });

    let mut accept_delay = 100; // ms

    let mut clients = HashMap::with_capacity(max_clients);
    let mut client_id = 0;
    loop {
        let delay = GLOBAL_DELAY.load(Ordering::Relaxed);
        let state = accept_state.load(Ordering::Relaxed);

        match AcceptState::from(state) {
            AcceptState::Accept => {
                if delay > DELAY_LIMIT1 {
                    accept_state.store(AcceptState::Disconnect as u32, Ordering::Relaxed);
                } else {
                    // accept
                    if NUM_CLIENTS.load(Ordering::Relaxed) < max_clients as u16 {
                        let client = Arc::new(Client::new().await.unwrap());
                        clients.insert(client_id, Arc::clone(&client));
                        client_id += 1;
                        let a = addr.to_string();
                        tokio::spawn(async move { client.run(get_next_uid(), &a).await });
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(accept_delay)).await;
                }
            }
            AcceptState::Stable => {
                if delay > STABLE_DELAY {
                    accept_state.store(AcceptState::Disconnect as u32, Ordering::Relaxed);
                } else {
                    if delay <= IDLE_DELAY {
                        accept_state.store(AcceptState::Accept as u32, Ordering::Relaxed);
                        accept_delay += 100;
                    } else {
                        // do nothing
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
            AcceptState::Disconnect => {
                if delay <= STABLE_DELAY {
                    accept_state.store(AcceptState::Stable as u32, Ordering::Relaxed);
                } else {
                    let disconnect_delay = if delay <= DELAY_LIMIT1 {
                        1000
                    } else if delay <= DELAY_LIMIT2 {
                        1000
                    } else {
                        500
                    };

                    // 연결 끊기
                    client_id -= 1;
                    if let Some(back) = clients.get(&client_id) {
                        back.connected.store(false, Ordering::Relaxed);
                        clients.remove(&client_id);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(disconnect_delay)).await;
                }
            }
        }

        clients.retain(|_, client| client.connected.load(Ordering::Relaxed));
    }

    // printer_handle.abort();

    // println!("Stress test finished");

    // Ok(())
}

/// 서버에 새로운 계정들을 생성합니다.
#[allow(dead_code)]
#[tokio::main]
async fn create_dummy_accounts(num: usize, addr: &str) {
    println!("Creating {} dummy accounts...", num);
    
    static SUCCESS: AtomicU32 = AtomicU32::new(0);
    static FAILURE: AtomicU32 = AtomicU32::new(0);

    let mut readers = Vec::with_capacity(num);
    let mut writers = Vec::with_capacity(num);
    for _ in 0..num {
        let stream = TcpStream::connect(addr).await
            .expect("Failed to connect to server");
        let (reader, writer) = stream.into_split();
        readers.push(reader);
        writers.push(writer);
    }
    
    let packet = LoginRequestPacket::new(UserId::NULL).as_raw();
    let mut write_handles = Vec::with_capacity(num);
    
    // 모든 write 작업을 먼저 시작
    for mut writer in writers.into_iter() {
        let p = packet.clone();
        let handle = tokio::spawn(async move {
            writer.write_all(&p.as_bytes()).await.unwrap();
            writer // writer를 반환하여 연결 유지
        });
        write_handles.push(handle);
    }

    let mut read_handles = Vec::with_capacity(num);
    readers.into_iter().for_each(|mut reader| {
        let handle = tokio::spawn(async move {
            let mut parser = PacketParser::new();
            
            let mut buf = [0; 1024];

            'readloop: loop {
                match reader.read(&mut buf).await {
                    Ok(0) => {
                        eprintln!("Connection closed by server");
                        FAILURE.fetch_add(1, Ordering::Relaxed);
                        break 'readloop;
                    }
                    Ok(n) => {
                        parser.push(&buf[..n]);
                    }
                    Err(e) => {
                        eprintln!("Failed to read from stream: {}", e);
                        FAILURE.fetch_add(1, Ordering::Relaxed);
                        break 'readloop;
                    }
                }

                while let Some(packet) = parser.pop() {
                    match packet.packet_type() {
                        PacketType::LoginSuccess => {
                            SUCCESS.fetch_add(1, Ordering::Relaxed);
                            break 'readloop;
                        }
                        PacketType::Ping => {
                            // Ping 패킷은 무시하고 계속 읽기
                            continue;
                        }
                        _ => {
                            eprintln!("Unexpected packet type: {:?}", packet.packet_type());
                            FAILURE.fetch_add(1, Ordering::Relaxed);
                            break 'readloop;
                        }
                    }
                }
            }
        });
        read_handles.push(handle);
    });

    let _write_completion = futures::future::join_all(write_handles).await;
    let _read_completion = futures::future::join_all(read_handles).await;

    assert_eq!(
        SUCCESS.load(Ordering::Relaxed) + FAILURE.load(Ordering::Relaxed),
        num as u32,
        "Some accounts were not processed correctly"
    );

    println!("created {} dummy accounts", SUCCESS.load(Ordering::Relaxed));
}

fn main() {
    let mut args = env::args();
    args.next();
    let mut addr = Addr::default();
    let mut max_clients = 200;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--set-addr" => {
                if let Some(addr_str) = args.next() {
                    addr = match Addr::from_str(&addr_str) {
                        Ok(addr) => addr,
                        Err(e) => {
                            eprintln!(
                                "명령줄 인자 형식이 잘못되었습니다.\n  `--set-addr` - 잘못된 주소 형식입니다.\n{}",
                                e
                            );
                            return;
                        }
                    }
                }
            }
            "--max-clients" => {
                if let Some(max_clients_str) = args.next() {
                    max_clients = match max_clients_str.parse::<usize>() {
                        Ok(n) => n,
                        Err(e) => {
                            eprintln!(
                                "명령줄 인자 형식이 잘못되었습니다.\n  `--max-clients` - 잘못된 숫자 형식입니다.\n{}",
                                e
                            );
                            return;
                        }
                    };
                }
            }
            _ => {
                eprintln!("Invalid option: {}", arg);
                return;
            }
        }
    }

    // {
    //     create_dummy_accounts(1000, addr.to_string().as_str());
    //     return;
    // }

    println!("Stress test started");

    let num_core = num_cpus::get();
    let num_threads = num_core / 4;
    println!("Using {} threads", num_threads);

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_threads)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { stress_test(addr, max_clients).await });
}
