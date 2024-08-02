mod enable_debug_layer;
pub(super) use self::enable_debug_layer::*;

mod num_threads;
pub(super) use self::num_threads::*;

mod show_frame_rate;
pub(super) use self::show_frame_rate::*;



use std::env::Args;
use hashbrown::HashMap;
use lazy_static::lazy_static;

use crate::app::AppBuilder;
use crate::error::ErrorMessage;

/// 명령어를 실행하는 함수 포인터 타입입니다.
type CmdFunc = fn(&mut Args, AppBuilder) -> Result<AppBuilder, ErrorMessage>;

lazy_static! {
    pub static ref OPTIONS: HashMap<&'static str, CmdFunc> = HashMap::from_iter([
        ("--num-threads", num_threads as CmdFunc),
        ("--show-frame-rate", show_frame_rate as CmdFunc), 
        ("--enable-debug-layer", enable_debug_layer as CmdFunc),
    ]);
}
