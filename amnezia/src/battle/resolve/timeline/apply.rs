use super::*;
use crate::battle::resolve::plan::{StateChange, StateEffect};

impl Battle {
    pub(super) fn action_custom(&mut self, action: Action) {
        match (action.source, action.kind) {
            (Source::Party(i), Command::Defend) => self.members[i].defending = true,
            (Source::Enemy(i), Command::Defend) => self.enemies[i].defending = true,
            (Source::Enemy(i), Command::Charge) => self.enemies[i].charging = true,
            (Source::Enemy(i), kind @ (Command::Escape | Command::SelfDestruct)) => {
                let explode = matches!(kind, Command::SelfDestruct);
                self.enemies[i].fled = true;
                self.enemies[i].dying = Some(Dying {
                    elapsed: 0.0,
                    secs: if explode { EXPLODE_SECS } else { DEATH_SECS },
                    explode,
                });
            }
            _ => {}
        }
    }

    pub(super) fn action_hp(&mut self, timeline: &Timeline, target: Source) -> i32 {
        if self.battler_hp(target) <= 0 {
            return 0;
        }
        let delta = self.restore_skill_hp(target, timeline.plan.hp.unwrap_or(0));
        if delta > 0 {
            self.report_hit(self.battler_pos(target), delta.to_string(), HitKind::Heal);
        }
        if timeline.plan.absorb_hp && delta < 0 && self.battler_hp(timeline.action().source) > 0 {
            let source = timeline.action().source;
            let gained = self.restore_skill_hp(source, -delta);
            self.report_hit(self.battler_pos(source), gained.to_string(), HitKind::Heal);
        }
        delta
    }

    pub(super) fn action_sp(&mut self, timeline: &Timeline, target: Source) -> i32 {
        let delta = self.change_sp(target, timeline.plan.sp);
        if timeline.plan.absorb_sp && delta < 0 {
            self.change_sp(timeline.action().source, -delta);
        }
        if delta != 0 {
            self.report_hit(
                self.battler_pos(target),
                delta.abs().to_string(),
                if delta > 0 {
                    HitKind::Heal
                } else {
                    HitKind::Damage
                },
            );
        }
        delta
    }

    pub(super) fn action_stat(&mut self, timeline: &Timeline, target: Source, index: usize) -> i32 {
        let delta = self.stat_change(target, index, timeline.plan.stats[index]);
        match target {
            Source::Party(i) => self.members[i].stat_modifiers[index] += delta,
            Source::Enemy(i) => self.enemies[i].stat_modifiers[index] += delta,
        }
        delta
    }

    pub(super) fn action_state_effect(
        &mut self,
        timeline: &Timeline,
        target: Source,
        effect: StateChange,
    ) {
        match effect.effect {
            StateEffect::Released | StateEffect::Healed => {
                let revive = effect.id == 1 && self.battler_hp(target) <= 0;
                self.cure_battler_state(target, effect.id);
                if revive {
                    let hp = timeline.plan.hp.unwrap_or(0).clamp(1, self.pools(target).0);
                    match target {
                        Source::Party(i) => self.members[i].hp = hp,
                        Source::Enemy(i) => {
                            self.enemies[i].hp = hp;
                            self.enemies[i].dying = None;
                        }
                    }
                    self.report_hit(self.battler_pos(target), hp.to_string(), HitKind::Heal);
                }
            }
            StateEffect::Inflicted => {
                self.inflict_battler_state(target, effect.id);
            }
            StateEffect::Already => {}
        }
    }

    pub(super) fn action_state_text(&self, target: Source, effect: StateChange) -> String {
        let Some(state) = self.states.iter().find(|state| state.id == effect.id) else {
            return String::new();
        };
        let suffix = match effect.effect {
            StateEffect::Released | StateEffect::Healed => &state.message_recovery,
            StateEffect::Already => &state.message_already,
            StateEffect::Inflicted if matches!(target, Source::Party(_)) => &state.message_actor,
            StateEffect::Inflicted => &state.message_enemy,
        };
        format!("{}{suffix}", self.battler_name(target))
    }
}
