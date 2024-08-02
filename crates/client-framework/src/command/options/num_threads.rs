use std::env::Args;
use std::num::NonZeroUsize;
use framework::concurrency::MAX_CORE_NUM;

use crate::app::AppBuilder;
use crate::command::CommandParsingError;
use crate::error::err_msg;
use crate::error::ErrorMessage;



/// 사용할 스레드 갯수를 설정하는 명령어 함수 입니다.
#[inline]
pub fn num_threads(args: &mut Args, builder: AppBuilder) -> Result<AppBuilder, ErrorMessage> {
    // 스레드의 갯수를 구문 분석 합니다.
    let argument = args.next().ok_or(err_msg!(CommandParsingError::NotEnough))?;
    let num_threads = argument.parse::<usize>()
        .map_err(|e| err_msg!(CommandParsingError::from(e)))?;

    // 스레드의 갯수를 설정합니다.
    // safety: `num_threads`는 항상 1보다 큼.
    //
    unsafe {
        Ok(builder.set_num_threads(
            NonZeroUsize::new_unchecked(match num_threads == 0 {
                true => *MAX_CORE_NUM,
                false => num_threads
            })
        ))
    }
}
