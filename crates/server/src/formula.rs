//게임 내 공식 파일
impl movement_formulas {
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

    //==========damage=================

    //기본
    pub fn default_damage(attack: f64, defense: f64, k:f64,) -> f64{
        attack*defense/(defense+k)
    }

    //명중확률
    pub fn cal_hitrate(hit:f64, evasion: f64, d: f64) -> f64{
        hit/ (hit+evasion+d)
    }

    //치명타 확률
    pub fn cal_crtrate (random_value: f64, crt: f64, c:f64)-> f64{
        (random_value*(crt/(crt+c))).ceil()
    }

    //최종 데미지 계산
    pub fn final_damage(
        default_dam: f64,
        hit_rate: f64,
        crt_rate: f64,
        crt_dam: f64
    ) -> f64{
        default_dam * hit_rate * (1.0 + crt_rate * ((crt_dam/100.0)-1.0))
    }
    
    //회복량 공식식
    pub fn cal_heal(
        heal_rate: f64,
        target_heal_rate: f64,
        target_max_hp: f64,
        h: f64,
        n: f64
    ) -> f64{
        heal_rate * (1 + (target_heal_rate/(target_max_hp+h)) * (1/n))
    }
        

}





