use super::*;

fn request(app: &mut App, slot: AnimationSlot, anim_id: u32, x: f32) {
    app.world_mut().write_message(PlayAnimation {
        map_target: None,
        slot,
        anim_id,
        targets: vec![AnimAnchor {
            pos: Vec2::new(x, 0.0),
            height: 24.0,
        }],
        screen_center: Vec2::ZERO,
        global: false,
        sound_only: slot == AnimationSlot::Party,
    });
}

fn live(app: &mut App) -> Vec<(Entity, AnimationSlot)> {
    app.world_mut()
        .query::<(Entity, &LiveAnimation)>()
        .iter(app.world())
        .map(|(e, a)| (e, a.slot))
        .collect()
}

#[test]
fn a_new_map_animation_replaces_the_old_cast_and_all_its_cells() {
    let mut app = app(60);
    let old = live(&mut app)[0].0;
    let cells = app
        .world_mut()
        .query_filtered::<Entity, With<MeshMaterial2d<cells::CellMaterial>>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 1);
    request(&mut app, AnimationSlot::Map, 1, 32.0);
    app.update();
    assert_eq!(app.world().resource::<ActiveAnimations>().0, 1);
    assert!(app.world().get_entity(old).is_err());
    for cell in cells {
        assert!(app.world().get_entity(cell).is_err());
    }
    let mut transforms = app
        .world_mut()
        .query_filtered::<&Transform, With<MeshMaterial2d<cells::CellMaterial>>>();
    assert_eq!(transforms.single(app.world()).unwrap().translation.x, 32.0);
}

#[test]
fn only_the_last_valid_request_per_slot_is_started_in_one_update() {
    let mut app = app(60);
    request(&mut app, AnimationSlot::Map, 1, 10.0);
    request(&mut app, AnimationSlot::Map, 1, 20.0);
    request(&mut app, AnimationSlot::Map, u32::MAX, 30.0);
    app.update();
    assert_eq!(live(&mut app).len(), 1);
    let mut transforms = app
        .world_mut()
        .query_filtered::<&Transform, With<MeshMaterial2d<cells::CellMaterial>>>();
    assert_eq!(transforms.single(app.world()).unwrap().translation.x, 20.0);
    let sounds = app.world().resource::<Messages<AudioRequest>>();
    assert_eq!(sounds.get_cursor().read(sounds).count(), 0);
}

#[test]
fn replacing_one_battle_side_keeps_the_other_side_and_map_cast_alive() {
    let mut app = app(60);
    let map = live(&mut app)[0].0;
    request(&mut app, AnimationSlot::Party, 1, 0.0);
    request(&mut app, AnimationSlot::Enemies, 1, 48.0);
    app.update();
    let party = live(&mut app)
        .iter()
        .find(|(_, slot)| *slot == AnimationSlot::Party)
        .unwrap()
        .0;
    let old_enemy = live(&mut app)
        .iter()
        .find(|(_, slot)| *slot == AnimationSlot::Enemies)
        .unwrap()
        .0;
    request(&mut app, AnimationSlot::Enemies, 1, -48.0);
    app.update();
    assert_eq!(app.world().resource::<ActiveAnimations>().0, 3);
    assert!(app.world().get::<LiveAnimation>(map).is_some());
    assert!(app.world().get::<LiveAnimation>(party).is_some());
    assert!(app.world().get_entity(old_enemy).is_err());
}

#[test]
fn an_empty_valid_animation_replaces_the_slot_without_leaving_old_cells() {
    let mut app = app(60);
    let mut empty = app.world().resource::<AnimationLibrary>().0[0].clone();
    empty.id = 2;
    empty.frames.clear();
    app.world_mut()
        .resource_mut::<AnimationLibrary>()
        .0
        .push(empty);
    request(&mut app, AnimationSlot::Map, 2, 0.0);
    app.update();
    assert_eq!(app.world().resource::<ActiveAnimations>().0, 0);
    assert_eq!(
        app.world_mut()
            .query::<&MeshMaterial2d<cells::CellMaterial>>()
            .iter(app.world())
            .count(),
        0
    );
}

#[test]
fn an_unknown_animation_id_does_not_cancel_the_current_cast() {
    let mut app = app(60);
    let old = live(&mut app)[0].0;
    request(&mut app, AnimationSlot::Map, u32::MAX, 0.0);
    app.update();
    assert_eq!(live(&mut app), [(old, AnimationSlot::Map)]);
}
