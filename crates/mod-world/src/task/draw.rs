use std::{error::Error, fmt, sync::Arc};

use crate::{
    render::brush::MeshBrush, 
    objects::{GameWorld, ObjectId}
};



/// `on_pre_draw` 콜백 함수
pub type OnPreDraw = dyn Fn(&wgpu::Device, &wgpu::Queue, &GameWorld, ObjectId) -> Result<(), Box<dyn Error + Send>>;

/// `on_post_draw` 콜백 함수
pub type OnPostDraw = dyn Fn(&wgpu::Device, &wgpu::Queue, &GameWorld, ObjectId) -> Result<(), Box<dyn Error + Send>>;

pub struct DrawTask {
    /// 대상 게임 오브젝트 식별자입니다.
    id: ObjectId, 

    /// 메쉬 그리기 브러쉬 집합입니다.
    brushes: Vec<Arc<dyn MeshBrush>>, 

    /// `on_pre_draw` 콜백 함수입니다.
    on_pre_draw_callback: Box<OnPreDraw>, 

    /// `on_post_draw` 콜백 함수입니다.
    on_post_draw_callback: Box<OnPostDraw>, 
}

impl DrawTask {
    /// 기본 콜백 함수입니다.
    #[inline]
    fn default_callback(
        _: &wgpu::Device, 
        _: &wgpu::Queue, 
        _: &GameWorld, 
        _: ObjectId
    ) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }
}

impl DrawTask {
    /// 새로운 그리기 작업을 생성합니다.
    /// 
    /// # Panics
    /// 주어진 게임 오브젝트 식별자가 `nil`인 경우 [`panic!`]을 호출합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn new(id: ObjectId) -> Self {
        assert!(!id.is_nil(), "The given game object identifier is nil!");
        unsafe { Self::new_unchecked(id) }
    }

    /// 새로운 그리기 작업을 생성합니다.
    #[inline]
    #[must_use]
    pub unsafe fn new_unchecked(id: ObjectId) -> Self {
        Self { 
            id, 
            brushes: Vec::with_capacity(8), 
            on_pre_draw_callback: Box::new(Self::default_callback), 
            on_post_draw_callback: Box::new(Self::default_callback) 
        }
    }

    /// `on_pre_draw` 콜백 함수를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_on_pre_draw(mut self, func: Option<Box<OnPreDraw>>) -> Self {
        self.on_pre_draw_callback = func.unwrap_or(Box::new(Self::default_callback));
        self
    }

    /// `on_post_draw` 콜백 함수를 설정합니다.
    #[inline]
    #[must_use]
    pub fn with_on_post_draw(mut self, func: Option<Box<OnPostDraw>>) -> Self {
        self.on_post_draw_callback = func.unwrap_or(Box::new(Self::default_callback));
        self
    }

    /// 메쉬 그리기 브러쉬들을 추가합니다.
    #[inline]
    #[must_use]
    pub fn append_brushes(mut self, mut brushes: Vec<Arc<dyn MeshBrush>>) -> Self {
        self.brushes.append(&mut brushes);
        self
    }

    /// 메쉬 그리기 브러쉬를 추가합니다.
    #[inline]
    #[must_use]
    pub fn add_brush(mut self, brush: Arc<dyn MeshBrush>) -> Self {
        self.brushes.push(brush);
        self
    }
}

impl DrawTask {
    /// 대상 게임 오브젝트 식별자를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn id(&self) -> ObjectId {
        self.id
    }

    /// 콜백 함수를 호출합니다.
    #[inline]
    pub fn on_pre_draw(
        &self, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        world: &GameWorld
    ) -> Result<(), Box<dyn Error + Send>> {
        (*self.on_pre_draw_callback)(device, queue, world, self.id)
    }

    /// 콜백 함수를 호출합니다.
    #[inline]
    pub fn on_post_draw(
        &self, 
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        world: &GameWorld
    ) -> Result<(), Box<dyn Error + Send>> {
        (*self.on_post_draw_callback)(device, queue, world, self.id)
    }

    /// 메쉬 그리기 브러쉬들을 가져옵니다.
    #[inline]
    #[must_use]
    pub fn brushes(&self) -> &[Arc<dyn MeshBrush>] {
        &self.brushes
    }
}

impl fmt::Debug for DrawTask {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple(stringify!(DrawCallback))
            .field(&self.id)
            .finish()
    }
}

unsafe impl Send for DrawTask { }
