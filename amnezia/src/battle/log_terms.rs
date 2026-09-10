//! A battle-local vocabulary snapshot. Applying database terms preserves blanks.

use amnezia_data::TermsDef;
use std::ops::{Deref, DerefMut};

mod format;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleText(TermsDef);

impl Default for BattleText {
    fn default() -> Self {
        Self(TermsDef {
            victory: "Győzelem!".into(),
            defeat: "Vereség...".into(),
            escape_success: "Sikeres menekülés!".into(),
            escape_failure: "Menekülés sikertelen!".into(),
            exp_received: " EXP".into(),
            gold_recieved_a: "+".into(),
            gold: " arany".into(),
            item_recieved: " megszerezve!".into(),
            dodge: " kivédi a támadást".into(),
            enemy_damaged: " HP-t sebződik".into(),
            actor_damaged: " HP-t veszít".into(),
            enemy_critical: "Kritikus ütés!".into(),
            actor_critical: "Kritikus csapás!".into(),
            enemy_undamaged: " kivédi a támadást".into(),
            actor_undamaged: " félreugrik".into(),
            skill_failure_a: " félreugrik".into(),
            skill_failure_b: " védekezik".into(),
            skill_failure_c: " kivédi a támadást".into(),
            health_points: "HP".into(),
            spirit_points: "SP".into(),
            hp_recovery: " visszatért".into(),
            use_item: " használata".into(),
            attacking: " megtámadja az ellenséget".into(),
            defending: " védekezik".into(),
            observing: " csak figyel és várakozik".into(),
            focus: " erőt gyüjt".into(),
            autodestruction: " előretör".into(),
            enemy_escape: " elmenekül!".into(),
            attack: "Támadóerő".into(),
            defense: "Védőerő".into(),
            spirit: "Mentál".into(),
            agility: "Gyorsaság".into(),
            parameter_increase: " ".into(),
            ..Default::default()
        })
    }
}

impl Deref for BattleText {
    type Target = TermsDef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BattleText {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl BattleText {
    pub fn apply(&mut self, terms: &TermsDef) {
        self.0.clone_from(terms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applying_database_terms_preserves_intentionally_blank_messages() {
        let mut text = BattleText::default();
        let terms = TermsDef {
            victory: "Megnyerted a harcot!".into(),
            parameter_increase: " ".into(),
            ..Default::default()
        };
        text.apply(&terms);
        assert_eq!(*text, terms);
        assert!(text.defeat.is_empty());
        assert_eq!(text.parameter_increase, " ");
    }
}
