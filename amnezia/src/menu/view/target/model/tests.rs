use super::*;
use crate::menu::testkit;

#[test]
fn item_and_skill_scopes_select_one_fixed_or_all_party_rows() {
    let mut data = testkit::data();
    for id in 2..=4 {
        let mut actor = data.actors[0].clone();
        actor.id = id;
        data.actors.push(actor);
    }
    data.skills = vec![testkit::heal_skill(2, "Gyógyítás", 9, 10)];
    let mut party = Party::default();
    party.restore(vec![1, 2, 3, 4]);
    let mut inventory = Inventory::default();
    inventory.add_item(testkit::ITEM_HERB, 12);
    let equipment = Equipment::default();
    let item = MenuScreen::ItemTarget {
        item_id: testkit::ITEM_HERB,
        cursor: 3,
    };
    let selection = Selection::new(item, &data, &party, &inventory, &equipment).unwrap();
    assert_eq!(selection.value, 12);
    assert_eq!(selection.cursor(4), Some((182, 48)));
    assert_eq!(selection.cursor(0), None);
    data.items[0].scope = 1;
    assert_eq!(
        Selection::new(item, &data, &party, &inventory, &equipment)
            .unwrap()
            .cursor(4),
        Some((8, 222))
    );
    let skill = MenuScreen::SkillTarget {
        member: 2,
        skill_id: 2,
        cursor: 0,
    };
    for (scope, expected) in [(3, (8, 48)), (2, (124, 48)), (4, (8, 222))] {
        data.skills[0].scope = scope;
        assert_eq!(
            Selection::new(skill, &data, &party, &inventory, &equipment)
                .unwrap()
                .cursor(4),
            Some(expected)
        );
    }
}

#[test]
fn displayed_cost_uses_the_casters_original_half_cost_equipment() {
    let mut data = testkit::data();
    data.skills = vec![testkit::heal_skill(2, "Gyógyítás", 9, 10)];
    data.items = crate::assets::load_ron(&format!("{}/items.ron", crate::assets::asset_root()));
    let mut equipment = Equipment::default();
    equipment.set_slot(&data.actors[0], 1, 152);
    equipment.set_slot(&data.actors[0], 2, 157);
    let selection = Selection::new(
        MenuScreen::SkillTarget {
            member: 0,
            skill_id: 2,
            cursor: 0,
        },
        &data,
        &Party::default(),
        &Inventory::default(),
        &equipment,
    )
    .unwrap();
    assert_eq!(selection.value, 5);
}

#[test]
fn target_cursor_has_the_same_twenty_one_frame_cycle_at_every_render_rate() {
    for fps in [15, 30, 60, 120, 144] {
        let mut clock = Clock::default();
        let mut frames = crate::timing::GameFrames::default();
        clock.advance(0, Some(Key::Item(5)), false);
        for _ in 0..fps {
            frames.advance(1.0 / fps as f64);
            clock.advance(frames.frame, Some(Key::Item(5)), false);
        }
        assert_eq!(clock.phase, 18, "{fps} FPS");
    }
}

#[test]
fn cursor_freezes_without_catch_up_and_resets_only_for_a_new_target_scene() {
    let mut clock = Clock::default();
    let key = Some(Key::Item(5));
    clock.advance(u32::MAX - 1, key, false);
    clock.advance(0, key, false);
    assert_eq!(clock.phase, 2);
    clock.advance(0, key, false);
    assert_eq!(clock.phase, 2);
    clock.advance(500, key, true);
    clock.advance(501, key, false);
    assert_eq!(clock.phase, 3);
    clock.advance(502, Some(Key::Skill(0, 2)), false);
    assert_eq!(clock.phase, 0);
    clock.advance(513, Some(Key::Skill(0, 2)), false);
    assert_eq!(clock.phase, 11);
    clock.advance(514, None, false);
    clock.advance(520, key, false);
    assert_eq!(clock.phase, 0);
}
