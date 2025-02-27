use super::Sphere;
use mod_math::{Segment, Line};


/// height 및 radius가 음수인 경우는 고려하지 않는다.  
/// direction은 단위벡터이다.  
/// height == 2 * radius이면 캡슐은 구와 같은 모양이 되는데, 구의 충돌체크로 최적화 하지는 않는다.  
/// height <= 2 * radius이면 캡슐에 기둥이 없고 위아래가 둥근 UFO형태가 되는데, 
/// 이때 일부 함수가 제대로 동작하지 않을 수 있다.  
#[derive(Debug, Clone)]
pub struct Capsule {
    pub center: glam::Vec3,    // 캡슐의 가장 아래 부분
    direction: glam::Vec3,     // 캡슐의 윗부분이 향하는 방향(default: Y축)
    pub height: f32,            // 캡슐의 전체 높이(direction방향으로 뻗은 길이) 
    pub radius: f32,
}

impl Capsule {
    /// direction을 단위벡터로 만들어 Capsule을 생성한다.  
    /// 영벡터가 주어지면 Error를 반환한다.  
    pub fn build(center: glam::Vec3, direction: glam::Vec3, height: f32, radius: f32) -> Result<Self, &'static str> {
        match direction.try_normalize() {
            Some(direction) => Ok(Self { 
                center, 
                direction, 
                height, 
                radius 
            }),
            None => Err("Direction cannot be zero vector")
        }
    }
    
    /// direction을 단위벡터로 만들어 Capsule의 방향을 설정한다.  
    /// 영벡터가 주어지면 Error를 반환한다.  
    pub fn set_direction(&mut self, direction: glam::Vec3A) -> Result<(), &'static str> {
        match direction.try_normalize() {
            Some(direction) => {
                self.direction = direction.into();
                Ok(())
            },
            None => Err("Direction cannot be zero vector")
        }
    }

    pub fn direction(&self) -> glam::Vec3 {
        self.direction
    }

    /// 기존 캡슐보다 radius만큼 확장된 새로운 캡슐을 생성한다.  
    /// - center는 radius만큼 -direction방향으로 이동한다.
    /// - direction은 그대로 유지된다.
    /// - height는 2 * radius만큼 증가한다.
    /// - radius는 radius만큼 증가한다.
    pub fn inflated(&self, radius: f32) -> Capsule {
        Capsule {
            center: self.center - self.direction * radius,
            direction: self.direction,
            height: self.height + 2.0 * radius,
            radius: self.radius + radius,
        }
    }

    /// 캡슐이 UFO형태인 경우에도 제대로 동작한다.
    pub fn check_point_collision(&self, point: &glam::Vec3A) -> bool {
        let center = glam::Vec3A::from(self.center);
        let p = point - center;    // center에 대한 point의 상대좌표
        let d = glam::Vec3A::from(self.direction);

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

        if self.direction == line.direction() {
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
        Segment {
            start: self.center + self.direction * self.radius,
            end: self.center + self.direction * (self.height - self.radius),
        }
    }
}




/// Y축에 정렬된 캡슐  
/// direction은 항상 <0, 1, 0>이다.  
/// height 및 radius가 음수인 경우는 고려하지 않는다.  
/// height == 2 * radius이면 캡슐은 구와 같은 모양이 되는데, 구의 충돌체크로 최적화 하지는 않는다.  
/// height <= 2 * radius이면 캡슐에 기둥이 없고 위아래가 둥근 UFO형태가 되는데, 
/// 이때 일부 함수가 제대로 동작하지 않을 수 있다.  
#[derive(Debug)]
pub struct YCapsule {
    pub center: glam::Vec3,    // 캡슐의 가장 아래 부분
    pub height: f32,            // 캡슐의 전체 높이
    pub radius: f32,
}

impl YCapsule {
    pub fn inflated(&self, radius: f32) -> YCapsule {
        YCapsule {
            center: self.center - glam::Vec3::Y * radius,
            height: self.height + 2.0 * radius,
            radius: self.radius + radius,
        }
    }
    
    /// 캡슐이 UFO형태인 경우에도 제대로 동작한다.  
    pub fn check_point_collision(&self, point: &glam::Vec3) -> bool {
        match self.get_y_range_at(point.x, point.z) {
            Some((bot_y, top_y)) => bot_y <= point.y && point.y <= top_y,
            None =>                 false,
        }
    }

    /// 캡슐이 UFO형태인 경우에도 제대로 동작한다.  
    /// 
    /// Capsule의 크기를 sphere의 radius만큼 확장하고 sphere의 중심점과 충돌하는지 체크
    pub fn check_sphere_collision(&self, sphere: &Sphere) -> bool {
        let capsule = self.inflated(sphere.radius);

        capsule.check_point_collision(&sphere.center)
    }

    /// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.  
    /// 
    /// 직선과 선분 사이의 최소 거리를 구하는 것과 같다.
    pub fn check_line_collision(&self, line: &Line) -> bool {   
        let center = glam::Vec3A::from(self.center);
        let radius_sq = self.radius.powi(2);

        let line_y = line.direction().y;
        if line_y == 1.0 || line_y == -1.0 {
            return line.distance_to_point_sq(&center) <= radius_sq;
        }
        
        let nearest = self.get_seg().nearest_to_line(line);
        let distance_sq = line.distance_to_point_sq(&nearest);

        distance_sq <= radius_sq
    }

    /// 캡슐이 UFO형태인 경우에 bottom > top이 되는 구간은 포함하지 않는다.  
    /// 
    /// x, z에서 캡슐의 y축 범위를 구하고(bottom, top)형태로 반환한다.  
    /// x, z가 캡슐의 외부에 있으면 None을 반환한다.
    pub fn get_y_range_at(&self, x: f32, z: f32) -> Option<(f32, f32)> {
        let dx2 = (self.center.x - x).powi(2);
        let dz2 = (self.center.z - z).powi(2);
        let r2 = self.radius.powi(2);

        // xz 거리 체크
        let dxz2 = dx2 + dz2;
        let h2 = r2 - dxz2;
        if h2 < 0.0 {
            return None;
        }

        let h = h2.sqrt();
        let top_y = self.center.y + self.height - self.radius   + h;
        let bot_y = self.center.y + self.radius                 - h;

        Some((bot_y, top_y))
    }

    /// 캡슐이 UFO형태인 경우는 고려하지 않는다.  
    /// 
    /// 캡슐의 아래 구의 중심과 윗 구의 중심을 이은 선분
    pub fn get_seg(&self) -> Segment {
        let mut start = self.center;
        start.y += self.radius;
        let mut end = self.center;
        end.y += self.height - self.radius;

        Segment { start, end }
    }
}
