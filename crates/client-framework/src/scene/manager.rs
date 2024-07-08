use super::GameScene;
use super::control_flow::ControlFlow;
use crate::app::Application;
use crate::error::AppError;

use std::fmt;
use std::collections::VecDeque;
use hecs::World;

/// 고정 시간 갱신 함수에서 사용되는 경과 시간입니다.
/// 초당 60번의 횟수로 갱신합니다.
pub const FIXED_TIME_SEC: f32 = 1.0 / 60.0;

/// 고정 시간 갱신 함수의 최대 갱신 횟수입니다.
pub const MAX_FIXED_UPDATE: usize = 30;



/// 게임 장면을 관리하는 관리자 입니다.
pub struct SceneManager {
    /// 게임 장면을 담고있는 `stack` 컨테이너 입니다.
    scene_stack: VecDeque<Box<dyn GameScene>>,

    /// 게임 장면 관리자의 제어자 입니다.
    control_flow: Option<ControlFlow>,

    /// 경과 시간을 저장합니다.
    /// 
    /// 이 맴버는 Fixed Update를 수행할 때 사용됩니다.
    /// 
    elapsed_time_sec: f32,

    /// 게임 Entity를 담고 있는 데이터 베이스 입니다.
    world: World,
}

impl SceneManager {
    /// 새로운 게임 장면 관리자를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new(start_scene: Box<dyn GameScene>) -> Self {
        Self { 
            scene_stack: VecDeque::with_capacity(16), 
            control_flow: Some(ControlFlow::Push(start_scene)),
            elapsed_time_sec: 0.0, 
            world: World::new() 
        }
    }

    /// 게임 장면 관리자의 제어자를 설정합니다.
    /// 이미 설정된 값이 있는 경우 그 값을 반환합니다.
    /// 
    /// <b>주의: 설정된 값은 즉시 반영되지 않습니다.</b>
    /// 
    #[inline]
    pub fn set_control_flow(&mut self, control_flow: ControlFlow) -> Option<ControlFlow> {
        self.control_flow.replace(control_flow)
    }

    /// 게임 장면 관리자를 실행합니다.
    pub fn run(&mut self, app: &dyn Application) -> Result<(), AppError> {
        // 현재 게임 장면이 존재할 경우 갱신합니다.
        if let Some(scene) = self.scene_stack.back_mut() {
            let timer = app.ref_timer();
            let elapsed_time_sec = timer.elapsed_time_sec();
            
            // 변동 시간 갱신 함수를 호출합니다.
            scene.on_update(&mut self.world, app, elapsed_time_sec)?;

            // 경과 시간을 갱신합니다.
            self.elapsed_time_sec += elapsed_time_sec;
            
            // 고정 시간 갱신 함수를 호출합니다.
            let mut update_count = 0;
            while self.elapsed_time_sec >= FIXED_TIME_SEC && update_count < MAX_FIXED_UPDATE {
                scene.on_fixed_update(&mut self.world, app, FIXED_TIME_SEC)?;
                self.elapsed_time_sec -= FIXED_TIME_SEC;
                update_count += 1;
            }
        }

        // 장면 관리자의 제어자가 존재할 경우 장면 관리자를 갱신합니다.
        if let Some(control_flow) = self.control_flow.take() {
            match control_flow {
                ControlFlow::Clear => {
                    // `stack`의 모든 게임 장면을 정리하고, 제거합니다.
                    while let Some(mut old_scene) = self.scene_stack.pop_back() {
                        old_scene.on_exit(&mut self.world, app)?;
                    }
                },
                ControlFlow::Reset(mut new_scene) => {
                    // `stack`의 모든 게임 장면을 정리하고, 제거합니다.
                    while let Some(mut old_scene) = self.scene_stack.pop_back() {
                        old_scene.on_exit(&mut self.world, app)?;
                    }

                    // 새로운 게임 장면을 초기화 하고, 추가합니다.
                    new_scene.on_enter(&mut self.world, app)?;
                    self.scene_stack.push_back(new_scene);
                },
                ControlFlow::Change(mut new_scene) => {
                    // `stack`의 현재 장면을 정리하고, 제거합니다.
                    if let Some(mut old_scene) = self.scene_stack.pop_back() {
                        old_scene.on_exit(&mut self.world, app)?;
                    }

                    // 새로운 게임 장면을 초기화 하고, 추가합니다.
                    new_scene.on_enter(&mut self.world, app)?;
                    self.scene_stack.push_back(new_scene);
                },
                ControlFlow::Push(mut new_scene) => {
                    // 새로운 게임 장면을 초기화 하고, 추가합니다.
                    new_scene.on_enter(&mut self.world, app)?;
                    self.scene_stack.push_back(new_scene);
                },
                ControlFlow::Pop => {
                    // `stack`의 현재 장면을 정리하고, 제거합니다.
                    if let Some(mut old_scene) = self.scene_stack.pop_back() {
                        old_scene.on_exit(&mut self.world, app)?;
                    }
                },
            }
        }

        Ok(())
    }
}

impl fmt::Debug for SceneManager {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(stringify!(SceneManager))
            .field("Current Game Scene", &self.scene_stack.back())
            .finish()
    }
}
