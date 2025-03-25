use cc;


fn main() {
    println!("cargo:rerun-if-changed=cpp/*"); // 파일이 변경되면 다시 빌드

    cc::Build::new()
        .cpp(true)
        .include("cpp/header/") // 헤더 파일 디렉터리
        .files(std::fs::read_dir("cpp/src/").unwrap().map(|entry| entry.unwrap().path()))   // 소스파일
        .compile("ebr"); // 라이브러리 이름
}