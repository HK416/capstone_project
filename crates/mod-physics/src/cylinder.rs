use super::RayIntersect;


pub struct Cylinder {
    pub center: gmm::Float3,
    direction: gmm::Float3,
    pub height: f32,
    pub radius: f32,   
}

impl Cylinder {
    pub fn build(center: gmm::Float3, direction: gmm::Vector, height: f32, radius: f32) -> Result<Self, &'static str> {
        match direction.vec3_normalize() {
            Some(direction) => Ok(Self { 
                center, 
                direction: direction.into(),
                height,
                radius
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
}

impl RayIntersect for Cylinder {
    fn ray_intersect(&self, ray: &crate::Ray) -> Option<f32> {
        todo!()
    }
}