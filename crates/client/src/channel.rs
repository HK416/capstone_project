use std::{error::Error, sync::Arc};

use mod_parallelism::collections::Queue;

/// 작업 결과를 전송하는 채널입니다.
#[derive(Debug, Clone)]
pub struct TaskResultChannel {
    inner: Arc<Queue<Result<(), Box<dyn Error + Send>>>>,
}

impl TaskResultChannel {
    /// 새로운 작업 결과 채널을 생성합니다.
    pub fn new() -> Self {
        Self::default()
    }

    /// 작업 결과를 전송합니다.
    pub fn send<T, E>(&self, result: Result<T, E>)
    where
        E: Error + Send + 'static,
    {
        let result = result
            .map(|_| ())
            .map_err(|e| Box::new(e) as Box<dyn Error + Send>);
        self.inner.push(result);
    }

    /// 작업 결과를 수신합니다.
    pub fn recv(&self) -> Option<Result<(), Box<dyn Error + Send>>> {
        self.inner.pop()
    }
}

impl Default for TaskResultChannel {
    fn default() -> Self {
        Self {
            inner: Arc::new(Queue::default()),
        }
    }
}
