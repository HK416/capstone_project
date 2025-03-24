use std::error::Error;

use mod_app::scene::GameScene;
use mod_network::components::{FormationPhasePlayer, LoginToken, Team, UserId};
use winit::window::Window;

use crate::config::Locale;

/// 인 게임 장면에 진입하기 전 캐릭터를 편성하는 장면입니다.  
pub struct CharacterFormationScene {
    /// 애플리케이션 표시 언어
    locale: Locale,
    /// 현재 사용자 식별자
    user_id: UserId,
    /// 로그인 토큰
    token: LoginToken,

    /// 캐릭터 편성까지 남은 시간
    remaining_time_sec: f32,

    /// 파란 팀에 속한 플레이어 집합
    blue_team_players: Vec<FormationPhasePlayer>,
    /// 빨간 팀에 속한 플레이어 집합
    red_team_players: Vec<FormationPhasePlayer>,
}

impl CharacterFormationScene {
    pub fn new(
        locale: Locale,
        user_id: UserId,
        token: LoginToken,
        remaining_time_sec: f32,
        players: Vec<FormationPhasePlayer>,
    ) -> Self {
        let (blue_team_players, red_team_players) = players
            .into_iter()
            .partition(|player| player.team() == Team::Blue);
        Self {
            locale,
            user_id,
            token,
            remaining_time_sec,
            blue_team_players,
            red_team_players,
        }
    }
}

impl GameScene for CharacterFormationScene {
    fn on_draw(
        &self,
        _window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target_view: &wgpu::TextureView,
        _depth_buffer_view: &wgpu::TextureView,
        _app: &dyn mod_app::app::AppHandle,
    ) -> Result<(), Box<dyn Error + Send>> {
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!(
                    "RenderPass({})",
                    stringify!(CharacterFormationScene)
                )),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    view: render_target_view,
                    resolve_target: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        Ok(())
    }
}
