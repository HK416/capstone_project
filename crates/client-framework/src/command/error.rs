use std::num::ParseIntError;
use std::num::ParseFloatError;
use thiserror::Error;



/// 명령줄 인수를 구문 분석할 때 발생할 수 있는 애러의 목록입니다.
#[derive(Debug, Error)]
pub enum CommandParsingError {
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
    RootPathNoFound, 

    /// 주어진 명령줄 인수를 정수 자료형으로 구문 분석하는데 실패할 경우 발생하는 오류입니다.
    #[error("Parsing integer failed for the following reasons: {0}")]
    ParsingIntFailure(#[from] ParseIntError), 

    /// 주어진 명령줄 인수를 실수 자료형으로 구문 분석하는데 실패할 경우 발생하는 오류입니다.
    #[error("Parsing float failed for the following reasons: {0}")]
    ParsingFloatFailure(#[from] ParseFloatError), 
}
