# TUKorea Capstone Project

[한국어](#개요)

# 개요
2024년 [한국공학대학교](https://www.tukorea.ac.kr) 게임공학과 졸업 작품 저장소입니다. </br>
[Rust](https://www.rust-lang.org)로 구현한 게임 프레임워크를 사용하여 게임 클라이언트와 게임 서버를 개발합니다. </br>

# 팀 구성
- 2017180030 이종민 (HK416 <<powerspirit127@gmail.com>>)
- 2020180028 이주형 (Jirung-E <<jh10590326@gmail.com>>)
- 2020182035 장하영 (HA0-TUK <<hayoungtuk@gmail.com>>)

# 전체 프로젝트 컴파일 방법

cmd 또는 터미널에서 아래의 명령어를 입력하세요.

<b>컴파일</b>
````shell
cargo build --release
````

<b>주의</b>
- `rust`의 패키지 관리자인 `cargo`가 필요합니다.
- `cargo`가 환경변수에 등록되어 있어야 합니다.
- 항상 `release`로 컴파일하고, 디버깅이 필요한 상황에서 `debug`모드를 사용하세요.
- <b>서버와 클라이언트를 개별로 컴파일 또는 실행하고 싶은 경우 각각의 `README.md` 파일을 참고하세요.</b>

# 게임 기획 (초안)
[Nexon Games](https://www.nexongames.co.kr)가 개발한 수집형 모바일 게임 [Blue Archive](https://bluearchive.nexon.com)의 2차 창작 게임을 개발합니다. "Blue Archive"의 2차 창작 가이드라인을 준수하며, 단순히 졸업을 위한 게임이 아닌 졸업 작품을 게임 커뮤니티에 개시하여 게임 운용 경험을 체험하는 것을 목표로 개발합니다. 샌드박스 오픈월드 롤플레잉 게임인 [Roblox](https://www.roblox.com)에 있는 "Blue Archive"의 2차 창작 게임 "Kivotos Battlegrounds"와 유사한 방식의 게임을 개발합니다.

<b>게임 타이틀</b> - Hello to Halo! </br>
<b>게임 장르</b> - 팀플레이 TPS PVP </br>
<b>플레이 인원</b> - 10인 (Primary Plan) 또는 1인 (Secondary Plan)

<b>대상 플랫폼 및 하드웨어 사양</b></br>
||Windows|macOS|Web|
|:-:|:-:|:-:|:-:|
|우선 순위|Primary|Primary|Secondary|
|버전|NT 10.0 이상|Big Sur 이상|-|
|CPU|SSE2 명령어 지원 x86_64 아키텍처</br>Neon 명령어 지원 Arm 아키텍처|Neon 명령어 지원 Apple Silicon|Web Assembly SIMD 지원 브라우저 및 하드웨어|
|GPU|Direct X 12 API 지원 하드웨어</br>BC 텍스처 압축 포맷 지원 하드웨어|Metal API 지원 하드웨어</br>BC 텍스처 압축 포맷 지원 하드웨어|WebGPU 지원 브라우저 및 하드웨어
