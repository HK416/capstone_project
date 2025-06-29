use hecs::{Entity, World};

use crate::component::{
    Player0, Player1, Player2, Player3, Player4, Player5, Player6, Player7, Player8, Player9,
    PlayerArchetype, ToParentTrans,
};

/// 서버로부터 받은 데이터로 플레이어 데이터를 갱신합니다.
pub fn pull_transform_data(
    world: &World,
    entity: Entity,
    archetype: PlayerArchetype,
    translation: [f32; 3],
    rotation: [f32; 4],
    velocity: [f32; 3],
    is_player: bool,
    is_overwrite: bool,
) {
    match archetype {
        PlayerArchetype::Player0 => {
            type Tag = Player0;
            type Q<'a> = &'a mut (Tag, ToParentTrans);
            let mut query = world.query_one::<Q>(entity).expect("invalid entity!");
            let (_, local_transform) = query.get().expect("invalid entity component!");

            let new_translation = glam::Vec3A::from_array(translation);
            let distance = local_transform
                .get_translation()
                .distance_squared(new_translation);

            let new_rotation = glam::Quat::from_array(rotation);
            let between = new_rotation
                .mul_vec3a(glam::Vec3A::Z)
                .angle_between(local_transform.get_look_vector());
            if is_overwrite || (between >= 5f32.to_radians() && (!is_player && distance >= 0.1)) {
                local_transform
                    .set_rotation_translation(new_rotation.into(), new_translation.into());
            } else if is_overwrite || (!is_player && distance >= 0.1) {
                local_transform.set_translation(new_translation.into());
            }
        }
        PlayerArchetype::Player1 => {
            type Tag = Player1;
            type Q<'a> = &'a mut (Tag, ToParentTrans);
            let mut query = world.query_one::<Q>(entity).expect("invalid entity!");
            let (_, local_transform) = query.get().expect("invalid entity component!");

            let new_translation = glam::Vec3A::from_array(translation);
            let distance = local_transform
                .get_translation()
                .distance_squared(new_translation);

            let new_rotation = glam::Quat::from_array(rotation);
            let between = new_rotation
                .mul_vec3a(glam::Vec3A::Z)
                .angle_between(local_transform.get_look_vector());
            if is_overwrite || (between >= 5f32.to_radians() && (!is_player && distance >= 0.1)) {
                local_transform
                    .set_rotation_translation(new_rotation.into(), new_translation.into());
            } else if is_overwrite || (!is_player && distance >= 0.1) {
                local_transform.set_translation(new_translation.into());
            }
        }
        PlayerArchetype::Player2 => {
            type Tag = Player2;
            type Q<'a> = &'a mut (Tag, ToParentTrans);
            let mut query = world.query_one::<Q>(entity).expect("invalid entity!");
            let (_, local_transform) = query.get().expect("invalid entity component!");

            let new_translation = glam::Vec3A::from_array(translation);
            let distance = local_transform
                .get_translation()
                .distance_squared(new_translation);

            let new_rotation = glam::Quat::from_array(rotation);
            let between = new_rotation
                .mul_vec3a(glam::Vec3A::Z)
                .angle_between(local_transform.get_look_vector());
            if is_overwrite || (between >= 5f32.to_radians() && (!is_player && distance >= 0.1)) {
                local_transform
                    .set_rotation_translation(new_rotation.into(), new_translation.into());
            } else if is_overwrite || (!is_player && distance >= 0.1) {
                local_transform.set_translation(new_translation.into());
            }
        }
        PlayerArchetype::Player3 => {
            type Tag = Player3;
            type Q<'a> = &'a mut (Tag, ToParentTrans);
            let mut query = world.query_one::<Q>(entity).expect("invalid entity!");
            let (_, local_transform) = query.get().expect("invalid entity component!");

            let new_translation = glam::Vec3A::from_array(translation);
            let distance = local_transform
                .get_translation()
                .distance_squared(new_translation);

            let new_rotation = glam::Quat::from_array(rotation);
            let between = new_rotation
                .mul_vec3a(glam::Vec3A::Z)
                .angle_between(local_transform.get_look_vector());
            if is_overwrite || (between >= 5f32.to_radians() && (!is_player && distance >= 0.1)) {
                local_transform
                    .set_rotation_translation(new_rotation.into(), new_translation.into());
            } else if is_overwrite || (!is_player && distance >= 0.1) {
                local_transform.set_translation(new_translation.into());
            }
        }
        PlayerArchetype::Player4 => {
            type Tag = Player4;
            type Q<'a> = &'a mut (Tag, ToParentTrans);
            let mut query = world.query_one::<Q>(entity).expect("invalid entity!");
            let (_, local_transform) = query.get().expect("invalid entity component!");

            let new_translation = glam::Vec3A::from_array(translation);
            let distance = local_transform
                .get_translation()
                .distance_squared(new_translation);

            let new_rotation = glam::Quat::from_array(rotation);
            let between = new_rotation
                .mul_vec3a(glam::Vec3A::Z)
                .angle_between(local_transform.get_look_vector());
            if is_overwrite || (between >= 5f32.to_radians() && (!is_player && distance >= 0.1)) {
                local_transform
                    .set_rotation_translation(new_rotation.into(), new_translation.into());
            } else if is_overwrite || (!is_player && distance >= 0.1) {
                local_transform.set_translation(new_translation.into());
            }
        }
        PlayerArchetype::Player5 => {
            type Tag = Player5;
            type Q<'a> = &'a mut (Tag, ToParentTrans);
            let mut query = world.query_one::<Q>(entity).expect("invalid entity!");
            let (_, local_transform) = query.get().expect("invalid entity component!");

            let new_translation = glam::Vec3A::from_array(translation);
            let distance = local_transform
                .get_translation()
                .distance_squared(new_translation);

            let new_rotation = glam::Quat::from_array(rotation);
            let between = new_rotation
                .mul_vec3a(glam::Vec3A::Z)
                .angle_between(local_transform.get_look_vector());
            if is_overwrite || (between >= 5f32.to_radians() && (!is_player && distance >= 0.1)) {
                local_transform
                    .set_rotation_translation(new_rotation.into(), new_translation.into());
            } else if is_overwrite || (!is_player && distance >= 0.1) {
                local_transform.set_translation(new_translation.into());
            }
        }
        PlayerArchetype::Player6 => {
            type Tag = Player6;
            type Q<'a> = &'a mut (Tag, ToParentTrans);
            let mut query = world.query_one::<Q>(entity).expect("invalid entity!");
            let (_, local_transform) = query.get().expect("invalid entity component!");

            let new_translation = glam::Vec3A::from_array(translation);
            let distance = local_transform
                .get_translation()
                .distance_squared(new_translation);

            let new_rotation = glam::Quat::from_array(rotation);
            let between = new_rotation
                .mul_vec3a(glam::Vec3A::Z)
                .angle_between(local_transform.get_look_vector());
            if is_overwrite || (between >= 5f32.to_radians() && (!is_player && distance >= 0.1)) {
                local_transform
                    .set_rotation_translation(new_rotation.into(), new_translation.into());
            } else if is_overwrite || (!is_player && distance >= 0.1) {
                local_transform.set_translation(new_translation.into());
            }
        }
        PlayerArchetype::Player7 => {
            type Tag = Player7;
            type Q<'a> = &'a mut (Tag, ToParentTrans);
            let mut query = world.query_one::<Q>(entity).expect("invalid entity!");
            let (_, local_transform) = query.get().expect("invalid entity component!");

            let new_translation = glam::Vec3A::from_array(translation);
            let distance = local_transform
                .get_translation()
                .distance_squared(new_translation);

            let new_rotation = glam::Quat::from_array(rotation);
            let between = new_rotation
                .mul_vec3a(glam::Vec3A::Z)
                .angle_between(local_transform.get_look_vector());
            if is_overwrite || (between >= 5f32.to_radians() && (!is_player && distance >= 0.1)) {
                local_transform
                    .set_rotation_translation(new_rotation.into(), new_translation.into());
            } else if is_overwrite || (!is_player && distance >= 0.1) {
                local_transform.set_translation(new_translation.into());
            }
        }
        PlayerArchetype::Player8 => {
            type Tag = Player8;
            type Q<'a> = &'a mut (Tag, ToParentTrans);
            let mut query = world.query_one::<Q>(entity).expect("invalid entity!");
            let (_, local_transform) = query.get().expect("invalid entity component!");

            let new_translation = glam::Vec3A::from_array(translation);
            let distance = local_transform
                .get_translation()
                .distance_squared(new_translation);

            let new_rotation = glam::Quat::from_array(rotation);
            let between = new_rotation
                .mul_vec3a(glam::Vec3A::Z)
                .angle_between(local_transform.get_look_vector());
            if is_overwrite || (between >= 5f32.to_radians() && (!is_player && distance >= 0.1)) {
                local_transform
                    .set_rotation_translation(new_rotation.into(), new_translation.into());
            } else if is_overwrite || (!is_player && distance >= 0.1) {
                local_transform.set_translation(new_translation.into());
            }
        }
        PlayerArchetype::Player9 => {
            type Tag = Player9;
            type Q<'a> = &'a mut (Tag, ToParentTrans);
            let mut query = world.query_one::<Q>(entity).expect("invalid entity!");
            let (_, local_transform) = query.get().expect("invalid entity component!");

            let new_translation = glam::Vec3A::from_array(translation);
            let distance = local_transform
                .get_translation()
                .distance_squared(new_translation);

            let new_rotation = glam::Quat::from_array(rotation);
            let between = new_rotation
                .mul_vec3a(glam::Vec3A::Z)
                .angle_between(local_transform.get_look_vector());
            if is_overwrite || (between >= 5f32.to_radians() && (!is_player && distance >= 0.1)) {
                local_transform
                    .set_rotation_translation(new_rotation.into(), new_translation.into());
            } else if is_overwrite || (!is_player && distance >= 0.1) {
                local_transform.set_translation(new_translation.into());
            }
        }
    }
}
