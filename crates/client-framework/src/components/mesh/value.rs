/// 인덱스 버퍼 데이터 입니다.
#[derive(Debug, Clone)]
pub enum IndexValues {
    Uint16(Vec<u16>), 
    Uint32(Vec<u32>), 
}

impl IndexValues {
    /// 인덱스 버퍼 데이터가 비어있는 경우 `true`를 반환합니다.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 인덱스 버퍼 데이터의 바이트 단위 크기를 반환합니다.
    #[inline]
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 인덱스 버퍼 데이터 요소의 바이트 단위 크기를 반환합니다.
    #[inline]
    pub fn stride(&self) -> usize {
        use std::mem::size_of;
        match self {
            IndexValues::Uint16(_) => size_of::<u16>(),
            IndexValues::Uint32(_) => size_of::<u32>(),
        }
    }

    /// 인덱스 버퍼 데이터 요소의 갯수를 반환합니다.
    #[inline]
    pub fn count(&self) -> usize {
        match self {
            IndexValues::Uint16(values) => values.len(),
            IndexValues::Uint32(values) => values.len(),
        }
    }

    /// 인덱스 버퍼 데이터의 바이트 배열을 반환합니다.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        use bytemuck::cast_slice;
        match self {
            IndexValues::Uint16(values) => cast_slice(&values),
            IndexValues::Uint32(values) => cast_slice(&values),
        }
    }

    /// 인덱스 버퍼 데이터의 형식을 반환합니다.
    #[inline]
    pub fn format(&self) -> wgpu::IndexFormat {
        match self {
            IndexValues::Uint16(_) => wgpu::IndexFormat::Uint16,
            IndexValues::Uint32(_) => wgpu::IndexFormat::Uint32,
        }
    }
}



/// 정점 버퍼 데이터 입니다.
#[derive(Debug, Clone)]
pub enum VertexAttributeValues {
    /// 1개의 요소를 갖는 32bit 실수
    Float32(Vec<f32>), 
    /// 2개의 요소를 갖는 32bit 실수
    Float32x2(Vec<[f32; 2]>), 
    /// 3개의 요소를 갖는 32bit 실수
    Float32x3(Vec<[f32; 3]>), 
    /// 4개의 요소를 갖는 32bit 실수
    Float32x4(Vec<[f32; 4]>), 
    /// 2개의 요소를 갖는 부호 있는 8bit 정수
    Sint8x2(Vec<[i8; 2]>), 
    /// 4개의 요소를 갖는 부호 있는 8bit 정수
    Sint8x4(Vec<[i8; 4]>), 
    /// 2개의 요소를 갖는 부호 없는 8bit 정수
    Uint8x2(Vec<[u8; 2]>), 
    /// 4개의 요소를 갖는 부호 없는 8bit 정수
    Uint8x4(Vec<[u8; 4]>), 
    /// 2개의 요소를 갖는 부호 있는 16bit 정수
    Sint16x2(Vec<[i16; 2]>), 
    /// 4개의 요소를 갖는 부호 있는 16bit 정수
    Sint16x4(Vec<[i16; 4]>), 
    /// 2개의 요소를 갖는 부호 없는 16bit 정수
    Uint16x2(Vec<[u16; 2]>), 
    /// 4개의 요소를 갖는 부호 없는 16bit 정수
    Uint16x4(Vec<[u16; 4]>), 
    /// 1개의 요소를 갖는 부호 있는 32bit 정수
    Sint32(Vec<i32>), 
    /// 2개의 요소를 갖는 부호 있는 32bit 정수
    Sint32x2(Vec<[i32; 2]>), 
    /// 3개의 요소를 갖는 부호 있는 32bit 정수
    Sint32x3(Vec<[i32; 3]>), 
    /// 4개의 요소를 갖는 부호 있는 32bit 정수
    Sint32x4(Vec<[i32; 4]>), 
    /// 1개의 요소를 갖는 부호 없는 32bit 정수
    Uint32(Vec<u32>), 
    /// 2개의 요소를 갖는 부호 없는 32bit 정수
    Uint32x2(Vec<[u32; 2]>), 
    /// 3개의 요소를 갖는 부호 없는 32bit 정수
    Uint32x3(Vec<[u32; 3]>), 
    /// 4개의 요소를 갖는 부호 없는 32bit 정수
    Uint32x4(Vec<[u32; 4]>), 
}

