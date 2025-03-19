use super::*;


#[derive(serde::Serialize, serde::Deserialize)]
struct SphereHelper {
    center: Vec3,
    radius: f32,
}


impl serde::Serialize for Sphere {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let helper = SphereHelper {
            center: Vec3::from_glam(self.center),
            radius: self.radius,
        };
        helper.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Sphere {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = SphereHelper::deserialize(deserializer)?;
        Ok(Sphere {
            center: helper.center.to_glam(),
            radius: helper.radius,
        })
    }
}