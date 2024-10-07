use std::sync::Arc;

use mod_parallelism::collections::{Queue, SkipMap};

use crate::component::{GameObject, IdGenerator, Transform, WorldID};

use super::Bullet;



/// 총알 게임 오브젝트를 생성하거나 재사용할 수 있도록 하는 오브젝트 풀 객체입니다.
#[derive(Debug)]
pub struct BulletPool {
    free_list: Queue<GameObject>, 
}

impl BulletPool {
    /// 새로운 총알 오브젝트 풀 객체를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 새로운 게임 오브젝트를 할당받습니다.
    /// 
    /// # Panics
    /// 주어진 `direction`이 정규화되지 않은 벡터인 경우 [`panic!`]을 호출할 것입니다.
    /// 
    pub fn alloc(
        &self,
        world: &Arc<SkipMap<WorldID, GameObject>>, 
        id_generator: &Arc<IdGenerator>, 
        bullet: Bullet, 
    ) -> WorldID {
        // 게임 오브젝트를 가져옵니다.
        let mut object = match self.free_list.pop() {
            Some(object) => object, 
            None => {
                // 새로운 게임 오브젝트를 생성합니다.
                GameObject::new(
                    id_generator, 
                    format!("Bullet({:?})", &bullet.kind), 
                    None
                )
            }
        };

        // 게임 오브젝트의 월드 변환 행렬을 계산합니다.
        let z_axis = bullet.direction;
        let y_axis = gmm::Vector::Y;
        let x_axis = y_axis.vec3_cross(z_axis);
        let y_axis = z_axis.vec3_cross(x_axis);
        let rotation = gmm::Quaternion::from_rotation_axes(x_axis, y_axis, z_axis);
        let transform = Transform(gmm::Matrix::from_rotation_translation(rotation, bullet.translation));

        // 게임 오브젝트의 월드 변환 행렬을 설정합니다.
        object.set_world_transform(transform);

        // 게임 오브젝트에 총알 요소를 추가합니다.
        object.insert(bullet);

        // 게임 오브젝트를 게임 월드에 추가하고 식별자를 반환합니다.
        let world_id = object.id().clone();
        world.insert(world_id.clone(), object);
        world_id
    }

    /// 게임 오브젝트를 회수합니다.
    /// 
    /// # Panics
    /// 게임 월드에 게임 오브젝트 식별자에 해당하는 게임 오브젝트가 존재하지 않는 경우
    /// [`panic!`]을 호출할 것입니다.
    /// 
    #[inline]
    pub fn retire(
        &self, 
        world: &Arc<SkipMap<WorldID, GameObject>>, 
        id: &WorldID
    ) {
        self.free_list.push(world.remove(id).unwrap());
    }
}

impl Default for BulletPool {
    #[inline]
    fn default() -> Self {
        Self { 
            free_list: Queue::new(), 
        }
    }
}
