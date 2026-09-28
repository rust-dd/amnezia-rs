use super::*;
use crate::player::{CameraPan, Player};
use crate::state::{Inventory, Party, Switches, Variables};
use crate::teleport::{Fade, PendingTeleport, TeleportPlugin};
use crate::world::{MapData, MapEvents, MoveQueue, RouteStepper};

#[test]
fn real_local_cross_map_and_reload_transfers_preserve_and_freeze_rain() {
    for (destination, reload) in [(3, false), (2, false), (3, true)] {
        let mut app = app(60);
        app.add_plugins((crate::transitions::TransitionPlugin, TeleportPlugin))
            .init_resource::<Switches>()
            .init_resource::<Variables>()
            .init_resource::<Party>()
            .init_resource::<Inventory>()
            .init_resource::<CameraPan>()
            .init_resource::<MapEvents>();
        let mut map = MapData::for_test(20, 15);
        map.map_id = 3;
        app.insert_resource(map);
        app.world_mut().spawn((
            Player {
                tile_x: 15,
                tile_y: 12,
                dir: 2,
                frame: 1,
                charset: "Chara1".into(),
                index: 0,
            },
            Transform::default(),
            MoveQueue::default(),
            RouteStepper::default(),
        ));
        if reload {
            app.world_mut()
                .resource_mut::<PendingTeleport>()
                .reload(destination, 8, 7);
        } else {
            app.world_mut().resource_mut::<PendingTeleport>().0 = Some((destination, 8, 7));
        }
        let initial = app.world().resource::<Rain>().drops.clone();
        let mut expected = Rain::new(crate::interpreter::EventRng::seeded(31415));
        assert_eq!(expected.drops, initial);
        if !reload {
            expected.advance(std::time::Duration::from_secs_f64(1.0 / 60.0).as_secs_f64());
        }
        let canvas = app
            .world_mut()
            .query_filtered::<Entity, With<Canvas>>()
            .single(app.world())
            .unwrap();
        let mut held = 0;
        for frame in 1..90 {
            app.world_mut()
                .resource_mut::<crate::timing::GameFrames>()
                .frame = frame;
            app.update();
            if !app.world().resource::<Fade>().busy() {
                break;
            }
            held += 1;
            assert_eq!(app.world().resource::<Rain>().drops, expected.drops);
            assert_eq!(app.world().resource::<Rain>().fraction, expected.fraction);
            assert_eq!(
                *app.world().get::<Visibility>(canvas).unwrap(),
                Visibility::Inherited
            );
        }
        assert!(held > 60);
        assert!(!app.world().resource::<Fade>().busy());
        assert_eq!(app.world().resource::<MapData>().map_id, destination);
        assert!(app.world().get::<Canvas>(canvas).is_some());
        assert_ne!(app.world().resource::<Rain>().drops, initial);
    }
}
