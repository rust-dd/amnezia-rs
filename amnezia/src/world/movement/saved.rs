use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct MotionState {
    steps: VecDeque<RouteAction>,
    active: Option<StepState>,
    step_secs: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct StepState {
    from: [f32; 2],
    to: [f32; 2],
    elapsed: f32,
    jumping: bool,
}

impl MoveQueue {
    pub(crate) fn snapshot(&self) -> MotionState {
        MotionState {
            steps: self.steps.clone(),
            active: self.active.as_ref().map(|step| StepState {
                from: step.from.to_array(),
                to: step.to.to_array(),
                elapsed: step.elapsed,
                jumping: step.jumping,
            }),
            step_secs: self.step_secs,
        }
    }
}

impl MotionState {
    pub(crate) fn valid(&self) -> bool {
        self.valid_with_facings(4)
    }

    pub(crate) fn valid_for_event(&self) -> bool {
        self.valid_with_facings(8)
    }

    fn valid_with_facings(&self, facings: u32) -> bool {
        self.step_secs.is_finite()
            && self.step_secs > 0.0
            && self.steps.iter().all(|step| {
                let (RouteAction::Step { dx, dy, face } | RouteAction::Jump { dx, dy, face }) =
                    step;
                *face < facings && dx.checked_abs().is_some() && dy.checked_abs().is_some()
            })
            && self.active.as_ref().is_none_or(|step| {
                (0.0..self.step_secs).contains(&step.elapsed)
                    && step
                        .from
                        .iter()
                        .chain(&step.to)
                        .all(|value| value.is_finite())
            })
    }

    pub(crate) fn into_queue(self) -> MoveQueue {
        MoveQueue {
            steps: self.steps,
            active: self.active.map(|step| Tween {
                from: Vec2::from_array(step.from),
                to: Vec2::from_array(step.to),
                elapsed: step.elapsed,
                jumping: step.jumping,
            }),
            step_secs: self.step_secs,
        }
    }
}
