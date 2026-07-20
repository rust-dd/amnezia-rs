//! Agility-driven turn order and the flee chance, plus the troop reward tally.

/// Order combatant indices by `agility` descending — RM2000 acts fastest-first.
/// The stable sort keeps equal-agility combatants in their given order, so the
/// result is deterministic.
pub fn turn_order(agilities: &[u32]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..agilities.len()).collect();
    order.sort_by(|&a, &b| agilities[b].cmp(&agilities[a]));
    order
}

/// A side's average agility (RM2000 / EasyRPG `Game_Party_Base::GetAverageAgility`):
/// the integer mean of the given `agilities`, or `1` for an empty side (which also
/// guards the escape-chance division). Each side averages over its own combatants.
pub fn average_agility(agilities: &[u32]) -> u32 {
    if agilities.is_empty() {
        return 1;
    }
    agilities.iter().sum::<u32>() / agilities.len() as u32
}

/// The party's starting escape chance in percent (RM2000 / EasyRPG
/// `Scene_Battle::InitEscapeChance`): `clamp(150 - round(100 * enemy_avg_agi /
/// party_avg_agi), 0, 100)` off the two sides' AVERAGE agilities. A party as fast as
/// the foes escapes at 50%, a faster party higher and a slower one lower;
/// `party_avg_agi` is guarded to at least 1 against a divide-by-zero. RM2000 computes
/// this once at battle start, then raises it by 10 on each failed attempt (see
/// [`crate::battle::model::Battle::attempt_escape`]).
pub fn init_escape_chance(party_avg_agi: u32, enemy_avg_agi: u32) -> u32 {
    let party = party_avg_agi.max(1) as f64;
    let base = (100.0 * enemy_avg_agi as f64 / party).round() as i32;
    (150 - base).clamp(0, 100) as u32
}

/// Whether a `roll` in `0..=99` succeeds against the escape `chance` percent
/// (EasyRPG `Rand::PercentChance`: `GetRandomNumber(0, 99) < rate`).
pub fn escape_succeeds(chance: u32, roll: u32) -> bool {
    roll < chance
}

/// Sum the experience and gold from every defeated monster's `(exp, gold)`.
pub fn total_rewards(rewards: &[(u32, u32)]) -> (u32, u32) {
    rewards
        .iter()
        .fold((0, 0), |(exp, gold), &(e, g)| (exp + e, gold + g))
}
