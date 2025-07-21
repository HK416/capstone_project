//! 캐릭터 모델 생성과 관련된 코드를 관리합니다.
//!

use hecs::{Component, Entity, EntityBuilder, World};
use mod_network::components::{CharacterKind, InGamePlayerInitData};

use crate::asset::{ModelPool, TextureDataPool};

use super::*;

/// 플레이어 엔터티를 생성합니다.
pub fn spawn_player<Tag: Copy + Component>(
    tag: Tag,
    world: &World,
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    staging_buffers: &mut Vec<wgpu::Buffer>,
    texture_data_pool: &TextureDataPool,
    model_pool: &ModelPool,
    data: InGamePlayerInitData,
) -> (Entity, Vec<(Entity, EntityBuilder)>) {
    let func = match data.character_kind {
        CharacterKind::ArisOriginal => aris_original::spawn_player,
        CharacterKind::MomoiOriginal => momoi_original::spawn_player,
        CharacterKind::MidoriOriginal => midori_original::spawn_player,
        CharacterKind::YuukaOriginal => yuuka_original::spawn_player,
    };

    func(
        tag,
        world,
        device,
        encoder,
        staging_buffers,
        texture_data_pool,
        model_pool,
        data,
    )
}
