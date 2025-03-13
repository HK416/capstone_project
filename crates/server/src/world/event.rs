use std::sync::Arc;

use mod_network::components::{CharacterKind, ObjectId, UserId};

use crate::session::Session;

/// 게임 월드에서 발생하는 이벤트 목록입니다.
#[derive(Debug)]
pub enum GameWorldEvent {
    /// 플레이어 오브젝트를 추가합니다.
    AddPlayer(Arc<Session>, CharacterKind),
    /// 플레이어 오브젝트를 제거합니다.
    RemovePlayer(UserId),
    /// 총알 오브젝트를 추가합니다.
    AddBullet { shooter_id: UserId, delay: f32 },
    /// 총알 오브젝트를 제거합니다.
    RemoveBullet(ObjectId),
}
