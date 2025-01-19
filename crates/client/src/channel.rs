use std::{error::Error, sync::Arc};

use mod_parallelism::collections::Queue;

/// 작업 결과를 전송하는 채널입니다.
#[derive(Debug)]
pub struct TaskResultChannel<T> {
    inner: Arc<Queue<Result<T, Box<dyn Error + Send>>>>,
}

impl<T> TaskResultChannel<T> {
    /// 새로운 작업 결과 채널을 생성합니다.
    pub fn new() -> Self {
        Self::default()
    }

    /// 작업 결과를 전송합니다.
    pub fn send<E>(&self, result: Result<T, E>)
    where
        E: Error + Send + 'static,
    {
        let result = result.map_err(|e| Box::new(e) as Box<dyn Error + Send>);
        self.inner.push(result);
    }

    /// 작업 결과를 전송합니다.
    pub fn send_ok(&self, success: T) {
        self.inner.push(Ok(success));
    }

    /// 작업 결과를 전송합니다.
    pub fn send_err(&self, error: Box<dyn Error + Send>) {
        self.inner.push(Err(error));
    }

    /// 작업 결과를 수신합니다.
    pub fn recv(&self) -> Option<Result<T, Box<dyn Error + Send>>> {
        self.inner.pop()
    }
}

impl<T> Clone for TaskResultChannel<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Default for TaskResultChannel<T> {
    fn default() -> Self {
        Self {
            inner: Arc::new(Queue::default()),
        }
    }
}
