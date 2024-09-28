/// build 연관함수와 set_direction메서드를 통해 direction이 단위벡터임을 보장한다.
pub struct Line {
    pub point: gmm::Float3,
    direction: gmm::Float3,
}

// constructors/desctructors
impl Line {
    pub fn build(point: gmm::Float3, direction: gmm::Float3) -> Result<Self, &'static str> {
        if direction == gmm::Float3::ZERO {
            return Err("Direction cannot be zero vector");
        }

        let direction = gmm::Vector::from(direction)
            .vec3_normalize().unwrap().into();

        Ok(Self { point, direction })
    }
}

// methods
impl Line {
    pub fn set_direction(&mut self, direction: gmm::Float3) -> Result<(), &'static str> {
        if direction == gmm::Float3::ZERO {
            return Err("Direction cannot be zero vector");
        }

        self.direction = gmm::Vector::from(direction)
            .vec3_normalize().unwrap().into();

        Ok(())
    }

    pub fn direction(&self) -> gmm::Float3 {
        self.direction
    }

    /// point까지의 최소 거리
    pub fn distance_to_point(&self, point: &gmm::Vector) -> f32 {
        let h = self.foot_of_perpendicular_from_point(point);
        let ah = gmm::Vector::from(h - *point);
        ah.vec3_len()
    }

    /// point까지의 최소 거리의 제곱
    pub fn distance_to_point_sq(&self, point: &gmm::Vector) -> f32 {
        let h = self.foot_of_perpendicular_from_point(point);
        let ah = gmm::Vector::from(h - *point);
        ah.vec3_len_sq()
    }

    /// point로 부터의 수선의 발
    pub fn foot_of_perpendicular_from_point(&self, point: &gmm::Vector) -> gmm::Vector {
        let p = gmm::Vector::from(self.point);
        let v = gmm::Vector::from(self.direction);
        let a = *point;

        let pa = a - p;

        let v_dot_pa: gmm::Float3 = pa.vec3_dot(v).into();
        let proj = v_dot_pa.x;
        let add = v * gmm::Vector::from([proj, proj, proj, 0.0]);

        p + add
    }

    /// 다른 직선으로부터의 수선의 발, 두 직선이 평행하지 않다고 가정
    pub fn foot_of_perpendicular_from_other(&self, other: &Line) -> gmm::Vector {
        let p = gmm::Vector::from(other.point);
        let h = self.foot_of_perpendicular_from_point(&p);

        let v1 = gmm::Vector::from(self.direction);
        let v2 = gmm::Vector::from(other.direction);
        let dot: gmm::Float3 = v1.vec3_dot(v2).into();
        let c = dot.x;
        if c == 0.0 {   // 두 직선이 수직할 경우
            return h;
        }

        let hp = p - h;
        let hs2 = hp.vec3_len_sq() - Line::distance_between(self, other).powi(2);

        let c2 = c * c;
        let ah2 = hs2 * (c2) / (1.0 - c2);
        let ah = ah2.sqrt();

        let a_h = v1 * gmm::Vector::from([ah, ah, ah, 0.0]);

        // 어떤걸 골라야할지 모르겠다 -> 비교 선택
        let a1 = h + a_h;
        let a2 = h - a_h;

        let d1 = other.distance_to_point(&a1);
        let d2 = other.distance_to_point(&a2);

        if d1 < d2 {
            a1
        } else {
            a2
        }
    }
}

// associate functions
impl Line {
    /// 두 직선 사이의 최소 거리
    pub fn distance_between(line1: &Line, line2: &Line) -> f32 {
        if line1.point == line2.point {
            return 0.0;
        }

        let v1 = gmm::Vector::from(line1.direction);
        let v2 = gmm::Vector::from(line2.direction);
        let cross = v1.vec3_cross(v2);

        match cross.vec3_normalize() {
            Some(cross) => {
                let p1 = gmm::Vector::from(line1.point);
                let p2 = gmm::Vector::from(line2.point);
                let p_to_p = p1 - p2;

                let dot: gmm::Float3 = p_to_p.vec3_dot(cross).into();

                dot.x.abs()
            },

            // 두 선이 평행할 경우
            None => line1.distance_to_point(&gmm::Vector::from(line2.point))
        }
    }
}