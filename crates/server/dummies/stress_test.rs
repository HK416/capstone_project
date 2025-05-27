use std::{
    collections::HashMap,
    env,
    io::Write, 
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering},
        Arc, Mutex
    },
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use mod_network::{
    addr::Addr,
    components::{
        CharacterKind, Email, GameInputBits, 
        LatLon, LoginToken, Passwd, Permission, 
        SelectResult, UserAccount, UserId, 
        ViewState, ViewStateTimer, WorldId
    },
    protocol::{
        AvailableWorldsPacket, 
        CustomGameJoinFailedPacket, CustomGameJoinRequestPacket,
        CustomGamePullPacket, CustomGameReadyPacket, 
        FormationSelectPacket, FormationSelectResponsePacket,
        LoginRequestPacket, LoginSuccessPacket, 
        Packet, PacketParser, PacketType, 
        PingPacket, PushStatusPacket, 
        PushSyncPacket, RequestAvailableWorldsPacket
    }
};


lazy_static::lazy_static! {
    /// milliseconds
    static ref GLOBAL_DELAY: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));

    /// 서버에 접속한 클라이언트 수
    static ref NUM_CLIENTS: Arc<AtomicU16> = Arc::new(AtomicU16::new(0));

}


struct ReadyClient {
    account: UserAccount,
    token: LoginToken,
    
    reader: tokio::net::tcp::OwnedReadHalf,
    writer: tokio::net::tcp::OwnedWriteHalf,
    packet_parser: PacketParser,
}

impl ReadyClient {
    fn new(account: UserAccount, stream: tokio::net::TcpStream) -> Self {
        let (reader, writer) = stream.into_split();

        Self {
            account,
            token: LoginToken::default(),
            
            reader,
            writer,
            packet_parser: PacketParser::new(),
        }
    }

