use std::collections::HashMap;
use std::env;
use std::env::Args;
use std::error::Error;
use std::num::NonZeroUsize;
use std::num::ParseFloatError;
use std::num::ParseIntError;
use std::path::Path;

use lazy_static::lazy_static;
use mod_error::err_msg;
use mod_error::RuntimeError;
use mod_util::AppFlags;

use crate::AppBuilder;

/// 명령어를 실행하는 함수 포인터 타입입니다.
type CmdFunc = fn(&mut Args, AppBuilder) -> Result<AppBuilder, Box<dyn Error + Send>>;

lazy_static! {
    /// 명령어 목록입니다.
    static ref OPTIONS: HashMap<&'static str, CmdFunc> = HashMap::from_iter([
        ("--num-threads", num_thread_callback as CmdFunc), 
        ("--show-frame-rate", show_frame_rate_callback as CmdFunc), 
        ("--no-vsync", no_vsync_callback as CmdFunc), 
        ("--enable-debug-layer", enable_debug_layer_callback as CmdFunc)
    ]);
}



/// 사용할 스레드 갯수를 설정하는 명령어 함수입니다.
fn num_thread_callback(args: &mut Args, mut builder: AppBuilder) -> Result<AppBuilder, Box<dyn Error + Send>> {
    // 스레드의 갯수를 가져옵니다.
    let argument = match args.next() {
        Some(argument) => argument, 
        None => return Err(err_msg!(ParsingError::EmptyCommand)), 
    };
    let num_threads = match argument.parse::<usize>() {
        Ok(num_threads) => num_threads, 
        Err(e) => return Err(err_msg!(e)),
    };

    // 스레드의 갯수가 0이 아닌 경우 스레드의 갯수를 설정합니다.
    if num_threads != 0 {
        builder = builder.with_num_threads(unsafe { 
            // safety: 스레드의 갯수는 항상 0보다 큼
            NonZeroUsize::new_unchecked(num_threads) 
        });
    }

    Ok(builder)
}

/// 프레임 레이트를 화면에 표시를 활성화하는 명령어 함수입니다.
fn show_frame_rate_callback(_: &mut Args, builder: AppBuilder) -> Result<AppBuilder, Box<dyn Error + Send>> {
    Ok(builder.with_flags(AppFlags::SHOW_FRAME_RATE))
}

/// 수직 동기화를 비활성화하는 명령어 함수입니다.
fn no_vsync_callback(_: &mut Args, builder: AppBuilder) -> Result<AppBuilder, Box<dyn Error + Send>> {
    Ok(builder.with_flags(AppFlags::DISABLE_VSYNC))
}

/// 쉐이더 디버깅 레이어를 활성화하는 명령어 함수입니다.
fn enable_debug_layer_callback(_: &mut Args, builder: AppBuilder) -> Result<AppBuilder, Box<dyn Error + Send>> {
    Ok(builder.with_flags(AppFlags::ENABLE_DEBUG_LAYER))
}



/// 명령줄 인수를 구문 분석할 때 발생할 수 있는 오류 목록입니다.
#[derive(Debug, thiserror::Error)]
pub enum ParsingError {
    /// 유효하지 않은 커맨드 입력이 들어온 경우 발생하는 오류입니다.
    #[error("Command line arguments are invalid!")]
    InvalidCommand, 

    /// 주어진 명령줄 인수가 비어있는 경우 발생하는 오류입니다.
    #[error("Command line arguments are empty!")]
    EmptyCommand, 

    /// 주어진 명령줄 인수가 부족한 경우 발생하는 오류입니다.
    #[error("Not enough command line argument!")]
    NotEnough, 

    /// 애플리케이션 실행 디렉토리 경로를 찾지 못한 경우 발생하는 오류입니다.
    #[error("Application execution directory path not found!")]
    RootPathNotFound, 

    /// 주어진 명령줄 인수를 정수 자료형으로 구문 분석하는데 실패할 경우 발생하는 오류입니다.
    #[error("Parsing integer failed for the following reasons: {0}")]
    ParsingIntFailure(#[from] ParseIntError), 

    /// 주어진 명령줄 인수를 실수 자료형으로 구문 분석하는데 실패할 경우 발생하는 오류입니다.
    #[error("Parsing float failed for the following reasons: {0}")]
    ParsingFloatFailure(#[from] ParseFloatError), 
}



/// 명령줄 인수를 구문 분석합니다.
/// 주어진 명령줄 인수에 따라 애플리케이션 빌더의 설정된 값을 변경합니다.
/// 
/// # Errors
/// 명령줄 인수 구문 분석에 실패할 경우 `ParsingError`를 반환합니다.
/// 
#[must_use]
pub(crate) fn parse_command_line_args(mut builder: AppBuilder) -> Result<AppBuilder, Box<dyn Error + Send>> {
    // 현재 애플리케이션 실행 디렉토리 경로를 가져옵니다.
    let mut args = env::args();
    let argument = match args.next() {
        Some(argument) => argument,
        None => return Err(err_msg!(ParsingError::EmptyCommand)), 
    };
    let current_exe = Path::new(&argument);
    let current_dir = match current_exe.parent() {
        Some(path) => path, 
        None => return Err(err_msg!(ParsingError::RootPathNotFound)), 
    };
    builder = builder.with_current_path(current_dir);

    #[cfg(feature = "command-line-args")] {
        while let Some(argument) = args.next() {
            // 해당 명령줄 인수에 대한 명령어 함수를 가져옵니다.
            let func = match OPTIONS.get(argument.as_str()) {
                Some(func) => func, 
                None => return Err(err_msg!(ParsingError::InvalidCommand)), 
            };

            // 명령어 함수를 실행합니다.
            builder = func(&mut args, builder)?;
        }
    }

    Ok(builder)
}
