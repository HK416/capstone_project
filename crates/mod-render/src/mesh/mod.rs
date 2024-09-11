mod buffer;
pub use self::buffer::*;

mod values;
pub use self::values::*;

use std::sync::Arc;
use std::collections::HashMap;

use crate::skin::BindMatrixBuffer;



/// 정점 속성의 식별자입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Attribute {
    Colors, 
    Normals, 
    Tangents, 
    Texcoords0, 
    Texcoords1, 
    BoneIndices, 
    BoneWeights, 
}



/// 메쉬 컴포넌트 타입입니다.
pub type MeshComponent = Arc<Mesh>;

/// 3차원 메쉬 데이터입니다.
#[derive(Debug)]
pub struct Mesh {
    /// 3차원 메쉬의 이름입니다.
    name: String, 

    /// 3차원 메쉬 정점의 갯수입니다.
    num_vertices: u32, 

    /// 3차원 메쉬의 정점 버퍼입니다.
    buffer: VertexBuffer, 

    /// 3차원 메쉬의 하위 메쉬입니다.
    submeshes: Vec<IndexBuffer>, 

    /// 3차원 메쉬가 가지고 있는 정점 속성의 버퍼입니다.
    attributes: HashMap<Attribute, VertexBuffer>, 

    /// 초기 뼈의 위치 변환 행렬의 유니폼 버퍼입니다.
    bindpose: Option<BindMatrixBuffer>, 
}

impl Mesh {
    /// 컴포넌트의 이름을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 정점의 갯수를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn num_vertices(&self) -> u32 {
        self.num_vertices
    }

    /// 정점 버퍼를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn vertices(&self) -> &VertexBuffer {
        &self.buffer
    }

    /// 하위 메쉬의 갯수를 반환합니다.
    #[inline]
    #[must_use]
    pub fn num_submeshes(&self) -> usize {
        self.submeshes.len()
    }

    /// 하위 메쉬들을 반환합니다.
    #[inline]
    #[must_use]
    pub fn submeshes(&self) -> &[IndexBuffer] {
        &self.submeshes
    }

    /// 정점 속성을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn attribute(&self, id: Attribute) -> Option<&VertexBuffer> {
        self.attributes.get(&id)
    }

    /// 메쉬의 초기 뼈의 위치 변환 행렬을 반환합니다.
    #[inline]
    #[must_use]
    pub fn bindpose(&self) -> Option<&BindMatrixBuffer> {
        self.bindpose.as_ref()
    }
}



/// 3차원 메쉬를 생성하는 빌더입니다.
#[derive(Debug, Clone)]
pub struct MeshBuilder {
    /// 메쉬의 이름입니다.
    pub name: String, 

    /// 메쉬의 정점 데이터입니다.
    vertices: Vertices, 

    /// 메쉬의 정점 속성 데이터입니다.
    attributes: HashMap<Attribute, VertexAttributeValues>, 

    /// 메쉬의 하위 메쉬 데이터입니다.
    submeshes: Vec<Indices>, 

    /// 메쉬의 초기 상태에서의 뼈의 위치 변환 행렬입니다.
    bindposes: Vec<gmm::Float4x4>, 
}

impl MeshBuilder {
    /// 새로운 메쉬 빌더를 생성합니다.
    /// 
    /// # Panics
    /// 주어진 정점 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn new<T, I>(name: T, values: I) -> Self 
    where 
        T: Into<String>, 
        I: IntoIterator<Item = gmm::Float3>,  
        I::IntoIter: ExactSizeIterator, 
    {
        let name = name.into();
        let vertices = Vertices(values.into_iter().collect());
        assert!(!vertices.is_empty(), "The given vertex data is empty!");

        Self {
            name, 
            vertices, 
            attributes: HashMap::with_capacity(8), 
            submeshes: Vec::with_capacity(8), 
            bindposes: Vec::new(), 
        }
    }

    /// 메쉬 빌더에 주어진 정점 속성을 추가합니다.
    /// 이미 해당 정점 속성이 존재할 경우 덮어씁니다.
    /// 
    /// # Panics
    /// 주어진 정점 속성 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    /// 
    #[must_use]
    pub fn insert_attribute(mut self, values: VertexAttributeValues) -> Self {
        assert!(!values.is_empty(), "The given attribute data is empty!");
        self.attributes.insert(values.attribute(), values);
        self
    }

    /// 메쉬 빌더에 주어진 정점 속성을 제거합니다.
    #[must_use]
    pub fn remove_attribute(mut self, attribute: Attribute) -> Self {
        self.attributes.remove(&attribute);
        self
    }

    /// 메쉬 빌더에 하위 메쉬 데이터를 추가합니다.
    /// 
    /// # Panics
    /// 주어진 하위 메쉬 데이터가 비어있는 경우 [`panic!`]을 호출합니다.
    /// 
    #[must_use]
    pub fn add_submesh(mut self, values: Indices) -> Self {
        assert!(!values.is_empty(), "The given submesh data is empty!");
        self.submeshes.push(values);
        self
    }

    /// 메쉬 빌더에 해당 인덱스의 하위 메쉬 데이터를 제거합니다.
    /// 
    /// # Panics
    /// 주어진 인덱스가 하위 메쉬 배열 범위를 벗어나는 경우 [`panic!`]을 호출합니다.
    /// 
    #[must_use]
    pub fn remove_submesh(mut self, index: usize) -> Self {
        self.submeshes.remove(index);
        self
    }

    /// 메쉬 빌더에 초기 상태의 뼈의 위치 변환 행렬을 설정합니다.
    /// 
    /// # Panics
    /// 주어진 초기 상태의 뼈의 위치 변환 행렬이 비어있는 경우 [`panic!`]을 호출합니다.
    /// 
    #[must_use]
    pub fn set_bindposes<I>(mut self, bindposes: I) -> Self 
    where I: IntoIterator<Item = gmm::Float4x4>, I::IntoIter: ExactSizeIterator {
        let bindposes: Vec<_> = bindposes.into_iter().collect();
        assert!(!bindposes.is_empty(), "The given bindpose data is empty!");
        self.bindposes = bindposes;
        self
    }

    /// 메쉬 빌더에 설정된 초기 상태의 뼈의 위치 변환 행렬을 제거합니다.
    #[must_use]
    pub fn reset_bindposes(mut self) -> Self {
        self.bindposes.clear();
        self
    }

    #[must_use]
    pub fn build(self, device: &wgpu::Device, queue: &wgpu::Queue) -> MeshComponent {
        Mesh {
            name: self.name.to_string(), 
            num_vertices: self.vertices.count() as u32, 
            buffer: VertexBuffer::from_vertices(
                Some(&format!("Vertex({})", &self.name)), 
                device, 
                queue, 
                self.vertices
            ), 
            submeshes: self.submeshes.into_iter()
                .map(|values| IndexBuffer::new(
                    Some(&format!("Index({})", &self.name)), 
                    device, 
                    queue, 
                    values
                ))
                .collect(), 
            attributes: self.attributes.into_iter()
                .map(|(attribute, values)|(
                    attribute, 
                    VertexBuffer::from_attribute(
                        Some(&format!("Attribute({})", &self.name)), 
                        device, 
                        queue, 
                        values
                    )
                ))
                .collect(), 
            bindpose: (!self.bindposes.is_empty()).then(|| {
                BindMatrixBuffer::from_data(
                    Some(&format!("Uniform(Bindpose({}))", &self.name)), 
                    device, 
                    queue, 
                    self.bindposes
                )
            })
        }.into()
    }
}