impl VertexAttributeValues {
    /// 정점 버퍼 데이터가 비어있는 경우 `true`를 반환합니다.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// 정점 버퍼 데이터의 바이트 단위 크기를 반환합니다.
    #[inline]
    pub fn size(&self) -> usize {
        self.stride() * self.count()
    }

    /// 정점 버퍼 데이터 요소의 바이트 단위 크기를 반환합니다.
    #[inline]
    pub fn stride(&self) -> usize {
        use std::mem::size_of;
        match self {
            VertexAttributeValues::Float32(_) => size_of::<f32>(),
            VertexAttributeValues::Float32x2(_) => size_of::<[f32; 2]>(),
            VertexAttributeValues::Float32x3(_) => size_of::<[f32; 3]>(),
            VertexAttributeValues::Float32x4(_) => size_of::<[f32; 4]>(),
            VertexAttributeValues::Sint8x2(_) => size_of::<[i8; 2]>(),
            VertexAttributeValues::Sint8x4(_) => size_of::<[i8; 4]>(),
            VertexAttributeValues::Uint8x2(_) => size_of::<[u8; 2]>(),
            VertexAttributeValues::Uint8x4(_) => size_of::<[u8; 4]>(),
            VertexAttributeValues::Sint16x2(_) => size_of::<[i16; 2]>(),
            VertexAttributeValues::Sint16x4(_) => size_of::<[i16; 4]>(),
            VertexAttributeValues::Uint16x2(_) => size_of::<[u16; 2]>(),
            VertexAttributeValues::Uint16x4(_) => size_of::<[u16; 4]>(),
            VertexAttributeValues::Sint32(_) => size_of::<i32>(),
            VertexAttributeValues::Sint32x2(_) => size_of::<[i32; 2]>(),
            VertexAttributeValues::Sint32x3(_) => size_of::<[i32; 3]>(),
            VertexAttributeValues::Sint32x4(_) => size_of::<[i32; 4]>(),
            VertexAttributeValues::Uint32(_) => size_of::<u32>(),
            VertexAttributeValues::Uint32x2(_) => size_of::<[u32; 2]>(),
            VertexAttributeValues::Uint32x3(_) => size_of::<[u32; 3]>(),
            VertexAttributeValues::Uint32x4(_) => size_of::<[u32; 4]>(),
        }
    }

    /// 인덱스 버퍼 데이터 요소의 갯수를 반환합니다.
    #[inline]
    pub fn count(&self) -> usize {
        match self {
            VertexAttributeValues::Float32(values) => values.len(),
            VertexAttributeValues::Float32x2(values) => values.len(),
            VertexAttributeValues::Float32x3(values) => values.len(),
            VertexAttributeValues::Float32x4(values) => values.len(),
            VertexAttributeValues::Sint8x2(values) => values.len(),
            VertexAttributeValues::Sint8x4(values) => values.len(),
            VertexAttributeValues::Uint8x2(values) => values.len(),
            VertexAttributeValues::Uint8x4(values) => values.len(),
            VertexAttributeValues::Sint16x2(values) => values.len(),
            VertexAttributeValues::Sint16x4(values) => values.len(),
            VertexAttributeValues::Uint16x2(values) => values.len(),
            VertexAttributeValues::Uint16x4(values) => values.len(),
            VertexAttributeValues::Sint32(values) => values.len(),
            VertexAttributeValues::Sint32x2(values) => values.len(),
            VertexAttributeValues::Sint32x3(values) => values.len(),
            VertexAttributeValues::Sint32x4(values) => values.len(),
            VertexAttributeValues::Uint32(values) => values.len(),
            VertexAttributeValues::Uint32x2(values) => values.len(),
            VertexAttributeValues::Uint32x3(values) => values.len(),
            VertexAttributeValues::Uint32x4(values) => values.len(),
        }
    }

