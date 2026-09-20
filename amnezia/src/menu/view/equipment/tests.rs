use super::*;
use crate::font::bitmap::Run;
use crate::menu::testkit;

#[test]
fn clear_rect_also_erases_the_empty_unequip_cell_without_drawing_a_label() {
    let name = "g".repeat(60);
    let mut data = testkit::data();
    data.items = vec![testkit::weapon(10, &name, 4)];
    let mut inventory = Inventory::default();
    inventory.add_item(10, 1);
    for ids in [vec![10], vec![10, 0]] {
        super::super::cell_tests::assert_row(
            &text::entries(&ids, &data, &inventory),
            vec![Run::new(&name, 0, 2, 0), Run::new(":  1", 120, 2, 0)],
            Vec::new(),
            ids.len() == 2,
        );
    }
}

#[test]
fn original_equipment_scene_has_four_windows_and_only_bitmap_text() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_systems(Startup, |mut commands: Commands| {
            commands
                .spawn(Node::default())
                .with_children(|root| spawn(root, &Handle::default()));
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
            (Val::Px(0.0), Val::Px(96.0), Val::Px(372.0), Val::Px(288.0)),
            (
                Val::Px(372.0),
                Val::Px(96.0),
                Val::Px(588.0),
                Val::Px(288.0)
            ),
            (Val::Px(0.0), Val::Px(384.0), Val::Px(960.0), Val::Px(336.0)),
        ]
    );
    assert_eq!(world.query::<&PixelText>().iter(world).count(), 4);
    assert_eq!(world.query::<&Text>().iter(world).count(), 0);
    for (part, node) in world.query::<(&Part, &Node)>().iter(world) {
        let width = match part {
            Part::SlotCursor => 564.0,
            Part::ItemCursor => 456.0,
            _ => continue,
        };
        assert_eq!(
            (node.left, node.top, node.width, node.height),
            (Val::Px(12.0), Val::Px(24.0), Val::Px(width), Val::Px(48.0))
        );
    }
}

#[test]
fn slots_use_database_terms_and_blank_empty_equipment_names() {
    let mut data = testkit::data();
    data.actors[0].weapon = 10;
    data.items.push(testkit::weapon(10, "Kard", 4));
    let terms = Terms::default();
    let equipment = Equipment::default();
    let slots = text::slots(&data.actors[0], &data, &equipment, &terms);
    assert_eq!(slots.size, UVec2::new(180, 80));
    assert_eq!(slots.runs.len(), 6);
    assert_eq!(
        slots.runs[0],
        Run::new(crate::i18n::tr(&terms.0.weapon), 0, 2, 1)
    );
    assert_eq!(slots.runs[1], Run::new("Kard", 60, 2, 0));
    assert_eq!(
        slots.runs[2],
        Run::new(crate::i18n::tr(&terms.0.shield), 0, 18, 1)
    );
    data.actors[0].two_weapons = true;
    assert_eq!(
        text::slots(&data.actors[0], &data, &equipment, &terms).runs[2],
        Run::new(crate::i18n::tr(&terms.0.weapon), 0, 18, 1)
    );
}

#[test]
fn candidate_rows_draw_names_then_counts_and_leave_unequip_blank() {
    let mut data = testkit::data();
    data.items = vec![
        testkit::weapon(10, "Kard", 4),
        testkit::weapon(11, "Másik kard", 5),
        testkit::weapon(12, "Hosszú név a számláló alatt", 6),
    ];
    let mut inventory = Inventory::default();
    for (id, count) in [(10, 1), (11, 99), (12, 2)] {
        inventory.add_item(id, count);
    }
    let text = text::entries(&[10, 11, 12, 0], &data, &inventory);
    assert_eq!(text.size, UVec2::new(304, 96));
    assert_eq!(
        text.runs,
        [
            Run::clear(0, 2, 144, 12),
            Run::new("Kard", 0, 2, 0),
            Run::new(":  1", 120, 2, 0),
            Run::clear(160, 2, 144, 12),
            Run::new("Másik kard", 160, 2, 0),
            Run::new(": 99", 280, 2, 0),
            Run::clear(0, 18, 144, 12),
            Run::new("Hosszú név a számláló alatt", 0, 18, 0),
            Run::new(":  2", 120, 18, 0),
            Run::clear(160, 18, 144, 12),
        ]
    );
    assert_eq!(
        text::entries(&[0], &data, &inventory).runs,
        [Run::clear(0, 2, 144, 12)]
    );
    assert!(text::entries(&[], &data, &inventory).runs.is_empty());
}

#[test]
fn status_uses_original_stat_columns_and_comparison_colors() {
    let font = BitmapFont::from_id(0);
    let terms = Terms::default();
    let result = status(
        "Áron",
        [25, 32, 20, 25],
        Some([45, 22, 20, 1998]),
        &terms,
        &font,
    );
    assert_eq!(result.size, UVec2::new(108, 80));
    assert_eq!(result.runs[0], Run::new("Áron", 0, 2, 0));
    for (row, label, old, new, color) in [
        (0, &terms.0.attack, "25", "45", 2),
        (1, &terms.0.defense, "32", "22", 3),
        (2, &terms.0.spirit, "20", "20", 0),
        (3, &terms.0.agility, "25", "1998", 2),
    ] {
        let y = 18 + row as i32 * 16;
        assert_eq!(
            &result.runs[1 + row * 4..5 + row * 4],
            [
                Run::new(crate::i18n::tr(label), 0, y, 1),
                Run::new(old, 78 - font.width(old), y, 0),
                Run::new(">", 81, y, 1),
                Run::new(new, 108 - font.width(new), y, color),
            ]
        );
    }
    assert_eq!(status("", [1; 4], None, &terms, &font).runs.len(), 9);
}

#[test]
fn help_preserves_original_descriptions_and_clears_for_empty_equipment() {
    let mut data = testkit::data();
    let mut item = testkit::weapon(10, "Kard", 4);
    item.description = "Árvíztűrő tükörfúrógép".into();
    data.items.push(item);
    assert_eq!(
        text::help(10, &data).runs,
        [Run::new("Árvíztűrő tükörfúrógép", 0, 2, 0)]
    );
    assert_eq!(text::help(0, &data).runs, [Run::new("", 0, 2, 0)]);
}
