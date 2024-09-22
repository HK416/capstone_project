mod camera;

pub use self::camera::*;



/// 게임 오브젝트에 연결된 요소입니다.
pub trait Element: Send + Sync + 'static { }

impl<T: Sync + Send + 'static>  Element for T { }
