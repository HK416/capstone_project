use std::sync::Arc;
use std::panic;
use std::process;

use winit::window::Window;

use crate::alert_error;



/// [`panic!`] 호출시 처리할 함수를 설정합니다.
pub fn set_panic_hooker(owner: Option<Arc<Window>>) {
    panic::set_hook(Box::new(move |info| {
        log::error!("{:?}", info);
        alert_error("Runtime error", info.to_string(), owner.as_deref());
        process::exit(-1);
    }))
}
