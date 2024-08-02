use std::env;
use std::path::Path;
use std::path::PathBuf;

use crate::app::AppBuilder;
use crate::command::CommandParsingError;
use crate::command::OPTIONS;
use crate::error::err_msg;
use crate::error::ErrorMessage;



/// 명령줄 인수를 구문 분석 합니다. 
/// 주어진 명령줄 인수에 따라 애플리케이션 빌더를 변경하며,  
/// 기본적으로 애플리케이션 실행 디렉토리 경로를 반환합니다.
/// 
/// # Errors
/// 명령줄 인수 구문 분석에 실패할 경우 `ErrorMessage`를 반환합니다.
/// 
#[must_use]
#[allow(unused_mut)]
pub fn parse_command_line_args(mut builder: AppBuilder) -> Result<(AppBuilder, PathBuf), ErrorMessage> {
    // 현재 애플리케이션 실행 디렉토리 경로를 가져옵니다.
    let mut args = env::args();
    let argument = args.next().ok_or(err_msg!(CommandParsingError::EmptyCommand))?;
    let path = Path::new(&argument);
    let path = path.parent()
        .ok_or(err_msg!(CommandParsingError::RootPathNoFound))?
        .to_path_buf();

    #[cfg(feature = "command-line-args")] {
        while let Some(argument) = args.next() {
            // 해당 명령줄 인수에 대한 명령어 함수를 가져옵니다.
            let func = OPTIONS.get(argument.as_str())
                .ok_or(err_msg!(CommandParsingError::InvalidCommand))?;

            // 명령어 함수를 실행합니다.
            builder = func(&mut args, builder)?;
        }
    }

    Ok((builder, path))
}
