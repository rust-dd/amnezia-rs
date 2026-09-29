use super::*;

impl Battle {
    pub(super) fn action_prepare(
        &mut self,
        action: Action,
        inventory: &mut impl FnMut(u32, bool) -> bool,
    ) {
        let restriction = self.state_restriction(action.source);
        let replacement = match restriction {
            1 => Some(Command::Nothing),
            2 | 3 => {
                let party = matches!(action.source, Source::Party(_)) == (restriction == 3);
                let targets = self.action_group(party, false);
                let roll = rng_next(&mut self.rng) as usize;
                Some(
                    targets
                        .get(roll % targets.len().max(1))
                        .map_or(Command::Nothing, |source| {
                            let (Source::Party(target) | Source::Enemy(target)) = *source;
                            Command::Attack { target }
                        }),
                )
            }
            _ => {
                let possible = match action.kind {
                    Command::Skill { skill_id, target } => self
                        .skills
                        .iter()
                        .find(|s| s.id == skill_id)
                        .is_some_and(|skill| {
                            self.skill_usable_by(action.source, skill)
                                && match action.source {
                                    Source::Enemy(i) if matches!(skill.scope, 2 | 3) => self
                                        .enemy_ally_skill_target(
                                            if skill.scope == 2 { i } else { target },
                                            skill,
                                        ),
                                    _ => true,
                                }
                        }),
                    Command::Item { item_id, .. } => {
                        self.items.iter().any(|item| item.id == item_id)
                            && inventory(item_id, false)
                    }
                    _ => true,
                };
                (!possible).then_some(Command::Nothing)
            }
        };
        if let Some(kind) = replacement {
            self.queue[self.queue_at].kind = kind;
            match action.source {
                Source::Party(i) => self.members[i].defending = false,
                Source::Enemy(i) => {
                    self.enemies[i].defending = false;
                    self.enemies[i].switch_on_after_action = None;
                    self.enemies[i].switch_off_after_action = None;
                }
            }
        }
    }

    pub(super) fn action_begin(&mut self, timeline: &mut Timeline, action: Action) {
        self.messages.console.clear();
        let log_start = self.log.len();
        self.recover_before_action(action.source);
        self.tick_state_hp(action.source);
        let condition = (self.log.len() > log_start).then(|| self.log.pop().unwrap());
        let action = self.queue[self.queue_at];
        self.queue_at += 1;
        if condition.is_some() || !matches!(action.kind, Command::Nothing) {
            self.pending_action_flashes.push(action.source);
        }
        timeline.action = Some(action);
        if let Some(message) = condition {
            timeline.text_after_gap(message, Stage::AfterCondition, 20, 60);
        } else {
            timeline.stage = Stage::AfterCondition;
        }
    }

    pub(super) fn action_usage(
        &mut self,
        timeline: &mut Timeline,
        inventory: &mut impl FnMut(u32, bool) -> bool,
    ) {
        let action = timeline.action();
        let paid = match action.kind {
            Command::Item { item_id, .. } => inventory(item_id, true),
            Command::Skill { skill_id, .. } => {
                let skill = self.skills.iter().find(|s| s.id == skill_id).cloned();
                skill.is_some_and(|skill| {
                    self.change_sp(
                        action.source,
                        -(self.skill_cost(action.source, &skill) as i32),
                    );
                    true
                })
            }
            _ => true,
        };
        if !paid {
            timeline.stage = Stage::Finished;
            return;
        }
        if let Source::Enemy(i) = action.source {
            timeline.charged = std::mem::take(&mut self.enemies[i].charging);
        }
        timeline.targets = self.action_targets(action);
        timeline.repeat = u32::from(matches!(action.kind, Command::DoubleAttack { .. }));
        timeline.usage = self.action_usage_messages(action).into();
        self.messages.console.clear();
        timeline.stage = Stage::UsageLine;
    }

    pub(super) fn action_usage_messages(&self, action: Action) -> Vec<String> {
        let name = self.battler_name(action.source);
        let term = match action.kind {
            Command::Attack { .. } | Command::DoubleAttack { .. } => &self.text.attacking,
            Command::Defend => &self.text.defending,
            Command::Observe => &self.text.observing,
            Command::Charge => &self.text.focus,
            Command::SelfDestruct => &self.text.autodestruction,
            Command::Escape => &self.text.enemy_escape,
            Command::Skill { skill_id, .. } => {
                let Some(skill) = self.skills.iter().find(|s| s.id == skill_id) else {
                    return vec![];
                };
                let mut lines = vec![format!("{name}{}", skill.using_message1)];
                if !skill.using_message2.is_empty() {
                    lines.push(skill.using_message2.clone());
                }
                return lines;
            }
            Command::Item { item_id, .. } => {
                return self
                    .items
                    .iter()
                    .find(|i| i.id == item_id)
                    .map(|item| vec![format!("{name} {}{}", item.name, self.text.use_item)])
                    .unwrap_or_default();
            }
            _ => return vec![],
        };
        vec![format!("{name}{term}")]
    }

