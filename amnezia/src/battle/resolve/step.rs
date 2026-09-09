//! The resolution engine: draining the queue and its deferred sub-steps, and the
//! action dispatcher that routes each command to its effect helper.

use super::*;

impl Battle {
    /// Advance the resolution by one beat and report whether more remains. A
    /// pending sub-step (a further target of a multi-target cast, or a critical's
    /// damage beat) is drained first, one per call, so those beats stagger across
    /// ticks; otherwise the next queued action is applied (skipping a fainted
    /// actor, retargeting a dead target) and its log line appended. Returns
    /// `false` only once both the step buffer and the queue are spent.
    #[cfg(test)]
    pub fn resolve_next(&mut self) -> bool {
        self.resolve_next_with_items(|_| true)
    }

    /// Consume a held item only when its living user actually begins the action.
    /// The callback returns false if an earlier action used the last copy.
    pub fn resolve_next_with_items(&mut self, mut consume: impl FnMut(u32) -> bool) -> bool {
        if let Some(step) = self.steps.pop_front() {
            self.run_step(step);
            self.finish_action();
            return true;
        }
        let Some(&action) = self.queue.get(self.queue_at) else {
            return false;
        };
        self.queue_at += 1;
        if self.source_alive(action.source) {
            // RM2000 applies an HP-changing state (poison drain, regen) at the
            // start of the battler's turn, before it acts; a drain that fells the
            // battler cancels its action through the HP-based death path.
            self.tick_state_hp(action.source);
            if self.source_alive(action.source) {
                if let Command::Item { item_id, .. } = action.kind
                    && !consume(item_id)
                {
                    return true;
                }
                if let Command::Skill { skill_id, .. } = action.kind
                    && !self
                        .skills
                        .iter()
                        .find(|skill| skill.id == skill_id)
                        .is_some_and(|skill| self.skill_usable_by(action.source, skill))
                {
                    return true;
                }
                self.action_source = Some(action.source);
                self.apply(action);
                self.finish_action();
            }
        }
        true
    }

    fn finish_action(&mut self) {
        if !self.steps.is_empty() {
            return;
        }
        if let Some(Source::Enemy(i)) = self.action_source.take() {
            let on = self.enemies[i].switch_on_after_action.take();
            let off = self.enemies[i].switch_off_after_action.take();
            for (id, enabled) in on
                .map(|id| (id, true))
                .into_iter()
                .chain(off.map(|id| (id, false)))
            {
                if enabled {
                    self.ai_switches.insert(id);
                } else {
                    self.ai_switches.remove(&id);
                }
                self.pending_switches.push((id, enabled));
            }
        }
    }

