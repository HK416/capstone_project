/// build 연관함수와 set_direction메서드를 통해 direction이 단위벡터임을 보장한다.
pub struct Line {
    pub point: glam::Vec3,
    direction: glam::Vec3,
}

// constructors/desctructors
impl Line {
    pub fn build(point: glam::Vec3A, direction: glam::Vec3A) -> Result<Self, &'static str> {
        match direction.try_normalize() {
            Some(direction) => Ok(Self { 
                point: point.into(), 
                direction: direction.into()
            }),
            None => Err("Direction cannot be zero vector")
        }
    }
}

// methods
impl Line {
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

    /// point까지의 최소 거리
    pub fn distance_to_point(&self, point: &glam::Vec3A) -> f32 {
        let h = self.foot_of_perpendicular_from_point(point);
        let ah = h - point;
        ah.length()
    }

    /// point까지의 최소 거리의 제곱
    pub fn distance_to_point_sq(&self, point: &glam::Vec3A) -> f32 {
        let h = self.foot_of_perpendicular_from_point(point);
        let ah = h - point;
        ah.length_squared()
    }

    /// point로 부터의 수선의 발
    pub fn foot_of_perpendicular_from_point(&self, point: &glam::Vec3A) -> glam::Vec3A {
        let p = glam::Vec3A::from(self.point);
        let v = glam::Vec3A::from(self.direction);
        let a = point;

        let pa = a - p;

        let proj = pa.dot(v);
        let add = v * proj;

        p + add
    }

    /// 다른 직선으로부터의 수선의 발, 두 직선이 평행하지 않다고 가정
    pub fn foot_of_perpendicular_from_other(&self, other: &Line) -> glam::Vec3A {
        let p = glam::Vec3A::from(other.point);
        let h = self.foot_of_perpendicular_from_point(&p);

        let v1 = glam::Vec3A::from(self.direction);
        let v2 = glam::Vec3A::from(other.direction);
        let c = v1.dot(v2);
        if c == 0.0 {   // 두 직선이 수직할 경우
            return h;
        }

        let hp = p - h;
        let hs2 = hp.length_squared() - Line::distance_between(self, other).powi(2);

        let c2 = c * c;
        let ah2 = hs2 * (c2) / (1.0 - c2);
        let ah = ah2.sqrt();

        let a_h = v1 * ah;

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

    /// 다른 직선으로부터의 수선의 발과 사이 거리, 두 직선이 평행하지 않다고 가정
    pub fn distance_sq_and_foot_from_other(&self, other: &Line) -> (f32, glam::Vec3A) {
        let p = glam::Vec3A::from(other.point);
        let h = self.foot_of_perpendicular_from_point(&p);
        let hp = p - h;

        let v1 = glam::Vec3A::from(self.direction);
        let v2 = glam::Vec3A::from(other.direction);
        let cos = v1.dot(v2);

        if cos == 0.0 {   // 두 직선이 수직할 경우
            let dot = hp.dot(v2);
            let dist_sq = hp.length_squared() - (dot * dot);
            return (dist_sq, h);
        }

        let hs2 = hp.length_squared() - Line::distance_between(self, other).powi(2);

        let cos_sq = cos * cos;
        let ah2 = hs2 * (cos_sq) / (1.0 - cos_sq);
        let ah = ah2.sqrt();

        let a_h = v1 * ah;

        // 어떤걸 골라야할지 모르겠다 -> 비교 선택
        let a1 = h + a_h;
        let a2 = h - a_h;

        let d1 = other.distance_to_point(&a1);
        let d2 = other.distance_to_point(&a2);

        if d1 < d2 {
            (d1, a1)
        } else {
            (d2, a2)
        }
    }
}

// associate functions
impl Line {
    /// 두 직선 사이의 최소 거리
    pub fn distance_between(line1: &Line, line2: &Line) -> f32 {
        let p1 = glam::Vec3A::from(line1.point);
        let p2 = glam::Vec3A::from(line2.point);

        if p1 == p2 {
            return 0.0;
        }

        let v1 = glam::Vec3A::from(line1.direction);
        let v2 = glam::Vec3A::from(line2.direction);
        let cross = v1.cross(v2);

        match cross.try_normalize() {
            Some(cross) => (p1 - p2).dot(cross).abs(),

            // 두 선이 평행할 경우
            None => line1.distance_to_point(&p2)
        }
    }
}