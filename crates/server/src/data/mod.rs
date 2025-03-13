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
        let argument = match args.next() {
            Some(arg) => arg,
            None => {
                log::error!("command line arguments are empty!");
                panic!("명령줄 인자가 비어있습니다!");
            }
        };

        let current_exe = PathBuf::from(argument);
        let current_path = match current_exe.parent() {
            Some(path) => path,
            None => {
                log::error!("the path to the executable file could not be found!");
                panic!("실행 파일의 디렉토리 경로를 찾을 수 없습니다!")
            }
        }
        .to_path_buf();

        current_path
    })
}
