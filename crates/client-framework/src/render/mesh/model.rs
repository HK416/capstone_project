use std::fmt;
use std::sync::Arc;
use std::ops::Range;
use hashbrown::HashMap;

use crate::render::mesh::Indices;
use crate::render::mesh::RenderableMesh;
use crate::render::mesh::VertexAttribute;
use crate::render::mesh::VertexAttributeValues;

use super::IndexValues;



/// 표준 3차원 모델의 메쉬 입니다.
/// 
/// 정점의 속성들은 미리 지정된 슬롯을 사용합니다.
/// 
#[derive(PartialEq, Eq)]
pub struct ModelMesh3D {
    ///메쉬의 이름 입니다.
    name: String, 

    /// 메쉬의 인덱스 버퍼 입니다.
    /// 
    /// ※ 기본값은 `None` 입니다.
    /// 
    indices: Option<Arc<Indices>>, 

    /// 메쉬의 정점 속성들 입니다.
    /// 
    /// 정점 속성의 기본 최대 슬롯의 갯수는 16개 입니다.
    /// 
    attributes: HashMap<u32, Arc<VertexAttribute>>, 
}

impl ModelMesh3D {
    /// 정정의 색상 속성의 슬롯 번호 입니다.
    pub const ATTRIBUTE_COLOR: u32 = 0;

    /// 정점의 위치 속성의 슬롯 번호 입니다.
    pub const ATTRIBUTE_POSITION: u32 = 1;

    /// 정점의 법선 속성의 슬롯 번호 입니다.
    pub const ATTRIBUTE_NORMAL: u32 = 2;
    
    /// 정점의 탄젠트 공간 법선 속성의 슬롯 번호 입니다.
    pub const ATTRIBUTE_TANGENT: u32 = 3;

    /// 정정의 0번 텍스처 좌표계 속성의 슬롯 번호 입니다.
    pub const ATTRIBUTE_TEXCOORD0: u32 = 4;
    
    /// 정점의 1번 텍스처 좌표계 속성의 슬롯 번호 입니다.
    pub const ATTRIBUTE_TEXCOORD1: u32 = 5;

    /// 정점의 뼈 번호 속성의 슬롯 번호 입니다.
    pub const ATTRIBUTE_JOINT_INDEX: u32 = 6;

    /// 정점의 뼈 가중치 속성의 슬롯 번호 입니다.
    pub const ATTRIBUTE_JOINT_WEIGHT: u32 = 7;

    /// 정점의 속성 데이터 변환 오류 메시지 입니다.
    const DATA_CONVERT_ERR_MSG: &'static str = "The given data format is invalid!";
}

