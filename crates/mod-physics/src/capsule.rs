use mod_math::{Segment, Line};


/// height 및 radius가 음수인 경우는 고려하지 않는다.  
/// direction은 단위벡터이다.  
/// height == 2 * radius이면 캡슐은 구와 같은 모양이 되는데, 구의 충돌체크로 최적화 하지는 않는다.  
/// height <= 2 * radius이면 캡슐에 기둥이 없고 위아래가 둥근 UFO형태가 되는데, 
/// 이때 일부 함수가 제대로 동작하지 않을 수 있다.  
#[derive(Debug, Clone)]
pub struct Capsule {
    pub center: glam::Vec3,    // 캡슐의 가장 아래 부분
    direction: Option<glam::Vec3>,     // 캡슐의 윗부분이 향하는 방향(default: Y축)
    pub height: f32,            // 캡슐의 전체 높이(direction방향으로 뻗은 길이) 
    pub radius: f32,
}

impl Capsule {
    pub fn new(center: glam::Vec3, height: f32, radius: f32) -> Capsule {
        Self {
            center,
            direction: Some(glam::Vec3::Y),
            height,
            radius,
        }
    }

    /// direction을 단위벡터로 만들어 Capsule을 생성한다.  
    /// 영벡터가 주어지면 Error를 반환한다.  
    pub fn new_rotated(center: glam::Vec3, direction: glam::Vec3, height: f32, radius: f32) -> Result<Self, &'static str> {
        match direction.try_normalize() {
            Some(direction) => Ok(Self {
                center,
                direction: Some(direction),
                height,
                radius,
            }),
            None => Err("Direction cannot be zero vector")
        }
    }
    
    /// direction을 단위벡터로 만들어 Capsule의 방향을 설정한다.  
    /// 영벡터가 주어지면 Error를 반환한다.  
    pub fn set_direction(&mut self, direction: glam::Vec3A) -> Result<(), &'static str> {
        match direction.try_normalize() {
            Some(direction) => {
                self.direction = Some(direction.into());
                Ok(())
            },
            None => Err("Direction cannot be zero vector")
        }
    }

    pub fn direction(&self) -> Option<glam::Vec3> {
        self.direction
    }

    /// 기존 캡슐보다 amount만큼 확장된 새로운 캡슐을 생성한다.  
    /// - center는 amount만큼 -direction방향으로 이동한다.
    /// - direction은 그대로 유지된다.
    /// - height는 2 * amount만큼 증가한다.
    /// - radius는 amount만큼 증가한다.
    pub fn inflated(&self, amount: f32) -> Capsule {
        let direction = match self.direction {
            Some(direction) => direction,
            None => glam::Vec3::Y,
        };

        Capsule {
            center: self.center - direction * amount,
            direction: Some(direction),
            height: self.height + 2.0 * amount,
            radius: self.radius + amount,
        }
    }

    /// 캡슐이 UFO형태인 경우에도 제대로 동작한다.
    pub fn check_point_collision(&self, point: &glam::Vec3A) -> bool {
        let center = glam::Vec3A::from(self.center);
        let p = point - center;    // center에 대한 point의 상대좌표
        let d = match self.direction {
            Some(d) => glam::Vec3A::from(d),
            None => glam::Vec3A::Y,
        };

        // 캡슐에 대한 point의 상대지역좌표 y
        let y = d.dot(p);
        if y < 0.0 {
            return false;
        }

        // 캡슐에 대한 point의 상대지역좌표 x(원통의 회전은 신경쓰지 않아도 되므로 z무시)
        let x_sq = p.length_squared() - y * y;
        let radius_sq = self.radius.powi(2);
        if x_sq > radius_sq {
            return false;
        }

        let h = (radius_sq - x_sq).sqrt();
        let top_y = self.height - self.radius   + h;
        let bot_y = self.radius                 - h;

        // y축 범위 체크
        bot_y <= y && y <= top_y
    }

    /// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.  
    /// 
    /// 직선과 선분 사이의 최소 거리를 구하는 것과 같다.  
    pub fn check_line_collision(&self, line: &Line) -> bool {
        let center = glam::Vec3A::from(self.center);
        let radius_sq = self.radius.powi(2);
        let direction = match self.direction {
            Some(d) => d,
            None => glam::Vec3::Y,
        };

        if direction == line.direction() {
            return line.distance_to_point_sq(&center) <= radius_sq;
        }
        
        let nearest = self.get_seg().nearest_to_line(line);
        let nearest = glam::Vec3A::from(nearest);
        let distance_sq = line.distance_to_point_sq(&nearest);

        distance_sq <= radius_sq
    }

    /// 캡슐이 UFO형태인 경우는 고려하지 않는다.  
    /// 
    /// 캡슐의 아래 구의 중심과 윗 구의 중심을 이은 선분
    pub fn get_seg(&self) -> Segment {
        let direction = match self.direction {
            Some(d) => glam::Vec3A::from(d),
            None => glam::Vec3A::Y,
        };
        let center = glam::Vec3A::from(self.center);
        let start = center + direction * self.radius;
        let end = center + direction * (self.height - self.radius);

        Segment {
            start: start.into(),
            end: end.into(),
        }
    }
}
