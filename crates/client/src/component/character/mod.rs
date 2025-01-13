pub mod animation;
pub mod aris_original;

use hecs::{Entity, EntityBuilder, World};
use mod_app::asset::AssetManager;
use mod_network::{components::CharacterKind, Player};

use crate::{
    asset::ModelAssetError,
    component::{
        Acceleration, ActionStateTimer, Child, Force, MovementStateTimer, Sibling, ToParentTrans,
        Velocity, ViewStateTimer, WorldTransform,
    },
};

pub use self::animation::*;

/// 캐릭터 헤일로의 종류입니다.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterHaloKind {
    ArisOriginalHalo = 0,
}

impl From<CharacterKind> for CharacterHaloKind {
    fn from(value: CharacterKind) -> Self {
        match value {
            CharacterKind::ArisOriginal => CharacterHaloKind::ArisOriginalHalo,
            CharacterKind::MomoiOriginal => todo!(),
        }
    }
}

impl ToString for CharacterHaloKind {
    fn to_string(&self) -> String {
        match self {
            CharacterHaloKind::ArisOriginalHalo => "Aris Original Halo",
        }
        .to_string()
    }
}

/// 플레이어 캐릭터를 구성하는 엔터티를 생성합니다.
///
/// 생성된 엔터티는 아래 컴포넌트를 가집니다
/// - 자식 엔터티(`Child`)
/// - 캐릭터 종류(`CharacterKind`)
/// - 로컬 변환 행렬(`ToParentTrans`)
/// - 월드 변환 행렬(`WorldTransform`)
/// - 스키닝 애니메이션(`SkinningAnimation`)
/// - 힘의 총량(`Force`)
/// - 가속도(`Acceleration`)
/// - 속도(`Velocity`)
/// - 행동 상태(`ActionState`)
/// - 행동 상태 지속 시간 타이머(`ActionStateTimer`)
/// - 움직임 상태(`MovementState`)
/// - 움직임 상태 지속 시간 타이머(`MovementStateTimer`)
/// - 시야 상태(`ViewState`)
/// - 시야 상태 지속 시간 타이머(`ViewStateTimer`)
///
pub fn spawn_player_character(
    player_data: &Player,
    asset_manager: &AssetManager,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    world: &World,
) -> Result<(Entity, Vec<(Entity, EntityBuilder)>), ModelAssetError> {
    // 엔터티를 하나 할당받습니다.
    let entity = world.reserve_entity();
    let mut builder = EntityBuilder::new();

    // 컴포넌트 데이터를 준비합니다.
    let character_kind = player_data.character_kind;
    let local_transform = ToParentTrans(glam::Mat4::from_rotation_translation(
        glam::Quat::from_array(player_data.rotation.to_array()),
        glam::Vec3::from_array(player_data.translation.to_array()),
    ));
    let world_transform = WorldTransform::default();
    let action_state = player_data.action_state;
    let action_state_timer = ActionStateTimer::default();
    let movement_state = player_data.movement_state;
    let movement_state_timer = MovementStateTimer::default();
    let view_state = player_data.view_state;
    let view_state_timer = ViewStateTimer::default();

    // 컴포넌트를 추가합니다.
    builder.add(character_kind);
    builder.add(local_transform);
    builder.add(world_transform);
    builder.add_bundle((
        Force::default(),
        Acceleration::default(),
        Velocity::default(),
    ));
    builder.add_bundle((action_state, action_state_timer));
    builder.add_bundle((movement_state, movement_state_timer));
    builder.add_bundle((view_state, view_state_timer));

    // 캐릭터 종류에 따른 캐릭터 모델을 구성하는 엔터티를 생성합니다.
    let parent = entity;
    let (model_root_entity, skinning_animation, mut batch_commands) = match character_kind {
        CharacterKind::ArisOriginal => {
            aris_original::spawn_aris_original_model(asset_manager, device, queue, world, parent)
        }
        CharacterKind::MomoiOriginal => todo!(),
    }?;

    // 캐릭터 모델 루트 노드와 스키닝 애니메이션 컴포넌트를 추가합니다.
    builder.add(Child(model_root_entity));
    builder.add(skinning_animation);

    // 캐릭터 종류에 따른 캐릭터 헤일로 모델을 구성하는 엔터티를 생성합니다.
    let parent = entity;
    let (halo_root_entity, mut halo_batch_commands) = match character_kind {
        CharacterKind::ArisOriginal => aris_original::spawn_aris_original_model_halo(
            asset_manager,
            device,
            queue,
            world,
            parent,
        ),
        CharacterKind::MomoiOriginal => todo!(),
    }?;

    // 캐릭터 헤일로 모델의 최상위 엔터티를 캐릭터 모델 엔터티의 형제 엔터티로 추가합니다.
    let (last_entity, last_builder) = batch_commands
        .last_mut()
        .expect("entity builder must not be empty");
    assert_eq!(*last_entity, model_root_entity);
    last_builder.add(Sibling(halo_root_entity));

    // 엔터티 생성 명령어를 추가합니다.
    batch_commands.append(&mut halo_batch_commands);
    batch_commands.push((entity, builder));

    Ok((entity, batch_commands))
}
