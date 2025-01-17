use std::collections::{HashMap, VecDeque};
use mod_network::{
    Player,
    BulletBlob,
    components::{
        ObjectId, 
        CharacterKind
    },
};
use mod_parallelism::collections::Queue;
use mod_physics::{Ray, YCapsule};



pub type WorldPointer = usize;


struct Bullet {
    blob: BulletBlob,       // 총알의 기본 정보
    alive: bool,            // 충돌하거나 사거리를 넘어가면 false로 변경
    moved_distance: f32,    // 총알의 이동거리
}

impl Bullet {
    fn new(blob: BulletBlob) -> Self {
        Self {
            blob,
            alive: true,
            moved_distance: 0.0,
        }
    }
}


pub struct World {
    players: HashMap<ObjectId, Player>,
    player_move_queue: Queue<(ObjectId, f32, f32, f32)>,    // Session에서 플레이어 이동을 추가할 때 사용
    
    alive_bullets: VecDeque<Bullet>,    // get_objects에서 Queue::pop을 하지 않기 위해 사용, 중간값 삭제가 빈번할것으로 예상되어 VecDeque로 사용
    bullet_blobs: Queue<BulletBlob>,        // Session에서 총알을 추가할 때 사용
}

impl World {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
            player_move_queue: Queue::new(),

