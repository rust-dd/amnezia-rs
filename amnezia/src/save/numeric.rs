use super::SaveGame;
use crate::gamedata::GameData;
use crate::progression::{MAX_EXP, Progression};
use crate::state::{MAX_GOLD, MAX_ITEM_COUNT, MAX_VAR, MIN_VAR};

pub(super) fn prepare(game: &mut SaveGame, data: Option<&GameData>) -> bool {
    if !game.timer_remaining.is_finite() {
        return false;
    }
    let mut valid = clamp(&mut game.gold, 0, MAX_GOLD);
    for (_, count) in &mut game.items {
        valid &= clamp(count, 0, MAX_ITEM_COUNT);
    }
    for (_, value) in &mut game.variables {
        valid &= clamp(value, MIN_VAR, MAX_VAR);
    }
    for (_, experience) in &mut game.progression {
        valid &= clamp(experience, 0, MAX_EXP);
    }
    let mut progression = Progression::default();
    progression.load(game.progression.clone());
    for (id, (hp, sp)) in &mut game.vitals {
        let (max_hp, max_sp) = data
            .and_then(|data| data.actor(*id))
            .map(|actor| {
                crate::battle::logic::actor_hp_sp_at(
                    &actor.curves,
                    progression.level(actor),
                    actor.hp,
                    actor.sp,
                )
            })
            .unwrap_or((999, 999));
        valid &= clamp(hp, 0, max_hp.clamp(1, 999) as i32);
        valid &= clamp(sp, 0, max_sp.min(999) as i32);
    }
    if game.timer_remaining < 0.0 {
        game.timer_remaining = 0.0;
        valid = false;
    }
    valid || game.format_version < 14
}

fn clamp<T: Ord + Copy>(value: &mut T, min: T, max: T) -> bool {
    let before = *value;
    *value = before.clamp(min, max);
    before == *value
}
