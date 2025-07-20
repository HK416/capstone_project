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


#[derive(Debug, Clone)]
pub enum AIStateEnum {
    Idle,
    Move,
    Attack,
}

#[derive(Debug, Clone)]
pub struct AIPlayerFSM {
    pub ctx: AIPlayerContext,
    pub state: AIStateEnum,
}

impl AIPlayerFSM {
    pub fn new(user_id: Uuid, position: Vec3A, target: Vec3A) -> Self {
        let q = SimpleQLearning::new(vec!["MoveTo".into(), "Attack".into(), "Idle".into()]);
        Self {
            ctx: AIPlayerContext { user_id, position, target, q },
            state: AIStateEnum::Idle,
        }
    }
    pub fn update(&mut self) {
        match self.state {
            AIStateEnum::Idle => {
                // Idle 상태: 타겟 위치로 이동 명령
                self.state = AIStateEnum::Move;
            }
            AIStateEnum::Move => {
                let dir = (self.ctx.target - self.ctx.position).normalize_or_zero();
                let step_size = 0.5; // 이동 속도
                self.ctx.position += dir * step_size;
                if (self.ctx.position - self.ctx.target).length() < 1.0 {
                    self.state = AIStateEnum::Idle;
                } else {
                    // 실제 이동 로직은 외부에서 처리
                }
            }
            AIStateEnum::Attack => {
                // Attack 상태: 예시로 바로 Idle로 전환
                self.state = AIStateEnum::Idle;
            }
        }
    }
}
