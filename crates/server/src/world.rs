use std::collections::{HashMap, VecDeque};
use mod_network::components::{
    Player,
    Bullet,
    ClientId,
    ObjectId, 
    CharacterKind,
    BulletKind,
    StageKind,
    ActionState,
    MovementState,
    ViewState,
    ActionStateTimer,
    MovementStateTimer,
    ViewStateTimer,
};
use mod_parallelism::collections::Queue;
use mod_physics::{Ray, YCapsule};

use super::formula::movement_formulas as formulas;



pub type WorldPointer = usize;


pub struct World {
    stage_kind: StageKind,

    players: HashMap<ObjectId, Player>,
    player_move_queue: Queue<(ObjectId, f32, f32, f32)>,    // Session에서 플레이어 이동을 추가할 때 사용
    
    alive_bullets: VecDeque<Bullet>,
    new_bullets: Queue<Bullet>,    // Session에서 총알을 추가할 때 사용
    
    // static_objects: Vec<GameObject>,  // 월드의 움직이지 않는 오브젝트(맵, 건물 등)
    // dynamic_objects: HashMap<GameObject>,     // 월드의 움직이는 오브젝트(화물 등)
}

impl World {
    pub fn new(stage_kind: StageKind) -> Self {
        Self {
            stage_kind,

            players: HashMap::new(),
            player_move_queue: Queue::new(),

            alive_bullets: VecDeque::new(),
            new_bullets: Queue::new(),
        }
    }


    pub fn get_stage_kind(&self) -> StageKind {
        self.stage_kind
    }

    pub fn add_player(&mut self, object_id: ObjectId, character_kind: CharacterKind) {
        self.players.insert(
            object_id, 
            Player { 
                object_id, 
                character_kind, 
                ..Default::default() 
            }
        );
    }
    
    pub fn remove_player(&mut self, object_id: ObjectId) {
        self.players.remove(&object_id);
    }

