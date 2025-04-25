mod enter;

use ahash::HashMap;
use hecs::{Entity, World};
use mod_app::{app::AppHandle, scene::GameScene};
use mod_network::components::{FinishPhasePlayer, LoginToken, Team, UserId};
use winit::window::Window;

use crate::{component::Skybox, config::Locale};

pub use self::enter::*;

/// 게임 장면의 최대 지속 시간입니다.
const MAX_SCENE_DURATION: f32 = 10.0;

/// 인게임 장면의 결과를 보여주는 장면입니다.
pub struct InGameResultScene {
    /// 애플리케이션 표시 언어입니다.
    locale: Locale,
    /// 현재 사용자 식별자입니다.
    user_id: UserId,
    /// 로그인 토큰입니다.
    token: LoginToken,

    /// 승리 팀
    winner: Team,
    /// 게임 장면의 남은 시간
    remaining_time_sec: f32,

    ///엔터티를 관리하는 월드 객체입니다.
    world: World,
    /// 스카이박스입니다.
    skybox: Skybox,
    /// 게임 결과 장면의 메인 카메라 엔터티입니다.
    camera: Entity,
    /// 플레이어 엔터티 집합입니다.
    players: HashMap<UserId, Entity>,
    /// 스테이지 엔터티 집합입니다.
    stages: Vec<Entity>,

    /// 게임 인터페이스 레이아웃 텍스처 식별자입니다.
    ui_textures: HashMap<String, egui::load::SizedTexture>,
}

impl InGameResultScene {
    /// 새로운 `InGameResultScene`을 생성합니다.
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        winner: Team,
        remaining_time_sec: Option<f32>,
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
            world,
            winner,
            remaining_time_sec: MAX_SCENE_DURATION,
            skybox,
            camera,
            players,
            stages,
            ui_textures,
        }
    }
}

impl GameScene for InGameResultScene {
    fn on_enter(&mut self, window: &Window, app: &dyn AppHandle) {}

    fn on_update(&mut self, elapsed_time_sec: f32, window: &Window, app: &dyn AppHandle) {
        // 남은 시간을 갱신합니다.
        self.remaining_time_sec = (self.remaining_time_sec - elapsed_time_sec).max(0.0);

        if self.remaining_time_sec <= 0.0 {
            todo!()
        }
    }
}
