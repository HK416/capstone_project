// AI FSM, Q-learning, 상태 패턴, 컨텍스트, 상태 트레잇, 상태 구조체, 매니저 등 구현

use glam::Vec3A;
use rand::Rng;
use std::collections::HashMap;
use uuid::Uuid;

// Q-learning용 trait (간단화)
pub trait QLearning {
    fn select_action(&mut self, state: &str) -> String;
    fn update(&mut self, state: &str, action: &str, reward: f32, next_state: &str);
}

#[derive(Clone, Debug)]
pub struct SimpleQLearning {
    q_table: HashMap<(String, String), f32>,
    actions: Vec<String>,
    epsilon: f32,
}

impl SimpleQLearning {
    pub fn new(actions: Vec<String>) -> Self {
        Self { q_table: HashMap::new(), actions, epsilon: 0.1 }
    }
}

impl QLearning for SimpleQLearning {
    fn select_action(&mut self, state: &str) -> String {
        // Epsilon-greedy
        if rand::random::<f32>() < self.epsilon {
            let mut rng = rand::rng();
            let idx = rng.random_range(0..self.actions.len());
            self.actions[idx].clone()
        } else {
            let mut best = None;
            let mut best_val = f32::MIN;
            for action in &self.actions {
                let val = *self.q_table.get(&(state.to_string(), action.clone())).unwrap_or(&0.0);
                if val > best_val {
                    best = Some(action.clone());
                    best_val = val;
                }
            }
            best.unwrap_or_else(|| self.actions[0].clone())
        }
    }
    fn update(&mut self, state: &str, action: &str, reward: f32, next_state: &str) {
        let key = (state.to_string(), action.to_string());
        let next_max = self.actions.iter().map(|a| *self.q_table.get(&(next_state.to_string(), a.clone())).unwrap_or(&0.0)).fold(0.0, f32::max);
        let entry = self.q_table.entry(key).or_insert(0.0);
        *entry += 0.1 * (reward + 0.9 * next_max - *entry);
    }
}

// AI FSM State/Context/Event
#[derive(Clone, Debug)]
pub struct AIPlayerContext {
    pub user_id: Uuid,
    pub position: Vec3A,
    pub target: Vec3A,
    pub q: SimpleQLearning,
}

pub trait AIPlayerState {
    fn on_enter(&mut self, ctx: &mut AIPlayerContext);
    fn on_update(&mut self, ctx: &mut AIPlayerContext) -> Option<AIEvent>;
    fn on_exit(&mut self, ctx: &mut AIPlayerContext);
}

pub enum AIEvent {
    MoveTo(Vec3A),
    Attack,
    Idle,
}

pub struct IdleState;
impl AIPlayerState for IdleState {
    fn on_enter(&mut self, _ctx: &mut AIPlayerContext) {}
    fn on_update(&mut self, ctx: &mut AIPlayerContext) -> Option<AIEvent> {
        Some(AIEvent::MoveTo(ctx.target))
    }
    fn on_exit(&mut self, _ctx: &mut AIPlayerContext) {}
}

pub struct MoveState;
impl AIPlayerState for MoveState {
    fn on_enter(&mut self, _ctx: &mut AIPlayerContext) {}
    fn on_update(&mut self, ctx: &mut AIPlayerContext) -> Option<AIEvent> {
        if (ctx.position - ctx.target).length() < 1.0 {
            Some(AIEvent::Idle)
        } else {
            None
        }
    }
    fn on_exit(&mut self, _ctx: &mut AIPlayerContext) {}
}

pub struct AttackState;
impl AIPlayerState for AttackState {
    fn on_enter(&mut self, _ctx: &mut AIPlayerContext) {
        // 공격 시작 시 필요한 초기화 (예: 쿨타임, 애니메이션 등)
    }
    fn on_update(&mut self, ctx: &mut AIPlayerContext) -> Option<AIEvent> {
        // 공격 로직: 예시로 바로 Idle로 전환
        // 실제로는 타격 판정, 쿨타임, 타겟 확인 등 구현 가능
        Some(AIEvent::Idle)
    }
    fn on_exit(&mut self, _ctx: &mut AIPlayerContext) {
        // 공격 종료 시 처리
    }
}

// FSM Manager
pub struct AIPlayerFSM {
    pub ctx: AIPlayerContext,
    pub state: Box<dyn AIPlayerState>,
}

impl AIPlayerFSM {
    pub fn new(user_id: Uuid, position: Vec3A, target: Vec3A) -> Self {
        let q = SimpleQLearning::new(vec!["MoveTo".into(), "Attack".into(), "Idle".into()]);
        Self {
            ctx: AIPlayerContext { user_id, position, target, q },
            state: Box::new(IdleState),
        }
    }
    pub fn update(&mut self) {
        if let Some(event) = self.state.on_update(&mut self.ctx) {
            match event {
                AIEvent::MoveTo(target) => {
                    self.ctx.target = target;
                    self.state = Box::new(MoveState);
                }
                AIEvent::Idle => {
                    self.state = Box::new(IdleState);
                }
                AIEvent::Attack => {
                    self.state = Box::new(AttackState);
                }
            }
        }
    }
}
