use super::PetState;

/// 状态转移结果
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: PetState,
    pub to: PetState,
    pub legal: bool,
}

/// 显式宠物状态机，定义合法转移矩阵
/// 非法转移只 warn 不 block，保持向前兼容
pub struct StateMachine {
    current: PetState,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            current: PetState::Idle,
        }
    }

    pub fn current(&self) -> &PetState {
        &self.current
    }

    /// 尝试转移状态，返回转移结果
    /// 非法转移也允许（warn 但不阻止），确保不影响现有功能
    pub fn transition(&mut self, next: PetState) -> Transition {
        let from = self.current.clone();
        let legal = is_legal_transition(&from, &next);

        if !legal {
            tracing::warn!(
                from = from.as_str(),
                to = next.as_str(),
                "illegal state transition (allowed but logged)"
            );
        } else {
            tracing::debug!(
                from = from.as_str(),
                to = next.as_str(),
                "state transition"
            );
        }

        self.current = next.clone();
        Transition {
            from,
            to: next,
            legal,
        }
    }
}

/// 合法转移矩阵
fn is_legal_transition(from: &PetState, to: &PetState) -> bool {
    use PetState::*;
    match (from, to) {
        // 正常流程
        (Idle, Thinking) => true,
        (Thinking, Coding) => true,
        (Coding, Success) => true,
        (Success, Idle) => true,
        // thinking 可直接到 success（短回复无工具调用）
        (Thinking, Success) => true,
        // success 期间新消息到达（3 秒回退到 idle 之前用户发了新消息）
        (Success, Thinking) => true,
        // coding 过程中持续 coding（多次工具调用）
        (Coding, Coding) => true,
        // coding 后继续 thinking（工具结果后思考）
        (Coding, Thinking) => true,
        // 任何状态都可能进 error
        (_, Error) => true,
        // error 后回到正常流程
        (Error, Thinking) => true,
        (Error, Idle) => true,
        // 任何状态都可能进 waiting
        (_, Waiting) => true,
        // waiting 后回到之前的状态
        (Waiting, Thinking) => true,
        (Waiting, Coding) => true,
        (Waiting, Idle) => true,
        // idle → sleeping
        (Idle, Sleeping) => true,
        // 任何活动唤醒 sleeping
        (Sleeping, _) => true,
        // 幂等（同状态不变）
        (a, b) if a == b => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use PetState::*;

    #[test]
    fn normal_flow_is_legal() {
        let mut sm = StateMachine::new();
        assert!(sm.transition(Thinking).legal);
        assert!(sm.transition(Coding).legal);
        assert!(sm.transition(Success).legal);
        assert!(sm.transition(Idle).legal);
    }

    #[test]
    fn any_state_can_enter_error() {
        for from in [Idle, Thinking, Coding, Success, Waiting, Sleeping] {
            let mut sm = StateMachine::new();
            sm.current = from;
            assert!(sm.transition(Error).legal, "should allow {:?} -> Error", sm.current);
        }
    }

    #[test]
    fn idempotent_transition_is_legal() {
        let mut sm = StateMachine::new();
        sm.current = Coding;
        assert!(sm.transition(Coding).legal);
    }

    #[test]
    fn idle_to_success_is_illegal() {
        let mut sm = StateMachine::new();
        let t = sm.transition(Success);
        assert!(!t.legal);
        // 状态仍应更新（warn 但不 block）
        assert_eq!(*sm.current(), Success);
    }

    #[test]
    fn sleeping_can_wake_to_any() {
        for to in [Idle, Thinking, Coding, Success, Error, Waiting] {
            let mut sm = StateMachine::new();
            sm.current = Sleeping;
            assert!(sm.transition(to).legal);
        }
    }

    #[test]
    fn thinking_to_success_is_legal() {
        let mut sm = StateMachine::new();
        sm.current = Thinking;
        assert!(sm.transition(Success).legal);
    }

    #[test]
    fn success_to_thinking_is_legal() {
        let mut sm = StateMachine::new();
        sm.current = Success;
        assert!(sm.transition(Thinking).legal);
    }
}
