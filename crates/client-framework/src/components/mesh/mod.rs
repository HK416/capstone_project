mod attribute;
pub use self::attribute::*;

mod index;
pub use self::index::*;

mod standard;
pub use self::standard::*;

mod value;
pub use self::value::*;



use std::fmt;
use std::ops::Range;   

/// 그리기 가능한 메쉬의 `trait` 입니다.
pub trait RenderableMesh : fmt::Debug {
    /// 렌더 상태 머신에 Mesh의 버텍스 버퍼와 인덱스 버퍼를 바인드 합니다.
    fn bind<'a>(&'a self, encoder: &mut dyn wgpu::util::RenderEncoder<'a>);

    /// 렌더 명령 대기열에 Mesh 그리기 명령을 추가합니다.
    fn draw<'a>(&'a self, instances: Range<u32>, encoder: &mut dyn wgpu::util::RenderEncoder<'a>);
}
