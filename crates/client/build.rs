use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
};

use fs_extra::file::CopyOptions;
use serde::Deserialize;

/// 에셋 목록 파일의 임베딩 데이터입니다.
const ASSET_LIST: &'static str = include_str!("assets.json");

const TARGET_PATH: &'static str = "target";
const ASSET_PATH: &'static str = "assets";

/// 에셋 디렉토리 계층 구조를 나타내는 구조체입니다.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
struct Hierarchy {
    files: Vec<String>,
    directories: HashMap<String, Hierarchy>,
}

fn main() {
    println!("cargo:rerun-if-changed=./assets.json");

    // 에셋 리스트 json 파일을 구문 분석합니다.
    let result = serde_json::de::from_str::<Hierarchy>(&ASSET_LIST);
    let root = match result {
        Ok(root) => root,
        Err(e) => panic!(
            "Failed to parse asset list file for the following reason: {}",
            e
        ),
    };

    // 에셋 디렉토리 경로를 생성합니다.
    let workspace_dir = Path::new(env!("CARGO_WORKSPACE_DIR"));
    let mut src_path = workspace_dir.to_path_buf();
    src_path.push(ASSET_PATH);

    // 에셋 디렉토리 경로가 존재하지 않는 경우 `panic!`을 호출합니다.
    if !src_path.is_dir() {
        panic!(
            "Asset directory does not exist! (PATH:{})",
            src_path.display()
        );
    }

    // 빌드 대상과 프로파일을 가져옵니다.
    let target = env::var("TARGET").ok();
    let profile = env::var("PROFILE").ok();

    // 빌드 경로를 생성합니다. (빌드 대상 경로가 포함되지 않은)
    let mut build_path = workspace_dir.to_path_buf();
    build_path.push(TARGET_PATH);
    if let Some(profile) = &profile {
        build_path.push(profile);
    }

    // 빌드 경로가 존재할 경우
    if build_path.is_dir() {
        // 대상 경로를 생성합니다.
        let mut dst_path = build_path.clone();
        dst_path.push(ASSET_PATH);

        // 기존 에셋 디렉토리가 있는 경우 삭제 후 다시 생성합니다.
        if let Err(e) = fs_extra::dir::create(&dst_path, true) {
            panic!(
                "Failed to create build target assets directory for the following reason: {}",
                e
            )
        }

        // 에셋 리스트에 있는 파일을 복사합니다.
        return copy_asset(src_path, dst_path, &root, &CopyOptions::default());
    }

    // 빌드 경로를 생성합니다. (빌드 대상 경로가 포함된)
    let mut build_path = workspace_dir.to_path_buf();
    build_path.push(TARGET_PATH);
    if let Some(target) = &target {
        build_path.push(target);
    }
    if let Some(profile) = &profile {
        build_path.push(profile);
    }

    // 빌드 경로가 존재할 경우
    if build_path.is_dir() {
        // 대상 경로를 생성합니다.
        let mut dst_path = build_path.clone();
        dst_path.push(ASSET_PATH);

        // 기존 에셋 디렉토리가 있는 경우 삭제 후 다시 생성합니다.
        if let Err(e) = fs_extra::dir::create(&dst_path, true) {
            panic!(
                "Failed to create build target assets directory for the following reason: {}",
                e
            )
        }

        // 에셋 리스트에 있는 파일을 복사합니다.
        return copy_asset(src_path, dst_path, &root, &CopyOptions::default());
    }

    panic!("Build path not found!")
}

/// 에셋 리스트에 있는 에셋을 빌드 대상 디렉토리에 복사합니다.
fn copy_asset(src: PathBuf, dst: PathBuf, hierarchy: &Hierarchy, options: &CopyOptions) {
    // 에셋 파일을 복사합니다.
    for filename in hierarchy.files.iter() {
        let mut from = src.clone();
        from.push(filename);

        let mut to = dst.clone();
        to.push(filename);

        // 원본 에셋 파일이 존재하는지 확인합니다.
        if !from.is_file() {
            panic!("Could not find asset file! (PATH:{})", from.display())
        }

        // 에셋 파일을 복사합니다.
        if let Err(e) = fs_extra::file::copy(from, to, options) {
            panic!("Copying asset files failed for the following reason: {}", e)
        }
    }

    // 하위 디렉토리로 이동합니다.
    for (dir, node) in hierarchy.directories.iter() {
        let mut src = src.clone();
        src.push(dir);

        let mut dst = dst.clone();
        dst.push(dir);

        // 원본 에셋 디렉토리가 존재하는지 확인합니다.
        if !src.is_dir() {
            panic!("Could not find asset directory! (PATH:{})", src.display())
        }

        // 대상 에셋 디렉토리를 생성합니다.
        if let Err(e) = fs_extra::dir::create(&dst, true) {
            panic!(
                "Failed to create asset directory for the following reason: {}",
                e
            )
        }

        // 에셋을 복사합니다.
        copy_asset(src, dst, node, options);
    }
}
