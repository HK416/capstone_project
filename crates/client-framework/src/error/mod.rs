mod debug;
pub use self::debug::*;

mod process;
pub use self::process::*;

use thiserror::Error;

use crate::app::WindowError;
use crate::command::CommandParsingError;
use crate::render::error::RenderError;



/// 에러 메시지 입니다.
#[derive(Debug, Error)]
pub enum ErrorMessage {
    #[cfg(feature = "enable-debug-info")]
    #[error("{0}, ({1})")]
    Window(WindowError, DebugInfo), 

    #[cfg(feature = "enable-debug-info")]
    #[error("{0}, ({1})")]
    CommandParsing(CommandParsingError, DebugInfo), 

    #[cfg(feature = "enable-debug-info")]
    #[error("{0}, ({1})")]
    Render(RenderError, DebugInfo), 

    #[cfg(not(feature = "enable-debug-info"))]
    #[error("{0}")]
    Window(WindowError), 

    #[cfg(not(feature = "enable-debug-info"))]
    #[error("{0}")]
    CommandParsing(CommandParsingError), 

    #[cfg(not(feature = "enable-debug-info"))]
    #[error("{0}")]
    Render(RenderError), 
}

#[cfg(feature = "enable-debug-info")]
impl From<(WindowError, DebugInfo)> for ErrorMessage {
    #[inline]
    fn from(value: (WindowError, DebugInfo)) -> Self {
        Self::Window(value.0, value.1)
    }
}

#[cfg(feature = "enable-debug-info")]
impl From<(CommandParsingError, DebugInfo)> for ErrorMessage {
    #[inline]
    fn from(value: (CommandParsingError, DebugInfo)) -> Self {
        Self::CommandParsing(value.0, value.1)
    }
}

#[cfg(feature = "enable-debug-info")]
impl From<(RenderError, DebugInfo)> for ErrorMessage {
    #[inline]
    fn from(value: (RenderError, DebugInfo)) -> Self {
        Self::Render(value.0, value.1)
    }
}

#[cfg(not(feature = "enable-debug-info"))]
impl From<WindowError> for ErrorMessage {
    #[inline]
    fn from(value: WindowError) -> Self {
        Self::Window(value)
    }
}

#[cfg(not(feature = "enable-debug-info"))]
impl From<CommandParsingError> for ErrorMessage {
    #[inline]
    fn from(value: CommandParsingError) -> Self {
        Self::CommandParsing(value)
    }
}

#[cfg(not(feature = "enable-debug-info"))]
impl From<RenderError> for ErrorMessage {
    #[inline]
    fn from(value: RenderError) -> Self {
        Self::Render(value)
    }
}



/// 에러 메시지를 생성합니다.
#[macro_export]
macro_rules! err_msg {
    ($err:expr) => {{
        #[cfg(feature = "enable-debug-info")] {
            ErrorMessage::from((
                $err, 
                DebugInfo {
                    file: file!(), 
                    line: line!(), 
                    column: column!()
                }
            ))
        }
        #[cfg(not(feature = "enable-debug-info"))] {
            ErrorMessage::from($err)
        }
    }};
}
