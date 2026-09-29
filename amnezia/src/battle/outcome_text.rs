use crate::dialogue::Dialogue;
use crate::events::MessageBox;

#[derive(Default)]
pub(in crate::battle) struct Script {
    pages: Vec<Vec<String>>,
    pub(in crate::battle) started: bool,
}

impl Script {
    pub(in crate::battle) fn reward(&mut self, line: &str, pause: &str) {
        if self.pages.is_empty() {
            self.pages.push(Vec::new());
        }
        self.pages[0].push(format!("{line}{pause}"));
    }

    pub(in crate::battle) fn actor(&mut self, lines: Vec<String>) {
        self.pages.push(lines);
    }

    pub(in crate::battle) fn open(&mut self, dialogue: &mut Dialogue) {
        self.started = true;
        dialogue.open(
            self.pages
                .iter()
                .map(|lines| MessageBox {
                    face: None,
                    face_index: 0,
                    lines: lines.clone(),
                })
                .collect(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::BattleOutcome;
    use crate::battle::model::testkit::build_1v2;

    #[test]
    fn rewards_keep_original_pauses_four_line_pages_and_separate_actor_pages() {
        let mut battle = build_1v2();
        battle.finish(BattleOutcome::Victory);
        battle
            .outcome_message
            .reward("Jégkarom megszerezve!", "\\.");
        battle.outcome_message.reward("Topáz megszerezve!", "\\.");
        battle
            .outcome_message
            .actor(vec!["Ron Sz 3 elérve!".into(), "Új képesség".into()]);
        battle
            .outcome_message
            .actor(vec!["Tiffany Sz 4 elérve!".into()]);
        let mut dialogue = Dialogue::default();
        battle.outcome_message.open(&mut dialogue);
        assert_eq!(dialogue.boxes.len(), 4);
        assert_eq!(
            dialogue.boxes[0].lines,
            [
                "Győzelem!\\|",
                "20 EXP\\.",
                "+ 60 arany\\.",
                "Jégkarom megszerezve!\\."
            ]
        );
        assert_eq!(dialogue.boxes[1].lines, ["Topáz megszerezve!\\."]);
        assert_eq!(dialogue.boxes[2].lines, ["Ron Sz 3 elérve!", "Új képesség"]);
        assert_eq!(dialogue.boxes[3].lines, ["Tiffany Sz 4 elérve!"]);
        assert!(dialogue.boxes.iter().all(|page| page.face.is_none()));
    }
}
