use super::*;
use crate::font::bitmap::{DEFAULT, DISABLED, Run};
use crate::menu::testkit;
use amnezia_data::Learning;

#[test]
fn clear_rect_removes_left_skill_overlap_without_clipping_the_missing_next_cell() {
    let name = "g".repeat(60);
    let mut data = testkit::data();
    data.skills = vec![
        testkit::heal_skill(2, &name, 1, 1),
        testkit::heal_skill(3, "", 1, 1),
    ];
    for right in [false, true] {
        data.actors[0].learnings = (2..=if right { 3 } else { 2 })
            .map(|skill_id| Learning { level: 1, skill_id })
            .collect();
        super::super::cell_tests::assert_row(
            &text::entries(
                0,
                &data,
                &Party::default(),
                &Progression::default(),
                &Vitals::default(),
                &Equipment::default(),
            ),
            vec![
                Run::new("-  1", 120, 2, DEFAULT),
                Run::new(&name, 0, 2, DEFAULT),
            ],
            if right {
                vec![Run::new("-  1", 280, 2, DEFAULT)]
            } else {
                Vec::new()
            },
            right,
        );
    }
}

#[test]
fn original_skill_scene_has_help_status_and_a_ten_row_list() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Startup, |mut commands: Commands| {
            commands
                .spawn(Node::default())
                .with_children(|panel| spawn(panel, &Handle::default()));
        });
    app.update();
    let world = app.world_mut();
    let (_, children) = world
        .query::<(&Part, &Children)>()
        .iter(world)
        .find(|(part, _)| matches!(part, Part::Root))
        .unwrap();
    let rectangles = children
        .iter()
        .map(|entity| {
            let node = world.get::<Node>(entity).unwrap();
            (node.left, node.top, node.width, node.height)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rectangles,
        [
            (Val::Px(0.0), Val::Px(0.0), Val::Px(960.0), Val::Px(96.0)),
            (Val::Px(0.0), Val::Px(96.0), Val::Px(960.0), Val::Px(96.0)),
            (Val::Px(0.0), Val::Px(192.0), Val::Px(960.0), Val::Px(528.0)),
        ]
    );
    assert_eq!(world.query::<&PixelText>().iter(world).count(), 3);
    assert_eq!(world.query::<&Text>().iter(world).count(), 0);
    let (_, cursor) = world
        .query::<(&Part, &Node)>()
        .iter(world)
        .find(|(part, _)| matches!(part, Part::Cursor))
        .unwrap();
    assert_eq!(
        (cursor.left, cursor.top, cursor.width, cursor.height),
        (Val::Px(12.0), Val::Px(24.0), Val::Px(456.0), Val::Px(48.0))
    );
}

#[test]
fn skill_rows_use_the_original_cost_grid_palette_and_draw_order() {
    let mut data = testkit::data();
    data.skills = vec![
        testkit::heal_skill(2, "Gyógyítás", 8, 40),
        testkit::skill(3, "Csapás", 12),
        testkit::heal_skill(4, "Nagy gyógyítás", 40, 99),
    ];
    data.actors[0].learnings = (2..=4)
        .map(|skill_id| Learning { level: 1, skill_id })
        .collect();
    let text = text::entries(
        0,
        &data,
        &Party::default(),
        &Progression::default(),
        &Vitals::default(),
        &Equipment::default(),
    );
    assert_eq!(text.size, UVec2::new(304, 160));
    assert_eq!(
        text.runs,
        [
            Run::clear(0, 2, 144, 12),
            Run::new("-  8", 120, 2, DEFAULT),
            Run::new("Gyógyítás", 0, 2, DEFAULT),
            Run::clear(160, 2, 144, 12),
            Run::new("- 12", 280, 2, DISABLED),
            Run::new("Csapás", 160, 2, DISABLED),
            Run::clear(0, 18, 144, 12),
            Run::new("- 40", 120, 18, DISABLED),
            Run::new("Nagy gyógyítás", 0, 18, DISABLED),
        ]
    );
}

#[test]
fn half_cost_updates_both_the_printed_number_and_availability() {
    let mut data = testkit::data();
    data.items = crate::assets::load_ron(&format!("{}/items.ron", crate::assets::asset_root()));
    data.skills = vec![testkit::heal_skill(2, "Gyógyítás", 9, 40)];
    data.actors[0].learnings = vec![Learning {
        level: 1,
        skill_id: 2,
    }];
    let mut equipment = Equipment::default();
    equipment.set_slot(&data.actors[0], 1, 152);
    let mut vitals = Vitals::default();
    for (sp, color) in [(4, DISABLED), (5, DEFAULT)] {
        vitals.set(1, 20, sp);
        let text = text::entries(
            0,
            &data,
            &Party::default(),
            &Progression::default(),
            &vitals,
            &equipment,
        );
        assert_eq!(text.runs[1], Run::new("-  5", 120, 2, color));
        assert_eq!(text.runs[2].color, color);
    }
}

