use super::Sphere;


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
        let d = match gmm::Vector::from(self.direction).vec3_normalize() {
            Some(v) => v,
            None => gmm::Vector::from(gmm::Float3::Y),  // default: Y축
        };

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
        let d: gmm::Float3 = match gmm::Vector::from(self.direction).vec3_normalize() {
            Some(v) => v,
            None => gmm::Vector::from(gmm::Float3::Y),            // default: Y축
        }.into();

        let capsule = Capsule {
            center: self.center - d * sphere.radius,
            direction: self.direction,
            height: self.height + 2.0 * sphere.radius,
            radius: self.radius + sphere.radius,
        };

        capsule.check_point_collision(&sphere.center)
    }
}



/// Axis-Aligned Capsule
#[derive(Debug)]
pub struct YCapsule {
    pub center: gmm::Float3,     // 캡슐의 가장 아래 부분
    pub height: f32,
    pub radius: f32,
}

impl YCapsule {
    pub fn check_point_collision(&self, point: &gmm::Float3) -> bool {
        let dx2 = (self.center.x - point.x).powi(2);
        let dz2 = (self.center.z - point.z).powi(2);
        let r2 = self.radius.powi(2);

        // xz 거리 체크
        let dxz2 = dx2 + dz2;
        let h2 = r2 - dxz2;
        if h2 < 0.0 {
            return false;
        }

        let h = h2.sqrt();
        let top_y = self.center.y + self.height - self.radius   + h;
        let bot_y = self.center.y + self.radius                 - h;

        // y축 범위 체크
        bot_y <= point.y && point.y <= top_y
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
}
