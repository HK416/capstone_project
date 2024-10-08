use std::collections::{HashMap, VecDeque};
use mod_network::{
    Player,
    BulletBlob,
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
    timer: tokio::time::Instant,
    
    players: HashMap<u32, Player>,
    
    alive_bullets: VecDeque<Bullet>,    // get_objects에서 Queue::pop을 하지 않기 위해 사용, 중간값 삭제가 빈번할것으로 예상되어 VecDeque로 사용
    bullet_blobs: Queue<BulletBlob>,        // Session에서 총알을 추가할 때 사용
}

impl World {
    pub fn new() -> Self {
        Self {
            timer: tokio::time::Instant::now(),

            players: HashMap::new(),

            alive_bullets: VecDeque::new(),
            bullet_blobs: Queue::new(),
        }
    }


    pub fn add_player(&mut self, id: u32) {
        self.players.insert(id, Player { id, ..Default::default() });
    }
    
    pub fn remove_player(&mut self, id: u32) {
        self.players.remove(&id);
    }

    pub fn update_player(&mut self, player: Player) {
        if let Some(old_player) = self.players.get_mut(&player.id) {
            *old_player = player;
        }
    }

    pub fn move_player(&mut self, id: u32, x: f32, y: f32, z: f32) {
        if let Some(player) = self.players.get_mut(&id) {
            player.translation.x += x;
            player.translation.y += y;
            player.translation.z += z;
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


    /// 총알 이동 및 충돌 처리
    pub async fn update_loop(&mut self) {
        self.timer = tokio::time::Instant::now();

        let update_time = 1.0 / 30.0;   // 초당 업데이트 횟수 제한(30fps)
        let mut elapsed = 0.0;          // 누적 경과 시간
        
        loop {
            // 경과 시간 추가
            elapsed += self.timer.elapsed().as_secs_f32();
            self.timer = tokio::time::Instant::now();

            // 업데이트 시간이 되지 않았으면 대기
            if elapsed < update_time {
                tokio::task::yield_now().await;     // 루프를 돌면서 cpu를 낭비하지 않도록 양보
                continue;
            }
            
            // 경과 시간을 업데이트 시간만큼 감소(고정된 시간만큼 업데이트하기 위해)
            elapsed -= update_time;

            // 받은 총알을 alive_bullets로 이동
            while let Some(bullet) = self.bullet_blobs.pop() {
                self.alive_bullets.push_back(Bullet::new(bullet));
            }

            for bullet in self.alive_bullets.iter_mut() {
                let move_distance = bullet.blob.speed * update_time;

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
                    
                    let player_position = gmm::Vector::from(player.translation);

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
                    center.y -= BULLET_RADIUS;

                    // mod-network의 Player에 make_collider()를 추가해서 클라이언트에서도 표시할 수 있도록 해도 좋아보임.
                    let player_capsule = YCapsule {
                        center,
                        radius: PLAYER_RADIUS + BULLET_RADIUS,
                        height: PLAYER_HEIGHT + BULLET_RADIUS * 2.0,
                    };

                    if let Some(dist) = ray.intersect(&player_capsule) {
                        println!("Bullet find player (player id: {})", player.id);
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
                        println!("Player {} hit by bullet", id);
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

    pub fn get_players(&self) -> Vec<Player> {
        self.as_mut().get_players()
    }


    pub fn add_bullet(&self, bullet: BulletBlob) {
        self.as_mut().add_bullet(bullet);
    }

    pub fn get_bullets(&self) -> Vec<BulletBlob> {
        self.as_mut().get_bullets()
    }


    fn as_mut(&self) -> &mut World {
        unsafe { 
            &mut *(self.world as *mut World)
        }
    }
}
