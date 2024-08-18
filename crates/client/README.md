# Capstone Project (Client)

## 컴파일 및 실행 방법

cmd 또는 터미널에서 아래의 명령어를 입력하세요.

<b>컴파일</b>
````shell
cargo build --bin client --release
````

<b>실행</b>
````shell
cargo run --bin client --release
````

<b>주의</b>
- `rust`의 패키지 관리자인 `cargo`가 필요합니다.
- `cargo`가 환경변수에 등록되어 있어야 합니다.
- 항상 `release`로 컴파일하고, 디버깅이 필요한 상황에서 `debug`모드를 사용하세요.



## 실행 옵션

명령줄 인수를 전달하여 애플리케이션 디버깅에 용이한 옵션을 활성화 할 수 있습니다. 

cmd 또는 터미널에서 아래와 같이 명령어를 입력하세요.

<b>실행</b>
````shell
cargo run --bin client --release -- <OPTIONS>
````

### 옵션 목록

> <b>주의</b>: 현재 애플리케이션의 옵션과 다를 수 있습니다.

|명령어|기능|
|:---|:---|
| --num-threads | (예정) 사용 가능한 최대 스레드의 갯수를 지정합니다. |
| --show-frame-rate | (예정) 현재 프레임 레이트를 출력합니다. |
| --no-vsync | 수직 동기화를 비활성화 합니다. |
| --enable-debug-layer | 렌더러의 디버깅 레이어를 활성화 합니다. |

</br>

## 예제 애플리케이션

클라이언트 애플리케이션을 완성하기 전에 테스트 용도로 사용되는 예제 애플리케이션이 포함되어 있습니다.

자세한 내용은 examples 디렉토리의 `README.md`파일을 확인해 주세요.
