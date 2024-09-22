mod camera;
mod projection;

pub use self::camera::*;
pub use self::projection::*;



/// 게임 오브젝트에 연결된 요소입니다.
pub trait Element: Send + Sync + 'static { }

impl<T: Sync + Send + 'static>  Element for T { }
