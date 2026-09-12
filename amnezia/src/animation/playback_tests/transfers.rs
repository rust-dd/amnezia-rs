use super::*;
use crate::world::{MapChanged, MapRebuilt};

#[test]
fn a_real_same_map_teleport_keeps_the_animation_frozen_but_visible() {
    let (mut app, hero, _) = super::map_targets::map_app(60);
    app.add_plugins((
        crate::transitions::TransitionPlugin,
        crate::teleport::TeleportPlugin,
    ))
    .init_resource::<crate::state::Switches>()
    .init_resource::<crate::state::Variables>()
    .init_resource::<crate::state::Party>()
    .init_resource::<crate::state::Inventory>()
    .init_resource::<crate::player::CameraPan>()
    .init_resource::<crate::world::MapEvents>()
    .configure_sets(
        Update,
        crate::teleport::MapTransfer.before(playback::clear_map_animations),
    );
    let mut map = crate::world::MapData::for_test(20, 15);
    map.map_id = 3;
    app.insert_resource(map);
    app.world_mut().entity_mut(hero).insert((
        crate::world::MoveQueue::default(),
        crate::world::RouteStepper::default(),
    ));
    app.update();
    let (entity, cells, frame) = app
        .world_mut()
        .query::<(Entity, &LiveAnimation)>()
        .single(app.world())
        .map(|(e, a)| (e, a.cells.clone(), a.frame))
        .unwrap();
    app.world_mut()
        .resource_mut::<crate::teleport::PendingTeleport>()
        .0 = Some((3, 8, 7));
    let mut held = 0;
    for _ in 0..80 {
        app.update();
        if !app.world().resource::<crate::teleport::Fade>().busy() {
            break;
        }
        held += 1;
        let animation = app.world().get::<LiveAnimation>(entity).unwrap();
        assert_eq!(animation.frame, frame);
        assert_eq!(animation.cells, cells);
        for &cell in &cells {
            assert_ne!(
                *app.world().get::<Visibility>(cell).unwrap(),
                Visibility::Hidden
            );
        }
    }
    assert!(held > 20);
    assert!(!app.world().resource::<crate::teleport::Fade>().busy());
    assert!(app.world().get::<LiveAnimation>(entity).is_some());
}

#[test]
fn same_map_arrivals_preserve_the_live_cast_cells_and_target_flash() {
    let (mut app, hero, _) = super::map_flashes::fixture(60);
    super::map_flashes::play(&mut app, AnimTarget::Hero);
    app.update();
    let (entity, cells) = app
        .world_mut()
        .query::<(Entity, &LiveAnimation)>()
        .single(app.world())
        .map(|(e, a)| (e, a.cells.clone()))
        .unwrap();
    let color = super::map_flashes::color(&app, hero);
    assert_ne!(color, [0; 4]);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    app.world_mut().write_message(MapChanged);
    app.update();
    assert!(app.world().get::<LiveAnimation>(entity).is_some());
    assert_eq!(
        app.world().get::<LiveAnimation>(entity).unwrap().cells,
        cells
    );
    assert_eq!(super::map_flashes::color(&app, hero), color);
}

#[test]
fn rebuilt_maps_clear_the_screen_flash_even_without_a_live_animation() {
    use bevy::ecs::system::RunSystemOnce;
    let (mut app, _, _) = super::map_targets::map_app(60);
    playback::reset_transient(app.world_mut());
    app.add_message::<MapRebuilt>();
    app.world_mut()
        .run_system_once(|mut commands: Commands| {
            spawn_screen_flash(&mut commands, [248, 80, 40], 31, FlashStamp::default());
        })
        .unwrap();
    app.world_mut().write_message(MapRebuilt);
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&render::FlashQuad>()
            .iter(app.world())
            .count(),
        0
    );
}
