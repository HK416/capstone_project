
//게임 내 공식

pub mod movement_formulas {
    use rand::Rng;
    // 이동 거리 계산 함수
    pub fn cal_distance(
        t_elapsed: f32,
        force_total: f32,
        mass: f32,
        current_speed: f32,
        direction: f32,
    ) -> (f32, f32, f32) {
        // 총 가속도 계산
        let a_total = force_total / mass;

        // 속도 계산
        let v_speed = a_total * t_elapsed + direction * current_speed;

        // 이동 거리 계산
        let d_distance = v_speed * t_elapsed;

        (a_total, v_speed, d_distance)
    }

    //========damage==========

    // 기본 데미지 계산 함수 (애니메이션 길이에 따른 가중치 반영)
    pub fn default_damage( attack: f32, defense: f32, k: f32, dur: f32, cnt: f32) -> f32 {
        let mut rng = rand::thread_rng(); // 랜덤 생성기 초기화

        // 발사 횟수에 따른 데미지 감소 (횟수가 많을수록 개별 탄환의 데미지가 감소)
        let adjustment: f32 =6.0;
        let total_damage = (attack * defense / (defense + k)) * (dur * adjustment);
        let damage_per_bullet = total_damage / cnt ;

        let range: f32 = rng.gen_range(0.7..=1.0); // 70% ~ 100% 사이의 난수
        let default_damage = damage_per_bullet * range;
    
        // 총 데미지 리턴
        default_damage
    }

    // 명중 확률 계산 함수
    pub fn cal_hit_rate(hit: f32, evasion: f32, d: f32) -> f32 {
        hit / (hit + evasion + d)
    }

    // 치명타 확률 계산 함수
    pub fn cal_crt_rate(random_value: f32, critical: f32, c: f32) -> f32 {
        (random_value - (critical / (critical + c))).ceil()
    }

    // 최종 데미지 계산 함수
    pub fn final_damage(default_damage: f32, hit_rate: f32, crt_rate: f32, crt_damage: f32) -> f32 {
        default_damage * hit_rate * (1.0 + crt_rate * ((crt_damage / 100.0) - 1.0))
    }

    //==============게임목표===============

    // 초당 거점 포인트 상승량 계산 함수
    pub fn cal_point_gain() -> f32 {
        1.0 // 포인트 상승량: 1 Pt/Sec
    }

    // 추가 시간 제공 계산 함수
    pub fn cal_additional_time(current_points: f32) -> (f32, f32) {
        let delta = (current_points + 60.0) / (2.0 * 60.0);

        let f = |x: f32| {
            if (0.0..=0.35).contains(&x) || (0.65..=1.0).contains(&x) {
                0.0
            } else {
                (x * std::f32::consts::PI).sin()
            }
        };

        (delta, f(delta))
    }
}
