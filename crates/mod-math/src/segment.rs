use super::line::Line;


pub struct Segment {
    pub start: gmm::Float3,
    pub end: gmm::Float3,
}

// methods
impl Segment {
    /// point까지의 최소 거리
    pub fn distance_to_point(&self, point: &gmm::Vector) -> f32 {
        let nearest = self.nearest_to_point(point);
        let pn = gmm::Vector::from(nearest - *point);
        pn.vec3_len()
    }

    /// point까지의 최소 거리의 제곱
    pub fn distance_to_point_sq(&self, point: &gmm::Vector) -> f32 {
        let nearest = self.nearest_to_point(point);
        let pn = gmm::Vector::from(nearest - *point);
        pn.vec3_len_sq()
    }

    /// point까지의 거리가 최소가 되는 점  
    /// 
    /// 시작점과 끝점이 같으면 시작점을 반환한다.  
    pub fn nearest_to_point(&self, point: &gmm::Vector) -> gmm::Vector {
        if self.start == self.end {
            return gmm::Vector::from(self.start);
        }

        let a = gmm::Vector::from(self.start);
        let b = gmm::Vector::from(self.end);
        let p = *point;

        let ab = b - a;
        let ab2 = ab.vec3_dot(ab);
        let ap = p - a;

        // 선형보간을 이용한 선분 내의 점
        let t: gmm::Float3 = (ab.vec3_dot(ap) / ab2).into();
        let t = t.x;
        let t = t.max(0.0).min(1.0);

        a + ab * gmm::Vector::from([t, t, t, 0.0])
    }

    /// 다른 선분까지의 거리가 최소가 되는 점
    pub fn nearest_to_other(&self, other: &Segment) -> gmm::Vector {
        let start = gmm::Vector::from(self.start);
        let end = gmm::Vector::from(self.end);
        let other_start = gmm::Vector::from(other.start);
        let other_end = gmm::Vector::from(other.end);

        let this_line = match Line::build(self.start, end - start) {
            Ok(line) => line,
            Err(_)   => return start,
        };

        let other_line = match Line::build(other.start, other_end - other_start) {
            Ok(line) => line,
            Err(_)   => return self.nearest_to_point(&other_start),
        };

        let this_h = this_line.foot_of_perpendicular_from_other(&other_line);
        let other_nearest = other.nearest_to_point(&this_h);
        self.nearest_to_point(&other_nearest)
    }

    /// line까지의 거리가 최소가 되는 선분 위의 점
    pub fn nearest_to_line(&self, line: &Line) -> gmm::Vector {
        let start = gmm::Vector::from(self.start);
        let end = gmm::Vector::from(self.end);

        let this_line = match Line::build(self.start, end - start) {
            Ok(line) => line,
            Err(_)   => return gmm::Vector::from(self.start),
        };

        let h = line.foot_of_perpendicular_from_other(&this_line);

        self.nearest_to_point(&h)
    }

    /// 두 선분 사이의 최소 거리
    pub fn distance_to_other(&self, other: &Segment) -> f32 {
        let start = gmm::Vector::from(self.start);
        let end = gmm::Vector::from(self.end);
        let other_start = gmm::Vector::from(other.start);
        let other_end = gmm::Vector::from(other.end);

        let this_line = match Line::build(self.start, end - start) {
            Ok(line) => line,
            Err(_)   => return other.distance_to_point(&start),
        };

        let other_line = match Line::build(other.start, other_end - other_start) {
            Ok(line) => line,
            Err(_)   => return self.distance_to_point(&other_start),
        };

        let this_h = this_line.foot_of_perpendicular_from_other(&other_line);
        let other_nearest = other.nearest_to_point(&this_h);
        self.distance_to_point(&other_nearest)
    }
}

// associated functions
impl Segment {
    pub fn distance_between_segments(a: &Segment, b: &Segment) -> f32 {
        a.distance_to_other(b)
    }
}