mod attribute;
mod stage;

use std::{
    env,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub use self::{attribute::*, stage::*};

/// 프로그램의 디렉토리 경로를 가져옵니다.  
/// 이 함수는 명령줄 인수를 통해 프로그램의 현재 경로를 가져옵니다.
pub fn get_current_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let mut args = env::args();
        let argument = args.next().expect("command line arguments are empty!");
        let current_exe = PathBuf::from(argument);
        let current_path = current_exe
            .parent()
            .expect("the path to the executable file could not be found!")
            .to_path_buf();
        current_path
    })
}
