use std::{collections::HashMap, sync::{Arc, Mutex}};

use lazy_static::lazy_static;

use crate::material::{Material, MaterialBuilder};

lazy_static! {
    /// 생성된 재질을 관리하는 풀 객체입니다.
    static ref POOL: Mutex<HashMap<String, Arc<Material>>> = Mutex::new(HashMap::with_capacity(32));
}



/// 생성된 재질을 관리하는 풀 객체입니다.
/// 
/// 실제 풀 객체는 전역 변수로 선언되어 있으며, 
/// `MaterialPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
/// 
pub struct MaterialPool;

impl MaterialPool {
    /// 재질을 가져옵니다.
    /// 만약 재질이 풀 객체에 존재하지 않을 경우 재질을 생성합니다.
    #[must_use]
    pub fn get_or_init(
        device: &Arc<wgpu::Device>, 
        queue: &Arc<wgpu::Queue>, 
        builder: MaterialBuilder
    ) -> Arc<Material> {
        let material_name = builder.name.clone();
        let mut pool_guard = POOL.lock().unwrap();
        match pool_guard.get(&material_name) {
            Some(material) => material.clone(), 
            None => {
                let material = Arc::new(builder.build(device, queue));
                pool_guard.insert(material_name, material.clone());
                material
            }
        }
    }
    
    /// 주어진 재질에 해당하는 재질이 풀 객체에 포함되어있는지 여부를 반환합니다.
    #[must_use]
    pub fn contains<N: AsRef<String>>(material_name: N) -> bool {
        let pool_guard = POOL.lock().unwrap();
        pool_guard.contains_key(material_name.as_ref())
    }

    /// 주어진 재질에 해당하는 재질을 풀 객체에서 제거합니다.
    pub fn remove<N: AsRef<String>>(material_name: N) -> Option<Arc<Material>> {
        let mut pool_guard = POOL.lock().unwrap();
        pool_guard.remove(material_name.as_ref())
    }

    /// 풀 객체에 존재하는 모든 재질을 제거합니다.
    pub fn clear() {
        let mut pool_guard = POOL.lock().unwrap();
        pool_guard.clear();
    }
}