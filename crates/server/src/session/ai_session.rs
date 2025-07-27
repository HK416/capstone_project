
use mod_network::components::UserId;

/// Trait for session-like objects (real or AI)
pub trait SessionLike: Send + Sync + std::fmt::Debug {
    fn ping(&self) -> u32;
    fn tcp_write(&self, data: &[u8]);
    fn close(&self);
    fn user_id(&self) -> UserId;
}

/// A session type for AI players. Implements the same interface as Session but does nothing for network operations.

#[derive(Clone)]
pub struct AISession {
    uid: UserId,
}


impl AISession {
    pub fn new(uid: UserId) -> Self {
        Self { uid }
    }
}

impl SessionLike for AISession {
    fn ping(&self) -> u32 { 0 }
    fn tcp_write(&self, _data: &[u8]) { /* No-op for AI */ }
    fn close(&self) { /* No-op for AI */ }
    fn user_id(&self) -> UserId { self.uid }
}

impl std::fmt::Debug for AISession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AISession({:?})", self.uid)
    }
}
