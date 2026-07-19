//! The real RM2000 message terms the battle-end log lines are built from —
//! victory / defeat / escape outcomes and the experience / gold reward lines.
//! Captured onto the [`super::model::Battle`] at battle start (from the loaded
//! [`crate::terms::Terms`]) so the resolution code, which has no Bevy resources,
//! can compose faithful lines. Each field keeps its invented Hungarian default
//! until a non-blank real term overrides it, so a missing `terms.ron` still reads
//! naturally and the model's unit tests (which never apply terms) are unchanged.
//!
//! The reward lines follow the RM2000 (2000, non-placeholder) concatenation
//! order from EasyRPG `game_message_terms.cpp`: experience is `<value><exp_received>`
//! and gold is `<gold_received_a> <value><gold><gold_received_b>`.

use amnezia_data::TermsDef;

/// The battle-end message terms, defaulting to the faithful Hungarian
/// placeholders the log shipped with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleText {
    pub victory: String,
    pub defeat: String,
    pub escape_success: String,
    pub escape_failure: String,
    pub exp_received: String,
    pub gold_received_a: String,
    pub gold: String,
    pub gold_received_b: String,
}

impl Default for BattleText {
    fn default() -> Self {
        Self {
            victory: "Győzelem!".to_string(),
            defeat: "Vereség...".to_string(),
            escape_success: "Sikeres menekülés!".to_string(),
            escape_failure: "Menekülés sikertelen!".to_string(),
            exp_received: " EXP".to_string(),
            gold_received_a: "+".to_string(),
            gold: " arany".to_string(),
            gold_received_b: String::new(),
        }
    }
}

impl BattleText {
    /// Override each message with its real RM2000 term when the parsed term is
    /// non-blank, leaving the Hungarian default in place otherwise.
    pub fn apply(&mut self, t: &TermsDef) {
        set(&mut self.victory, &t.victory);
        set(&mut self.defeat, &t.defeat);
        set(&mut self.escape_success, &t.escape_success);
        set(&mut self.escape_failure, &t.escape_failure);
        set(&mut self.exp_received, &t.exp_received);
        set(&mut self.gold_received_a, &t.gold_recieved_a);
        set(&mut self.gold, &t.gold);
        set(&mut self.gold_received_b, &t.gold_recieved_b);
    }
}

/// Replace `dst` with `src` only when `src` carries a term (the original database
/// leaves some terms blank; those keep the Hungarian default).
fn set(dst: &mut String, src: &str) {
    if !src.is_empty() {
        *dst = src.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_overrides_only_non_blank_terms() {
        let mut text = BattleText::default();
        // defeat left blank -> keeps the Hungarian default.
        let terms = TermsDef {
            victory: "Megnyerted a harcot!".into(),
            ..Default::default()
        };
        text.apply(&terms);
        assert_eq!(text.victory, "Megnyerted a harcot!");
        assert_eq!(text.defeat, "Vereség...");
    }
}
