mod bone;
pub use self::bone::*;

mod keyframe;
pub use self::keyframe::*;

mod mesh;
pub use self::mesh::*;

mod mode;
pub use self::mode::*;

use hecs::World;

use crate::render::object::update_hierarchy;
use crate::render::object::Transform;
use crate::render::object::WorldTransform;
use crate::render::skin::BoneMatrixDataLayout;



/// 애니메이션 집합입니다.
#[derive(Debug)]
pub struct Animation {
    /// 애니메이션 이름입니다.
    name: String, 
    
    /// 애니메이션의 재생 길이입니다.
    length: f32, 
    
    /// 애니메이션 플레이 모드입니다.
    mode: PlayMode, 

    /// 애니메이션 시간입니다.
    play_time: f32, 

    /// 애니메이션 키 프레임입니다.
    keyframes: Vec<KeyFrame>, 
}

impl Animation {
    /// 새로운 애니메이션 집합을 생성합니다.
    /// 
    /// # Panics
    /// 1. 주어진 키 프레임이 비어있는 경우 [`panic!`]을 호출합니다.
    /// 2. 주어진 애니메이션 길이가 0보다 작거나 같을 경우 [`panic!`]을 호출합니다.
    /// 
    #[must_use]
    pub fn new<T, I>(
        name: T, 
        length: f32, 
        mode: PlayMode, 
        keyframes: I
    ) -> Self 
    where 
        T: Into<String>, 
        I: IntoIterator<Item = KeyFrame>, 
        I::IntoIter: ExactSizeIterator, 
    {
        // 애니메이션 길이를 확인합니다.
        assert!(0.0 <= length, "The length of the given animation must be greater than zero!");

        // 키 프레임 데이터를 확인합니다.
        let mut keyframes: Vec<_> = keyframes.into_iter().collect();
        assert!(!keyframes.is_empty(), "The given keyframe data is empty!");

        // 키 프레임을 정렬합니다.
        keyframes.sort();

        Self { 
            name: name.into(), 
            length, 
            mode, 
            play_time: 0.0, 
            keyframes, 
        }
    }

    /// 애니메이션 재생 시간을 초기화합니다.
    #[inline]
    pub fn reset(&mut self) {
        self.play_time = 0.0;
    }

    /// 애니메이션을 재생하고 데이터를 갱신합니다.
    pub fn play(&mut self, world: &mut World, queue: &wgpu::Queue, elapsed_time_sec: f32) {
        // 애니메이션 재생 시간을 갱신합니다.
        self.update_play_time(elapsed_time_sec);

        let index = self.keyframes.binary_search_by(|frame| 
            frame.time_point().total_cmp(&self.play_time)
        ).unwrap_or_else(|index| index)
        .min(self.keyframes.len() - 1);

        for (mesh, transforms) in self.get_bone_transforms(index) {
            let iter = transforms.into_iter()
                .map(|transform| transform.as_matrix());

            for (entity, bone_transform) in mesh.skinning().bones().iter().cloned().zip(iter) {
                if let Ok(transform) = world.query_one_mut::<&mut Transform>(entity) {
                    let bone_transform: gmm::Float4x4 = bone_transform.into();
                    *transform = bone_transform.into();
                }
            }

            let root_bone = mesh.skinning().root_bone().clone();
            update_hierarchy(world, None, root_bone);

            let iter = mesh.skinning().bones().iter()
                .map(|&entity| **world.get::<&WorldTransform>(entity).unwrap())
                .map(|matrix| matrix.into());

            mesh.skinning().update(queue, BoneMatrixDataLayout::new(iter));
        }
    }

    /// 애니메이션 재생 시간을 갱신합니다.
    fn update_play_time(&mut self, elapsed_time_sec: f32) {
        self.play_time = match self.mode {
            PlayMode::Once => (self.play_time + elapsed_time_sec).min(self.length), 
            PlayMode::Loop => (self.play_time + elapsed_time_sec) % self.length, 
            PlayMode::Pingpong { is_reverse } => 
            if !is_reverse {
                let time = self.play_time + elapsed_time_sec;
                if self.length <= time {
                    self.mode = PlayMode::Pingpong { is_reverse: !is_reverse };
                    self.length - (time - self.length)
                } else {
                    time
                }
            } else { 
                let time = self.play_time - elapsed_time_sec;
                if time <= 0.0 {
                    self.mode = PlayMode::Pingpong { is_reverse: !is_reverse };
                    time.abs()
                } else {
                    time
                }
            }
        }
    }

    /// 각 뼈의 변환 데이터를 반환합니다.
    fn get_bone_transforms<'a>(&'a self, index: usize) -> Vec<(&'a SkinnedMesh, Vec<BoneTransform>)> {
        if index == 0 {
            self.keyframes[index].meshes().iter()
                .map(|mesh| (mesh, mesh.bone_transforms().cloned().collect()))
                .collect()
        } else {
            let prev = &self.keyframes[index - 1];
            let next = &self.keyframes[index];
            let duration = next.time_point() - prev.time_point();
            let elapsed_time = self.play_time - prev.time_point();
            let t = elapsed_time / duration;
            prev.meshes().iter().zip(next.meshes().iter())
                .map(|(prev, next)| {
                    let transforms: Vec<_> = prev.bone_transforms().zip(next.bone_transforms())
                        .map(|(prev, next)| {
                            prev.linear_interpolation(&next, t)
                        })
                        .collect();

                    (next, transforms)
                })
                .collect()
        }
    }
}

impl Animation {
    /// 애니메이션의 이름을 반환합니다.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 애니메이션 길이를 반환합니다.
    #[inline]
    #[must_use]
    pub fn length(&self) -> f32 {
        self.length
    }

    /// 애니메이션 실행 시간을 반환합니다.
    #[inline]
    #[must_use]
    pub fn play_time(&self) -> f32 {
        self.play_time
    }

    /// 애니메이션 재생 모드를 반환합니다.
    #[inline]
    #[must_use]
    pub fn mode(&self) -> &PlayMode {
        &self.mode
    }
}