#[test]
fn no_skills_is_blank_and_descriptions_are_unmodified_database_text() {
    let mut data = testkit::data();
    data.actors[0].learnings.clear();
    let party = Party::default();
    let progression = Progression::default();
    let entries = text::entries(
        0,
        &data,
        &party,
        &progression,
        &Vitals::default(),
        &Equipment::default(),
    );
    assert!(entries.runs.is_empty());
    assert_eq!(entries.size, UVec2::new(304, 160));
    assert_eq!(
        text::help(0, 0, &data, &party, &progression).runs,
        [Run::new("", 0, 2, DEFAULT)]
    );
    data.skills[0].description = "Árvíztűrő leírás".into();
    data.actors[0].learnings.push(Learning {
        level: 1,
        skill_id: data.skills[0].id,
    });
    assert_eq!(
        text::help(0, 0, &data, &party, &progression).runs,
        [Run::new("Árvíztűrő leírás", 0, 2, DEFAULT)]
    );
}

#[test]
fn skill_status_uses_original_columns_colors_and_the_live_hero_name() {
    let mut vitals = Vitals::default();
    vitals.set(1, 7, 0);
    vitals.set_states(1, vec![2]);
    let row = render::members(
        &HeroName("Áron".into()),
        &testkit::data(),
        &Party::default(),
        &Progression::default(),
        &vitals,
    )
    .remove(0);
    let result = status(&row, &Terms::default(), &BitmapFont::from_id(0));
    assert_eq!(result.size, UVec2::new(304, 16));
    assert_eq!(result.runs[0], Run::new("Áron", 0, 2, DEFAULT));
    assert_eq!(result.runs[1].position, IVec2::new(80, 2));
    assert_eq!(result.runs[2], Run::new("2", 98, 2, DEFAULT));
    assert_eq!(
        result.runs[3],
        Run::new("Méreg", 124, 2, row.condition_color.unwrap())
    );
    assert_eq!(result.runs[4].position, IVec2::new(184, 2));
    assert_eq!(
        result.runs[5],
        Run::new("7", 208, 2, crate::font::bitmap::CRITICAL)
    );
    assert_eq!(result.runs[6], Run::new("/", 214, 2, DEFAULT));
    assert_eq!(result.runs[7], Run::new("63", 226, 2, DEFAULT));
    assert_eq!(result.runs[8].position, IVec2::new(250, 2));
    assert_eq!(
        result.runs[9],
        Run::new("0", 274, 2, crate::font::bitmap::CRITICAL)
    );
    assert_eq!(result.runs[10], Run::new("/", 280, 2, DEFAULT));
    assert_eq!(result.runs[11], Run::new("37", 292, 2, DEFAULT));
}

#[test]
fn every_original_skill_keeps_its_name_cost_description_and_field_palette() {
    let mut data = testkit::data();
    data.skills = crate::assets::load_ron(&format!("{}/skills.ron", crate::assets::asset_root()));
    let mut progression = Progression::default();
    progression.load_skills(vec![(
        1,
        data.skills.iter().map(|skill| skill.id).collect(),
    )]);
    let mut vitals = Vitals::default();
    vitals.set(1, 63, 999);
    let party = Party::default();
    let original_order = data.skills.iter().map(|skill| skill.id).collect::<Vec<_>>();
    data.skills.reverse();
    let known = skills::known_skills(0, &data, &party, &progression);
    assert_eq!(
        known.iter().map(|skill| skill.id).collect::<Vec<_>>(),
        original_order
    );
    let entries = text::entries(
        0,
        &data,
        &party,
        &progression,
        &vitals,
        &Equipment::default(),
    );
    assert_eq!(entries.runs.len(), 210);
    for (index, skill) in known.iter().enumerate() {
        let color = if [
            7, 8, 9, 10, 11, 33, 34, 35, 37, 38, 39, 40, 46, 47, 49, 52, 53,
        ]
        .contains(&skill.id)
        {
            DEFAULT
        } else {
            DISABLED
        };
        assert_eq!(
            entries.runs[index * 3 + 1].text,
            format!("-{:>3}", skill.sp_cost)
        );
        assert_eq!(entries.runs[index * 3 + 2].text, skill.name);
        assert_eq!(entries.runs[index * 3 + 2].color, color, "{}", skill.id);
        assert_eq!(
            text::help(0, index, &data, &party, &progression).runs[0].text,
            skill.description
        );
    }
}
