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

    /// point까지의 거리가 최소가 되는 점
    pub fn nearest_to_point(&self, point: &gmm::Vector) -> gmm::Vector {
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
        let this_line = Line::build(self.start, self.end - self.start).unwrap();
        let other_line = Line::build(other.start, other.end - other.start).unwrap();

        let this_h = this_line.foot_of_perpendicular_from_other(&other_line);
        let other_nearest = other.nearest_to_point(&this_h);
        self.nearest_to_point(&other_nearest)
    }

    /// 두 선분 사이의 최소 거리
    pub fn distance_to_other(&self, other: &Segment) -> f32 {
        let this_line = Line::build(self.start, self.end - self.start).unwrap();
        let other_line = Line::build(other.start, other.end - other.start).unwrap();

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