    /// 클라이언트에서 보내온 플레이어 정보로 업데이트
    pub fn update_player(
        &mut self, 
        player_id: ObjectId,
        rotation: [f32; 4], 
        action_state: ActionState, 
        movement_state: MovementState, 
        view_state: ViewState, 
        action_state_timer: ActionStateTimer, 
        movement_state_timer: MovementStateTimer, 
        view_state_timer: ViewStateTimer,
    ) {
        if let Some(player) = self.players.get_mut(&player_id) {
            player.rotation = rotation;
            player.action_state = action_state;
            player.movement_state = movement_state;
            player.view_state = view_state;
            player.action_state_timer = action_state_timer;
            player.movement_state_timer = movement_state_timer;
            player.view_state_timer = view_state_timer;
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


    pub fn add_bullet(
        &mut self, 
        object_id: ObjectId,
        shooter_id: ClientId,
        direction: glam::Vec3A,
        rotation: glam::Quat,
    ) {
        if let Some(player) = self.players.get_mut(&shooter_id.into()) {
            const BULLET_SPEED: f32 = 50.0;
            let velocity = direction.normalize() * BULLET_SPEED;
            let velocity = velocity.to_array();
            let rotation = rotation.to_array();
            self.new_bullets.push(
                Bullet {
                    object_id,
                    shooter_id,
                    bullet_kind: BulletKind::default(),
                    translation: player.translation,
                    rotation,
                    velocity,
                    remaining_distance: 700.0,
                }
            );
        }
    }

    pub fn get_bullets(&self) -> Vec<Bullet> {
        self.alive_bullets.iter()
            .map(|bullet| bullet.clone())
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
        while let Some(bullet) = self.new_bullets.pop() {
            self.alive_bullets.push_back(bullet);
        }

        // 플레이어 이동 처리
        while let Some((id, x, y, z)) = self.player_move_queue.pop() {
            let p = self.players.get_mut(&id);
            if let Some(p) = p {
                p.velocity[0] = x;
                p.velocity[1] = y;
                p.velocity[2] = z;
            }
        }

        self.players.values_mut().for_each(|player| {
            player.translation[0] += player.velocity[0] * elapsed;
            player.translation[1] += player.velocity[1] * elapsed;
            player.translation[2] += player.velocity[2] * elapsed;
        });

        // TODO: 플레이어 - 지형 충돌처리
        // ...

        for bullet in self.alive_bullets.iter_mut() {
            let translation = glam::Vec3A::from(bullet.translation);
            let velocity = glam::Vec3A::from(bullet.velocity);
            let move_distance = velocity.length() * elapsed;

            // bullet.velocity가 영벡터가 아니라고 가정
            let ray = Ray::build(translation, velocity).unwrap();
            let bullet_position = translation;

            // 거리 한계를 넘어가면 충돌체크 하지 않음(+1.0은 여유 거리)
            let dist_limit_sq = velocity.length_squared() * 1.0;

            let mut nearest_distance = f32::MAX;
            let mut nearest_player_id = None;
            
            for player in self.players.values() {
                if player.object_id == bullet.shooter_id {
                    continue;
                }
                
                let player_position = glam::Vec3A::from(player.translation);

                // NOTE: 이부분은 나중에 글로벌상수로 따로 정의하는게 좋아보이는데, 테스트를 위해 일단 여기에 작성
                const BULLET_RADIUS: f32 = 0.15;
                const PLAYER_RADIUS: f32 = 1.0;
                const PLAYER_HEIGHT: f32 = 2.5;

                let dist_sq = (bullet_position - player_position).length_squared();
                if dist_sq > dist_limit_sq || dist_sq > bullet.remaining_distance.powi(2) {
                    continue;
                }
                
                // 충돌 처리: 플레이어 - 총알
                // 플레이어의 충돌체: YCapsule(총알의 크기 만큼 확대)           나중에 세분화
                // 총알은 점으로 raycasting
                
                let mut center = player.translation;
                center[1] -= BULLET_RADIUS;

                // mod-network의 Player에 make_collider()를 추가해서 클라이언트에서도 표시할 수 있도록 해도 좋아보임.
                let player_capsule = YCapsule {
                    center: glam::Vec3::from_array(center),
                    radius: PLAYER_RADIUS + BULLET_RADIUS,
                    height: PLAYER_HEIGHT + BULLET_RADIUS * 2.0,
                };

                if let Some(dist) = ray.intersect(&player_capsule) {
                    println!("Bullet find player (player id: {:?})", player.object_id);
                    if dist < nearest_distance {
                        nearest_distance = dist;
                        nearest_player_id = Some(player.object_id);
                    }
                }
            }

            match nearest_player_id {
                // 충돌했다면
                Some(id) => {
                    // 피격 처리(회피하더라도 일단 총알은 제거)
                    bullet.remaining_distance = 0.0;
                    
                    println!("Player {:?} hit by bullet", id);
                    let player = self.players.get_mut(&id).unwrap();
                    // TODO: 아래 코드처럼 동작해야함
                    // let character_info = get_character_info(player.character_kind).unwrap();
                    // let atk = character_info.attack_power;
                    // let def = character_info.defense_power;

                    // 플레이어 체력은 2000, 
                    // 공격력 200, 방어력 20, 
                    // 명중 수치: 200, 회피 수치: 200, 
                    // 치명 수치: 200, 치명 데미지: 200%, 
                    // 사거리 700
                    // 으로 가정
                    // > 각 식에서의 상수값은 제안서에 있는 값으로 설정

                    // 1. 회피 계산
                    // 2. 기본 데미지 계산
                    // 3. 치명타 계산 
                    // 4. 최종 데미지 계산

                    // 회피 계산
                    let accuracy = 200.0;   // 공격자 명중 수치
                    let evasion = 200.0;    // 피격자 회피 수치
                    let hit_rate = formulas::cal_hit_rate(accuracy, evasion, 100.0);
                    // if rand::random::<f64>() > hit_rate {
                    //     println!("  - miss");
                    //     continue;
                    // }
                    
                    // 데미지 계산
                    let atk = 200.0;        // 공격력
                    let def = 20.0;         // 방어력
                    let dmg = formulas::default_damage(atk, def, 100.0);

                    // 치명타 계산
                    let crit = 200.0;   // 치명 수치
                    let crit_rate = formulas::cal_crt_rate(rand::random::<f64>(), crit, 250.0);
                    if crit_rate == 1.0 {
                        println!("  - critical!");
                    }

                    // 최종 데미지 계산
                    let final_dmg = formulas::final_damage(dmg, hit_rate, crit_rate, 200.0);

                    let hp = &mut player.health_point;
                    hp.0 -= final_dmg as f32;
                    if hp.0 <= 0.0 {
                        hp.0 = 0.0;
                    }
                    println!("  - hp: {:?}(-{})", hp.0, final_dmg);
                },

                // 충돌하지 않았다면
                None => {
                    // 누적 이동거리 증가
                    bullet.remaining_distance -= move_distance;
                    
                    // println!("range: {}, moved: {}", bullet.blob.range, bullet.moved_distance);
                    
                    // 총알 사거리를 넘어가면 총알 제거
                    if bullet.remaining_distance <= 0.0 {
                        println!("Bullet range over");
                    }
                    else {
                        // 총알 위치 이동
                        let t = translation + velocity * elapsed;
                        bullet.translation = t.to_array();
                    }
                }
            }
        }

        // 살아남은 총알만 남김
        self.alive_bullets.retain(|bullet| bullet.remaining_distance > 0.0);
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

    pub fn get_stage_kind(&self) -> StageKind {
        self.as_mut().get_stage_kind()
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
    pub fn update_player(
        &mut self, 
        player_id: ObjectId,
        rotation: [f32; 4], 
        action_state: ActionState, 
        movement_state: MovementState, 
        view_state: ViewState, 
        action_state_timer: ActionStateTimer, 
        movement_state_timer: MovementStateTimer, 
        view_state_timer: ViewStateTimer,
    ) {
        self.as_mut().update_player(
            player_id,
            rotation,
            action_state,
            movement_state,
            view_state,
            action_state_timer,
            movement_state_timer,
            view_state_timer,
        );
    }

    /// 플레이어를 시간 경과에 관계 없이 x, y, z만큼 이동시킨다.
    pub fn move_player(&self, id: ObjectId, x: f32, y: f32, z: f32) {
        self.as_mut().move_player(id, x, y, z);
    }

    pub fn get_players(&self) -> Vec<Player> {
        self.as_mut().get_players()
    }


    pub fn add_bullet(
        &mut self, 
        object_id: ObjectId,
        shooter_id: ClientId,
        direction: glam::Vec3A,
        rotation: glam::Quat,
    ) {
        self.as_mut().add_bullet(object_id, shooter_id, direction, rotation);
    }

    pub fn get_bullets(&self) -> Vec<Bullet> {
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
