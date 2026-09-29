use super::*;

impl Battle {
    pub(super) fn action_target_valid(&self, action: Action, target: Source) -> bool {
        match action.kind {
            Command::Item { .. } => true,
            Command::Skill { skill_id, .. } => {
                self.source_alive(target)
                    || self
                        .skills
                        .iter()
                        .find(|skill| skill.id == skill_id)
                        .is_some_and(|skill| {
                            skill.scope >= 2
                                && skill.affected_states.contains(&1)
                                && !matches!(target, Source::Enemy(i) if self.enemies[i].fled)
                        })
            }
            _ => self.source_alive(target),
        }
    }

    pub(super) fn action_targets(&self, action: Action) -> Vec<Source> {
        let party_source = matches!(action.source, Source::Party(_));
        match action.kind {
            Command::Attack { target } | Command::DoubleAttack { target } => {
                let same_side = self.state_restriction(action.source) == 3;
                self.action_single_target(party_source == same_side, target, false)
            }
            Command::SelfDestruct => self.action_group(!party_source, false),
            Command::Skill { skill_id, target } => {
                let Some(skill) = self.skills.iter().find(|s| s.id == skill_id) else {
                    return vec![];
                };
                let revive = skill.affected_states.contains(&1);
                match skill.scope {
                    1 => self.action_group(!party_source, false),
                    2 => vec![action.source],
                    3 => self.action_single_target(party_source, target, revive),
                    4 => self.action_group(party_source, revive),
                    _ => self.action_single_target(!party_source, target, false),
                }
            }
            Command::Item { item_id, target } => {
                let Some(item) = self.items.iter().find(|i| i.id == item_id) else {
                    return vec![];
                };
                if item.scope == 1 {
                    self.action_group(party_source, true)
                } else {
                    let len = if party_source {
                        self.members.len()
                    } else {
                        self.enemies.len()
                    };
                    (target < len)
                        .then_some(if party_source {
                            Source::Party(target)
                        } else {
                            Source::Enemy(target)
                        })
                        .into_iter()
                        .collect()
                }
            }
            Command::Nothing => vec![],
            _ => vec![action.source],
        }
    }

    pub(super) fn action_group(&self, party: bool, dead: bool) -> Vec<Source> {
        if party && !dead {
            return self
                .living_members()
                .into_iter()
                .map(Source::Party)
                .collect();
        }
        if party {
            self.members
                .iter()
                .enumerate()
                .filter_map(|(i, member)| (dead || member.alive()).then_some(Source::Party(i)))
                .collect()
        } else {
            self.enemies
                .iter()
                .enumerate()
                .filter_map(|(i, enemy)| {
                    (!enemy.fled && (dead || enemy.alive())).then_some(Source::Enemy(i))
                })
                .collect()
        }
    }

    pub(super) fn action_single_target(
        &self,
        party: bool,
        target: usize,
        dead: bool,
    ) -> Vec<Source> {
        let candidates = self.action_group(party, dead);
        candidates
            .iter()
            .find(|source| match source {
                Source::Party(i) | Source::Enemy(i) => *i >= target,
            })
            .or_else(|| candidates.first())
            .copied()
            .into_iter()
            .collect()
    }
}
