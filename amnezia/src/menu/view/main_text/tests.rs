use super::*;

fn actor() -> render::MemberView {
    render::members(
        &crate::text::HeroName("Ron".into()),
        &crate::menu::testkit::data(),
        &crate::state::Party::default(),
        &crate::progression::Progression::default(),
        &crate::vitals::Vitals::default(),
    )
    .remove(0)
}

#[test]
fn current_vitals_have_separate_labels_slashes_and_maximum_palettes() {
    let mut actor = actor();
    actor.hp = 15;
    actor.sp = 0;
    let font = BitmapFont::from_id(0);
    let mut terms = Terms::default();
    terms.0.hp_short = "HP".into();
    terms.0.sp_short = "SP".into();
    let hp = member(&actor, MemberField::Hp, &terms, &font);
    assert_eq!(
        hp.iter()
            .map(|run| (run.text.as_str(), run.position.x, run.color))
            .collect::<Vec<_>>(),
        vec![
            ("HP", 0, 1),
            ("15", 18, CRITICAL),
            ("/", 30, DEFAULT),
            ("63", 42, DEFAULT)
        ]
    );
    let sp = member(&actor, MemberField::Sp, &terms, &font);
    assert_eq!(sp[1].color, CRITICAL);
    actor.hp = 0;
    assert_eq!(
        member(&actor, MemberField::Hp, &terms, &font)[1].color,
        KNOCKOUT
    );
}

#[test]
fn currency_has_its_own_palette_and_no_invented_separator_or_blank_term() {
    let font = BitmapFont::from_id(0);
    let mut terms = Terms::default();
    terms.0.gold = "GP".into();
    let runs = gold(250, &terms, &font);
    assert_eq!(
        runs,
        vec![Run::new("250", 42, 0, DEFAULT), Run::new("GP", 60, 0, 1)]
    );
    terms.0.gold.clear();
    assert_eq!(
        gold(250, &terms, &font),
        vec![Run::new("250", 54, 0, DEFAULT), Run::new("", 72, 0, 1)]
    );
}

#[test]
fn state_colors_and_empty_knockout_are_not_replaced_by_normal_text() {
    let mut actor = actor();
    let font = BitmapFont::from_id(0);
    let mut terms = Terms::default();
    terms.0.normal_status = "Ready".into();
    assert_eq!(
        member(&actor, MemberField::Condition, &terms, &font),
        vec![Run::new("Ready", 0, 0, DEFAULT)]
    );
    actor.condition = "Vakság".into();
    actor.condition_color = Some(10);
    assert_eq!(
        member(&actor, MemberField::Condition, &terms, &font),
        vec![Run::new("Vakság", 0, 0, 10)]
    );
    actor.condition.clear();
    actor.condition_color = Some(3);
    assert_eq!(
        member(&actor, MemberField::Condition, &terms, &font),
        vec![Run::new("", 0, 0, 3)]
    );
}

#[test]
fn experience_columns_and_max_level_dashes_match_rpg_rt_not_the_player_customization() {
    let mut actor = actor();
    let font = BitmapFont::from_id(0);
    let mut terms = Terms::default();
    terms.0.exp_short = "E".into();
    actor.exp = Some((30, 68));
    assert_eq!(
        member(&actor, MemberField::Exp, &terms, &font),
        vec![
            Run::new("E", 0, 0, 1),
            Run::new("    30/    68", 12, 0, DEFAULT)
        ]
    );
    actor.exp = None;
    assert_eq!(
        member(&actor, MemberField::Exp, &terms, &font)[1].text,
        "------/------"
    );
}

#[test]
fn disabled_save_and_empty_party_commands_use_the_disabled_palette() {
    for index in 0..3 {
        assert_eq!(command_color(index, 0, true), DISABLED);
    }
    assert_eq!(command_color(3, 4, false), DISABLED);
    assert_eq!(command_color(3, 0, true), DEFAULT);
    assert_eq!(command_color(4, 0, false), DEFAULT);
}

#[test]
fn three_and_four_digit_maxima_fit_their_original_vital_columns() {
    let font = BitmapFont::from_id(0);
    let terms = Terms::default();
    let mut actor = actor();
    for maximum in [116, 999, 9999] {
        actor.max_hp = maximum;
        actor.hp = maximum;
        let width = vital_width(&actor);
        assert_eq!(width, if maximum <= 999 { 54 } else { 66 });
        for run in member(&actor, MemberField::Hp, &terms, &font) {
            assert!(run.position.x >= 0);
            assert!(run.position.x + font.width(&run.text) <= width as i32);
        }
    }
}
