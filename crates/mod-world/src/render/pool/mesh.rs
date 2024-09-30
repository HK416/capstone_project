use std::{
    collections::HashMap, 
    sync::{Arc, Mutex}
};

use lazy_static::lazy_static;

use crate::render::mesh::{
    IndexBuffer, 
    MeshBuilder, 
    ModelMesh, 
    VertexBuffer
};

lazy_static! {
    /// 공유 가능한 메쉬 데이터를 관리하는 풀 객체입니다.
    static ref POOL: Mutex<HashMap<String, Arc<ModelMesh>>> = Mutex::new(HashMap::with_capacity(32));
}



/// 공유되는 메쉬 데이터를 관리하는 풀 객체입니다.
/// 
/// 실제 풀 객체는 전역 변수로 선언되어 있으며, 
/// `MeshPool`은 풀 객체에 접근할 수 있는 인터페이스를 제공합니다.
/// 
pub struct MeshPool;

impl MeshPool {
    /// 공유 가능한 메쉬를 가져옵니다.
    /// 만약 공유 가능한 메쉬가 풀 객체에 존재하지 않을 경우 공유 가능한 메쉬를 생성합니다.
    #[must_use]
    pub fn get_or_init(
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        builder: MeshBuilder
    ) -> Arc<ModelMesh> {
        let mesh_name = builder.name.clone();
        let mut pool_guard = POOL.lock().unwrap();
        match pool_guard.get(&mesh_name) {
            Some(mesh) => mesh.clone(), 
            None => {
                let mesh = Arc::new(ModelMesh {
                    name: builder.name.clone(), 
                    num_vertices: builder.vertices.count() as u32, 
                    vertex: VertexBuffer::from_vertices(
                        Some(&format!("Vertex({})", &builder.name)), 
                        device, 
                        queue, 
                        builder.vertices
                    ), 
                    attributes: builder.attributes.into_iter()
                        .map(|(attribute, values)| (
                            attribute, 
                            VertexBuffer::from_attribute(
                                Some(&format!("Attribute({})", &builder.name)), 
                                device, 
                                queue, 
                                values
                            )
                        ))
                        .collect(), 
                    submeshes: builder.submeshes.into_iter()
                        .map(|values| IndexBuffer::new(
                            Some(&format!("Index({})", &builder.name)), 
                            device, 
                            queue, 
                            values
                        ))
                        .collect()
                });
                pool_guard.insert(mesh_name, mesh.clone());
                mesh
            }
        }
    }

    /// 주어진 공유 가능한 메쉬에 해당하는 공유 가능한 메쉬가 풀 객체에 포함되어있는지 여부를 반환합니다.
    #[must_use]
    pub fn contains<N: AsRef<str>>(mesh_name: N) -> bool {
        let pool_guard = POOL.lock().unwrap();
        pool_guard.contains_key(mesh_name.as_ref())
    }

    /// 주어진 공유 가능한 메쉬에 해당하는 공유 가능한 메쉬를 풀 객체에서 제거합니다.
    pub fn remove<N: AsRef<str>>(mesh_name: N) -> Option<Arc<ModelMesh>> {
        let mut pool_guard = POOL.lock().unwrap();
        pool_guard.remove(mesh_name.as_ref())
    }

    /// 풀 객체에 존재하는 모든 공유 가능한 메쉬를 제거합니다.
    pub fn clear() {
        let mut pool_guard = POOL.lock().unwrap();
        pool_guard.clear();
    }
}
