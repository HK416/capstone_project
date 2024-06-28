//! 명령줄 인수를 구문 분석하는 코드를 작성합니다.
//! 

use crate::app::AppBuilder;

use std::fmt;
use hashbrown::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    static ref MAP: HashMap<Command<'static>, Attribute> = {
        let mut map = HashMap::new();

        map.insert(
            Command("--num-threads"),
            Attribute { 
                ty: OptionTypes::NumThreads,
                desc: "--num-threads <VAL>      :: Specifies the number of threads to be used by the application. (0 = native)"
            }
        );

        map.insert(
            Command("--show-frame-rate"), 
            Attribute { 
                ty: OptionTypes::ShowFrameRate, 
                desc: "--show-frame-rate        :: Displays the frame rate of the application."
            }
        );

        map.insert(
            Command("--enable-debug-layer"), 
            Attribute { 
                ty: OptionTypes::EnableDebugLayer, 
                desc: "--enable-debug-layer     :: Enables the debugging layer of the application renderer."
            }
        );

        map.insert(
            Command("--fullscreen"), 
            Attribute {
                ty: OptionTypes::Fullscreen, 
                desc: "--fullscreen             :: Run the application in full screen."
            }
        );

        map.insert(
            Command("--resizable"), 
            Attribute {
                ty: OptionTypes::Resizable, 
                desc: "--resizable              :: Allows the application window to be resized."
            }
        );

        // * 옵션 추가 시 주의 사항
        // 1. OptionTypes 수정
        // 2. AppOptions 수정
        // 3. `desc`에 적힌 명령어가 일치하는지 확인
        //

        map
    };
}


/// 명령줄 인수의 명령어 입니다.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Command<'a>(&'a str);

impl<'a> Into<Command<'a>> for &'a str {
    fn into(self) -> Command<'a> {
        Command(self)
    }
}

impl<'a> fmt::Debug for Command<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(Command<'a>))
            .field(&self.0)
            .finish()
    }
}

impl<'a> fmt::Display for Command<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

/// 명령어의 매플리케이션 설정 유형입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum OptionTypes {
    NumThreads,
    ShowFrameRate,
    EnableDebugLayer,
    Fullscreen,
    Resizable,
}

/// 명령어의 속성 입니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Attribute {
    ty: OptionTypes, 
    desc: &'static str,
}



/// 주어진 명령줄 인수를 구문 분석하여 애플리케이션 빌더를 생성합니다.
/// 
/// # Panics
/// 주어진 명령줄 인수가 잘못되었거나 애플리케이션 빌더를 생성하지 못한 경우 [`panic!`]을 호출합니다.
/// 
pub fn parse_command_line_args() -> AppBuilder {
    use std::path::Path;
    use std::num::NonZeroUsize;

    // 현재 애플리케이션 실행 디렉토리 경로를 가져옵니다.
    let mut args = std::env::args();
    let arg = args.next().expect("Command line arguments are empty!");
    let path = Path::new(&arg);
    let current_dir = path.parent().expect("Application execution directory path not found!");

    // 애플리케이션 빌더를 생성합니다.
    #[allow(unused_mut)]
    let mut builder = AppBuilder::new(current_dir);

    // 명령줄 인수를 구문 분석 합니다.
    if cfg!(feature = "command-line-args") {
        while let Some(arg) = args.next() {
            if let Some(attr) = MAP.get(&Command(&arg)) {
                match attr.ty {
                    OptionTypes::NumThreads => {
                        if let Some(arg) = args.next() {
                            let num_threads: usize = arg.parse().unwrap();
                            if num_threads == 0 {
                                builder.options.num_threads = NonZeroUsize::new(num_cpus::get_physical()).unwrap();
                            } else {
                                builder.options.num_threads = NonZeroUsize::new(num_threads).unwrap();
                            }
                        } else {
                            panic!("{}", print_usage());
                        }
                    },
                    OptionTypes::ShowFrameRate => builder.options.show_frame_rate = true,
                    OptionTypes::EnableDebugLayer => builder.options.enable_debug_layer = true,
                    OptionTypes::Resizable => builder.options.resizable = true,
                    OptionTypes::Fullscreen => builder.options.fullscreen = true,
                }
            } else {
                panic!("{}", print_usage());
            }
        }
    }

    return builder;
}

/// 명령줄 인수의 사용 방법을 문자열로 반환합니다.
/// 
/// # Panics
/// 명령줄 인수의 사용 방법 문자열을 생성하지 못할 경우 [`panic`]을 호출합니다.
/// 
fn print_usage() -> &'static str {
    use std::io::{self, Write};
    use std::sync::OnceLock;

    static USAGE: OnceLock<String> = OnceLock::new();
    USAGE.get_or_init(|| {
        let mut stream: io::BufWriter<Vec<u8>> = io::BufWriter::new(Vec::new());
        write!(stream, "Invalid Command-Line Arguments...\n\n").unwrap();
        write!(stream, "Options:\n").unwrap();
        
        for (_, attr) in MAP.iter() {
        write!(stream, "\t{}\n", attr.desc).unwrap();
        }

        let bytes = stream.into_inner().expect("Buffer flush failed.");
        String::from_utf8(bytes).expect("utf-8 encoding failed.")
    })
}
