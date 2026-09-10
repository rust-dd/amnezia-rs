use crate::battle::model::Fighter;
use crate::font::bitmap::{BitmapFont, CRITICAL, DEFAULT, KNOCKOUT, Run};
use crate::i18n;
use crate::terms::Terms;
use amnezia_data::StateDef;

fn value_color(current: i32, maximum: i32, hp: bool) -> u32 {
    if hp && current == 0 {
        KNOCKOUT
    } else if maximum > 0 && current <= maximum / 4 {
        CRITICAL
    } else {
        DEFAULT
    }
}

pub(super) fn runs(
    member: &Fighter,
    states: &[StateDef],
    terms: &Terms,
    font: &BitmapFont,
) -> Vec<Run> {
    let hp_digits = if member.max_hp > 999 { 4 } else { 3 };
    let sp_digits = if member.max_sp.max(member.sp) > 999 {
        4
    } else {
        3
    };
    let state = states
        .iter()
        .filter(|state| {
            (state.id == 1 && !member.alive())
                || member.states.iter().any(|(id, _)| *id == state.id)
        })
        .max_by_key(|state| (state.id == 1, state.priority, state.id));
    let (state_name, color) = state.map_or((terms.0.normal_status.as_str(), DEFAULT), |s| {
        (s.name.as_str(), s.color)
    });
    let hp = 178 - hp_digits * 6 - sp_digits * 6;
    let sp = 220 - sp_digits * 6;
    let right = |value: i32, x: i32, color| {
        let text = value.to_string();
        Run::new(text.clone(), x - font.width(&text), 0, color)
    };
    vec![
        Run::new(i18n::tr(&member.name), 4, 0, DEFAULT),
        Run::new(
            i18n::tr(state_name),
            if hp_digits == 3 && sp_digits == 3 {
                86
            } else {
                80
            },
            0,
            color,
        ),
        Run::new(i18n::tr(&terms.0.hp_short), hp, 0, 1),
        right(
            member.hp.max(0),
            hp + 12 + hp_digits * 6,
            value_color(member.hp.max(0), member.max_hp, true),
        ),
        Run::new("/", hp + 12 + hp_digits * 6, 0, DEFAULT),
        right(member.max_hp, hp + 18 + hp_digits * 12, DEFAULT),
        Run::new(i18n::tr(&terms.0.sp_short), sp, 0, 1),
        right(
            member.sp.max(0),
            sp + 12 + sp_digits * 6,
            value_color(member.sp.max(0), member.max_sp, false),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::model::testkit::build_party2;

    #[test]
    fn original_status_columns_have_blank_normal_state_and_distinct_hp_sp_colors() {
        let font = BitmapFont::from_id(0);
        let terms = Terms(crate::assets::load_ron(&format!(
            "{}/terms.ron",
            crate::assets::asset_root()
        )));
        let states = crate::assets::load_ron::<Vec<StateDef>>(&format!(
            "{}/states.ron",
            crate::assets::asset_root()
        ));
        let mut battle = build_party2();
        let actor = &mut battle.members[0];
        actor.hp = 25;
        actor.max_hp = 100;
        actor.sp = 0;
        actor.max_sp = 10;
        let row = runs(actor, &states, &terms, &font);
        assert!(row[1].text.is_empty());
        assert_eq!(row[1].position.x, 86);
        assert_eq!((row[2].position.x, row[2].color), (142, 1));
        assert_eq!((row[3].position.x, row[3].color), (160, CRITICAL));
        assert_eq!(row[4].position.x, 172);
        assert_eq!((row[6].position.x, row[7].color), (202, CRITICAL));
        actor.hp = 0;
        let row = runs(actor, &states, &terms, &font);
        assert_eq!(row[1].color, 3);
        assert!(row[1].text.is_empty());
        assert_eq!(row[3].color, KNOCKOUT);
        actor.hp = 26;
        actor.states = vec![(2, 0), (3, 0)];
        let row = runs(actor, &states, &terms, &font);
        assert_eq!((row[1].text.as_str(), row[1].color), ("Vakság", 10));
        assert_eq!(row[3].color, DEFAULT);
        assert_eq!(value_color(0, 0, false), DEFAULT);
    }

    #[test]
    fn four_digit_status_values_fit_without_an_sp_maximum() {
        let font = BitmapFont::from_id(0);
        let mut battle = build_party2();
        let actor = &mut battle.members[0];
        actor.hp = 9999;
        actor.max_hp = 9999;
        actor.sp = 9999;
        actor.max_sp = 9999;
        let row = runs(actor, &[], &Terms::default(), &font);
        assert_eq!(row.iter().filter(|r| r.text == "9999").count(), 3);
        assert!(
            row.iter()
                .all(|run| run.position.x + font.width(&run.text) <= 236)
        );
    }
}
