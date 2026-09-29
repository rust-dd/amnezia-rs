use super::plan::TargetPlan;
use super::*;
use crate::battle::message::{Controls, Wait};

mod action;
mod apply;
mod results;
mod targets;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Default)]
enum Stage {
    #[default]
    Begin,
    AfterCondition,
    Usage,
    UsageLine,
    Execute,
    Critical,
    Apply,
    Failure,
    Damage,
    DamageApply,
    Released(usize),
    Parameter(usize),
    State(usize),
    DeathStateGap,
    Text,
    Finished,
}

#[derive(Default)]
pub(in crate::battle) struct Timeline {
    stage: Stage,
    after: Stage,
    action: Option<Action>,
    targets: Vec<Source>,
    target: usize,
    repeat: u32,
    charged: bool,
    custom_applied: bool,
    wait: Wait,
    pending: String,
    text_wait: (u32, u32),
    usage: std::collections::VecDeque<String>,
    usage_end: usize,
    results_end: usize,
    damage_end: usize,
    plan: TargetPlan,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::battle) enum Progress {
    Waiting,
    Boundary,
    Done,
}

impl Timeline {
    fn action(&self) -> Action {
        self.action.unwrap()
    }

    fn recipient(&self) -> Option<Source> {
        self.targets.get(self.target).copied()
    }

    fn text_after_gap(&mut self, text: String, after: Stage, min: u32, max: u32) {
        self.pending = text;
        self.after = after;
        self.text_wait = (min, max);
        self.stage = Stage::Text;
        self.wait.set(4, 4);
    }
}

impl Battle {
    /// Advance one logical battle update, stopping at a wait or an action boundary.
    pub(in crate::battle) fn advance_action(
        &mut self,
        controls: Controls,
        mut animation_frames: impl FnMut(u32) -> u32,
        mut inventory: impl FnMut(u32, bool) -> bool,
    ) -> Progress {
        let mut timeline = std::mem::take(&mut self.timeline);
        let progress = loop {
            if !timeline.wait.ready(controls) {
                break Progress::Waiting;
            }
            match timeline.stage {
                Stage::Begin => {
                    let Some(&action) = self.queue.get(self.queue_at) else {
                        break Progress::Done;
                    };
                    if !self.source_alive(action.source) {
                        self.queue_at += 1;
                        break Progress::Boundary;
                    }
                    self.action_prepare(action, &mut inventory);
                    self.action_begin(&mut timeline, action);
                }
                Stage::AfterCondition => {
                    self.messages.console.clear();
                    if matches!(timeline.action().kind, Command::Nothing) {
                        timeline.stage = Stage::Finished;
                    } else {
                        timeline.wait.set(4, 4);
                        timeline.stage = Stage::Usage;
                    }
                }
                Stage::Usage => {
                    self.action_usage(&mut timeline, &mut inventory);
                }
                Stage::UsageLine => {
                    self.action_usage_line(&mut timeline, &mut animation_frames);
                }
                Stage::Execute => self.action_execute(&mut timeline),
                Stage::Critical => {
                    let term = if matches!(timeline.recipient(), Some(Source::Party(_))) {
                        &self.text.actor_critical
                    } else {
                        &self.text.enemy_critical
                    };
                    self.action_message(crate::i18n::tr(term));
                    timeline.wait.set(10, 30);
                    timeline.stage = Stage::Apply;
                }
                Stage::Apply => self.action_apply(&mut timeline),
                Stage::Failure => self.action_failure(&mut timeline),
                Stage::Damage => self.action_damage(&mut timeline),
                Stage::DamageApply => self.action_damage_apply(&mut timeline),
                Stage::Released(index) => self.action_release(&mut timeline, index),
                Stage::Parameter(index) => self.action_parameter(&mut timeline, index),
                Stage::State(index) => self.action_state(&mut timeline, index),
                Stage::DeathStateGap => {
                    // RPG_RT repeats this message wait; EasyRPG skips that legacy extra step.
                    let text = std::mem::take(&mut timeline.pending);
                    timeline.text_after_gap(text, Stage::Finished, 20, 60);
                }
                Stage::Text => {
                    self.action_message(std::mem::take(&mut timeline.pending));
                    timeline
                        .wait
                        .set(timeline.text_wait.0, timeline.text_wait.1);
                    timeline.stage = timeline.after;
                }
                Stage::Finished => {
                    if self.action_next_target(&mut timeline) {
                        continue;
                    }
                    self.action_post_switches(timeline.action().source);
                    self.messages.console.clear();
                    timeline = Timeline::default();
                    break Progress::Boundary;
                }
            }
        };
        self.timeline = timeline;
        progress
    }

    fn action_message(&mut self, text: String) {
        self.messages.console.push(&text);
        self.messages.console.scroll_to_end();
        self.log.push(text);
    }

    pub(in crate::battle) fn action_in_progress(&self) -> bool {
        self.timeline.action.is_some()
    }
}
