# Capstone Project (Framework)
클라이언트와 서버에서 사용되는 모듈을 작성합니다.

## 컴파일 방법

cmd 또는 터미널에서 아래의 명령어를 입력하세요.

<b>컴파일</b>
````shell
cargo build --lib framework --release
````

<b>주의</b>
- `rust`의 패키지 관리자인 `cargo`가 필요합니다.
- `cargo`가 환경변수에 등록되어 있어야 합니다.
- 항상 `release`로 컴파일하고, 디버깅이 필요한 상황에서 `debug`모드를 사용하세요.
- 서버와 클라이언트를 컴파일하거나 실행할 경우에는 자동으로 컴파일 됩니다.
