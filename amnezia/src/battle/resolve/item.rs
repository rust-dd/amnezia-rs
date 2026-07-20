//! Item use in battle: a member consumes a medicine to restore HP/SP and cure
//! statuses on an ally (or itself).

use super::*;

impl Battle {
    /// Apply item `item_id`, used by member `pi`, to ally `target` (falling back to
    /// the user when that member is gone): restore HP and SP by the item's flat
    /// amount plus its percent-of-maximum (each clamped to the maximum) and cure
    /// each of its `cure_states` from the recipient. The item was already consumed
    /// from the inventory when the order was committed, so an unknown id or an item
    /// with no restorative effect still logs a use line.
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
        let ti = if self.members.get(target).is_some_and(|m| m.alive()) {
            target
        } else {
            pi
        };
        let (max_hp, max_sp) = (self.members[ti].max_hp, self.members[ti].max_sp);
        let hp_gain = item.recover_hp as i32 + max_hp * item.recover_hp_rate as i32 / 100;
        let sp_gain = item.recover_sp as i32 + max_sp * item.recover_sp_rate as i32 / 100;
        if hp_gain > 0 {
            self.members[ti].hp = (self.members[ti].hp + hp_gain).min(max_hp);
        }
        if sp_gain > 0 {
            self.members[ti].sp = (self.members[ti].sp + sp_gain).min(max_sp);
        }
        let shown = hp_gain.max(sp_gain);
        if shown > 0 {
            let pos = (self.party_anim_x(ti), PARTY_ANIM_Y);
            self.push_number(pos, shown.to_string(), NumberKind::Heal);
        }
        let mut cured: Vec<String> = Vec::new();
        for &sid in &item.cure_states {
            if logic::has_state(&self.members[ti].states, sid) {
                logic::cure(&mut self.members[ti].states, sid);
                if let Some(state) = self.states.iter().find(|s| s.id == sid) {
                    cured.push(state.name.clone());
                }
            }
        }
        let gain = match (hp_gain > 0, sp_gain > 0) {
            (true, true) => format!(" (+{hp_gain} HP, +{sp_gain} SP)"),
            (true, false) => format!(" (+{hp_gain} HP)"),
            (false, true) => format!(" (+{sp_gain} SP)"),
            (false, false) => String::new(),
        };
        let recipient = self.members[ti].name.clone();
        let mut lines = vec![format!("{user} használ: {}{gain}", item.name)];
        for state_name in cured {
            lines.push(format!("{recipient} gyógyul: {state_name}"));
        }
        lines.join("\n")
    }
}