    /// 인덱스 버퍼 데이터의 바이트 배열을 반환합니다.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        use bytemuck::cast_slice;
        match self {
            VertexAttributeValues::Float32(values) => cast_slice(&values),
            VertexAttributeValues::Float32x2(values) => cast_slice(values),
            VertexAttributeValues::Float32x3(values) => cast_slice(values),
            VertexAttributeValues::Float32x4(values) => cast_slice(values),
            VertexAttributeValues::Sint8x2(values) => cast_slice(values),
            VertexAttributeValues::Sint8x4(values) => cast_slice(values),
            VertexAttributeValues::Uint8x2(values) => cast_slice(values),
            VertexAttributeValues::Uint8x4(values) => cast_slice(values),
            VertexAttributeValues::Sint16x2(values) => cast_slice(values),
            VertexAttributeValues::Sint16x4(values) => cast_slice(values),
            VertexAttributeValues::Uint16x2(values) => cast_slice(values),
            VertexAttributeValues::Uint16x4(values) => cast_slice(values),
            VertexAttributeValues::Sint32(values) => cast_slice(values),
            VertexAttributeValues::Sint32x2(values) => cast_slice(values),
            VertexAttributeValues::Sint32x3(values) => cast_slice(values),
            VertexAttributeValues::Sint32x4(values) => cast_slice(values),
            VertexAttributeValues::Uint32(values) => cast_slice(values),
            VertexAttributeValues::Uint32x2(values) => cast_slice(values),
            VertexAttributeValues::Uint32x3(values) => cast_slice(values),
            VertexAttributeValues::Uint32x4(values) => cast_slice(values),
        }
    }

    /// 인덱스 버퍼 데이터의 형식을 반환합니다.
    #[inline]
    pub fn format(&self) -> wgpu::VertexFormat {
        match self {
            VertexAttributeValues::Float32(_) => wgpu::VertexFormat::Float32,
            VertexAttributeValues::Float32x2(_) => wgpu::VertexFormat::Float32x2,
            VertexAttributeValues::Float32x3(_) => wgpu::VertexFormat::Float32x3,
            VertexAttributeValues::Float32x4(_) => wgpu::VertexFormat::Float32x4,
            VertexAttributeValues::Sint8x2(_) => wgpu::VertexFormat::Sint8x2,
            VertexAttributeValues::Sint8x4(_) => wgpu::VertexFormat::Sint8x4,
            VertexAttributeValues::Uint8x2(_) => wgpu::VertexFormat::Uint8x2,
            VertexAttributeValues::Uint8x4(_) => wgpu::VertexFormat::Uint8x4,
            VertexAttributeValues::Sint16x2(_) => wgpu::VertexFormat::Sint16x2,
            VertexAttributeValues::Sint16x4(_) => wgpu::VertexFormat::Sint16x4,
            VertexAttributeValues::Uint16x2(_) => wgpu::VertexFormat::Uint16x2,
            VertexAttributeValues::Uint16x4(_) => wgpu::VertexFormat::Uint16x4,
            VertexAttributeValues::Sint32(_) => wgpu::VertexFormat::Sint32,
            VertexAttributeValues::Sint32x2(_) => wgpu::VertexFormat::Sint32x2,
            VertexAttributeValues::Sint32x3(_) => wgpu::VertexFormat::Sint32x3,
            VertexAttributeValues::Sint32x4(_) => wgpu::VertexFormat::Sint32x4,
            VertexAttributeValues::Uint32(_) => wgpu::VertexFormat::Uint32,
            VertexAttributeValues::Uint32x2(_) => wgpu::VertexFormat::Uint32x2,
            VertexAttributeValues::Uint32x3(_) => wgpu::VertexFormat::Uint32x3,
            VertexAttributeValues::Uint32x4(_) => wgpu::VertexFormat::Uint32x4,
        }
    }
}
