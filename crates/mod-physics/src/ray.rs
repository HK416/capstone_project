pub struct Ray {
    pub origin: gmm::Float3,
    direction: gmm::Float3,
}

impl Ray {
    pub fn build(origin: gmm::Float3, direction: gmm::Vector) -> Result<Self, &'static str> {
        match direction.vec3_normalize() {
            Some(direction) => Ok(Self { 
                origin, 
                direction: direction.into()
            }),
            None => Err("Direction cannot be zero vector")
        }
    }

    pub fn set_direction(&mut self, direction: gmm::Vector) -> Result<(), &'static str> {
        match direction.vec3_normalize() {
            Some(direction) => {
                self.direction = direction.into();
                Ok(())
            },
            None => Err("Direction cannot be zero vector")
        }
    }

    pub fn direction(&self) -> gmm::Float3 {
        self.direction
    }

    pub fn intersect<T: RayIntersect>(&self, object: &T) -> Option<f32> {
        object.ray_intersect(self)
    }
}


/// Ray와 다른 객체가 충돌하는지 검사, 충돌한다면 가장 가까운 충돌 지점까지의 거리를 반환한다.
pub trait RayIntersect {
    fn ray_intersect(&self, ray: &Ray) -> Option<f32>;
}