            alive_bullets: VecDeque::new(),
            bullet_blobs: Queue::new(),
        }
    }


    pub fn add_player(&mut self, id: ObjectId, character_kind: CharacterKind) {
        self.players.insert(id, Player { id, character_kind, ..Default::default() });
    }
    
    pub fn remove_player(&mut self, id: ObjectId) {
        self.players.remove(&id);
    }

    /// 클라이언트에서 보내온 플레이어 정보로 업데이트
    pub fn update_player(&mut self, player: Player) {
        if let Some(old_player) = self.players.get_mut(&player.id) {
            old_player.rotation = player.rotation;
            old_player.action_state = player.action_state;
            old_player.movement_state = player.movement_state;
            old_player.view_state = player.view_state;
            old_player.action_state_timer = player.action_state_timer;
            old_player.movement_state_timer = player.movement_state_timer;
            old_player.view_state_timer = player.view_state_timer;
        }
    }

    /// 플레이어를 시간 경과에 관계 없이 x, y, z만큼 이동시킨다.
    pub fn move_player(&mut self, id: ObjectId, x: f32, y: f32, z: f32) {
        if let Some(player) = self.players.get_mut(&id) {
            player.translation[0] += x;
            player.translation[1] += y;
            player.translation[2] += z;
        }
    }

    pub fn get_players(&self) -> Vec<Player> {
        self.players.values()
            .cloned()
            .collect()
    }


    pub fn add_bullet(&mut self, bullet: BulletBlob) {
        self.bullet_blobs.push(bullet);
    }

    pub fn get_bullets(&self) -> Vec<BulletBlob> {
        self.alive_bullets.iter()
            .map(|bullet| bullet.blob.clone())
            .collect()
    }


    /// 플레이어 이동 정보 추가
    pub fn push_move_data(&mut self, id: ObjectId, x: f32, y: f32, z: f32) {
        self.player_move_queue.push((id, x, y, z));
    }


    /// update_loop에서 호출하는 월드 업데이트 함수  
    /// 
    /// - 총알 이동
    /// - 플레이어 이동
    /// - 충돌처리
    async fn update(&mut self, elapsed: tokio::time::Duration) {
        let elapsed = elapsed.as_secs_f32();
    
        // 받은 총알을 alive_bullets로 이동
        while let Some(bullet) = self.bullet_blobs.pop() {
            self.alive_bullets.push_back(Bullet::new(bullet));
        }

        // 플레이어 이동 처리
        while let Some((id, x, y, z)) = self.player_move_queue.pop() {
            self.move_player(id, x * elapsed, y * elapsed, z * elapsed);
        }

        for bullet in self.alive_bullets.iter_mut() {
            let move_distance = bullet.blob.speed * elapsed;

            // bullet.direction이 영벡터가 아니라고 가정
            let ray = Ray::build(bullet.blob.translation, gmm::Vector::from(bullet.blob.direction)).unwrap();
            let bullet_position = gmm::Vector::from(bullet.blob.translation);

            // 거리 한계를 넘어가면 충돌체크 하지 않음(+1.0은 여유 거리)
            let dist_limit_sq = (move_distance + 1.0).powi(2);

            let mut nearest_distance = f32::MAX;
            let mut nearest_player_id = None;
            
            for player in self.players.values() {
                if player.id == bullet.blob.shooter {
                    continue;
                }
                
                let player_position = gmm::Float3::from_array(player.translation);
                let player_position = gmm::Vector::from(player_position);

                // NOTE: 이부분은 나중에 글로벌상수로 따로 정의하는게 좋아보이는데, 테스트를 위해 일단 여기에 작성
                const BULLET_RADIUS: f32 = 0.5;
                const PLAYER_RADIUS: f32 = 1.0;
                const PLAYER_HEIGHT: f32 = 2.5;

                if (bullet_position - player_position).vec3_len_sq() > dist_limit_sq {
                    continue;
                }
                
                // 충돌 처리: 플레이어 - 총알
                // 플레이어의 충돌체: YCapsule(총알의 크기 만큼 확대)           나중에 세분화
                // 총알은 점으로 raycasting
                
                let mut center = player.translation;
                center[1] -= BULLET_RADIUS;

                // mod-network의 Player에 make_collider()를 추가해서 클라이언트에서도 표시할 수 있도록 해도 좋아보임.
                let player_capsule = YCapsule {
                    center: gmm::Float3::from_array(center),
                    radius: PLAYER_RADIUS + BULLET_RADIUS,
                    height: PLAYER_HEIGHT + BULLET_RADIUS * 2.0,
                };

                if let Some(dist) = ray.intersect(&player_capsule) {
                    println!("Bullet find player (player id: {:?})", player.id);
                    if dist < nearest_distance {
                        nearest_distance = dist;
                        nearest_player_id = Some(player.id);
                    }
                }
            }

            match nearest_player_id {
                Some(id) => {
                    // 총알 제거 -> pop했으므로 제거됨
                    // TODO: 플레이어에게 피해를 줌
                    // 해당 Session은 클라이언트에게 피해를 받았다는 패킷을 보내야함
                    println!("Player {:?} hit by bullet", id);
                    let hp = &mut self.players.get_mut(&id).unwrap().hp;
                    if *hp <= 40 {
                        *hp = 0;
                    }
                    else {
                        *hp -= 40;
                    }
                    println!("Player {:?} hp: {}", id, *hp);
                    bullet.alive = false;
                },

                // 충돌하지 않았다면
                None => {
                    // 누적 이동거리 증가
                    bullet.moved_distance += move_distance;
                    
                    // println!("range: {}, moved: {}", bullet.blob.range, bullet.moved_distance);
                    
                    // 총알 사거리를 넘어가면 총알 제거
                    if bullet.moved_distance >= bullet.blob.range {
                        println!("Bullet range over");
                        bullet.alive = false;
                    }
                    else {
                        // 총알 위치 이동
                        bullet.blob.translation += bullet.blob.direction * move_distance;
                    }
                }
            }
        }

        // 살아남은 총알만 남김
        self.alive_bullets.retain(|bullet| bullet.alive);
    }

    /// 업데이트 루프 실행  
    pub async fn update_loop(&mut self) {
        let mut timer = tokio::time::Instant::now();
        
        loop {
            // 경과 시간 계산
            let elapsed = timer.elapsed();
            timer = tokio::time::Instant::now();

            self.update(elapsed).await;

            // Tokio에서 loop가 비동기 함수 내에서 사용될 경우
            // 다른 작업이 실행될 수 없을 수 있다.
            // 
            // 이 때문에 Tokio 내부적으로 일정 실행 시간을 넘길 경우 tokio::task::yield_now를 사용하는 것으로 추측된다.
            // 다만 tokio::task::yield_now의 경우 다시 실행되는 시기를 모른다.
            // (아마 시스템 인터럽트가 발생할 경우 실행되는 것으로 추측됨)
            //
            // 따라서 tokio::time::sleep으로 잠시 실행을 중단.
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
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
    /// 2. lockfree HashMap을 사용한다.
    /// 3. 배열을 사용한다. (Vec<Option<Player>> 또는 [Option<Player>; MAX_PLAYER])     >>>>>>> 오브젝트용 HashMap과 플레이어용 배열을 따로 관리해야한다.
    pub fn add_player(&self, id: ObjectId, character_kind: CharacterKind) {
        self.as_mut().add_player(id, character_kind);
    }

    pub fn remove_player(&self, id: ObjectId) {
        self.as_mut().remove_player(id);
    }

    /// 클라이언트에서 보내온 플레이어 정보로 업데이트
    pub fn update_player(&self, player: Player) {
        self.as_mut().update_player(player);
    }

    /// 플레이어를 시간 경과에 관계 없이 x, y, z만큼 이동시킨다.
    pub fn move_player(&self, id: ObjectId, x: f32, y: f32, z: f32) {
        self.as_mut().move_player(id, x, y, z);
    }

    pub fn get_players(&self) -> Vec<Player> {
        self.as_mut().get_players()
    }


    pub fn add_bullet(&self, bullet: BulletBlob) {
        self.as_mut().add_bullet(bullet);
    }

    pub fn get_bullets(&self) -> Vec<BulletBlob> {
        self.as_mut().get_bullets()
    }


    /// 플레이어 이동 정보 추가
    pub fn push_move_data(&self, id: ObjectId, x: f32, y: f32, z: f32) {
        self.as_mut().push_move_data(id, x, y, z);
    }


    fn as_mut(&self) -> &mut World {
        unsafe { 
            &mut *(self.world as *mut World)
        }
    }
}
