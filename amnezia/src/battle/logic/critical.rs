/// The reference adds the weapon bonus to a float base rate before truncating
/// to an integer percentage. A zero denominator is treated as no base chance.
pub fn critical_chance(base_denominator: Option<u32>, weapon_bonus: u32) -> u32 {
    let base = base_denominator
        .filter(|value| *value > 0)
        .map_or(0.0, |value| 1.0_f32 / value as f32);
    (f64::from(base + weapon_bonus as f32 / 100.0) * 100.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_base_and_weapon_bonus_are_added_before_truncation() {
        assert_eq!(critical_chance(Some(30), 0), 3);
        assert_eq!(critical_chance(Some(20), 0), 5);
        assert_eq!(critical_chance(Some(50), 0), 1);
        assert_eq!(critical_chance(Some(30), 5), 8);
        assert_eq!(critical_chance(Some(20), 5), 10);
        assert_eq!(critical_chance(None, 5), 5);
        assert_eq!(critical_chance(Some(0), 5), 5);
    }
}
