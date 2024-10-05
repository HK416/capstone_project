use super::GameScene;



/// 게임 장면 스택을 제어합니다.
#[derive(Debug)]
pub enum GameSceneFlow {
    /// 모든 게임 장면을 제거합니다.
    Clear, 

    /// 모든 게임 장면을 제거하고, 주어진 게임 장면을 추가합니다.
    Reset(Box<dyn GameScene>), 

    /// 현재 게임 장면을 제거하고, 주어진 게임 장면을 추가합니다.
    Change(Box<dyn GameScene>), 

    /// 주어진 게임 장면을 추가합니다.
    Push(Box<dyn GameScene>), 

    /// 현재 게임 장면을 제거합니다.
    Pop, 
}
