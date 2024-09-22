use std::{mem, ops::{self, Index, IndexMut}, sync::Arc};

use bytemuck::{Pod, Zeroable};

/// 뼈의 최대 개수입니다.
const MAX_BONES: usize = 256;



/// 스키닝되지 않은 메쉬의 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeshDataLayout {
    /// 게임 오브젝트의 월드 변환 행렬입니다.
    pub trans: gmm::Float4x4, 
}

impl Default for MeshDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            trans: gmm::Float4x4::IDENTITY, 
        }
    }
}



/// 스키닝 되지 않은 메쉬의 유니폼 버퍼입니다.
#[derive(Debug, Clone)]
pub struct MeshUniform {
    buffer: Arc<wgpu::Buffer>
}

impl MeshUniform {
    /// 스키닝 되지 않은 메쉬의 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<MeshDataLayout>() as wgpu::BufferAddress;

    /// 스키닝 되지 않은 메쉬의 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl MeshUniform {
    /// 초기화되지 않은 새로운 스키닝되지 않은 메쉬의 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn new(label: Option<&str>, device: &Arc<wgpu::Device>) -> Self {
        Self { 
            buffer: device.create_buffer(
                &wgpu::BufferDescriptor {
                    label, 
                    mapped_at_creation: false, 
                    size: Self::SIZE, 
                    usage: Self::USAGES
                }
            ).into()
        }
    }

    /// 스키닝되지 않은 메쉬의 유니폼 버퍼의 데이터를 갱신합니다.
    pub fn update(&self, queue: &Arc<wgpu::Queue>, data: MeshDataLayout) {
        let capturable = self.buffer.clone();
        let queue_cloned = queue.clone();
        self.buffer.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut MeshDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                    queue_cloned.submit([]);
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (MeshUniform :: {})", e);
                }
            }
        });
    }
}

impl ops::Deref for MeshUniform {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}



/// 스키닝된 메쉬의 뼈 정보 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SkinnedMeshDataLayout {
    pub quality: u32, 
    pub num_bones: u32, 
    pub _padding0: [u8; 8]
}

impl Default for SkinnedMeshDataLayout {
    #[inline]
    fn default() -> Self {
        Self { 
            quality: 0, 
            num_bones: 0, 
            _padding0: [0; 8] 
        }
    }
}



/// 스키닝된 메쉬의 유니폼 버퍼입니다.
#[derive(Debug, Clone)]
pub struct SkinnedMeshUniform {
    buffer: Arc<wgpu::Buffer>
}

impl SkinnedMeshUniform {
    /// 스키닝된 메쉬 유니폼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<SkinnedMeshDataLayout>() as wgpu::BufferAddress;

    /// 스키닝된 메쉬의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl SkinnedMeshUniform {
    /// 초기화되지 않은 새로운 스키닝된 메쉬의 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn new(label: Option<&str>, device: &Arc<wgpu::Device>) -> Self {
        Self { 
            buffer: device.create_buffer(
                &wgpu::BufferDescriptor {
                    label, 
                    mapped_at_creation: false, 
                    size: Self::SIZE, 
                    usage: Self::USAGES
                }
            ).into()
        }
    }

    /// 스키닝된 메쉬의 유니폼 버퍼의 데이터를 갱신합니다.
    pub fn update(&self, queue: &Arc<wgpu::Queue>, data: SkinnedMeshDataLayout) {
        let capturable = self.buffer.clone();
        let queue_cloned = queue.clone();
        self.buffer.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut SkinnedMeshDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                    queue_cloned.submit([]);
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (SkinnedMeshUniform :: {})", e);
                }
            }
        });
    }
}

impl ops::Deref for SkinnedMeshUniform {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}



/// 뼈 변환 행렬의 데이터 레이아웃입니다.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BoneDataLayout([gmm::Float4x4; MAX_BONES]);

impl  BoneDataLayout {
    #[inline]
    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = &gmm::Float4x4> {
        self.0.iter()
    }

    #[inline]
    #[must_use]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut gmm::Float4x4> {
        self.0.iter_mut()
    }
}

impl Index<usize> for BoneDataLayout {
    type Output = gmm::Float4x4;
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for BoneDataLayout {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Default for BoneDataLayout {
    #[inline]
    fn default() -> Self {
        Self([gmm::Float4x4::IDENTITY; MAX_BONES])
    }
}



/// 뼈 변환 행렬의 유니폼 버퍼입니다.
#[derive(Debug, Clone)]
pub struct BoneUniform {
    buffer: Arc<wgpu::Buffer>
}

impl BoneUniform {
    /// 뼈 변환 행렬 유니폼 버퍼의 크기입니다.
    pub const SIZE: wgpu::BufferAddress = mem::size_of::<BoneDataLayout>() as wgpu::BufferAddress;

    /// 뼈 변환 행렬 유니폼 버퍼의 [wgpu::BufferUsages]입니다.
    pub const USAGES: wgpu::BufferUsages = wgpu::BufferUsages::UNIFORM
        .union(wgpu::BufferUsages::MAP_WRITE)
        .union(wgpu::BufferUsages::COPY_DST);
}

impl BoneUniform {
    /// 초기화되지 않은 새로운 뼈 변환 행렬 유니폼 버퍼를 생성합니다.
    #[must_use]
    pub fn new(label: Option<&str>, device: &Arc<wgpu::Device>) -> Self {
        Self { 
            buffer: device.create_buffer(
                &wgpu::BufferDescriptor {
                    label, 
                    mapped_at_creation: false, 
                    size: Self::SIZE, 
                    usage: Self::USAGES
                }
            ).into()
        }
    }

    /// 뼈 변환 행렬 유니폼 버퍼 데이터를 갱신합니다.
    pub fn update(&self, queue: &Arc<wgpu::Queue>, data: BoneDataLayout) {
        let capturable = self.buffer.clone();
        let queue_cloned = queue.clone();
        self.buffer.slice(..).map_async(wgpu::MapMode::Write, move |result| {
            match result {
                Ok(_) => {
                    let mut buffer_view = capturable.slice(..).get_mapped_range_mut();
                    let data_layout: &mut BoneDataLayout = bytemuck::from_bytes_mut(&mut buffer_view);

                    *data_layout = data;

                    drop(buffer_view);
                    capturable.unmap();
                    queue_cloned.submit([]);
                }, 
                Err(e) => {
                    log::warn!("Failed to write uniform buffer! (BoneDataLayout :: {})", e);
                }
            }
        });
    }
}

impl ops::Deref for BoneUniform {
    type Target = wgpu::Buffer;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}



static_assertions::const_assert_eq!(MeshUniform::SIZE as usize, mem::size_of::<MeshDataLayout>());
static_assertions::const_assert_eq!(SkinnedMeshUniform::SIZE as usize, mem::size_of::<SkinnedMeshDataLayout>());
static_assertions::const_assert_eq!(BoneUniform::SIZE as usize, mem::size_of::<BoneDataLayout>());
