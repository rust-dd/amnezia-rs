use amnezia_data::AttributeDef;

/// Each category uses its highest multiplier; physical scaling precedes magic.
pub fn attribute_damage(
    base: i32,
    ids: &[u32],
    attributes: &[AttributeDef],
    rank: impl Fn(u32) -> u8,
) -> i32 {
    let mut physical = None;
    let mut magical = None;
    for attr in attributes.iter().filter(|attr| ids.contains(&attr.id)) {
        let rate = super::damage::attribute_percent(attr, rank(attr.id));
        let category = if attr.attribute_type == 0 {
            &mut physical
        } else {
            &mut magical
        };
        *category = Some(category.map_or(rate, |old: u32| old.max(rate)));
    }
    let effect = i64::from(base) * i64::from(physical.unwrap_or(100)) / 100;
    (effect * i64::from(magical.unwrap_or(100)) / 100).clamp(0, i64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes() -> Vec<AttributeDef> {
        [(1, 0, 125), (2, 0, 50), (5, 1, 150), (6, 1, 0)]
            .into_iter()
            .map(|(id, attribute_type, c_rate)| AttributeDef {
                id,
                name: String::new(),
                attribute_type,
                a_rate: 200,
                b_rate: 150,
                c_rate,
                d_rate: 50,
                e_rate: 0,
            })
            .collect()
    }

    #[test]
    fn combines_category_maxima_in_physical_then_magic_order() {
        let attrs = attributes();
        assert_eq!(attribute_damage(101, &[1, 2, 5, 6], &attrs, |_| 2), 189);
        assert_eq!(attribute_damage(101, &[6, 5, 2, 1], &attrs, |_| 2), 189);
        assert_eq!(attribute_damage(101, &[1, 2], &attrs, |_| 2), 126);
        assert_eq!(attribute_damage(101, &[5, 6], &attrs, |_| 2), 151);
    }

    #[test]
    fn a_single_immune_category_stays_zero_but_missing_attributes_are_neutral() {
        let attrs = attributes();
        assert_eq!(attribute_damage(100, &[6], &attrs, |_| 2), 0);
        assert_eq!(attribute_damage(100, &[1, 6], &attrs, |_| 2), 0);
        assert_eq!(attribute_damage(100, &[0, 999], &attrs, |_| 2), 100);
        assert_eq!(attribute_damage(100, &[], &attrs, |_| 2), 100);
    }
}
