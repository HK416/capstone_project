//! 캐릭터 이동 방향과 관련된 코드를 관리합니다.
//!

use std::f32::{
    consts::{PI, SQRT_2},
    EPSILON,
};

use crate::components::{HeldInput, LatLon};

/// 플레이어 이동 방향 데이터입니다.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovingDirection(pub glam::Vec3A);

impl MovingDirection {
    /// 새로운 플레이어 이동 방향 데이터를 생성합니다.
    pub const fn new() -> Self {
        Self(glam::Vec3A::ZERO)
    }

    /// 이동 방향을 갱신합니다.
    pub fn update(&mut self, held_input: HeldInput, latlon: LatLon) {
        let m = glam::Mat4::from_rotation_y(latlon.lon);
        let right = glam::Vec3A::from_vec4(m.x_axis).normalize_or(glam::Vec3A::X);
        let look = glam::Vec3A::from_vec4(m.z_axis).normalize_or(glam::Vec3A::Z);

        let bits = held_input.bits() & 0b0000_0000_0000_1111;
        match bits {
            0b0000_0000_0000_0001 | 0b0000_0000_0000_1101 => {
                self.update_when_moving_left(right, look)
            }
            0b0000_0000_0000_0010 | 0b0000_0000_0000_1110 => {
                self.update_when_moving_right(right, look)
            }
            0b0000_0000_0000_0100 | 0b0000_0000_0000_0111 => {
                self.update_when_moving_forward(right, look)
            }
            0b0000_0000_0000_1000 | 0b0000_0000_0000_1011 => {
                self.update_when_moving_backward(right, look)
            }
            0b0000_0000_0000_0101 => self.update_when_moving_left_forward(right, look),
            0b0000_0000_0000_0110 => self.update_when_moving_right_forward(right, look),
            0b0000_0000_0000_1001 => self.update_when_moving_left_backward(right, look),
            0b0000_0000_0000_1010 => self.update_when_moving_right_backward(right, look),
            _ => {}
        }
    }

    /// 왼쪽 이동 상태일 때 플레이어 방향을 갱신합니다.
    fn update_when_moving_left(&mut self, right: glam::Vec3A, _look: glam::Vec3A) {
        // 이동 방향 벡터
        let direction = -right;

        // 두 벡터의 각도로 부터 보간 값을 계산합니다.
        let dst = direction;
        let src = self.0;
        let angle = dst.angle_between(src);
        if (angle - PI).abs() <= EPSILON {
            self.0 = direction;
        } else {
            let s = angle / PI * 0.5 + 0.5;
            self.0 = self.0.lerp(direction, s).normalize_or(direction);
        }
    }

    /// 오른쪽 이동 상태일 때 플레이어 방향을 갱신합니다.
    fn update_when_moving_right(&mut self, right: glam::Vec3A, _look: glam::Vec3A) {
        // 이동 방향 벡터
        let direction = right;

        // 두 벡터의 각도로 부터 보간 값을 계산합니다.
        let dst = direction;
        let src = self.0;
        let angle = dst.angle_between(src);
        if (angle - PI).abs() <= EPSILON {
            self.0 = direction;
        } else {
            let s = angle / PI * 0.5 + 0.5;
            self.0 = self.0.lerp(direction, s).normalize_or(direction);
        }
    }

    /// 앞쪽 이동 상태일 때 플레이어 방향을 갱신합니다.
    fn update_when_moving_forward(&mut self, _right: glam::Vec3A, look: glam::Vec3A) {
        // 이동 방향 벡터
        let direction = look;

        // 두 벡터의 각도로 부터 보간 값을 계산합니다.
        let dst = direction;
        let src = self.0;
        let angle = dst.angle_between(src);
        if (angle - PI).abs() <= EPSILON {
            self.0 = direction;
        } else {
            let s = angle / PI * 0.5 + 0.5;
            self.0 = self.0.lerp(direction, s).normalize_or(direction);
        }
    }

    /// 뒷쪽 이동 상태일 때 플레이어 방향을 갱신합니다.
    fn update_when_moving_backward(&mut self, _right: glam::Vec3A, look: glam::Vec3A) {
        // 이동 방향 벡터
        let direction = -look;

        // 두 벡터의 각도로 부터 보간 값을 계산합니다.
        let dst = direction;
        let src = self.0;
        let angle = dst.angle_between(src);
        if (angle - PI).abs() <= EPSILON {
            self.0 = direction;
        } else {
            let s = angle / PI * 0.5 + 0.5;
            self.0 = self.0.lerp(direction, s).normalize_or(direction);
        }
    }

    /// 왼쪽-앞쪽 이동 상태일 때 플레이어 방향을 갱신합니다.
    fn update_when_moving_left_forward(&mut self, right: glam::Vec3A, look: glam::Vec3A) {
        // 이동 방향 벡터
        let direction = -right / SQRT_2 + look / SQRT_2;

        // 두 벡터의 각도로 부터 보간 값을 계산합니다.
        let dst = direction;
        let src = self.0;
        let angle = dst.angle_between(src);
        if (angle - PI).abs() <= EPSILON {
            self.0 = direction;
        } else {
            let s = angle / PI * 0.5 + 0.5;
            self.0 = self.0.lerp(direction, s).normalize_or(direction);
        }
    }

    /// 오른쪽-앞쪽 이동 상태일 때 플레이어 방향을 갱신합니다.
    fn update_when_moving_right_forward(&mut self, right: glam::Vec3A, look: glam::Vec3A) {
        // 이동 방향 벡터
        let direction = right / SQRT_2 + look / SQRT_2;

        // 두 벡터의 각도로 부터 보간 값을 계산합니다.
        let dst = direction;
        let src = self.0;
        let angle = dst.angle_between(src);
        if (angle - PI).abs() <= EPSILON {
            self.0 = direction;
        } else {
            let s = angle / PI * 0.5 + 0.5;
            self.0 = self.0.lerp(direction, s).normalize_or(direction);
        }
    }

    /// 왼쪽-뒷쪽 이동 상태일 때 플레이어 방향을 갱신합니다.
    fn update_when_moving_left_backward(&mut self, right: glam::Vec3A, look: glam::Vec3A) {
        // 이동 방향 벡터
        let direction = -right / SQRT_2 - look / SQRT_2;

        // 두 벡터의 각도로 부터 보간 값을 계산합니다.
        let dst = direction;
        let src = self.0;
        let angle = dst.angle_between(src);
        if (angle - PI).abs() <= EPSILON {
            self.0 = direction;
        } else {
            let s = angle / PI * 0.5 + 0.5;
            self.0 = self.0.lerp(direction, s).normalize_or(direction);
        }
    }

    /// 오른쪽-뒷쪽 이동 상태일 때 플레이어 방향을 갱신합니다.
    fn update_when_moving_right_backward(&mut self, right: glam::Vec3A, look: glam::Vec3A) {
        // 이동 방향 벡터
        let direction = right / SQRT_2 - look / SQRT_2;

        // 두 벡터의 각도로 부터 보간 값을 계산합니다.
        let dst = direction;
        let src = self.0;
        let angle = dst.angle_between(src);
        if (angle - PI).abs() <= EPSILON {
            self.0 = direction;
        } else {
            let s = angle / PI * 0.5 + 0.5;
            self.0 = self.0.lerp(direction, s).normalize_or(direction);
        }
    }
}
