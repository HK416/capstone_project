use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, Default)]
pub struct Float3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
pub struct CharacterAttributes {
    pub speed: f32,
    pub muzzle_position: Float3,
    pub fire_delay_time: f32,
    pub health_point: f32,
    pub attack_power: f32,
    pub defense_power: f32,
    pub accuracy_stat: f32,
    pub evasion_stat: f32,
    pub critical_rate: f32,
    pub critical_damage: f32,
    pub attack_range: f32,
}



