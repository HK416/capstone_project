//! 인게임 결과 장면 진입과 관련된 코드를 관리합니다.
//!

use ahash::HashMap;
use hecs::{Entity, World};
use mod_app::scene::GameScene;
use mod_network::components::{LoginToken, UserId};

use crate::{component::Skybox, config::Locale};

/// 인게임 결과 장면에 진입하는 장면입니다.
pub struct InGameFinishEnterScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 현재 사용자의 식별자입니다.
    user_id: UserId,
    /// 현재 사용자의 로그인 토큰입니다.
    token: LoginToken,

    /// 게임 장면의 경과 시간입니다.
    elapsed_time_sec: f32,

    /// 엔터티를 관리하는 월드 객체입니다.
    world: World,
    /// 스카이박스입니다.
    skybox: Skybox,
    /// 메인 카메라 엔터티입니다.
    camera: Entity,
    /// 플레이어 엔터티 집합입니다.
    players: HashMap<UserId, Entity>,
    /// 지형 엔터티 집합입니다.
    stages: Vec<Entity>,

    /// 게임 인터페이스 텍스처 식별자입니다.
    ui_textures: HashMap<String, egui::load::SizedTexture>,
}

impl InGameFinishEnterScene {
    /// 새로운 `InGameFinishEnterScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        world: World,
        skybox: Skybox,
        camera: Entity,
        players: HashMap<UserId, Entity>,
        stages: Vec<Entity>,
        ui_textures: HashMap<String, egui::load::SizedTexture>,
    ) -> Self {
        Self {
            locale,
            user_id,
            token,
            elapsed_time_sec: 0.0,
            world,
            skybox,
            camera,
            players,
            stages,
            ui_textures,
        }
    }
}

impl GameScene for InGameFinishEnterScene {}
