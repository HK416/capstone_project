use mod_network::components::{
    ActionState, CharacterKind, Epoch, LatLon, MovementState, ObjectId, UserId, ViewState,
};

/// 게임 월드에서 발생하는 이벤트 목록입니다.
#[derive(Debug)]
pub enum WorldEvents {
    AddPlayer(UserId, CharacterKind),
    UpdatePlayerStatus(
        Epoch,
        UserId,
        glam::Quat,
        glam::Vec3A,
        ActionState,
        MovementState,
        ViewState,
        LatLon,
    ),
    AddBullet(UserId),
    RemovePlayer(UserId),
    RemoveBullet(ObjectId),
}
