use std::error::Error;



/// 디버깅 정보를 포하하고 있는 오류 메시지입니다.
#[cfg(feature = "enable-debug-info")]
#[derive(Debug, thiserror::Error)]
#[error("FILE:{file}, LINE:{line}, COLUMN:{column} :: {error}")]
pub struct RuntimeError {
    pub file: &'static str, 
    pub line: u32, 
    pub column: u32, 
    pub error: Box<dyn Error + Send>
}

/// 디버깅 정보를 포함하고 있는 오류 메시지를 생성합니다.
#[macro_export]
#[cfg(feature = "enable-debug-info")]
macro_rules! err_msg {
    ($err:expr) => {
        Box::new(RuntimeError {
            file: file!(), 
            line: line!(), 
            column: column!(), 
            error: Box::new($err), 
        }) as Box<dyn Error + Send>
    };
}



/// 디버깅 정보를 포함하지 않은 오류 메시지입니다.
#[cfg(not(feature = "enable-debug-info"))]
#[derive(Debug, thiserror::Error)]
#[error("{error}")]
pub struct RuntimeError {
    pub error: Box<dyn Error + Send>
}


/// 디버깅 정보를 포함하는 오류 메시지를 생성합니다.
#[macro_export]
#[cfg(not(feature = "enable-debug-info"))]
macro_rules! err_msg {
    ($err:expr) => {
        Box::new(RuntimeError {
            error: Box::new($err), 
        }) as Box<dyn Error + Send>
    };
}
