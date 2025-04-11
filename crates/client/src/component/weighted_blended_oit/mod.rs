//! Weighted Blended Order-Independent Transparency와 관련된 코드를 관리합니다.
//!
//! # 렌더링
//! 렌더링은 다음 순서로 진행됩니다.
//! 1. Opaque Pass
//! 불투명한 객체가 스왑체인에 연결된 렌터 타겟 텍스처에 그려집니다.
//!
//! 2. Transparent Pass
//! 투명한 객체가 누적값(Accumulate) 렌더 타겟 텍스처와 노출값(Revealage) 렌더 타겟 텍스처에 그려집니다.
//!
//! 3. Composite Pass
//! 아래 렌더 타겟 텍스처를 이용하여 스왑체인에 연결된 렌더 타겟 텍스처에 그립니다.
//! - 누적값(Accumulate)
//! - 노출값(Revealage)
//!

mod pipeline;
mod resource;

pub use self::{pipeline::*, resource::*};
