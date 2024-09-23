pub struct Sphere {
    pub center: gmm::Float3,
    pub radius: f32,
}

impl Sphere {
    pub fn check_point_collision(&self, point: &gmm::Float3) -> bool {
        let p = gmm::Vector::from(*point - self.center);
        p.vec3_len_sq() <= self.radius.powi(2)
    }

    pub fn check_sphere_collision(&self, sphere: &Sphere) -> bool {
        let p = gmm::Vector::from(self.center - sphere.center);
        p.vec3_len_sq() <= (self.radius + sphere.radius).powi(2)
    }
}