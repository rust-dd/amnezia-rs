//! Agility-driven turn order and the flee chance, plus the troop reward tally.

/// Fastest first, preserving input order for ties.
pub fn turn_order(agilities: &[u32]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..agilities.len()).collect();
    order.sort_by(|&a, &b| agilities[b].cmp(&agilities[a]));
    order
}

/// EasyRPG `Game_Party_Base::GetAverageAgility`; an empty side uses 1 to avoid division by zero.
pub fn average_agility(agilities: &[u32]) -> u32 {
    if agilities.is_empty() {
        return 1;
    }
    agilities.iter().sum::<u32>() / agilities.len() as u32
}

/// EasyRPG `InitEscapeChance` uses side averages once at battle start.
/// Equal speeds give 50%; each failed attempt later adds 10 percentage points.
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
