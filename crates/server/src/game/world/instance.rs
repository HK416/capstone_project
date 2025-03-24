use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering as MemOrdering},
};

use ahash::{HashMap, RandomState};
use dashmap::DashSet;
use mod_network::components::{
    JoinFailedReason, MAX_IN_GAME_PLAYERS, Permission, RecruitPhasePlayer, Team, UserAccount,
    UserId, WorldId,
};
use parking_lot::RwLock;
use rand::seq::SliceRandom;

use crate::{
    game::{GameWorldStatus, Player},
    session::Session,
};

/// 게임을 진행하는 객체입니다.
#[derive(Debug)]
pub struct GameWorld {
    /// 현재 게임 월드의 실행 상태입니다.
    pub(super) status: AtomicU8,
    /// 게임 월드 식별자입니다.
    pub(super) world_id: WorldId,

    /// 게임 관리자의 식별자입니다.  
    /// `UserId::NULL`인 경우 게임 서버가 관리합니다.
    pub(super) admin: RwLock<UserId>,

    /// 참여한 플레이어 데이터 집합입니다.
    pub(super) players: RwLock<HashMap<Arc<Session>, Arc<Player>>>,
    /// 레드팀에 속한 플레이어 데이터 집합입니다.
    pub(super) red_team_players: DashSet<Arc<Player>, RandomState>,
    /// 블루팀에 속한 플레이어 데이터 집합입니다.
    pub(super) blue_team_players: DashSet<Arc<Player>, RandomState>,
}

impl GameWorld {
    /// 새로운 게임 월드를 생성합니다.
    pub fn new(world_id: WorldId) -> Self {
        Self {
            status: AtomicU8::new(GameWorldStatus::Closed as u8),
            world_id,
            admin: RwLock::new(UserId::NULL),
            players: RwLock::new(HashMap::with_capacity_and_hasher(
                MAX_IN_GAME_PLAYERS,
                RandomState::default(),
            )),
            red_team_players: DashSet::with_capacity_and_hasher(
                MAX_IN_GAME_PLAYERS,
                RandomState::default(),
            ),
            blue_team_players: DashSet::with_capacity_and_hasher(
                MAX_IN_GAME_PLAYERS,
                RandomState::default(),
            ),
        }
    }

    /// 커스텀 게임으로 게임 월드를 재설정합니다.
    pub fn reset_custom(
        &self,
        account: UserAccount,
        session: &Arc<Session>,
    ) -> Vec<RecruitPhasePlayer> {
        // 게임 관리자를 설정합니다.
        *self.admin.write() = account.uid;

        // 게임 월드를 활성화 상태로 변경합니다.
        self.status
            .store(GameWorldStatus::Open as u8, MemOrdering::Release);

        // 게임 관리자를 플레이어 데이터를 생성합니다.
        let mut player = Player::new(account);
        player
            .game_play
            .get_mut()
            .with_team(Team::Blue)
            .with_permission(Permission::Admin);

        // 게임 관리자를 플레이어 집합에 추가합니다.
        let mut players = self.players.write();
        let player = Arc::new(player);
        players.insert(session.clone(), player.clone());
        self.blue_team_players.insert(player);

        players
            .values()
            .map(|player| {
                let game = player.game_play.lock();
                RecruitPhasePlayer::new(
                    player.account.clone(),
                    game.team(),
                    game.is_ready(),
                    game.permission(),
                )
            })
            .collect()
    }

