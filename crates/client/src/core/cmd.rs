use crate::core::app::builder::AppBuilder;
use crate::core::app::flag::AppFlags;
use crate::error::AppError;

use std::env::{self, Args};
use std::path::Path;
use std::path::PathBuf;
use hashbrown::HashMap;
use lazy_static::lazy_static;

/// 명령어를 실행하는 함수 포인터 타입입니다.
type CmdFunc = fn(&mut Args, AppBuilder) -> Result<AppBuilder, AppError>;

lazy_static! {
    static ref MAP: HashMap<&'static str, CmdFunc> = HashMap::from_iter([
        ("--num-threads", num_threads as CmdFunc),
        ("--show-frame-rate", show_frame_rate as CmdFunc), 
        ("--enable-debug-layer", enable_debug_layer as CmdFunc),
    ]);
}



/// 사용할 스레드 갯수를 설정하는 명령어 함수 입니다.
#[inline]
fn num_threads(args: &mut Args, builder: AppBuilder) -> Result<AppBuilder, AppError> {
    use std::num::NonZeroUsize;

    // 스레드의 갯수를 구문 분석 합니다.
    let argument = args.next().ok_or(AppError::CommandLine(
        format!("Not enough command line arguments!")
    ))?;
    let num_threads: usize = argument.parse().map_err(|e| AppError::CommandLine(
        format!("Parsing the number of threads failed for the following reasons: {}", e)
    ))?;

    // 스레드의 갯수를 설정합니다.
    // safety: `num_threads`는 항상 1보다 큼.
    //
    unsafe {
        Ok(builder.set_num_threads(
            NonZeroUsize::new_unchecked(match num_threads == 0 {
                true => num_cpus::get_physical(),
                false => num_threads
            })
        ))
    }
}

/// 애플리케이션 생성 플래그 옵션을 설정하는 명령어 함수 입니다.
#[inline]
fn show_frame_rate(_: &mut Args, builder: AppBuilder) -> Result<AppBuilder, AppError> {
    Ok(builder.add_flags(AppFlags::SHOW_FRAME_RATE))
}

/// 애플리케이션 생성 플래그 옵션을 설정하는 명령어 함수 입니다.
#[inline]
fn enable_debug_layer(_: &mut Args, builder: AppBuilder) -> Result<AppBuilder, AppError> {
    Ok(builder.add_flags(AppFlags::ENABLE_DEBUG_LAYER))
}



/// 명령줄 인수를 구문 분석 합니다.
/// 
/// 명령줄 인수 구문 분석에 성공할 경우 애플리케이션 빌더와 현재 애플리케이션 실행 디렉토리 경로를 반환합니다.
/// 
#[must_use]
#[allow(unused_mut)]
pub fn parse_command_line_args(
    mut builder: AppBuilder
) -> Result<(AppBuilder, PathBuf), AppError> {
    // 현재 애플리케이션 실행 디렉토리 경로를 가져옵니다.
    let mut args = env::args();
    let argument = args.next().ok_or(AppError::CommandLine(
        format!("Command line arguments are empty!")
    ))?;
    let path = Path::new(&argument);
    let path = path.parent().ok_or(AppError::CommandLine(
        format!("Application execution directory path not found!")
    ))?.to_path_buf();

    #[cfg(feature = "command-line-args")] {
        while let Some(argument) = args.next() {
            // 해당 명령줄 인수에 대한 명령어 함수를 가져옵니다.
            let func = MAP.get(argument.as_str()).ok_or(AppError::CommandLine(
                format!("Invalid command line argument!")
            ))?;

            // 명령어 함수를 실행합니다.
            builder = func(&mut args, builder)?;
        }
    }

    Ok((builder, path))
}
