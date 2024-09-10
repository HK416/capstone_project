use lockfree::map::Map;
use mod_network::Player;     // 플레이어 프로토콜



pub type WorldPointer = usize;


pub struct World {
    players: Map<u32, Player>,
}

impl World {
    pub fn new() -> Self {
        Self {
            players: Map::new(),
        }
    }


    pub fn add_player(&mut self, id: u32) {
        self.players.insert(id, Player { id, ..Default::default() });
    }

    pub fn remove_player(&mut self, id: u32) {
        self.players.remove(&id);
    }

    pub fn update_player(&mut self, player: Player) {
        self.players.insert(player.id, player);
    }

    pub fn move_player(&mut self, id: u32, x: f32, y: f32, z: f32) {
        let mut player = self.players.get(&id).unwrap().1.clone();
        player.translation.x += x;
        player.translation.y += y;
        player.translation.z += z;
        
        self.update_player(player);
    }

    pub fn get_objects(&self) -> Vec<Player> {
        self.players.iter()
            .map(|player| player.1.clone())
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
            world, 
        }
    }

    /// id가 겹치지 않음을 사용하는쪽에서 보장해야 함.
    /// 보장하더라도 해싱된 결과가 같으면 충돌이 발생할 수 있음.
    /// 1. mpsc를 사용해서 한 스레드에서만 add/remove를 수행하도록 한다.    >>>>>>> 자주 호출되지 않는 add/remove를 위해 task를 하나 할당해줘야함.
    /// 2. lockfree HashMap을 사용한다.     >>>>>>> lockfree::map::Map으로 테스트, 성능확인 아직 안함.
    /// 3. 배열을 사용한다. (Vec<Option<Player>> 또는 [Option<Player>; MAX_PLAYER])     >>>>>>> 오브젝트용 HashMap과 플레이어용 배열을 따로 관리해야한다.
    pub fn add_player(&self, id: u32) {
        self.as_mut().add_player(id);
    }

    pub fn remove_player(&self, id: u32) {
        self.as_mut().remove_player(id);
    }

    pub fn update_player(&self, player: Player) {
        self.as_mut().update_player(player);
    }

    pub fn move_player(&self, id: u32, x: f32, y: f32, z: f32) {
        self.as_mut().move_player(id, x, y, z);
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
