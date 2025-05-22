use std::{
    collections::HashSet,
    io::Write, 
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU16, Ordering},
        Arc, Mutex
    },
};

use rand::seq::IndexedRandom;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use mod_network::{
    components::{
        CharacterKind, Email, GameInputBits, LatLon, 
        UserId, LoginToken, Passwd, 
        Permission, SelectResult, UserAccount, 
        ViewState, ViewStateTimer, WorldId
    },
    protocol::{
        AvailableWorldsPacket, 
        CustomGameJoinFailedPacket, CustomGameJoinRequestPacket, 
        CustomGameJoinSuccessPacket, CustomGamePullPacket, 
        CustomGameReadyPacket, CustomGameStartFailedPacket, 
        FinishStagePacket, FormationPullPacket, 
        FormationSelectPacket, FormationSelectResponsePacket, 
        InitStagePacket, LoginRequestPacket, LoginSuccessPacket, 
        Packet, PacketParser, PacketType, 
        PrepareStagePacket, PullStagePacket, PushStatusPacket, 
        PushSyncPacket, RequestAvailableWorldsPacket
    }
};


lazy_static::lazy_static! {
    /// milliseconds
    static ref GLOBAL_DELAY: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));

    /// 서버에 접속한 클라이언트 수
    static ref NUM_CLIENTS: Arc<AtomicU16> = Arc::new(AtomicU16::new(0));

}
/// 서버에서 실행중인 월드 목록
// static ref RUNNING_WORLD_LIST: Arc<Mutex<HashSet<WorldId>>> = Arc::new(Mutex::new(HashSet::new()));


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
                *available.choose(&mut rand::rng()).unwrap()
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
                        let _p = CustomGameJoinSuccessPacket::from_raw(packet);
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
                self.read().await.unwrap();

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
                            let _p = CustomGameStartFailedPacket::from_raw(packet);
                            // println!("Game start failed: {:?}", p.reason);
                            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                            continue 'writeloop;
                        }
                        // 이 메세지를 받는다면 게임이 시작된것임
                        PacketType::FormationPull => {
                            let _p = FormationPullPacket::from_raw(packet);
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
                            let _p = FormationPullPacket::from_raw(packet);
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
                            let _p = InitStagePacket::from_raw(packet);
                            // println!("Stage initialized");
                            break 'writeloop;
                        }
                        PacketType::GamePlayStop => {
                            let _p = CustomGamePullPacket::from_raw(packet);
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
                        let _p = PrepareStagePacket::from_raw(packet);
                        // println!("Stage prepared");
                        // break 'readloop;
                    }
                    PacketType::PullStage => {
                        let _p = PullStagePacket::from_raw(packet);
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
    token: LoginToken,
    connected: AtomicBool,
}

impl Client {
    async fn new(account: UserAccount) -> Result<Client, std::io::Error> {
        Ok(Self {
            account,
            token: LoginToken::default(),
            connected: AtomicBool::new(false),
        })
    }

    async fn run(&mut self) {
        let stream = TcpStream::connect("localhost:7878").await.unwrap();
        self.connected.store(true, Ordering::Relaxed);
        NUM_CLIENTS.fetch_add(1, Ordering::Relaxed);

        let ready_client = ReadyClient::new(self.account.clone(), stream);
        let ready_client = match ready_client.run().await {
            Ok(client) => client,
            Err(_e) => {
                self.connected.store(false, Ordering::Relaxed);
                NUM_CLIENTS.fetch_sub(1, Ordering::Relaxed);
                // println!("Client ready failed: {:?}", e);
                return;
            }
        };

        self.account = ready_client.account;
        self.token = ready_client.token;

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
                self.account.uid,
                self.token,
                ready_client.writer,
            )
        );

        // 게임 시작
        match self.start_game(Arc::clone(&packet_parser)).await {
            Ok(_) => {
                self.connected.store(false, Ordering::Relaxed);
                NUM_CLIENTS.fetch_sub(1, Ordering::Relaxed);
            }
            Err(_) => {
                self.connected.store(false, Ordering::Relaxed);
                NUM_CLIENTS.fetch_sub(1, Ordering::Relaxed);
                return;
            }
        }

        match read_handle.await.unwrap() {
            Ok(_) => {
                // Never reached
            }
            Err(_) => {
                if self.connected.load(Ordering::Relaxed) == false {
                    self.connected.store(false, Ordering::Relaxed);
                    NUM_CLIENTS.fetch_sub(1, Ordering::Relaxed);
                }
            }
        }
        match write_handle.await.unwrap() {
            Ok(_) => {
                // Never reached
            }
            Err(_) => {
                // reader종료시 처리
            }
        }
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
        let packet = packet.as_raw();

        loop {
            writer.write_all(&packet.as_bytes()).await?;

            // 1초마다 패킷 전송
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    async fn start_game(
        &mut self, 
        packet_parser: Arc<Mutex<PacketParser>>,
    ) -> Result<(), std::io::Error> {
        let mut prev_remain_time = 0.0;

        loop {
            let mut delay_checked = false;
            while let Some(packet) = packet_parser.lock().unwrap().pop() {
                match packet.packet_type() {
                    PacketType::PullStage => {
                        let p = PullStagePacket::from_raw(packet);
                        if !delay_checked {
                            let delay = (prev_remain_time - p.remaining_time_sec) * 1000.0; // ms
                            if delay >= 0.0 {
                                let global_delay = GLOBAL_DELAY.load(Ordering::Relaxed) as f32;
                                if delay > global_delay {
                                    GLOBAL_DELAY.fetch_add(1, Ordering::Relaxed);
                                } else if delay < global_delay {
                                    GLOBAL_DELAY.fetch_sub(1, Ordering::Relaxed);
                                }
                            }
                            delay_checked = true;
                        }
                        prev_remain_time = p.remaining_time_sec;
                    }
                    PacketType::FinishStage => {
                        let _p = FinishStagePacket::from_raw(packet);
                        return Ok(());
                    }
                    _ => {
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

        // Ok(())
    }
}


#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    println!("Stress test started");

    let printer_handle = tokio::spawn(async move {
        loop {
            print!("\rDelay: {:?}ms     Clients: {}      ", 
                GLOBAL_DELAY.load(Ordering::Relaxed), 
                NUM_CLIENTS.load(Ordering::Relaxed)
            );
            std::io::stdout().flush().unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
    });

    const MAX_CLIENTS: usize = 10000;
    const STABLE_DELAY: u32 = 50;  // ms
    const DELAY_LIMIT1: u32 = 100;  // ms
    const DELAY_LIMIT2: u32 = 150;  // ms

    let mut accept_delay = 50;  // ms, 처음엔 초당 20개

    let mut clients = Vec::with_capacity(MAX_CLIENTS);
    for _ in 0..MAX_CLIENTS {
        let delay = GLOBAL_DELAY.load(Ordering::Relaxed);

        if delay < STABLE_DELAY {
            // accept
            let mut client = Client::new(UserAccount::default()).await.unwrap();
            clients.push(tokio::spawn(async move { client.run().await }));
        } else if delay < DELAY_LIMIT1 {
            // do nothing
            accept_delay = 500;
        } else if delay < DELAY_LIMIT2 {
            // 연결 끊기
            clients.pop();
            accept_delay = 1000;
        } else {
            // 연결 끊기
            clients.pop();
            accept_delay = 5000;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(accept_delay)).await;
    }

    // 모든 클라이언트가 종료될 때까지 대기
    for client in clients {
        client.await.unwrap();
    }

    printer_handle.abort();

    println!("Stress test finished");

    Ok(())
}