    /// Resolve one deferred [`Step`] of the action in progress: the next target of
    /// a multi-target cast (rolled now, so the RNG draw order matches a single-tick
    /// cast), or the damage beat that follows a critical's "Kritikus!" line. Each
    /// pushes its own log line(s), so a multi-target cast reads as one message per
    /// target and a felled target starts its own death-hold before the next beat.
    fn run_step(&mut self, step: Step) {
        match step {
            Step::EnemySkillTarget {
                ei,
                target,
                skill_id,
            } => {
                if let Some(skill) = self
                    .skills
                    .iter()
                    .find(|skill| skill.id == skill_id)
                    .cloned()
                {
                    let lines = self.enemy_skill_target(ei, target, &skill);
                    self.log.extend(lines);
                }
            }
            Step::HitEnemy { pi, ti, skill_id } => {
                if !self.enemies.get(ti).is_some_and(|e| e.alive()) {
                    return;
                }
                if let Some(skill) = self.skills.iter().find(|s| s.id == skill_id).cloned() {
                    let lines = self.skill_hit_enemy(pi, ti, &skill);
                    self.log.extend(lines);
                }
            }
            Step::HealAlly { pi, ti, skill_id } => {
                if let Some(skill) = self.skills.iter().find(|s| s.id == skill_id).cloned() {
                    let lines = self.skill_heal_ally(pi, ti, &skill);
                    self.log.extend(lines);
                }
            }
            Step::CritDamage { ti, dmg } => {
                self.land_strike(ti, dmg);
                let line = format!(
                    "{} {dmg}{}",
                    self.enemies[ti].name,
                    crate::i18n::tr(&self.text.enemy_damaged)
                );
                self.log.push(line);
            }
            Step::StrikeImpact {
                pi,
                ti,
                dmg,
                crit,
                miss,
            } => {
                let outcome = if miss {
                    Strike::Miss
                } else {
                    Strike::Hit { dmg, crit }
                };
                self.resolve_strike_impact(pi, ti, outcome);
            }
            Step::CastSkill {
                pi,
                skill_id,
                target,
            } => {
                self.suppress_anim = true;
                let line = self.cast_skill(pi, skill_id, target);
                self.suppress_anim = false;
                if let Some(line) = line {
                    self.log.push(line);
                }
            }
            Step::EnemyCast {
                ei,
                skill_id,
                target,
            } => {
                self.suppress_anim = true;
                let line = self.enemy_cast(ei, skill_id, target);
                self.suppress_anim = false;
                if let Some(line) = line {
                    self.log.push(line);
                }
            }
        }
    }

    fn source_alive(&self, source: Source) -> bool {
        match source {
            Source::Party(i) => self.members.get(i).is_some_and(|f| f.alive()),
            Source::Enemy(i) => self.enemies.get(i).is_some_and(|e| e.alive()),
        }
    }

