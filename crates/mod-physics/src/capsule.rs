use super::Sphere;
use mod_math::{Segment, Line};


/// height <= 2 * radius 인 경우, Capsule-Capsule 충돌은 제대로 동작하지 않는다.  
#[derive(Debug)]
pub struct Capsule {
    pub center: gmm::Float3,     // 캡슐의 가장 아래 부분
    pub direction: gmm::Float3,  // 캡슐의 윗부분이 향하는 방향(default: Y축)
    pub height: f32,
    pub radius: f32,
}

impl Capsule {
    pub fn check_point_collision(&self, point: &gmm::Float3) -> bool {
        let p = gmm::Vector::from(*point - self.center);    // center에 대한 point의 상대좌표
        let d = self.direction_normal();

        let dot: gmm::Float3 = d.vec3_dot(p).into();
        // 캡슐에 대한 point의 상대지역좌표 y
        let y = dot.y;
        if y < 0.0 {
            return false;
        }

        // 캡슐에 대한 point의 상대지역좌표 x(원통의 회전은 신경쓰지 않아도 되므로 z무시)
        let x2 = p.vec3_len_sq() - y * y;
        if x2 > self.radius.powi(2) {
            return false;
        }

        let h2 = self.radius * self.radius - x2;
        let h = h2.sqrt();
        let top_y = self.height - self.radius   + h;
        let bot_y = self.radius                 - h;

        // y축 범위 체크
        bot_y <= y && y <= top_y
    }

    /// Capsule의 크기를 sphere의 radius만큼 확장하고 sphere의 중심점과 충돌하는지 체크
    pub fn check_sphere_collision(&self, sphere: &Sphere) -> bool {
        let d: gmm::Float3 = self.direction_normal().into();

        let capsule = Capsule {
            center: self.center - d * sphere.radius,
            direction: self.direction,
            height: self.height + 2.0 * sphere.radius,
            radius: self.radius + sphere.radius,
        };

        capsule.check_point_collision(&sphere.center)
    }

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

    pub fn direction_normal(&self) -> gmm::Vector {
        match gmm::Vector::from(self.direction).vec3_normalize() {
            Some(v) => v,
            None => gmm::Vector::from(gmm::Float3::Y),            // default: Y축
        }
    }

    /// 캡슐의 아래 구의 중심과 윗 구의 중심을 이은 선분
    fn get_seg(&self) -> Segment {
        let d: gmm::Float3 = self.direction_normal().into();

        Segment {
            start: self.center + d * self.radius,
            end: self.center + d * (self.height - self.radius),
        }
    }
}



/// Y축에 정렬된 캡슐  
/// direction은 항상 <0, 1, 0>이다.  
#[derive(Debug)]
pub struct YCapsule {
    pub center: gmm::Float3,     // 캡슐의 가장 아래 부분
    pub height: f32,
    pub radius: f32,
}

impl YCapsule {
    pub fn check_point_collision(&self, point: &gmm::Float3) -> bool {
        match self.get_y_range_at(point.x, point.z) {
            Some((bot_y, top_y)) => bot_y <= point.y && point.y <= top_y,
            None =>                 false,
        }
    }

    /// 이 함수는 height <= 2 * radius인 경우에는 제대로 동작하지 않는다.  
    /// 
    /// #### y축과 평행한 경우
    /// 캡슐과 직선을 캡슐의 center가 원점이 되도록 평행이동 시킨 후, 
    /// 1. 직선을 xz평면으로 사영하여 점을 얻는다.(직선 위의 점 P의 y좌표를 0으로 만든다.)
    /// 2. 이 점과 원점 사이의 거리가 radius보다 작거나 같으면 충돌한다.
    /// 
    /// #### y축과 평행하지 않은 경우
    /// 1. 주어진 직선에서 캡슐의 중심이 되는 직선에 수선의 발을 내린다.
    /// 2. 수선의 발이 캡슐의 seg에 속하면 충돌한다.
    /// 3. 속하지 않는다면, 캡슐의 위 아래 구와 직선의 거리를 구한다.
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
        let h: gmm::Float3 = Line::build(self.center, gmm::Float3::Y).unwrap()
            .foot_of_perpendicular_from_other(&line).into();

        // 2. 수선의 발이 캡슐의 seg에 속하면 충돌
        let Segment { start, end } = self.get_seg();
        if start.y <= h.y && h.y <= end.y {
            return true;
        }

        // 3. 캡슐의 위 아래 구와 직선의 거리를 구한다.
        let start = gmm::Vector::from(start);
        let end = gmm::Vector::from(end);
        if line.distance_to_point_sq(&start) <= radius_sq || line.distance_to_point_sq(&end) <= radius_sq {
            return true;
        }

        false
    }

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

    /// Y축에 정렬된 두 캡슐의 충돌 체크
    pub fn check_ycapsule_collision(&self, other: &YCapsule) -> bool {
        let capsule = YCapsule {
            center: self.center,
            height: self.height + 2.0 * other.radius,
            radius: self.radius + other.radius,
        };

        match capsule.get_y_range_at(other.center.x, other.center.z) {
            Some((bot_y, top_y)) => {
                let other_bot_y = other.center.y;
                let other_top_y = other.center.y + other.height;

                bot_y <= other_bot_y && other_bot_y <= top_y ||
                bot_y <= other_top_y && other_top_y <= top_y
            },
            None => false,
        }
    }

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

    /// 캡슐의 아래 구의 중심과 윗 구의 중심을 이은 선분
    fn get_seg(&self) -> Segment {
        let mut start = self.center.clone();
        start.y += self.radius;
        let mut end = self.center.clone();
        end.y += self.height - self.radius;

        Segment { start, end }
    }
}
