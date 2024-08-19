mod enable_debug_layer;
mod no_vsync;
mod num_threads;
mod show_frame_rate;



use std::env::Args;
use std::collections::HashMap;
use lazy_static::lazy_static;

use crate::app::AppBuilder;
use crate::error::ErrorMessage;

/// 명령어를 실행하는 함수 포인터 타입입니다.
type CmdFunc = fn(&mut Args, AppBuilder) -> Result<AppBuilder, ErrorMessage>;

lazy_static! {
    pub static ref OPTIONS: HashMap<&'static str, CmdFunc> = HashMap::from_iter([
        ("--num-threads", num_threads::callback as CmdFunc),
        ("--show-frame-rate", show_frame_rate::callback as CmdFunc), 
        ("--no-vsync", no_vsync::callback as CmdFunc), 
        ("--enable-debug-layer", enable_debug_layer::callback as CmdFunc),
    ]);
}
