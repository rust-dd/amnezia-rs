use super::BattleText;

impl BattleText {
    pub(in crate::battle) fn damaged(&self, target: &str, ally: bool, amount: i32) -> String {
        let term = match (ally, amount == 0) {
            (true, true) => &self.actor_undamaged,
            (true, false) => &self.actor_damaged,
            (false, true) => &self.enemy_undamaged,
            (false, false) => &self.enemy_damaged,
        };
        if amount == 0 {
            format!("{target}{term}")
        } else {
            format!("{target} {amount}{term}")
        }
    }

    pub(in crate::battle) fn skill_failed(&self, target: &str, kind: u32) -> String {
        let term = match kind {
            0 => &self.skill_failure_a,
            1 => &self.skill_failure_b,
            2 => &self.skill_failure_c,
            3 => &self.dodge,
            _ => "",
        };
        format!("{target}{term}")
    }

    pub(in crate::battle) fn recovered(&self, target: &str, hp: bool, amount: i32) -> String {
        let points = if hp {
            &self.health_points
        } else {
            &self.spirit_points
        };
        format!("{target} {points} {amount}{}", self.hp_recovery)
    }

    pub(in crate::battle) fn absorbed(
        &self,
        target: &str,
        ally: bool,
        hp: bool,
        amount: i32,
    ) -> String {
        let points = if hp {
            &self.health_points
        } else {
            &self.spirit_points
        };
        let term = if ally {
            &self.actor_hp_absorbed
        } else {
            &self.enemy_hp_absorbed
        };
        format!("{target} {points} {amount}{term}")
    }

    pub(in crate::battle) fn parameter_changed(
        &self,
        target: &str,
        points: &str,
        delta: i32,
    ) -> String {
        if delta == 0 {
            return String::new();
        }
        let term = if delta > 0 {
            &self.parameter_increase
        } else {
            &self.parameter_decrease
        };
        format!("{target} {points} {}{term}", delta.unsigned_abs())
    }
}
