use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

use hecs::World;
use mod_util::AppHandle;
use winit::window::Window;

use crate::ControlFlow;
use crate::GameScene;



/// 생성된 게임 장면을 관리하는 관리자 입니다.
pub struct SceneManager {
    /// 게임 장면을 데이터를 담고있습니다.
    stack: VecDeque<Box<dyn GameScene>>, 

    /// 게임 장면 매니저의 제어자입니다.
    control_flow: Option<ControlFlow>, 
}

impl SceneManager {
    /// 주어진 `start_scene`으로 시작하는 
    /// 새로운 게임 장면 관리자를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new(start_scene: Box<dyn GameScene>) -> Self {
        Self { 
            stack: VecDeque::with_capacity(16), 
            control_flow: Some(ControlFlow::Push(start_scene)), 
        }
    }

    /// 게임 장면 관리자의 제어자를 설정합니다.
    /// 이미 설정된 제어자가 존재하는 경우 기존의 제어자를 반환하고, 새로운 제어자를 설정합니다.
    /// 
    /// 설정된 제어자가 바로 반영되지 않습니다.
    /// 게임 관리자의 `flush`함수가 호출될 때 제어자가 반영됩니다.
    /// 
    #[inline]
    pub fn set_control_flow(&mut self, control_flow: ControlFlow) -> Option<ControlFlow> {
        self.control_flow.replace(control_flow)
    }

    /// 현재 게임 장면을 가져옵니다.
    /// 현제 게임 장면이 없는 경우 `None`을 반환합니다.
    #[inline]
    #[must_use]
    pub fn top(&mut self) -> Option<&mut Box<dyn GameScene>> {
        self.stack.back_mut()
    }

    /// 게임 장면 관리자에 설정된 제어자에 따라 관리자를 갱신합니다.
    pub fn flush(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        if let Some(control_flow) = self.control_flow.take() {
            return match control_flow {
                ControlFlow::Clear => self.clear(Some(window), world, app), 
                ControlFlow::Reset(new_scene) => self.reset_scene(window, world, app, new_scene), 
                ControlFlow::Change(new_scene) => self.change_scene(window, world, app, new_scene), 
                ControlFlow::Push(new_scene) => self.push_scene(window, world, app, new_scene), 
                ControlFlow::Pop => self.pop_scene(window, world, app), 
            };
        };
        Ok(())
    }

    /// 게임 관리자에 있는 모든 게임 장면을 제거합니다.
    pub fn clear(
        &mut self, 
        window: Option<&Window>, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        while let Some(mut scene) = self.stack.pop_back() {
            scene.on_exit(window, world, app)?;
        }
        Ok(())
    }
}

impl SceneManager {
    /// 모든 게임 장면을 제거하고 새로운 장면을 추가합니다.
    fn reset_scene(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle, 
        new_scene: Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error>> {
        self.clear(Some(window), world, app)?;
        self.push_scene(window, world, app, new_scene)?;
        Ok(())
    }

    /// 현재 게임 장면을 제거하고 새로운 장면을 추가합니다.
    fn change_scene(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle, 
        new_scene: Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error>> {
        self.pop_scene(window, world, app)?;
        self.push_scene(window, world, app, new_scene)?;
        Ok(())
    }

    /// 새로운 장면을 초기화하고, 추가합니다.
    fn push_scene(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle, 
        mut new_scene: Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error>> {
        new_scene.on_enter(window, world, app)?;
        self.stack.push_back(new_scene);
        Ok(())
    }

    /// 현재 장면을 정리하고, 제거합니다.
    fn pop_scene(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error>> {
        if let Some(mut scene) = self.stack.pop_back() {
            scene.on_exit(Some(window), world, app)?;
        }
        Ok(())
    }
}

impl fmt::Debug for SceneManager {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(SceneManager))
            .field("Current Scene", &self.stack.back())
            .finish()
    }
}