    /// 커스텀 게임 참여를 시도합니다.
    /// - 플레이어 추가에 성공한 경우 현재 참여한 플레이어 정보를 반환합니다.
    /// - 플레이어 추가에 실패한 경우 실패 사유를 반환합니다.
    pub fn try_join(
        &self,
        account: UserAccount,
        session: &Arc<Session>,
    ) -> Result<Vec<RecruitPhasePlayer>, JoinFailedReason> {
        // 락을 획득합니다.
        let mut players = self.players.write();

        // 게임 월드 상태를 확인합니다.
        let val = self.status.load(MemOrdering::Relaxed);
        // Safe: 주어진 값은 범위를 벗어나지 않음
        let status = unsafe { GameWorldStatus::new(val).unwrap_unchecked() };
        match status {
            GameWorldStatus::Closed => return Err(JoinFailedReason::NotFound),
            GameWorldStatus::Running => return Err(JoinFailedReason::InProgress),
            _ => {}
        };

        // 게임 월드에 인원이 가득찼는지 확인합니다.
        if players.len() == MAX_IN_GAME_PLAYERS {
            return Err(JoinFailedReason::FullCapacity);
        }

        if self.red_team_players.len() < self.blue_team_players.len() {
            // 블루팀에 속한 플레이어를 생성합니다.
            let mut player = Player::new(account);
            player.game_play.get_mut().with_team(Team::Red);

            // 플레이어를 추가합니다.
            let player = Arc::new(player);
            players.insert(session.clone(), player.clone());
            self.red_team_players.insert(player);
        } else {
            // 레드팀에 속한 플레이어를 생성합니다.
            let mut player = Player::new(account);
            player.game_play.get_mut().with_team(Team::Blue);

            // 플레이어를 추가합니다.
            let player = Arc::new(player);
            players.insert(session.clone(), player.clone());
            self.blue_team_players.insert(player);
        }

        Ok(players
            .values()
            .map(|player| {
                let game = player.game_play.lock();
                RecruitPhasePlayer::new(
                    player.account.clone(),
                    game.team(),
                    game.is_ready(),
                    game.permission(),
                )
            })
            .collect())
    }

    /// 게임 월드에서 해당 플레이어를 제거합니다.
    pub fn exit(&self, session: &Session) {
        // 락을 획득합니다.
        let mut players = self.players.write();

        // 해당 플레이어를 제거합니다.
        if let Some(player) = players.remove(session) {
            self.red_team_players.remove(&player);
            self.blue_team_players.remove(&player);

            // 모든 플레이어가 게임 대기실에서 나간 경우 게임 월드를 비활성화합니다.
            if players.len() == 0 {
                self.status
                    .store(GameWorldStatus::Closed as u8, MemOrdering::Release);

                players.clear();
                self.red_team_players.clear();
                self.blue_team_players.clear();
                *self.admin.write() = UserId::NULL;

                return;
            }

            // 제거된 플레이어의 권한이 관리자인 경우
            // 남아있는 플레이어 중 무작위로 한 명을 선정하여 권한을 넘겨줍니다.
            let permission = player.game_play.lock().permission();
            let mut remaining_players: Vec<_> = players.values().collect();
            remaining_players.shuffle(&mut rand::rng());

            // Safe: 플레이어는 비어있지 않음
            let player = unsafe { remaining_players.pop().unwrap_unchecked() };
            player
                .game_play
                .lock()
                .with_ready(false)
                .with_permission(permission);

            // 게임 관리자를 설정합니다.
            *self.admin.write() = player.account.uid;
        }
    }

    /// 세션에 해당하는 게임 월드 플레이어에 접근합니다.  
    /// 주어진 세션에 해당하는 게임 월드 플레이어가 존재하지 않는 경우 `false`를 반환합니다.
    pub fn access<F>(&self, session: &Session, func: F) -> bool
    where
        F: FnOnce(&Player),
    {
        let players = self.players.read();
        if let Some(player) = players.get(session) {
            func(&player);
            return true;
        }
        false
    }

    /// 게임 월드 식별자를 반환합니다.
    pub fn id(&self) -> WorldId {
        self.world_id
    }

    /// 게임 월드 상태를 반환합니다.
    pub fn status(&self) -> GameWorldStatus {
        // Safe: 값이 범위를 벗어나지 않음
        let val = self.status.load(MemOrdering::Acquire);
        unsafe { GameWorldStatus::new(val).unwrap_unchecked() }
    }
}