    async fn run(mut self) -> Result<Self, std::io::Error> {
        // 일단 기본값으로 로그인. 나중에 DB에서 불러온 정보로 로그인하도록 해야함
        self.login(Email::default(), Passwd::default()).await?;

        // 방 접속 시도 
        loop {
            // 접속 가능한 월드 목록 불러오기
            let available = self.request_available_worlds().await?;

            let world_id = if available.is_empty() {
                // 방 생성
                WorldId::NULL
            } else {
                // 랜덤으로 방 선택
                // *available.choose(&mut rand::rng()).unwrap()
                available[0]    // 접속가능한 첫번째 방으로 접속
            };

            // 방 접속
            match self.join(world_id).await {
                Ok(_) => break,
                Err(e) => return Err(e),
            }
        }

        // 게임 시작 준비
        loop {
            // 준비 신호 전송
            self.ready().await?;

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
                        self.account.uid,
                    )
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

    async fn login(&mut self, email: Email, passwd: Passwd) -> Result<(), std::io::Error> {
        let packet = LoginRequestPacket::new(email, passwd).as_raw();
        self.writer.write_all(&packet.as_bytes()).await?;

        'readloop: loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                match packet.packet_type() {
                    PacketType::LoginSuccess => {
                        let p = LoginSuccessPacket::from_raw(packet);
                        self.account = p.account;
                        self.token = p.token;
                        break 'readloop;
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData, 
                            format!(
                                "Login failed - Invalid packet received: {:?}", 
                                packet.packet_type()
                            )
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    async fn request_available_worlds(&mut self) -> Result<Vec<WorldId>, std::io::Error> {
        let packet = RequestAvailableWorldsPacket::new(self.account.uid, self.token).as_raw();
        self.writer.write_all(&packet.as_bytes()).await?;

        loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                match packet.packet_type() {
                    PacketType::AvailableWorlds => {
                        let p = AvailableWorldsPacket::from_raw(packet);
                        return Ok(p.worlds);
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData, 
                            format!(
                                "RequestAvailableWorlds failed - Invalid packet received: {:?}", 
                                packet.packet_type()
                            )
                        ));
                    }
                }
            }
        }
    }

    async fn join(&mut self, world_id: WorldId) -> Result<(), std::io::Error> {
        let packet = CustomGameJoinRequestPacket::new(world_id, self.account.uid, self.token).as_raw();
        self.writer.write_all(&packet.as_bytes()).await?;

        'readloop: loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                match packet.packet_type() {
                    PacketType::CustomGameJoinSuccess => {
                        // let _p = CustomGameJoinSuccessPacket::from_raw(packet);
                        break 'readloop;
                    }
                    PacketType::CustomGameJoinFailed => {
                        let p = CustomGameJoinFailedPacket::from_raw(packet);
                        // println!("Join failed: {:?}", p.reason);
                        // continue;
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::NotConnected,
                            format!("Join failed: {:?}", p.reason)
                        ));
                    }
                    // Success보다 먼저 도착할 수도?
                    PacketType::CustomGamePull => {
                        continue;
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData, 
                            format!(
                                "Join failed - Invalid packet received: {:?}", 
                                packet.packet_type()
                            )
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    async fn ready(&mut self) -> Result<(), std::io::Error> {
        let ready_packet = CustomGameReadyPacket::new(self.account.uid, self.token, true).as_raw();

        'writeloop: loop {
            self.writer.write_all(&ready_packet.as_bytes()).await?;

            'readloop: loop {
                self.read().await?;

                while let Some(packet) = self.packet_parser.pop() {
                    match packet.packet_type() {
                        // 플레이어들의 상태에 업데이트가 있다면 
                        PacketType::CustomGamePull => {
                            let p = CustomGamePullPacket::from_raw(packet);

                            if p.players.iter()
                                .find(|p| p.account.uid == self.account.uid)
                                .filter(|p| p.permission() == Permission::Admin)
                                .is_some() 
                            {
                                if p.players.iter()
                                    .filter(|p| p.account.uid != self.account.uid)
                                    .all(|p| p.is_ready())
                                {
                                    // 방장이면 게임 시작 신호(ready packet) 재전송
                                    continue 'writeloop;
                                }
                            } else {
                                // 방장이 아니면 게임 시작 신호 대기
                                continue 'readloop;
                            }
                        }
                        // 이 메세지를 받는다면 방장임
                        PacketType::CustomGameStartFailed => {
                            // let p = CustomGameStartFailedPacket::from_raw(packet);
                            // println!("Game start failed: {:?}", p.reason);
                            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                            continue 'writeloop;
                        }
                        // 이 메세지를 받는다면 게임이 시작된것임
                        PacketType::FormationPull => {
                            // let _p = FormationPullPacket::from_raw(packet);
                            break 'writeloop;
                        }
                        _ => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData, 
                                format!(
                                    "Ready failed - Invalid packet received: {:?}", 
                                    packet.packet_type()
                                )
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn select_character(&mut self) -> Result<(), std::io::Error> {
        let select_packet = FormationSelectPacket::new(
            self.account.uid,
            self.token,
            CharacterKind::default(),
        ).as_raw();

        'writeloop: loop {
            // 캐릭터 선택 패킷 전송
            self.writer.write_all(&select_packet.as_bytes()).await?;

            // 캐릭터 선택 완료 대기
            'readloop: loop {
                self.read().await?;

                while let Some(packet) = self.packet_parser.pop() {
                    match packet.packet_type() {
                        // 캐릭터 선택 완료 패킷
                        PacketType::FormationPull => {
                            // let _p = FormationPullPacket::from_raw(packet);
                        }
                        PacketType::FormationSelectResponse => {
                            let p = FormationSelectResponsePacket::from_raw(packet);
                            if p.result == SelectResult::Success {
                                // println!("Character selected successfully");
                                continue 'readloop;
                            } else {
                                println!("Character selection failed: {:?}", p.result);
                                continue 'writeloop;
                            }
                        }
                        PacketType::InitStage => {
                            // let _p = InitStagePacket::from_raw(packet);
                            // println!("Stage initialized");
                            break 'writeloop;
                        }
                        PacketType::GamePlayStop => {
                            // let _p = CustomGamePullPacket::from_raw(packet);
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::NotFound, 
                                "Game stopped"
                            ));
                        }
                        _ => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData, 
                                format!(
                                    "Character selection failed - Invalid packet received: {:?}", 
                                    packet.packet_type()
                                )
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn sync(&mut self) -> Result<(), std::io::Error> {
        let packet = PushSyncPacket::new(
            self.account.uid,
            self.token,
            true,
        ).as_raw();
        self.writer.write_all(&packet.as_bytes()).await?;

        'readloop: loop {
            self.read().await?;

            while let Some(packet) = self.packet_parser.pop() {
                match packet.packet_type() {
                    PacketType::PrepareStage => {
                        // let _p = PrepareStagePacket::from_raw(packet);
                        // println!("Stage prepared");
                        // break 'readloop;
                    }
                    PacketType::PullStage => {
                        // let _p = PullStagePacket::from_raw(packet);
                        // println!("Stage pulled");
                        break 'readloop;
                    }
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData, 
                            format!(
                                "Sync failed - Invalid packet received: {:?}",
                                packet.packet_type()
                            )
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}


struct Client {
    account: UserAccount,
    // token: LoginToken,
    connected: AtomicBool,
}

impl Client {
    async fn new(account: UserAccount) -> Result<Client, std::io::Error> {
        Ok(Self {
            account,
            // token: LoginToken::default(),
            connected: AtomicBool::new(false),
        })
    }

    async fn run(&self, addr: &str) {
        let stream = TcpStream::connect(addr).await.unwrap();
        self.connected.store(true, Ordering::Relaxed);
        NUM_CLIENTS.fetch_add(1, Ordering::Relaxed);

        let ready_client = ReadyClient::new(self.account.clone(), stream);
        let ready_client = match ready_client.run().await {
            Ok(client) => client,
            Err(_e) => {
                self.connected.store(false, Ordering::Relaxed);
                // println!("Client ready failed: {:?}", e);
                return;
            }
        };

        // self.account = ready_client.account;
        // self.token = ready_client.token;

        let packet_parser = Arc::new(Mutex::new(ready_client.packet_parser));

        let read_handle = tokio::spawn(
            // 읽기 루프 시작
            Client::start_read_loop(
                ready_client.reader,
                Arc::clone(&packet_parser),
            )
        );
        let write_handle = tokio::spawn(
            // 쓰기 루프 시작
            Client::start_write_loop(
                ready_client.account.uid,
                ready_client.token,
                ready_client.writer,
            )
        );

        // 게임 시작
        let _ = self.start_game(Arc::clone(&packet_parser)).await;

        read_handle.abort();
        write_handle.abort();
    }

    async fn start_read_loop(
        mut reader: tokio::net::tcp::OwnedReadHalf,
        packet_parser: Arc<Mutex<PacketParser>>,
    ) -> Result<(), std::io::Error> {
        let mut buf = [0; 32];

        loop {
            match reader.read(&mut buf).await {
                Ok(0) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof, "Connection closed by server"
                    ));
                }
                Ok(n) => {
                    packet_parser.lock().unwrap().push(&buf[..n]);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        
        // Ok(())
    }

    async fn start_write_loop(
        user_id: UserId,
        token: LoginToken,
        mut writer: tokio::net::tcp::OwnedWriteHalf,
    ) -> Result<(), std::io::Error> {
        let packet = PushStatusPacket {
            user_id,
            token,
            rotation: [0.0, 0.0, 0.0, 1.0],
            direction: [0.0, 0.0, 0.0],
            input_flags: GameInputBits::Forward,
            view_state: ViewState::default(),
            view_state_timer: ViewStateTimer::default(),
            view_rotation: LatLon::default(),
        };
        let packet = packet.as_raw().as_bytes();

        loop {
            writer.write_all(&packet).await?;

            let ping_packet = PingPacket {
                send_time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            };
            writer.write_all(&ping_packet.as_raw().as_bytes()).await?;

            // 1초마다 패킷 전송
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    async fn start_game(
        &self, 
        packet_parser: Arc<Mutex<PacketParser>>,
    ) -> Result<(), std::io::Error> {
        while self.connected.load(Ordering::Relaxed) {
            while let Some(packet) = packet_parser.lock().unwrap().pop() {
                match packet.packet_type() {
                    PacketType::PullStage => {
                        // let p = PullStagePacket::from_raw(packet);
                    }
                    PacketType::FinishStage => {
                        // let p = FinishStagePacket::from_raw(packet);
                        self.connected.store(false, Ordering::Relaxed);
                        return Ok(());
                    }
                    PacketType::Ping => {
                        let p = PingPacket::from_raw(packet);
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_nanos();

                        let delay = (now - p.send_time) as f32 / 1_000_000.0; // ms
                        if delay >= 0.0 {
                            let global_delay = GLOBAL_DELAY.load(Ordering::Relaxed) as f32;
                            if delay > global_delay {
                                GLOBAL_DELAY.fetch_add(1, Ordering::Relaxed);
                            } else if delay < global_delay {
                                GLOBAL_DELAY.fetch_sub(1, Ordering::Relaxed);
                            }
                        }
                    }
                    PacketType::UdpDamageLog => {
                        // udp 패킷이 tcp로 오고있음
                    }
                    _ => {
                        // println!("InGame - Invalid packet received: {:?}", packet.packet_type());
                        self.connected.store(false, Ordering::Relaxed);
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData, 
                            format!(
                                "InGame - Invalid packet received: {:?}",
                                packet.packet_type()
                            )
                        ));
                    }
                }
            }

            tokio::task::yield_now().await;
        }

        Ok(())
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

const MAX_CLIENTS: usize = 10000;
const IDLE_DELAY: u32 = 10;  // ms
const STABLE_DELAY: u32 = 30;  // ms
const DELAY_LIMIT1: u32 = 50;  // ms
const DELAY_LIMIT2: u32 = 100;  // ms


async fn stress_test(addr: Addr) {
    let accept_state = Arc::new(AtomicU32::new(AcceptState::Accept as u32));
    let state = Arc::clone(&accept_state);

    let _printer_handle = tokio::spawn(async move {
        loop {
            print!("\rDelay: {:?}ms  \tClients: {}  \t\t{:?}          ", 
                GLOBAL_DELAY.load(Ordering::Relaxed), 
                NUM_CLIENTS.load(Ordering::Relaxed),
                AcceptState::from(state.load(Ordering::Relaxed))
            );
            std::io::stdout().flush().unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    });

    let mut accept_delay = 100; // ms

    let mut clients = HashMap::with_capacity(MAX_CLIENTS);
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
                    if NUM_CLIENTS.load(Ordering::Relaxed) < MAX_CLIENTS as u16 {
                        let client = Arc::new(Client::new(UserAccount::default()).await.unwrap());
                        clients.insert(client_id, Arc::clone(&client));
                        client_id += 1;
                        let a = addr.to_string();
                        tokio::spawn(async move { client.run(&a).await });
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


fn main() {
    println!("Stress test started");

    let num_core = num_cpus::get();
    let num_threads = num_core / 4;
    println!("Using {} threads", num_threads);

    let mut args = env::args();
    args.next();
    let mut addr = Addr::default();

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
            _ => {
                eprintln!("Invalid option: {}", arg);
                return;
            }
        }
    }

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_threads)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            stress_test(addr).await
        });
}