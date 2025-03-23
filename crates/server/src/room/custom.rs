use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering as MemOrdering},
};

use ahash::{HashMap, RandomState};
use mod_network::{
    components::{
        JoinFailedReason, Permission, RecruitPhasePlayer, Team, UserAccount, UserId, WorldId,
    },
    protocol::{CustomGamePullPacket, Packet},
};
use parking_lot::{FairMutex, RwLock};
use rand::seq::SliceRandom;

use crate::session::Session;

/// 최대 수용 가능한 플레이어 수 입니다.
const MAX_PLAYERS: usize = 10;

/// 게임 대기실의 상태 목록입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RoomStatus {
    Activate = 0,
    Sleep = 1,
    Deactivate = 2,
}

impl RoomStatus {
    /// 주어진 정수로부터 `RoomStatus`를 생성합니다.
    /// 주어진 정수가 범위를 벗어난 경우 `None`을 반환합니다.
    pub fn new(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Activate),
            1 => Some(Self::Sleep),
            2 => Some(Self::Deactivate),
            _ => {
                log::error!(
                    "the value is out of range for `{}`, (VALUE:{})",
                    stringify!(RoomStatus),
                    val
                );
                None
            }
        }
    }
}

/// 커스텀 게임 대기실입니다.
#[derive(Debug)]
pub struct CustomGameRoom {
    /// 커스텀 게임 대기실의 활성화 여부
    status: AtomicU8,
    /// 커스텀 게임의 게임 월드 식별자입니다.
    world_id: WorldId,

    /// 커스텀 게임 관리자의 식별자입니다.
    admin: RwLock<UserId>,

    /// 참여한 플레이어 집합입니다.
    players: FairMutex<HashMap<Arc<Session>, RecruitPhasePlayer>>,
}

impl CustomGameRoom {
    /// 새로운 커스텀 게임 대기실을 생성합니다.
    pub fn new(world_id: WorldId) -> Self {
        Self {
            status: AtomicU8::new(RoomStatus::Deactivate as u8),
            world_id,
            admin: RwLock::new(UserId::NULL),
            players: FairMutex::new(HashMap::with_capacity_and_hasher(
                MAX_PLAYERS,
                RandomState::new(),
            )),
        }
    }

    /// 게임 월드 식별자를 반환합니다.
    pub fn id(&self) -> WorldId {
        self.world_id
    }

    /// 커스텀 게임 대기실을 재설정합니다.
    pub fn reset(&self, account: UserAccount, session: &Arc<Session>) -> Vec<RecruitPhasePlayer> {
        // 플레이어 집합을 비웁니다.
        let mut players = self.players.lock();
        players.clear();

        // 커스텀 게임 관리자 플레이어를 추가합니다.
        *self.admin.write() = account.uid;
        players.insert(
            session.clone(),
            RecruitPhasePlayer::new(account, Team::Blue, false, Permission::Admin),
        );

        // 커스텀 게임 대기실을 활성화합니다.
        self.status
            .store(RoomStatus::Activate as u8, MemOrdering::Relaxed);

        players.values().cloned().collect()
    }

    /// 커스텀 게임 대기실의 상태를 가져옵니다.
    pub fn get_status(&self) -> RoomStatus {
        let val = self.status.load(MemOrdering::Relaxed);
        RoomStatus::new(val & 0x3).expect("out of bounds")
    }

    /// 커스텀 게임에 해당 플레이어를 추가합니다.  
    /// - 플레이어 추가에 성공한 경우 현재 참여한 플레이어 정보를 반환합니다.  
    /// - 플레이어 추가에 실패한 경우 실패 사유를 반환합니다.  
    pub fn join(
        &self,
        account: UserAccount,
        session: &Arc<Session>,
    ) -> Result<Vec<RecruitPhasePlayer>, JoinFailedReason> {
        // 락을 획득합니다.
        let mut players = self.players.lock();

        // 커스텀 게임이 활성화 상태인지 확인합니다.
        let val = self.status.load(MemOrdering::Relaxed);
        let status = RoomStatus::new(val & 0x3).expect("out of bounds");
        match status {
            RoomStatus::Sleep => return Err(JoinFailedReason::InProgress),
            RoomStatus::Deactivate => return Err(JoinFailedReason::NotFound),
            _ => {}
        };

        // 커스텀 게임 대기실에 인원이 가득찼는지 확인합니다.
        if players.len() == MAX_PLAYERS {
            return Err(JoinFailedReason::FullCapacity);
        }

        // 현재 커스텀 게임의 각 팀 인원을 계산합니다.
        let mut red_team_players = 0;
        let mut blue_team_players = 0;
        for player in players.values() {
            match player.team() {
                Team::Blue => blue_team_players += 1,
                Team::Red => red_team_players += 1,
            };
        }

        // 플레이어를 생성합니다.
        let team = if red_team_players < blue_team_players {
            Team::Red
        } else {
            Team::Blue
        };
        players.insert(
            session.clone(),
            RecruitPhasePlayer::new(account, team, false, Permission::User),
        );

        Ok(players.values().cloned().collect())
    }

    /// 커스텀 게임에서 해당 플레이어를 제거합니다.
    pub fn exit(&self, session: &Session) {
        // 락을 획득합니다.
        let mut players = self.players.lock();

        // 해당 플레이어를 제거합니다.
        if let Some(player) = players.remove(session) {
            // 모든 플레이어가 게임 대기실에서 나간 경우 대기실을 비활성화 합니다.
            if players.len() == 0 {
                self.status
                    .store(RoomStatus::Deactivate as u8, MemOrdering::Release);
                return;
            }

            // 제거된 플레이어의 권한이 관리자인 경우
            // 남아있는 플레이어 중 무작위로 한 명을 선정하여 권한을 넘겨줍니다.
            if player.permission() == Permission::Admin {
                let mut remaining_players: Vec<_> = players.values_mut().collect();
                remaining_players.shuffle(&mut rand::rng());

                let player = remaining_players.pop().unwrap();
                player.with_permission(Permission::Admin).with_ready(false);
                *self.admin.write() = player.account.uid;
            }
        }
    }

    /// 세션에 해당하는 커스텀 게임 참가 플레이어에 접근합니다.
    pub fn access<F>(&self, session: &Arc<Session>, func: F) -> bool
    where
        F: FnOnce(&mut RecruitPhasePlayer),
    {
        let mut players = self.players.lock();
        if let Some(player) = players.get_mut(session) {
            func(player);
            return true;
        }
        false
    }

    /// 커스텀 게임에 참가한 세션에 주기적으로 패킷을 전송합니다.
    pub fn on_process(&self) {
        // 락을 획득합니다.
        let players = self.players.lock();

        // 패킷을 생성합니다.
        let packet = CustomGamePullPacket::from_iter(players.values().cloned());

        // 패킷을 각 세션에 전송합니다.
        for session in players.keys() {
            session.tcp_write(packet.as_raw());
        }
    }
}
