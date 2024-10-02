/// ### Rigid Body
/// 강체에 대한 물리 역학입니다.
/// 
#[derive(Debug, Clone, Copy)]
pub struct RigidBody {
    /// 힘의 총합입니다.
    pub force_accum: gmm::Vector, 

    /// 물체의 가속도입니다.
    pub acceleration: gmm::Vector, 
    
    /// 물체의 속도입니다.
    pub velocity: gmm::Vector, 

    /// 물체 무게의 역수입니다. 
    /// 
    /// 모든 비트가 0인 경우 물체의 무게는 무한대입니다.
    /// 
    inverse_mass: f32,

    /// 마찰력 대신 사용하는 간단한 제동 값 입니다.
    pub damping: f32, 
}

impl RigidBody {
    /// 새로운 강체를 생성합니다.
    /// 
    /// # Panics
    /// 주어진 무게가 0보다 작거나 같을 경우 [`panic!`]을 호출합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn new(mass: Option<f32>) -> Self {
        let inverse_mass = match mass {
            Some(mass) => {
                assert!(mass > 0.0, "The given mass is greater than zero!");
                mass.recip()
            }, 
            None => 0.0
        };
        
        Self { 
            force_accum: gmm::Vector::ZERO, 
            acceleration: gmm::Vector::ZERO, 
            velocity: gmm::Vector::ZERO, 
            inverse_mass, 
            damping: 0.0 
        }
    }

    /// 힘의 총량을 초기화합니다.
    #[inline]
    pub fn reset_force(&mut self) {
        self.force_accum = gmm::Vector::ZERO
    }

    /// 적분을 통해 이동 거리를 계산합니다.
    #[must_use]
    pub fn integral(&mut self, elapsed_time_sec: f32) -> gmm::Vector {
        // 물체의 무계가 무한대일 경우 함수를 실행하지 않습니다.
        if self.inverse_mass == 0.0 {
            return gmm::Vector::ZERO;
        }
        
        // 이동 거리를 구합니다.
        let distance = self.velocity * elapsed_time_sec;

        // 가속도를 구합니다.
        let mut acceleration = self.acceleration;
        acceleration += self.force_accum * self.inverse_mass;

        // 속도를 갱신합니다.
        self.velocity += acceleration * elapsed_time_sec;

        // 저항을 적용합니다.
        self.velocity *= self.damping.powf(elapsed_time_sec);
        if self.velocity.vec3_len() <= f32::EPSILON {
            self.velocity = gmm::Vector::ZERO;
        }

        distance
    }
}