    pub(super) fn action_usage_line(
        &mut self,
        timeline: &mut Timeline,
        frames: &mut impl FnMut(u32) -> u32,
    ) {
        if let Some(line) = timeline.usage.pop_front() {
            if !line.is_empty() {
                self.action_message(line);
            }
            if !timeline.usage.is_empty() {
                timeline.usage_wait(0);
                return;
            }
        }
        timeline.usage_end = self.messages.console.len();
        let action = timeline.action();
        match action.kind {
            Command::Item { .. } => self.pending_se.push(BattleSe::UseItem),
            Command::SelfDestruct => self.pending_se.push(BattleSe::EnemyDefeated),
            Command::Escape => self.pending_se.push(BattleSe::Escape),
            _ => {}
        }
        let animation = match action.kind {
            Command::Attack { .. } | Command::DoubleAttack { .. } => match action.source {
                Source::Party(i) => self.members[i].attack_animation,
                Source::Enemy(_) => 0,
            },
            Command::Skill { skill_id, .. } => self
                .skills
                .iter()
                .find(|s| s.id == skill_id)
                .map_or(0, |skill| skill.animation_id),
            _ => 0,
        };
        let mut duration = 0;
        if animation != 0 && !timeline.targets.is_empty() {
            let sound_only = matches!(timeline.targets[0], Source::Party(_));
            let anchors = timeline
                .targets
                .iter()
                .map(|&target| self.battler_pos(target))
                .collect();
            self.push_anim_mode(animation, anchors, sound_only);
            duration = frames(animation);
            if sound_only {
                duration = duration.min(40);
            }
        }
        timeline.usage_wait(duration);
        timeline.stage = Stage::Execute;
    }

    pub(super) fn action_execute(&mut self, timeline: &mut Timeline) {
        let action = timeline.action();
        if let Some(target) = timeline.recipient()
            && !self.action_target_valid(action, target)
        {
            timeline.stage = Stage::Finished;
            return;
        }
        timeline.plan = if let Some(target) = timeline.recipient() {
            match action.kind {
                Command::Attack { .. } | Command::DoubleAttack { .. } => {
                    self.plan_attack(action.source, target, timeline.charged)
                }
                Command::SelfDestruct => self.plan_explosion(action.source, target),
                Command::Skill { skill_id, .. } => {
                    let skill = self
                        .skills
                        .iter()
                        .find(|s| s.id == skill_id)
                        .cloned()
                        .unwrap();
                    self.plan_skill(action.source, target, &skill)
                }
                Command::Item { item_id, .. } => {
                    let item = self.items.iter().find(|i| i.id == item_id).unwrap();
                    self.plan_item(target, item)
                }
                _ => TargetPlan {
                    success: true,
                    ..Default::default()
                },
            }
        } else {
            TargetPlan {
                success: true,
                ..Default::default()
            }
        };
        if matches!(
            action.kind,
            Command::Attack { .. } | Command::DoubleAttack { .. } | Command::SelfDestruct
        ) {
            timeline.wait.set(4, 4);
        }
        timeline.stage = if timeline.plan.critical {
            Stage::Critical
        } else {
            Stage::Apply
        };
    }

    pub(super) fn action_next_target(&mut self, timeline: &mut Timeline) -> bool {
        if timeline.repeat > 0 {
            timeline.repeat -= 1;
            if timeline
                .recipient()
                .is_some_and(|target| self.source_alive(target))
            {
                self.messages.console.pop_until(timeline.usage_end);
                timeline.stage = Stage::Execute;
                return true;
            }
        }
        timeline.target += 1;
        if timeline.target < timeline.targets.len() {
            timeline.repeat = u32::from(matches!(
                timeline.action().kind,
                Command::DoubleAttack { .. }
            ));
            self.messages.console.pop_until(timeline.usage_end);
            timeline.stage = Stage::Execute;
            return true;
        }
        false
    }

    pub(super) fn action_post_switches(&mut self, source: Source) {
        let Source::Enemy(i) = source else { return };
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

impl Timeline {
    pub(super) fn usage_wait(&mut self, animation: u32) {
        let (min, max) = match self.action().kind {
            Command::Attack { .. } | Command::DoubleAttack { .. } => (20, 40),
            Command::Escape => (36, 60),
            Command::Nothing | Command::DoNothing => (0, 0),
            _ => (20, 60),
        };
        self.wait.set(min.max(animation), max.max(animation));
    }
}
