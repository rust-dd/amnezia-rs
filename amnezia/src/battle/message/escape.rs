use super::*;
use crate::battle::{Battle, BattleOutcome, Phase, model::BattleSe};

impl Battle {
    pub(in crate::battle) fn begin_escape(&mut self, controls: Controls) {
        let escaped = self.attempt_escape();
        self.phase = Phase::Escape;
        self.messages.escaped = escaped;
        self.messages.console.clear();
        let line = crate::i18n::tr(if escaped {
            &self.text.escape_success
        } else {
            &self.text.escape_failure
        });
        self.messages.console.push(&line);
        self.log.push(line);
        self.messages.wait.set(10, 60);
        self.messages.wait.ready(controls);
    }
}

pub(super) fn advance(battle: &mut Battle, controls: Controls) -> bool {
    battle.messages.console.update();
    if !battle.messages.wait.ready(controls) {
        return false;
    }
    if battle.messages.escaped {
        battle.pending_se.push(BattleSe::Escape);
        battle.finish(BattleOutcome::Escape);
    } else {
        for member in &mut battle.members {
            member.command = None;
        }
        battle.begin_resolve();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::model::{Command, Source, testkit::build_party2};

    #[test]
    fn success_sounds_and_finishes_after_the_original_wait_without_confirmation() {
        for fast in [false, true] {
            let mut battle = build_party2();
            battle.first_strike = true;
            let controls = Controls { fast, hold: false };
            battle.begin_escape(controls);
            assert_eq!(battle.phase, Phase::Escape);
            assert!(battle.pending_se.is_empty());
            for _ in 0..100 {
                assert!(!advance(&mut battle, Controls { fast, hold: true }));
            }
            assert_eq!(battle.messages.console.visible(), ["Sikeres menekülés!"]);
            for _ in 0..if fast { 8 } else { 58 } {
                assert!(!advance(&mut battle, controls));
                assert!(battle.pending_se.is_empty());
            }
            assert!(advance(&mut battle, controls));
            assert_eq!(battle.phase, Phase::Outcome);
            assert_eq!(battle.outcome, Some(BattleOutcome::Escape));
            assert_eq!(battle.pending_se, [BattleSe::Escape]);
            assert_eq!(
                battle
                    .log
                    .iter()
                    .filter(|line| *line == "Sikeres menekülés!")
                    .count(),
                1
            );
        }
    }

    #[test]
    fn failure_waits_without_escape_sound_and_only_enemies_act_afterward() {
        let mut battle = build_party2();
        battle.escape_chance = 0;
        battle.members[0].command = Some(Command::Defend);
        let controls = Controls::default();
        battle.begin_escape(controls);
        assert_eq!(battle.escape_chance, 10);
        assert_eq!(battle.events.turn, 0);
        for _ in 0..58 {
            assert!(!advance(&mut battle, controls));
            assert_eq!(battle.events.turn, 0);
        }
        assert!(advance(&mut battle, controls));
        assert_eq!(battle.phase, Phase::Resolve);
        assert_eq!(battle.outcome, None);
        assert!(battle.pending_se.is_empty());
        assert_eq!(battle.events.turn, 1);
        assert!(battle.members.iter().all(|member| member.command.is_none()));
        assert!(!battle.queue.is_empty());
        assert!(
            battle
                .queue
                .iter()
                .all(|action| matches!(action.source, Source::Enemy(_)))
        );
    }
}