    pub(in crate::battle::resolve) fn apply(&mut self, action: Action) {
        let line = match (action.source, action.kind) {
            (Source::Party(pi), Command::Attack { target }) => {
                if logic::worst_restriction(&self.members[pi].states, &self.states) == 3 {
                    let Some(ti) = self.retarget_ally(pi, target) else {
                        return;
                    };
                    let base = logic::physical_damage(
                        self.members[pi].stats.attack,
                        self.members[ti].stats.defense,
                    );
                    let dmg = self.hit_member(ti, base, 4);
                    format!(
                        "{} zavartan lesújt: {} -{}",
                        self.members[pi].name, self.members[ti].name, dmg
                    )
                } else {
                    let Some(ti) = self.retarget_enemy(target) else {
                        return;
                    };
                    let outcome = self.plan_strike(pi, ti);
                    // RPG_RT applies damage after the swing animation completes.
                    if self.members[pi].attack_animation != 0 {
                        let (dmg, crit, miss) = match outcome {
                            Strike::Miss => (0, false, true),
                            Strike::Hit { dmg, crit } => (dmg, crit, false),
                        };
                        self.steps.push_back(Step::StrikeImpact {
                            pi,
                            ti,
                            dmg,
                            crit,
                            miss,
                        });
                        self.begin_anim_hold();
                    } else {
                        self.resolve_strike_impact(pi, ti, outcome);
                    }
                    return;
                }
            }
            (Source::Party(pi), Command::Skill { skill_id, target }) => {
                // The animation must complete before the skill changes its targets.
                let Some(skill) = self.skills.iter().find(|s| s.id == skill_id).cloned() else {
                    return;
                };
                let anchors = self.skill_anim_anchors(pi, &skill, target);
                if skill.animation_id != 0 && !anchors.is_empty() {
                    if !self.pay_skill(Source::Party(pi), &skill) {
                        return;
                    }
                    self.push_anim(skill.animation_id, anchors);
                    self.steps.push_back(Step::CastSkill {
                        pi,
                        skill_id,
                        target,
                    });
                    self.begin_anim_hold();
                    return;
                }
                match self.cast_skill(pi, skill_id, target) {
                    Some(line) => line,
                    None => return,
                }
            }
            (Source::Party(pi), Command::Item { item_id, target }) => {
                self.apply_item(pi, item_id, target)
            }
            (Source::Party(pi), Command::Defend) => {
                self.members[pi].defending = true;
                format!("{} védekezik", self.members[pi].name)
            }
            (Source::Party(pi), Command::Nothing) => {
                format!("{} tétovázik", self.members[pi].name)
            }
            (Source::Enemy(ei), Command::Attack { target }) => {
                if logic::worst_restriction(&self.enemies[ei].states, &self.states) == 3 {
                    let Some(ti) = self.retarget_other_enemy(ei, target) else {
                        return;
                    };
                    let base = logic::physical_damage(
                        self.enemies[ei].stats.attack,
                        self.enemies[ti].stats.defense,
                    );
                    let dmg = self.hit_enemy(ti, base, 4);
                    format!(
                        "{} zavartan lesújt: {} -{}",
                        self.enemies[ei].name, self.enemies[ti].name, dmg
                    )
                } else {
                    let Some(ti) = self.retarget_member(target) else {
                        return;
                    };
                    let enemy = self.enemies[ei].name.clone();
                    let member = self.members[ti].name.clone();
                    match self.enemy_strike_member(ei, ti) {
                        Some(dmg) => format!("{enemy} támad: {member} -{dmg}"),
                        None => format!("{enemy} támad: {member} elkerülte"),
                    }
                }
            }
            (Source::Enemy(ei), Command::Skill { skill_id, target }) => {
                let Some(skill) = self.skills.iter().find(|s| s.id == skill_id).cloned() else {
                    return;
                };
                let Some(target) = self.enemy_skill_target_index(&skill, target) else {
                    return;
                };
                let anchors = self.enemy_skill_anim_anchors(ei, &skill, target);
                if skill.animation_id != 0 && !anchors.is_empty() {
                    if !self.pay_skill(Source::Enemy(ei), &skill) {
                        return;
                    }
                    self.push_anim(skill.animation_id, anchors);
                    self.steps.push_back(Step::EnemyCast {
                        ei,
                        skill_id,
                        target,
                    });
                    self.begin_anim_hold();
                    return;
                }
                match self.enemy_cast(ei, skill_id, target) {
                    Some(line) => line,
                    None => return,
                }
            }
            (Source::Enemy(ei), Command::DoubleAttack { target }) => {
                let Some(ti) = self.retarget_member(target) else {
                    return;
                };
                let d1 = self.enemy_strike_member(ei, ti);
                let d2 = self.enemy_strike_member(ei, ti);
                let enemy = self.enemies[ei].name.clone();
                let member = self.members[ti].name.clone();
                let show =
                    |d: Option<i32>| d.map_or_else(|| "elkerülte".to_string(), |v| format!("-{v}"));
                format!("{enemy} kétszer támad: {member} {}, {}", show(d1), show(d2))
            }
            (Source::Enemy(ei), Command::Defend) => {
                self.enemies[ei].defending = true;
                format!("{} védekezik", self.enemies[ei].name)
            }
            (Source::Enemy(ei), Command::SelfDestruct) => {
                let atk = self.enemies[ei].stats.attack as i32;
                let name = self.enemies[ei].name.clone();
                for ti in self.living_members() {
                    let base = (atk - self.members[ti].stats.defense as i32 / 2).max(0);
                    self.hit_member(ti, base, 4);
                }
                self.enemies[ei].hp = 0;
                self.start_foe_death(ei, true);
                format!("{name} felrobban!")
            }
            (Source::Enemy(ei), Command::Escape) => {
                self.enemies[ei].fled = true;
                format!("{} elmenekül", self.enemies[ei].name)
            }
            (Source::Enemy(ei), Command::Charge) => {
                self.enemies[ei].charging = true;
                format!("{} erőt gyűjt", self.enemies[ei].name)
            }
            (Source::Enemy(ei), Command::Nothing) => {
                format!("{} tétovázik", self.enemies[ei].name)
            }
            _ => return,
        };
        self.log.push(line);
    }
}
