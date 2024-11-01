use std::{
    collections::HashMap, 
    env::{self, Args}, 
    io, 
    num::{ParseFloatError, ParseIntError}, 
    path::Path
};

use lazy_static::lazy_static;

use crate::etc::AppFlags;

use super::AppBuilder;

/// 명령어를 실행하는 함수 포인터 타입입니다.
type CmdFunc = fn(&mut Args, AppBuilder) -> Result<AppBuilder, CmdParsingError>;

lazy_static! {
    /// 사용 가능한 명령어 목록입니다.
    static ref OPTIONS: HashMap<&'static str, CmdFunc> = HashMap::from_iter([
        ("--num-threads", num_thread_fn as CmdFunc), 
        ("--show-frame-rate", show_frame_rate_fn as CmdFunc), 
        ("--no-vsync", no_vsync_fn as CmdFunc), 
        ("--enable-debug-layer", enable_debug_layer_fn as CmdFunc), 
    ]);
}



/// 명령줄 인수를 구문 분석할 때 발생할 수 있는 오류 목록입니다.
#[derive(Debug, thiserror::Error)]
pub enum CmdParsingError {
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

    #[error("Parsing address failed for the following reasons: {0}")]
    ParsingSocketAddrFailure(#[from] io::Error), 
}



/// 스레드 수를 설정하는 명령어 함수입니다.
fn num_thread_fn(args: &mut Args, mut builder: AppBuilder) -> Result<AppBuilder, CmdParsingError> {
    // 다음 명령줄 인자를 가져옵니다.
    let argument = match args.next() {
        Some(argument) => argument, 
        None => return Err(CmdParsingError::EmptyCommand),
    };

    // 명령줄 인자를 구문 분석합니다.
    let num_threads = match argument.parse::<usize>() {
        Ok(num_threads) => num_threads, 
        Err(e) => return Err(CmdParsingError::from(e))
    };

    // 전달된 스레드 수가 0이 아닌 경우 스레드 수를 설정합니다.
    if num_threads > 0 {
        builder = builder.with_num_threads(num_threads);
    } else {
        log::warn!("사용 가능한 스레드 수를 설정하지 못했습니다.");
    }

    Ok(builder)
}

/// 화면에 프레임 레이트 표시를 활성화하는 명령어 함수입니다.
fn show_frame_rate_fn(_: &mut Args, mut builder: AppBuilder) -> Result<AppBuilder, CmdParsingError> {
    builder = builder.with_flags(AppFlags::SHOW_FRAME_RATE);
    Ok(builder)
}

/// 수직 동기화를 비활성화하는 명령어 함수입니다.
fn no_vsync_fn(_: &mut Args, mut builder: AppBuilder) -> Result<AppBuilder, CmdParsingError> {
    builder = builder.with_flags(AppFlags::DISABLE_VSYNC);
    Ok(builder)
}

/// 쉐이더 디버깅 레이어를 활성화하는 명령어 함수입니다.
fn enable_debug_layer_fn(_: &mut Args, mut builder: AppBuilder) -> Result<AppBuilder, CmdParsingError> {
    builder = builder.with_flags(AppFlags::ENABLE_DEBUG_LAYER);
    Ok(builder)
}



/// 명령줄 인수를 구문 분석합니다.
/// 주어진 명령줄 인수에 따라 애플리케이션 빌더의 설정된 값을 변경합니다.
/// 
/// # Errors
/// 명령줄 인수 구문 분석에 실패한 경우 `CmdParsingError`를 반환합니다.
/// 
pub(crate) fn parse_command_line_args(mut builder: AppBuilder) -> Result<AppBuilder, CmdParsingError> {
    // 현재 애플리케이션 실행 디렉토리 경로를 가져옵니다.
    let mut args = env::args();
    let argument = match args.next() {
        Some(argument) => argument,
        None => return Err(CmdParsingError::EmptyCommand), 
    };
    let current_exe = Path::new(&argument);
    let current_dir = match current_exe.parent() {
        Some(path) => path.canonicalize()?, 
        None => return Err(CmdParsingError::RootPathNotFound), 
    };
    builder = builder.with_current_path(current_dir);

    #[cfg(feature = "command-line-args")] {
        while let Some(argument) = args.next() {
            // 해당 명령줄 인수에 대한 명령어 함수를 가져옵니다.
            let func = match OPTIONS.get(argument.as_str()) {
                Some(func) => func, 
                None => return Err(CmdParsingError::InvalidCommand), 
            };

            // 명령어 함수를 실행합니다.
            builder = func(&mut args, builder)?;
        }
    }

    Ok(builder)
}
