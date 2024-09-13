# Capstone Project (Server)

## 컴파일 및 실행 방법

cmd 또는 터미널에서 아래의 명령어를 입력하세요.

<b>컴파일</b>
````shell
cargo build --bin server --release
````

<b>실행</b>
```shell
cargo run --bin server --release
```
또는 
```shell
cargo run --bin server --release -- ip:port
```

**스트레스 테스트(더미 클라이언트)**
```shell
cargo run --example {더미클라이언트} --release
```

|더미클라이언트 목록|설명|
|:---:|:---:|
|client100|클라이언트 100개 접속, 각각 1초마다 랜덤 이동|
|ar100|클라이언트 100개가 1초마다 접속/종료 반복|

<b>주의</b>
- `rust`의 패키지 관리자인 `cargo`가 필요합니다.
- `cargo`가 환경변수에 등록되어 있어야 합니다.
- 항상 `release`로 컴파일하고, 디버깅이 필요한 상황에서 `debug`모드를 사용하세요.
