use std::{io, path::PathBuf};



/// 에셋 관리자 또는 캐싱된 에셋에서 발생할 수 있는 오류 목록입니다.
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    /// 주어진 경로를 찾을 수 없는 경우 이 오류를 발생시킵니다.
    #[error("The given path could not be found (PATH:{0})")]
    PathNotFound(PathBuf), 

    /// 파일을 읽거나 쓰는 도중 오류가 발생한 경우 이 오류를 발생시킵니다.
    #[error("File access failed for the following reason: {0}")]
    IOError(#[from] io::Error), 
}
