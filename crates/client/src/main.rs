#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod asset;
mod component;
mod config;
mod render;
mod scenes;

use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use mod_app::net::IpAddress;
use tracing::Level;
use tracing_appender::{non_blocking::WorkerGuard, rolling};

pub const SERVER_IP: IpAddr = IpAddr::V6(Ipv6Addr::LOCALHOST);
pub const SERVER_TCP_ADDR: IpAddress = IpAddress::Tcp(SocketAddr::new(SERVER_IP, 7878));

pub const CLIENT_IP: IpAddr = IpAddr::V6(Ipv6Addr::LOCALHOST);
pub const UDP_SOCKET_ADDR: IpAddress = IpAddress::Udp {
    port: 19261,
    remote: SocketAddr::new(SERVER_IP, 7878),
};

/// 64bit `Windows`, `macOS` 플랫폼의
/// 애플리케이션 진입점입니다.
///
/// 게임 화면은 16 : 9 비율의 scaled 크기를 가집니다.
///
/// `Windows`, `macOS` 플랫폼의 경우 최초 실행시 전체 화면으로 실행됩니다.
///
#[cfg(target_pointer_width = "64")]
#[cfg(any(target_os = "windows", target_os = "macos"))]
fn main() {
    use mod_app::app::AppBuilder;
    use scenes::GameStartupScene;

    // 로그 시스템을 초기화 합니다.
    // env_logger::builder()
    //     .filter_module("wgpu_core", log::LevelFilter::Warn)
    //     .filter_module("wgpu_hal", log::LevelFilter::Warn)
    //     .filter_module("naga", log::LevelFilter::Warn)
    //     .init();
    let _guard = init_log_system();
    log::info!("클라이언트 애플리케이션 실행...");

    AppBuilder::new(Box::new(GameStartupScene::new()))
        .with_window_title("Hello to Halo!")
        .with_fullscreen(false)
        .with_visible(false)
        .build_and_run()
}

/// 로그 시스템을 초기화 합니다.
///
/// # Note
/// 반환되는 `WorkerGuard`를 유지해야 로그가 정상적으로 저장됩니다.
///
fn init_log_system() -> WorkerGuard {
    // 로그 시스템을 생성합니다.
    let formatted = chrono::Local::now().format("%Y_%m_%d_%H_%M_%S").to_string();
    let file_name = format!("log-{}", formatted);
    let file_appender = rolling::never(get_current_path(), file_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(non_blocking)
        .init();

    guard
}

/// 프로그램의 디렉토리 경로를 가져옵니다.  
/// 이 함수는 명령줄 인수를 통해 프로그램의 현재 경로를 가져옵니다.
pub fn get_current_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let mut args = std::env::args();
        let argument = match args.next() {
            Some(arg) => arg,
            None => {
                log::error!("command line arguments are empty!");
                panic!("command line arguments are empty!");
            }
        };

        let current_exe = PathBuf::from(argument);
        let current_path = match current_exe.parent() {
            Some(path) => path,
            None => {
                log::error!("the path to the executable file could not be found!");
                panic!("the path to the executable file could not be found!")
            }
        }
        .to_path_buf();

        current_path
    })
}
