use super::Sphere;
use mod_math::{Segment, Line};


/// height 및 radius가 음수인 경우는 고려하지 않는다.  
/// direction은 단위벡터이다.  
/// height == 2 * radius이면 캡슐은 구와 같은 모양이 되는데, 구의 충돌체크로 최적화 하지는 않는다.  
/// height <= 2 * radius이면 캡슐에 기둥이 없고 위아래가 둥근 UFO형태가 되는데, 
/// 이때 일부 함수가 제대로 동작하지 않을 수 있다.  
#[derive(Debug)]
pub struct Capsule {
    pub center: gmm::Float3,    // 캡슐의 가장 아래 부분
    direction: gmm::Float3,     // 캡슐의 윗부분이 향하는 방향(default: Y축)
    pub height: f32,            // 캡슐의 전체 높이(direction방향으로 뻗은 길이) 
    pub radius: f32,
}

impl Capsule {
    /// direction을 단위벡터로 만들어 Capsule을 생성한다.  
    /// 영벡터가 주어지면 Error를 반환한다.  
    pub fn build(center: gmm::Float3, direction: gmm::Float3, height: f32, radius: f32) -> Result<Self, &'static str> {
        match gmm::Vector::from(direction).vec3_normalize() {
            Some(direction) => Ok(Self { 
                center, direction: direction.into(), height, radius 
            }),
            None => Err("Direction cannot be zero vector")
        }
    }
    
    /// 캡슐이 UFO형태인 경우에도 제대로 동작한다.
    pub fn check_point_collision(&self, point: &gmm::Float3) -> bool {
        let p = gmm::Vector::from(*point - self.center);    // center에 대한 point의 상대좌표
        let d = gmm::Vector::from(self.direction);

        let dot: gmm::Float3 = d.vec3_dot(p).into();
        // 캡슐에 대한 point의 상대지역좌표 y
        let y = dot.y;
        if y < 0.0 {
            return false;
        }

        // 캡슐에 대한 point의 상대지역좌표 x(원통의 회전은 신경쓰지 않아도 되므로 z무시)
        let x_sq = p.vec3_len_sq() - y * y;
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

    /// 캡슐이 UFO형태인 경우에도 제대로 동작한다.
    /// 
    /// Capsule의 크기를 sphere의 radius만큼 확장하고 sphere의 중심점과 충돌하는지 체크
    pub fn check_sphere_collision(&self, sphere: &Sphere) -> bool {
        let capsule = Capsule {
            center: self.center - self.direction * sphere.radius,
            direction: self.direction,
            height: self.height + 2.0 * sphere.radius,
            radius: self.radius + sphere.radius,
        };

        capsule.check_point_collision(&sphere.center)
    }

    /// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.
    /// 
    /// 두 선분 사이의 거리를 구하는 것과 같은가?
    /// 캡슐은 한 선분에서 radius거리 이내의 모든 점의 집합  
    /// 이는 self를 sphere.radius만큼 확장한 캡슐과 나머지 캡슐의 양쪽 구의 중심을 이은 선분이 충돌하는지 체크하는것과 같다.
    /// 따라서 두 선분의 최소 거리가 self.radius + sphere.radius보다 작거나 같으면 충돌한다.
    pub fn check_capsule_collision(&self, other: &Capsule) -> bool {
        let c_to_c = gmm::Vector::from(other.center - self.center);
        if c_to_c.vec3_len_sq() > (self.height + other.height).powi(2) {
            return false;
        }

        // 테스트 필요
        let distance = Segment::distance_between_segments(&self.get_seg(), &other.get_seg());

        distance <= self.radius + other.radius
    }

    /// 캡슐의 아래 구의 중심과 윗 구의 중심을 이은 선분
    fn get_seg(&self) -> Segment {
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
    pub center: gmm::Float3,    // 캡슐의 가장 아래 부분
    pub height: f32,            // 캡슐의 전체 높이
    pub radius: f32,
}

impl YCapsule {
    /// 캡슐이 UFO형태인 경우에도 제대로 동작한다.  
    pub fn check_point_collision(&self, point: &gmm::Float3) -> bool {
        match self.get_y_range_at(point.x, point.z) {
            Some((bot_y, top_y)) => bot_y <= point.y && point.y <= top_y,
            None =>                 false,
        }
    }

    /// 캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.  
    /// 
    /// #### y축과 평행한 경우
    /// 캡슐과 직선을 캡슐의 center가 원점이 되도록 평행이동 시킨 후, 
    /// 1. 직선을 xz평면으로 사영하여 점을 얻는다.(직선 위의 점 P의 y좌표를 0으로 만든다.)
    /// 2. 이 점과 원점 사이의 거리가 radius보다 작거나 같으면 충돌한다.
    /// 
    /// #### y축과 평행하지 않은 경우
    /// 1. 주어진 직선에서 캡슐의 중심이 되는 직선에 수선의 발을 내린다.
    /// 2. 두 직선의 거리가 radius보다 크면 충돌하지 않음
    /// 3. radius이내라면, 수선의 발이 캡슐의 seg에 속하면 충돌한다.
    /// 4. 속하지 않는다면, 캡슐의 위 아래 구의 중심과 직선의 거리를 구한다.
    pub fn check_line_collision(&self, line: &Line) -> bool {   
        let radius_sq = self.radius.powi(2);
        
        // #### y축과 평행한 경우
        let d = line.direction();
        if d.y == 1.0 || d.y == -1.0 {
            // 1. xz평면으로 사영
            let mut v = line.point - self.center;     
            v.y = 0.0;
            // 2. 해당 점과 원점 사이 거리가 radius보다 작거나 같으면 충돌
            return gmm::Vector::from(v).vec3_len_sq() <= radius_sq;
        }

        // #### y축과 평행하지 않은 경우
        // 1. 수선의 발을 구한다.
        let (distance, h) = Line::build(self.center, gmm::Float3::Y).unwrap()
            .distance_sq_and_foot_from_other(&line);

        // 2. 두 직선의 거리가 radius보다 크면 충돌하지 않음
        if distance > radius_sq {
            return false;
        }

        let h: gmm::Float3 = h.into();

        // 3. 수선의 발이 캡슐의 seg에 속하면 충돌
        let Segment { start, end } = self.get_seg();
        if start.y <= h.y && h.y <= end.y {
            return true;
        }

        // 4. 캡슐의 위 아래 구와 직선의 거리를 구한다.
        let start = gmm::Vector::from(start);
        let end = gmm::Vector::from(end);
        if line.distance_to_point_sq(&start) <= radius_sq || line.distance_to_point_sq(&end) <= radius_sq {
            return true;
        }

        false
    }

    /// 캡슐이 UFO형태인 경우에도 제대로 동작한다.  
    /// 
    /// Capsule의 크기를 sphere의 radius만큼 확장하고 sphere의 중심점과 충돌하는지 체크
    pub fn check_sphere_collision(&self, sphere: &Sphere) -> bool {
        let mut center = self.center.clone();
        center.y -= sphere.radius;

        let capsule = YCapsule {
            center,
            height: self.height + 2.0 * sphere.radius,
            radius: self.radius + sphere.radius,
        };

        capsule.check_point_collision(&sphere.center)
    }

    /// other캡슐이 UFO형태인 경우에는 제대로 동작하지 않는다.
    /// 
    /// Y축에 정렬된 두 캡슐의 충돌 체크
    pub fn check_ycapsule_collision(&self, other: &YCapsule) -> bool {
        let mut center = self.center.clone();
        center.y -= other.radius;

        let capsule = YCapsule {
            center,
            height: self.height + 2.0 * other.radius,
            radius: self.radius + other.radius,
        };

        match capsule.get_y_range_at(other.center.x, other.center.z) {
            Some((bot_y, top_y)) => {
                let Segment { start, end } = other.get_seg();

                if end.y < bot_y || top_y < start.y {
                    return false;
                }
                else {
                    return true;
                }
            },
            None => false,
        }
    }

    /// 캡슐이 UFO형태인 경우에 bottom > top이 되는 구간은 포함하지 않는다.  
    /// 
    /// x, z에서 캡슐의 y축 범위를 구하고(bottom, top)형태로 반환한다.  
    /// x, z가 캡슐의 외부에 있으면 None을 반환한다.
    fn get_y_range_at(&self, x: f32, z: f32) -> Option<(f32, f32)> {
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
    fn get_seg(&self) -> Segment {
        let mut start = self.center.clone();
        start.y += self.radius;
        let mut end = self.center.clone();
        end.y += self.height - self.radius;

        Segment { start, end }
    }
}
