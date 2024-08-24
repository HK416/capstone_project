use std::{collections::VecDeque, error::Error};

use hecs::World;
use winit::window::Window;

use crate::{AppHandle, GameScene, GameSceneFlow};



/// 생성된 게임 장면을 관리합니다.
pub struct GameSceneStack(VecDeque<Box<dyn GameScene>>);

impl GameSceneStack {
    /// 새로운 게임 장면 스택을 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self(VecDeque::with_capacity(8))
    }

    /// 현재 게임 장면을 가져옵니다. 
    /// 현재 게임 장면이 없는 경우 `None`을 반환합니다.
    #[inline]
    #[must_use]
    pub fn top(&self) -> Option<&Box<dyn GameScene>> {
        self.0.back()
    }

    /// 현재 게임 장면을 가져옵니다.
    /// 현재 게임 장면이 없는 경우 `None`을 반환합니다.
    #[inline]
    #[must_use]
    pub fn top_mut(&mut self) -> Option<&mut Box<dyn GameScene>> {
        self.0.back_mut()
    }

    /// 게임 장면 스택에 있는 모든 게임 장면을 제거합니다.
    pub fn clear(
        &mut self, 
        window: Option<&Window>, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        while let Some(mut scene) = self.0.pop_back() {
            scene.on_exit(window, world, app)?;
        }
        Ok(())
    }

    /// 주어진 게임 장면 흐름에 따라 게임 장면 스택을 갱신합니다.
    pub fn flush(
        &mut self, 
        flow: &mut Option<GameSceneFlow>, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        if let Some(flow) = flow.take() {
            return match flow {
                GameSceneFlow::Clear => self.clear(Some(window), world, app), 
                GameSceneFlow::Reset(new_scene) => self.reset_scene(window, world, app, new_scene), 
                GameSceneFlow::Change(new_scene) => self.change_scene(window, world, app, new_scene), 
                GameSceneFlow::Push(new_scene) => self.push_scene(window, world, app, new_scene), 
                GameSceneFlow::Pop => self.pop_scene(window, world, app), 
            };
        };
        Ok(())
    }
}

impl GameSceneStack {
    /// 모든 게임 장면을 제거하고, 새로운 게임 장면을 추가합니다.
    fn reset_scene(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle, 
        new_scene: Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error + Send>> {
        self.clear(Some(window), world, app)?;
        self.push_scene(window, world, app, new_scene)?;
        Ok(())
    }

    /// 현재 게임 장면을 제거하고, 새로운 게임 장면을 추가합니다.
    fn change_scene(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle, 
        new_scene: Box<dyn GameScene>
    ) -> Result<(), Box<dyn Error + Send>> {
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
    ) -> Result<(), Box<dyn Error + Send>> {
        new_scene.on_enter(window, world, app)?;
        self.0.push_back(new_scene);
        Ok(())
    }

    /// 현재 장면을 정리하고, 제거합니다.
    fn pop_scene(
        &mut self, 
        window: &Window, 
        world: &mut World, 
        app: &dyn AppHandle
    ) -> Result<(), Box<dyn Error + Send>> {
        if let Some(mut curr_scene) = self.0.pop_back() {
            curr_scene.on_exit(Some(window), world, app)?;
        }
        Ok(())
    }
}
