use mod_network::components::{
    ActionState, ActionStateTimer, CharacterKind, ClientId, Epoch, LatLon, MovementState,
    MovementStateTimer, ObjectId, ViewState, ViewStateTimer,
};

/// 게임 월드에서 발생하는 이벤트 목록입니다.
#[derive(Debug)]
pub enum WorldEvents {
    AddPlayer(ClientId, ObjectId, CharacterKind),
    UpdatePlayerStatus(
        Epoch,
        ClientId,
        glam::Quat,
        glam::Vec3A,
        ActionState,
        ActionStateTimer,
        MovementState,
        MovementStateTimer,
        ViewState,
        ViewStateTimer,
        LatLon,
    ),
    AddBullet(ClientId),
    RemovePlayer(ClientId),
    RemoveBullet(ObjectId),
}
