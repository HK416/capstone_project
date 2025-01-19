//게임 내 공식

mod movement_formulas {
    // 이동 속력 계산 함수
    pub fn cal_speed(t_duration: f64, s_move_speed: f64) -> (f64, f64, f64) {
        let t = 2.0 * f64::min(0.5, t_duration);

        let s_acceleration = s_move_speed * (3.0 * t.powi(2) - 2.0 * t.powi(3));
        let s_deceleration = s_move_speed * (1.0 - 3.0 * t.powi(2) + 2.0 * t.powi(3));

        (t, s_acceleration, s_deceleration)
    }

    // 이동 거리 계산 함수
    pub fn cal_distance(
        t_elapsed: f64,
        force_total: f64,
        mass: f64,
        current_speed: f64,
        direction: f64,
    ) -> (f64, f64, f64) {
        // 총 가속도 계산
        let a_total = force_total / mass;

        // 속도 계산
        let v_speed = a_total * t_elapsed + direction * current_speed;

        // 이동 거리 계산
        let d_distance = v_speed * t_elapsed;

        (a_total, v_speed, d_distance)
    }

    //========damage==========

    // 기본 데미지 계산 함수
    pub fn default_damage(attack: f64, defense: f64, k: f64) -> f64 {
        attack * defense / (defense + k)
    }

    // 명중 확률 계산 함수
    pub fn cal_hit_rate(hit: f64, evasion: f64, d: f64) -> f64 {
        hit / (hit + evasion + d)
    }

    // 치명타 확률 계산 함수
    pub fn cal_crt_rate(random_value: f64, critical: f64, c: f64) -> f64 {
        (random_value * (critical / (critical + c))).ceil()
    }

    // 최종 데미지 계산 함수
    pub fn final_damage(
        default_damage: f64,
        hit_rate: f64,
        crt_rate: f64,
        crt_damage: f64,
    ) -> f64 {
        default_damage * hit_rate * (1.0 + crt_rate * ((crt_damage / 100.0) - 1.0))
    }

    //==============게임목표===============

    // 초당 거점 포인트 상승량 계산 함수
    pub fn cal_point_gain() -> f64 {
        1.0 // 포인트 상승량: 1 Pt/Sec
    }

    // 추가 시간 제공 계산 함수
    pub fn cal_additional_time(current_points: f64) -> (f64, f64) {
        let delta = (current_points + 60.0) / (2.0 * 60.0);

        let f = |x: f64| {
            if (0.0..=0.35).contains(&x) || (0.65..=1.0).contains(&x) {
                0.0
            } else {
                (x * std::f64::consts::PI).sin()
            }
        };

        (delta, f(delta))
    }
}
