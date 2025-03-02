use mod_math::Line;


#[derive(Debug, Clone)]
pub struct Sphere {
    pub center: glam::Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn check_point_collision(&self, point: &glam::Vec3A) -> bool {
        let center = glam::Vec3A::from(self.center);
        (point - center).length_squared() <= self.radius.powi(2)
    }

    pub fn check_line_collision(&self, line: &Line) -> bool {
        let dist = line.distance_to_point_sq(&self.center.into());
        dist <= self.radius.powi(2)
    }
}
