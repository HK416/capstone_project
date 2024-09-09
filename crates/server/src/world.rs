use std::collections::HashMap;
use mod_network::Player;     // 플레이어 프로토콜



pub type WorldPointer = usize;


pub struct World {
    players: HashMap<u32, Player>,
}

impl World {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
        }
    }


    pub fn add_player(&mut self, id: u32) {
        self.players.insert(id, Player { id, ..Default::default() });
    }

    pub fn move_player(&mut self, id: u32, x: f32, y: f32, z: f32) {
        if let Some(player) = self.players.get_mut(&id) {
            player.translation.x += x;
            player.translation.y += y;
            player.translation.z += z;
        }
    }

    pub fn remove_player(&mut self, id: u32) {
        self.players.remove(&id);
    }

    pub fn get_objects(&self) -> Vec<Player> {
        self.players.values()
            .cloned()
            .collect()
    }
}

impl Into<WorldPointer> for &World {
    fn into(self) -> WorldPointer {
        self as *const World as WorldPointer
    }
}



/// Mutex를 적용하면 read할때도 lock을 걸어야 하기 때문에 사용하지 않음.
pub struct WorldInterface {
    /// raw pointer가 future간 이동이 안돼서, usize타입으로 변환하여 사용
    world: WorldPointer,
}

impl WorldInterface {
    pub fn new(world: WorldPointer) -> Self {
        Self { 
            world: world as WorldPointer, 
        }
    }

    /// id가 겹치지 않음을 사용하는쪽에서 보장해야 함.
    /// 보장하더라도 해싱된 결과가 같으면 충돌이 발생할 수 있음.
    /// 1. mpsc를 사용해서 한 스레드에서만 add/remove를 수행하도록 한다.
    /// 2. lockfree HashMap을 사용한다.
    /// 3. 배열을 사용한다. (Vec<Option<Player>> 또는 [Option<Player>; MAX_PLAYER])
    pub async fn add_player(&self, id: u32) {
        self.as_mut().add_player(id);
    }

    pub async fn move_player(&self, id: u32, x: f32, y: f32, z: f32) {
        self.as_mut().move_player(id, x, y, z);
    }

    pub async fn remove_player(&self, id: u32) {
        self.as_mut().remove_player(id);
    }

    pub fn get_objects(&self) -> Vec<Player> {
        self.as_mut().get_objects()
    }

    fn as_mut(&self) -> &mut World {
        unsafe { 
            &mut *(self.world as *mut World)
        }
    }
}
