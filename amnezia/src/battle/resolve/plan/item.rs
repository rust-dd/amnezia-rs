use super::*;
use amnezia_data::ItemDef;

impl Battle {
    pub(in crate::battle::resolve) fn plan_item(
        &self,
        target: Source,
        item: &ItemDef,
    ) -> TargetPlan {
        let mut plan = TargetPlan {
            success: true,
            positive: true,
            ..Default::default()
        };
        let dead = self.battler_hp(target) <= 0;
        if item.ko_only && !dead {
            return plan;
        }
        if let Source::Party(i) = target
            && !item.usable_by_actor(self.members[i].actor_id)
        {
            return plan;
        }
        let states = self.planning_states(target);
        for &id in &item.cure_states {
            if logic::has_state(&states, id) {
                plan.states.push(StateChange {
                    id,
                    effect: StateEffect::Healed,
                });
                plan.revived |= id == 1 && dead;
            }
        }
        let (max_hp, _, max_sp) = self.pools(target);
        if (item.recover_hp != 0 || item.recover_hp_rate != 0) && (!dead || plan.revived) {
            plan.hp = Some(item.recover_hp as i32 + max_hp * item.recover_hp_rate as i32 / 100);
        }
        plan.sp = item.recover_sp as i32 + max_sp * item.recover_sp_rate as i32 / 100;
        plan
    }
}
