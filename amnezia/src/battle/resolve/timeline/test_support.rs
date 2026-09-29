use super::*;

mod effects;
mod physical;

impl Battle {
    pub(in crate::battle) fn tick_action(&mut self) -> Progress {
        self.messages.console.update();
        self.advance_deaths(1.0 / 60.0);
        self.advance_action(Controls::default(), |_| 0, |_, _| true)
    }

    pub(in crate::battle::resolve) fn start_test_action(&mut self, action: Action) {
        self.timeline = Timeline {
            stage: if matches!(action.kind, Command::Nothing) {
                Stage::Finished
            } else {
                Stage::Usage
            },
            action: Some(action),
            ..Default::default()
        };
        self.tick_action();
    }

    pub(in crate::battle) fn resolve_next(&mut self) -> bool {
        self.resolve_next_with_items(|_, _| true)
    }

    pub(in crate::battle) fn resolve_next_with_items(
        &mut self,
        mut inventory: impl FnMut(u32, bool) -> bool,
    ) -> bool {
        for _ in 0..10000 {
            self.messages.console.update();
            self.advance_deaths(1.0 / 60.0);
            match self.advance_action(Controls::default(), |_| 0, &mut inventory) {
                Progress::Waiting => {}
                Progress::Boundary => return true,
                Progress::Done => return false,
            }
        }
        panic!("action did not finish");
    }

    pub(in crate::battle::resolve) fn apply(&mut self, action: Action) {
        if let Command::Skill { skill_id, .. } = action.kind
            && !self
                .skills
                .iter()
                .find(|s| s.id == skill_id)
                .is_some_and(|s| self.skill_usable_by(action.source, s))
        {
            return;
        }
        if let Command::Item { item_id, .. } = action.kind
            && !self.items.iter().any(|item| item.id == item_id)
        {
            return;
        }
        self.timeline = Timeline {
            stage: if matches!(action.kind, Command::Nothing) {
                Stage::Finished
            } else {
                Stage::Usage
            },
            action: Some(action),
            ..Default::default()
        };
        assert!(self.resolve_next());
    }

    fn run_effect(
        &mut self,
        source: Source,
        target: Source,
        plan: TargetPlan,
        stage: Stage,
    ) -> Vec<String> {
        let start = self.log.len();
        self.timeline = Timeline {
            stage,
            action: Some(Action {
                source,
                kind: Command::Observe,
                agility: 1,
            }),
            targets: vec![target],
            plan,
            custom_applied: true,
            ..Default::default()
        };
        assert!(self.resolve_next());
        self.log[start..].to_vec()
    }

    pub(in crate::battle::resolve) fn cast_skill(
        &mut self,
        pi: usize,
        skill_id: u32,
        target: usize,
    ) -> Option<String> {
        self.test_cast(Source::Party(pi), skill_id, target)
    }

    pub(in crate::battle::resolve) fn enemy_cast(
        &mut self,
        ei: usize,
        skill_id: u32,
        target: usize,
    ) -> Option<String> {
        self.test_cast(Source::Enemy(ei), skill_id, target)
    }

    fn test_cast(&mut self, source: Source, skill_id: u32, target: usize) -> Option<String> {
        let skill = self.skills.iter().find(|s| s.id == skill_id)?;
        if !self.skill_usable_by(source, skill) {
            return None;
        }
        let usage = 1 + usize::from(!skill.using_message2.is_empty());
        let start = self.log.len();
        self.apply(Action {
            source,
            kind: Command::Skill { skill_id, target },
            agility: 1,
        });
        Some(self.log[start + usage..].join("\n"))
    }

    pub(in crate::battle::resolve) fn skill_hit_battler(
        &mut self,
        source: Source,
        target: Source,
        skill: &SkillDef,
    ) -> Vec<String> {
        let plan = self.plan_skill(source, target, skill);
        self.run_effect(source, target, plan, Stage::Apply)
    }

    pub(in crate::battle::resolve) fn skill_heal_battler(
        &mut self,
        source: Source,
        target: Source,
        skill: &SkillDef,
    ) -> Vec<String> {
        self.skill_hit_battler(source, target, skill)
    }

    pub(in crate::battle::resolve) fn skill_hit_enemy(
        &mut self,
        pi: usize,
        ti: usize,
        skill: &SkillDef,
    ) -> Vec<String> {
        self.skill_hit_battler(Source::Party(pi), Source::Enemy(ti), skill)
    }

    pub(in crate::battle::resolve) fn skill_heal_ally(
        &mut self,
        pi: usize,
        ti: usize,
        skill: &SkillDef,
    ) -> Vec<String> {
        if self.members[ti].alive() || skill.affected_states.contains(&1) {
            self.skill_hit_battler(Source::Party(pi), Source::Party(ti), skill)
        } else {
            Vec::new()
        }
    }

    pub(in crate::battle::resolve) fn apply_item(
        &mut self,
        pi: usize,
        item_id: u32,
        target: usize,
    ) -> String {
        let start = self.log.len();
        self.apply(Action {
            source: Source::Party(pi),
            kind: Command::Item { item_id, target },
            agility: 1,
        });
        self.log[start..].join("\n")
    }

    pub(in crate::battle::resolve) fn push_skill_anim(
        &mut self,
        source: Source,
        skill: &SkillDef,
        target: usize,
    ) {
        let targets = self.test_skill_targets(source, skill, target);
        let sound_only = targets
            .first()
            .is_some_and(|t| matches!(t, Source::Party(_)));
        let anchors = targets.into_iter().map(|t| self.battler_pos(t)).collect();
        self.push_anim_mode(skill.animation_id, anchors, sound_only);
    }

    fn test_skill_targets(
        &mut self,
        source: Source,
        skill: &SkillDef,
        target: usize,
    ) -> Vec<Source> {
        let existing = self.skills.iter().position(|s| s.id == skill.id);
        let old = existing.map(|index| std::mem::replace(&mut self.skills[index], skill.clone()));
        if existing.is_none() {
            self.skills.push(skill.clone());
        }
        let targets = self.action_targets(Action {
            source,
            kind: Command::Skill {
                skill_id: skill.id,
                target,
            },
            agility: 1,
        });
        if let Some(old) = old {
            self.skills[existing.unwrap()] = old;
        } else {
            self.skills.pop();
        }
        targets
    }

    pub(in crate::battle::resolve) fn skill_anim_anchors(
        &mut self,
        pi: usize,
        skill: &SkillDef,
        target: usize,
    ) -> Vec<(f32, f32)> {
        self.test_skill_targets(Source::Party(pi), skill, target)
            .into_iter()
            .map(|t| self.battler_pos(t))
            .collect()
    }

    pub(in crate::battle::resolve) fn retarget_enemy(&self, target: usize) -> Option<usize> {
        self.test_retarget(false, target)
    }

    pub(in crate::battle::resolve) fn retarget_ally(
        &self,
        _pi: usize,
        target: usize,
    ) -> Option<usize> {
        self.test_retarget(true, target)
    }

    pub(in crate::battle::resolve) fn retarget_other_enemy(
        &self,
        _ei: usize,
        target: usize,
    ) -> Option<usize> {
        self.test_retarget(false, target)
    }

    fn test_retarget(&self, party: bool, target: usize) -> Option<usize> {
        self.action_single_target(party, target, false)
            .first()
            .map(|source| {
                let (Source::Party(i) | Source::Enemy(i)) = *source;
                i
            })
    }
}
