use super::*;
use crate::battle::resolve::plan::StateEffect;

impl Battle {
    pub(super) fn action_apply(&mut self, timeline: &mut Timeline) {
        if !timeline.custom_applied {
            self.action_custom(timeline.action());
            timeline.custom_applied = true;
        }
        timeline.results_end = self.messages.console.len();
        if !timeline.plan.success {
            timeline.wait.set(4, 4);
            timeline.stage = Stage::Failure;
        } else if timeline.recipient().is_none() {
            timeline.stage = Stage::Finished;
        } else if timeline.plan.damages_hp() {
            timeline.wait.set(4, 4);
            timeline.stage = Stage::Damage;
        } else {
            timeline.stage = Stage::Parameter(0);
        }
    }

    pub(super) fn action_failure(&mut self, timeline: &mut Timeline) {
        if let Some(target) = timeline.recipient() {
            if timeline.plan.failure == 3 {
                self.pending_se.push(BattleSe::Dodge);
            }
            self.report_hit(self.battler_pos(target), "Miss".into(), HitKind::Miss);
            self.action_message(
                self.text
                    .skill_failed(self.battler_name(target), timeline.plan.failure),
            );
        }
        timeline.wait.set(20, 60);
        timeline.stage = Stage::Finished;
    }

    pub(super) fn action_damage(&mut self, timeline: &mut Timeline) {
        let target = timeline.recipient().unwrap();
        let amount = -timeline.plan.hp.unwrap();
        let ally = matches!(target, Source::Party(_));
        if !timeline.plan.absorb_hp {
            self.pending_se.push(if ally {
                BattleSe::ActorDamaged
            } else {
                BattleSe::EnemyDamaged
            });
            self.pending_shake |= ally && amount > 0;
            if !ally {
                self.pending_blinks.push(self.battler_pos(target));
            }
        }
        let (text, kind) = damage_report(amount);
        self.report_hit(self.battler_pos(target), text, kind);
        let name = self.battler_name(target);
        let line = if timeline.plan.absorb_hp {
            self.text.absorbed(name, ally, true, amount)
        } else {
            self.text.damaged(name, ally, amount)
        };
        self.action_message(line);
        timeline
            .wait
            .set(20, if timeline.plan.absorb_hp { 60 } else { 40 });
        timeline.stage = Stage::DamageApply;
    }

    pub(super) fn action_damage_apply(&mut self, timeline: &mut Timeline) {
        let target = timeline.recipient().unwrap();
        self.action_hp(timeline, target);
        if self.battler_hp(target) <= 0 {
            self.action_death(timeline, target);
        }
        timeline.damage_end = self.messages.console.len();
        timeline.stage = Stage::Released(0);
    }

    pub(super) fn action_release(&mut self, timeline: &mut Timeline, index: usize) {
        let target = timeline.recipient().unwrap();
        let next = timeline
            .plan
            .states
            .iter()
            .enumerate()
            .skip(index)
            .find(|(_, state)| state.effect == StateEffect::Released)
            .map(|(i, state)| (i, *state));
        if let Some((index, effect)) = next {
            self.action_state_effect(timeline, target, effect);
            let text = self.action_state_text(target, effect);
            self.messages.console.pop_until(timeline.damage_end);
            timeline.text_after_gap(text, Stage::Released(index + 1), 20, 40);
        } else {
            timeline.wait.set(0, 10);
            timeline.stage = Stage::Parameter(0);
        }
    }

    pub(super) fn action_parameter(&mut self, timeline: &mut Timeline, index: usize) {
        let target = timeline.recipient().unwrap();
        for index in index..6 {
            let delta = match index {
                0 if timeline.plan.hp.is_some_and(|hp| hp > 0) && !timeline.plan.revived => {
                    self.action_hp(timeline, target)
                }
                0 => continue,
                1 => self.action_sp(timeline, target),
                _ => self.action_stat(timeline, target, index - 2),
            };
            let name = self.battler_name(target);
            let text = match index {
                0 => self.text.recovered(name, true, delta),
                1 if timeline.plan.absorb_sp => {
                    self.text
                        .absorbed(name, matches!(target, Source::Party(_)), false, -delta)
                }
                1 if delta > 0 => self.text.recovered(name, false, delta),
                1 if delta < 0 => {
                    self.text
                        .parameter_changed(name, &self.text.spirit_points, delta)
                }
                1 => continue,
                _ if delta != 0 => {
                    let points = [
                        &self.text.attack,
                        &self.text.defense,
                        &self.text.spirit,
                        &self.text.agility,
                    ][index - 2];
                    self.text.parameter_changed(name, points, delta)
                }
                _ => continue,
            };
            self.messages.console.pop_until(timeline.results_end);
            timeline.text_after_gap(text, Stage::Parameter(index + 1), 20, 60);
            return;
        }
        timeline.stage = Stage::State(0);
    }

    pub(super) fn action_state(&mut self, timeline: &mut Timeline, index: usize) {
        let target = timeline.recipient().unwrap();
        for index in index..timeline.plan.states.len() {
            let effect = timeline.plan.states[index];
            if effect.effect == StateEffect::Released {
                continue;
            }
            if !self.states.iter().any(|state| state.id == effect.id) {
                continue;
            }
            let was_dead = self.battler_hp(target) <= 0;
            self.action_state_effect(timeline, target, effect);
            let text = self.action_state_text(target, effect);
            self.messages.console.pop_until(timeline.results_end);
            if !was_dead && self.battler_hp(target) <= 0 {
                self.action_death(timeline, target);
                timeline.pending = text;
                timeline.stage = Stage::DeathStateGap;
                return;
            }
            if !text.is_empty() {
                timeline.text_after_gap(text, Stage::State(index + 1), 20, 60);
                return;
            }
        }
        timeline.stage = Stage::Finished;
    }

    pub(super) fn action_death(&mut self, timeline: &mut Timeline, target: Source) {
        self.mark_knocked_out(target);
        if let Source::Enemy(i) = target {
            self.start_foe_death(i, false);
        }
        self.action_message(self.death_message(target).unwrap_or_default());
        timeline.wait.set(36, 60);
    }
}
