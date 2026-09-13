use super::*;
use crate::save::{LoadOutcome, LoadRequest, SavePlugin};
use bevy::ecs::system::RunSystemOnce;

#[test]
fn loading_playtime_does_not_keep_the_previous_sessions_fraction() {
    for version in 0..=crate::save::SAVE_FORMAT_VERSION {
        let path = std::env::temp_dir().join(format!(
            "amnezia-playtime-{version}-{}.ron",
            std::process::id()
        ));
        let original = format!(
            "(format_version:{version},map_id:2,x:3,y:4,dir:2,switches:[],variables:[],party:[1],items:[],gold:0,playtime:100)"
        );
        std::fs::write(&path, &original).unwrap();
        let mut app = crate::save::tests::save_resources(path.clone());
        app.add_plugins((AssetPlugin::default(), SavePlugin))
            .init_asset::<Image>()
            .init_resource::<crate::interpreter::RunningEvent>()
            .insert_resource(PlayTime {
                seconds: 7,
                frac: 0.75,
            });
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
        let playtime = app.world().resource::<PlayTime>();
        assert_eq!(playtime.seconds, 100);
        assert_eq!(playtime.frac, 0.0, "version {version}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn a_maximum_playtime_counter_does_not_overflow_on_the_next_second() {
    let mut world = World::new();
    world.insert_resource(PlayTime {
        seconds: u64::MAX,
        frac: 0.0,
    });
    let mut time = Time::<()>::default();
    time.advance_by(std::time::Duration::from_secs(1));
    world.insert_resource(time);
    world.run_system_once(tick_playtime).unwrap();
    assert_eq!(world.resource::<PlayTime>().seconds, u64::MAX);
    assert_eq!(world.resource::<PlayTime>().frac, 0.0);
}
