//! Restorative battle items, including revival and party-wide medicine.

use super::*;

impl Battle {
    /// Effects stay on the selected recipient even if that actor is knocked out
    /// before the user's turn. The resolution driver has consumed the item.
    pub(in crate::battle::resolve) fn apply_item(
        &mut self,
        pi: usize,
        item_id: u32,
        target: usize,
    ) -> String {
        let user = self.members[pi].name.clone();
        let Some(item) = self.items.iter().find(|i| i.id == item_id).cloned() else {
            return format!("{user} használ");
        };
        let targets = if item.scope == 1 {
            (0..self.members.len()).collect::<Vec<_>>()
        } else if target < self.members.len() {
            vec![target]
        } else {
            Vec::new()
        };
        let mut lines = vec![format!("{user} használ: {}", item.name)];
        for ti in targets {
            lines.extend(self.restore_with_item(ti, &item));
        }
        lines.join("\n")
    }

    fn restore_with_item(&mut self, ti: usize, item: &amnezia_data::ItemDef) -> Vec<String> {
        let old_restriction = self.state_restriction(Source::Party(ti));
        let member = &mut self.members[ti];
        let was_dead = !member.alive();
        if item.ko_only && !was_dead {
            return Vec::new();
        }
        let revives = item.cure_states.contains(&1);
        let old_hp = member.hp.max(0);
        let old_sp = member.sp;
        if !was_dead || revives {
            let gain = item.recover_hp as i32 + member.max_hp * item.recover_hp_rate as i32 / 100;
            member.hp = (old_hp + gain).min(member.max_hp);
            if was_dead {
                member.hp = member.hp.max(1);
            }
        }
        let sp_gain = item.recover_sp as i32 + member.max_sp * item.recover_sp_rate as i32 / 100;
        member.sp = (old_sp + sp_gain).min(member.max_sp);
        let hp_gain = (member.hp - old_hp).max(0);
        let sp_gain = member.sp - old_sp;
        let recipient = member.name.clone();
        let mut lines = Vec::new();
        for &sid in &item.cure_states {
            if logic::has_state(&member.states, sid) || (sid == 1 && was_dead) {
                logic::cure(&mut member.states, sid);
                if let Some(state) = self.states.iter().find(|s| s.id == sid) {
                    lines.push(format!("{recipient} gyógyul: {}", state.name));
                }
            }
        }
        let gain = match (hp_gain > 0, sp_gain > 0) {
            (true, true) => format!(" (+{hp_gain} HP, +{sp_gain} SP)"),
            (true, false) => format!(" (+{hp_gain} HP)"),
            (false, true) => format!(" (+{sp_gain} SP)"),
            (false, false) => String::new(),
        };
        if !gain.is_empty() {
            lines.insert(0, format!("{recipient}{gain}"));
        }
        let shown = hp_gain.max(sp_gain);
        self.states_changed(Source::Party(ti), old_restriction, false);
        if shown > 0 {
            let pos = (self.party_anim_x(ti), PARTY_ANIM_Y);
            self.report_hit(pos, shown.to_string(), HitKind::Heal);
        }
        lines
    }
}