impl ModelMesh3D {
    /// 새로운 표준 3차원 모델의 메쉬를 생성합니다.
    #[inline]
    #[must_use]
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self { 
            name: name.into(), 
            indices: None, 
            attributes: HashMap::with_capacity(16) 
        }
    }

    /// 정점의 색상 속성 데이터로 변환합니다.
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 색상 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    fn convert_color_values(values: VertexAttributeValues) -> VertexAttributeValues {
        VertexAttributeValues::Float32x4(match values {
            VertexAttributeValues::Float32x3(values) => values.into_iter()
                .map(|[r, g, b]| [r, g, b, 1.0])
                .collect(),
            VertexAttributeValues::Float32x4(values) => values,
            VertexAttributeValues::Uint8x4(values) => values.into_iter()
                .map(|[r, g, b, a]| {
                    const RHS: f32 = u8::MAX as f32;
                    [r as f32 / RHS, g as f32 / RHS, b as f32 / RHS, a as f32 / RHS]
                })
                .collect(),
            VertexAttributeValues::Uint16x4(values) => values.into_iter()
                .map(|[r, g, b, a]| {
                    const RHS: f32 = u16::MAX as f32;
                    [r as f32 / RHS, g as f32 / RHS, b as f32 / RHS, a as f32 / RHS]
                })
                .collect(), 
            _ => panic!("{}", Self::DATA_CONVERT_ERR_MSG)
        })
    }

    /// 정점의 위치 속성 데이터로 변환합니다.
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 위치 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    fn convert_position_values(values: VertexAttributeValues) -> VertexAttributeValues {
        VertexAttributeValues::Float32x3(match values {
            VertexAttributeValues::Float32x3(values) => values,
            _ => panic!("{}", Self::DATA_CONVERT_ERR_MSG)
        })
    }

    /// 정점의 법선 속성 데이터로 변환합니다.
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 법선 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    fn convert_normal_values(values: VertexAttributeValues) -> VertexAttributeValues {
        VertexAttributeValues::Float32x3(match values {
            VertexAttributeValues::Float32x3(values) => values,
            _ => panic!("{}", Self::DATA_CONVERT_ERR_MSG)
        })
    }

    /// 정점의 탄젠트 공간 법선 속성 데이터로 변환합니다.
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 탄젠트 공간 법선 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    fn convert_tangent_values(values: VertexAttributeValues) -> VertexAttributeValues {
        VertexAttributeValues::Float32x3(match values {
            VertexAttributeValues::Float32x3(values) => values,
            _ => panic!("{}", Self::DATA_CONVERT_ERR_MSG)
        })
    }

    /// 정점의 텍스처 좌표계 속성 데이터로 변환합니다.
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 텍스처 좌표계 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    fn convert_texcoord_values(values: VertexAttributeValues) -> VertexAttributeValues {
        VertexAttributeValues::Float32x2(match values {
            VertexAttributeValues::Float32x2(values) => values, 
            _ => panic!("{}", Self::DATA_CONVERT_ERR_MSG)
        })
    }

    /// 정점의 뼈 번호 속성 데이터로 변환합니다.
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 뼈 번호 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    fn convert_joint_index_values(values: VertexAttributeValues) -> VertexAttributeValues {
        VertexAttributeValues::Uint32x4(match values {
            VertexAttributeValues::Uint8x4(values) => values.into_iter()
                .map(|[x, y, z, w]| [x as u32, y as u32, z as u32, w as u32])
                .collect(),
            VertexAttributeValues::Uint16x4(values) => values.into_iter()
                .map(|[x, y, z, w]| [x as u32, y as u32, z as u32, w as u32])
                .collect(),
            VertexAttributeValues::Uint32x4(values) => values,
            _ => panic!("{}", Self::DATA_CONVERT_ERR_MSG)
        })
    }

    /// 정점의 뼈 가중치 속성 데이터로 변환합니다.
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 뼈 가중치 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    fn convert_joint_weight_values(values: VertexAttributeValues) -> VertexAttributeValues {
        VertexAttributeValues::Float32x4(match values {
            VertexAttributeValues::Float32x4(values) => values, 
            _ => panic!("{}", Self::DATA_CONVERT_ERR_MSG)
        })
    }



    /// 표준 3차원 모델 메쉬에 정점의 인덱스를 삽입합니다. </br>
    /// 정점의 인덱스가 이미 존재할 경우 이전 인덱스를 반환하고 새로운 인덱스를 저장합니다. </br>
    /// 
    #[inline]
    pub fn insert_indices(
        &mut self, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: IndexValues
    ) -> Option<Arc<Indices>> {
        self.indices.replace(
            Indices::new(Some(&format!("Index({})", self.name)), device, queue, values)
        )
    }

    /// 표준 3차원 모델 메쉬에 정점의 인덱스를 제거합니다. </br>
    /// 정점의 인덱스가 존재하지 않는 경우 `None`을 반환합니다. </br>
    #[inline]
    pub fn remove_indices(&mut self) -> Option<Arc<Indices>> {
        self.indices.take()
    }

    /// 표준 3차원 모델 메쉬에 정점의 색상 속성을 삽입합니다. </br>
    /// 정점의 색상 속성이 이미 존재할 경우 이전의 속성을 반환하고 새로운 속성을 저장합니다. </br>
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 색상 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    #[inline]
    pub fn insert_color(
        &mut self, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: VertexAttributeValues
    ) -> Option<Arc<VertexAttribute>> {
        self.attributes.insert(
            Self::ATTRIBUTE_COLOR, 
            VertexAttribute::new(
                Some(&format!("Color({})", self.name)), 
                device, 
                queue, 
                Self::convert_color_values(values)
            )
        )
    }

    /// 표준 3차원 모델 메쉬에 정점의 색상 속성을 제거합니다. </br>
    /// 정점의 색상 속성이 존재하지 않는 경우 `None`을 반환합니다. </br>
    #[inline]
    pub fn remove_color(&mut self) -> Option<Arc<VertexAttribute>> {
        self.attributes.remove(&Self::ATTRIBUTE_COLOR)
    }

    /// 표준 3차원 모델 메쉬에 정점의 위치 속성을 삽입합니다. </br>
    /// 정점의 위치 속성이 이미 존재할 경우 이전의 속성을 반환하고 새로운 속성을 저장합니다. </br>
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 위치 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    #[inline]
    pub fn insert_position(
        &mut self, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: VertexAttributeValues
    ) -> Option<Arc<VertexAttribute>> {
        self.attributes.insert(
            Self::ATTRIBUTE_POSITION, 
            VertexAttribute::new(
                Some(&format!("Position({})", self.name)), 
                device, 
                queue, 
                Self::convert_position_values(values)
            )
        )
    }

    /// 표준 3차원 모델 메쉬에 정점의 위치 속성을 제거합니다. </br>
    /// 정점의 위치 속성이 존재하지 않는 경우 `None`을 반환합니다. </br>
    #[inline]
    pub fn remove_position(&mut self) -> Option<Arc<VertexAttribute>> {
        self.attributes.remove(&Self::ATTRIBUTE_POSITION)
    }

    /// 표준 3차원 모델 메쉬에 정점의 법선 속성을 삽입합니다. </br>
    /// 정점의 법선 속성이 이미 존재할 경우 이전의 속성을 반환하고 새로운 속성을 저장합니다. </br>
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 법선 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    #[inline]
    pub fn insert_normal(
        &mut self, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: VertexAttributeValues
    ) -> Option<Arc<VertexAttribute>> {
        self.attributes.insert(
            Self::ATTRIBUTE_NORMAL, 
            VertexAttribute::new(
                Some(&format!("Normal({})", self.name)), 
                device, 
                queue, 
                Self::convert_normal_values(values)
            )
        )
    }

    /// 표준 3차원 모델 메쉬에 정점의 법선 속성을 제거합니다. </br>
    /// 정점의 법선 속성이 존재하지 않는 경우 `None`을 반환합니다. </br>
    #[inline]
    pub fn remove_normal(&mut self) -> Option<Arc<VertexAttribute>> {
        self.attributes.remove(&Self::ATTRIBUTE_NORMAL)
    }

    /// 표준 3차원 모델 메쉬에 정점의 탄젠트 공간 법선 속성을 삽입합니다. </br>
    /// 정점의 탄젠트 공간 법선 속성이 이미 존재할 경우 이전의 속성을 반환하고 새로운 속성을 저장합니다. </br>
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 탄젠트 공간 법선 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    #[inline]
    pub fn insert_tangent(
        &mut self, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: VertexAttributeValues
    ) -> Option<Arc<VertexAttribute>> {
        self.attributes.insert(
            Self::ATTRIBUTE_TANGENT, 
            VertexAttribute::new(
                Some(&format!("Tangent({})", self.name)), 
                device, 
                queue, 
                Self::convert_tangent_values(values)
            )
        )
    }

    /// 표준 3차원 모델 메쉬에 정점의 탄젠트 공간 법선 속성을 제거합니다. </br>
    /// 정점의 탄젠트 공간 법선 속성이 존재하지 않는 경우 `None`을 반환합니다. </br>
    #[inline]
    pub fn remove_tangent(&mut self) -> Option<Arc<VertexAttribute>> {
        self.attributes.remove(&Self::ATTRIBUTE_TANGENT)
    }

    /// 표준 3차원 모델 메쉬에 정점의 0번 텍스처 좌표 속성을 삽입합니다. </br>
    /// 정점의 0번 텍스처 좌표 속성이 이미 존재할 경우 이전의 속성을 반환하고 새로운 속성을 저장합니다. </br>
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 텍스터 좌표 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    #[inline]
    pub fn insert_texcoord0(
        &mut self, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: VertexAttributeValues
    ) -> Option<Arc<VertexAttribute>> {
        self.attributes.insert(
            Self::ATTRIBUTE_TEXCOORD0, 
            VertexAttribute::new(
                Some(&format!("Texcoord0({})", self.name)), 
                device, 
                queue, 
                Self::convert_texcoord_values(values)
            )
        )
    }

    /// 표준 3차원 모델 메쉬에 정점의 0번 텍스처 좌표 속성을 제거합니다. </br>
    /// 정점의 0번 텍스처 좌표 속성이 존재하지 않는 경우 `None`을 반환합니다. </br>
    #[inline]
    pub fn remove_texcoord0(&mut self) -> Option<Arc<VertexAttribute>> {
        self.attributes.remove(&Self::ATTRIBUTE_TEXCOORD0)
    }

    /// 표준 3차원 모델 메쉬에 정점의 1번 텍스처 좌표 속성을 삽입합니다. </br>
    /// 정점의 1번 텍스처 좌표 속성이 이미 존재할 경우 이전의 속성을 반환하고 새로운 속성을 저장합니다. </br>
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 텍스터 좌표 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    #[inline]
    pub fn insert_texcoord1(
        &mut self, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: VertexAttributeValues
    ) -> Option<Arc<VertexAttribute>> {
        self.attributes.insert(
            Self::ATTRIBUTE_TEXCOORD1, 
            VertexAttribute::new(
                Some(&format!("Texcoord1({})", self.name)), 
                device, 
                queue, 
                Self::convert_texcoord_values(values)
            )
        )
    }

    /// 표준 3차원 모델 메쉬에 정점의 1번 텍스처 좌표 속성을 제거합니다. </br>
    /// 정점의 1번 텍스처 좌표 속성이 존재하지 않는 경우 `None`을 반환합니다. </br>
    #[inline]
    pub fn remove_texcoord1(&mut self) -> Option<Arc<VertexAttribute>> {
        self.attributes.remove(&Self::ATTRIBUTE_TEXCOORD1)
    }

    /// 표준 3차원 모델 메쉬에 정점의 뼈 번호 속성을 삽입합니다. </br>
    /// 정점의 뼈 번호 속성이 이미 존재할 경우 이전의 속성을 반환하고 새로운 속성을 저장합니다. </br>
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 뼈 번호 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    #[inline]
    pub fn insert_joint_index(
        &mut self, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: VertexAttributeValues
    ) -> Option<Arc<VertexAttribute>> {
        self.attributes.insert(
            Self::ATTRIBUTE_JOINT_INDEX, 
            VertexAttribute::new(
                Some(&format!("Index({})", self.name)), 
                device, 
                queue, 
                Self::convert_joint_index_values(values)
            )
        )
    }

    /// 표준 3차원 모델 메쉬에 정점의 뼈 번호 속성을 제거합니다. </br>
    /// 정점의 뼈 번호 속성이 존재하지 않는 경우 `None`을 반환합니다. </br>
    #[inline]
    pub fn remove_joint_index(&mut self) -> Option<Arc<VertexAttribute>> {
        self.attributes.remove(&Self::ATTRIBUTE_JOINT_INDEX)
    }

    /// 표준 3차원 모델 메쉬에 정점의 뼈 가중치 속성을 삽입합니다. </br>
    /// 정점의 뼈 가중치 속성이 이미 존재할 경우 이전의 속성을 반환하고 새로운 속성을 저장합니다. </br>
    /// 
    /// # Panics
    /// 주어진 데이터로 정점의 뼈 가중치 속성을 생성할 수 없는 경우 `panic!`을 호출합니다.
    /// 
    #[inline]
    pub fn insert_joint_weight(
        &mut self, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        values: VertexAttributeValues
    ) -> Option<Arc<VertexAttribute>> {
        self.attributes.insert(
            Self::ATTRIBUTE_JOINT_WEIGHT, 
            VertexAttribute::new(
                Some(&format!("Weight({})", self.name)), 
                device, 
                queue, 
                Self::convert_joint_weight_values(values)
            )
        )
    }

    /// 표준 3차원 모델 메쉬에 정점의 뼈 가중치 속성을 제거합니다. </br>
    /// 정점의 뼈 가중치 속성이 존재하지 않는 경우 `None`을 반환합니다. </br>
    #[inline]
    pub fn remove_joint_weight(&mut self) -> Option<Arc<VertexAttribute>> {
        self.attributes.remove(&Self::ATTRIBUTE_JOINT_WEIGHT)
    }
}

impl RenderableMesh for ModelMesh3D {
    fn bind<'a>(&'a self, attributes: &[u32], encoder: &mut dyn wgpu::util::RenderEncoder<'a>) {
        for (slot, attribute) in attributes.iter().enumerate() {
            encoder.set_vertex_buffer(
                slot as u32, 
                self.attributes.get(attribute)
                    .expect("The required attribute could not be found!")
                    .buffer
                    .slice(..)
            );
        }

        if let Some(indices) = &self.indices {
            encoder.set_index_buffer(indices.buffer.slice(..), indices.format);
        }
    }
    
    fn draw<'a>(&'a self, instances: Range<u32>, encoder: &mut dyn wgpu::util::RenderEncoder<'a>) {
        if let Some(indices) = &self.indices {
            encoder.draw_indexed(0..indices.count, 0, instances);
        } else if let Some(attribute) = self.attributes.get(&Self::ATTRIBUTE_POSITION) {
            encoder.draw(0..attribute.count, instances);
        } else {
            log::warn!("표준 3차원 모델 메쉬에 정점의 위치 속성이 비어있습니다!");
        }
    }
}

impl fmt::Debug for ModelMesh3D {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(Self))
            .field(&self.name)
            .finish()
    }
}
