#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod asset;
mod channel;
mod component;
mod config;
mod render;
mod scenes;

use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use constcat::concat;
use mod_app::net::IpAddress;

pub const SERVER_IP: IpAddr = IpAddr::V6(Ipv6Addr::LOCALHOST);
pub const SERVER_ADDR: IpAddress = IpAddress::Tcp(SocketAddr::new(SERVER_IP, 7878));

pub const USER_CONFIG: &'static str = "user_config";

pub const FONT_WORKSPACE: &'static str = "font/";
pub const FONT_STYLE_0: &'static str = concat!(FONT_WORKSPACE, "NEXON_Lv2_Gothic.ttf");
pub const FONT_STYLE_0_BOLD: &'static str = concat!(FONT_WORKSPACE, "NEXON_Lv2_Gothic_Bold.ttf");

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

    // 로그 시스템을 초기화 합니다.
    env_logger::builder()
        .filter_module("wgpu_core", log::LevelFilter::Warn)
        .filter_module("wgpu_hal", log::LevelFilter::Warn)
        .filter_module("naga", log::LevelFilter::Warn)
        .init();
    log::info!("클라이언트 애플리케이션 실행...");

    AppBuilder::new(Box::new(scenes::StartupScene::new()))
        .with_window_title("Hello to Halo!")
        .with_fullscreen(false)
        .with_visible(false)
        .build_and_run()
}
