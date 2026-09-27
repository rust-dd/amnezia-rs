use super::*;
use crate::world::{Character, MainCamera, MapChanged, MapRebuilt};

fn app() -> (App, Entity) {
    let mut app = interp_app();
    app.add_plugins((
        AssetPlugin::default(),
        crate::teleport::TeleportPlugin,
        crate::map_bgm::MapBgmPlugin,
    ))
    .init_asset::<Image>()
    .add_message::<MapChanged>();
    let mut data = MapData::for_test(40, 30);
    data.map_id = 3;
    app.insert_resource(data);
    let hero = app
        .world_mut()
        .query_filtered::<Entity, With<Player>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(hero)
        .insert(Transform::default());
    app.world_mut().spawn((
        MainCamera,
        Transform::from_xyz(-150.0, 100.0, 0.0),
        Projection::Orthographic(OrthographicProjection {
            area: Rect::new(-160.0, -120.0, 160.0, 120.0),
            ..OrthographicProjection::default_2d()
        }),
    ));
    let mut vehicles = app.world_mut().resource_mut::<crate::vehicles::Vehicles>();
    vehicles.set_location(0, 3, 5, 5);
    vehicles.save.riding = Some(0);
    app.update();
    (app, hero)
}

fn coordinates() -> Vec<EventCommand> {
    (0..6)
        .map(|op| cmd(10220, 0, vec![0, op + 1, op + 1, 0, 6, 10001, op]))
        .collect()
}

#[test]
fn quick_vehicle_transfer_resumes_in_place_without_fading_or_erasing_event_scope() {
    let (mut app, hero) = app();
    let mut commands = vec![cmd(10850, 0, vec![0, 0, 4, 7, 6])];
    commands.extend(coordinates());
    commands.push(cmd(11410, 0, vec![10]));
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, commands);
    super::super::driver::foreground(app.world_mut());
    assert_eq!(app.world().resource::<MapData>().map_id, 4);
    assert_eq!(app.world().get::<Player>(hero).unwrap().tile(), (7, 6));
    let vars = app.world().resource::<Variables>();
    assert_eq!((vars.get(1), vars.get(2), vars.get(3)), (4, 7, 6));
    assert_eq!((vars.get(5), vars.get(6)), (120, 112));
    assert_eq!(app.world().resource::<RunningEvent>().debug_id(), Some(7));
    assert!(app.world().resource::<PendingTeleport>().0.is_none());
    assert!(!app.world().resource::<Fade>().busy());
    assert!(
        !app.world()
            .resource::<crate::transitions::Transition>()
            .busy()
    );
    assert_eq!(app.world().resource::<Messages<MapRebuilt>>().len(), 1);
    assert_eq!(app.world().resource::<Messages<MapChanged>>().len(), 1);
}

#[test]
fn same_map_vehicle_relocation_updates_the_camera_before_the_next_operand_read() {
    let (mut app, _) = app();
    let mut commands = vec![cmd(10850, 0, vec![0, 0, 3, 20, 15])];
    commands.extend(coordinates());
    app.world_mut()
        .resource_mut::<RunningEvent>()
        .start(7, commands);
    super::super::driver::foreground(app.world_mut());
    let vars = app.world().resource::<Variables>();
    assert_eq!((vars.get(5), vars.get(6)), (152, 128));
}

#[test]
fn quick_common_transfer_keeps_its_tail_in_the_same_visit_without_an_extra_preupdate() {
    let (mut app, _) = app();
    app.insert_resource(CommonEvents(vec![
        common(1, 4, 0, vec![cmd(10220, 0, vec![0, 20, 20, 1, 0, 1])]),
        common(
            2,
            4,
            0,
            vec![
                cmd(10850, 0, vec![0, 0, 4, 7, 6]),
                cmd(10220, 0, vec![0, 21, 21, 1, 0, 1]),
                cmd(11410, 0, vec![10]),
            ],
        ),
        common(3, 4, 0, vec![cmd(10220, 0, vec![0, 22, 22, 1, 0, 1])]),
    ]));
    super::super::parallel::run_parallel(app.world_mut());
    let vars = app.world().resource::<Variables>();
    assert_eq!((vars.get(20), vars.get(21), vars.get(22)), (1, 1, 1));
    assert_eq!(app.world().resource::<MapData>().map_id, 4);
}

#[test]
fn quick_transfer_keeps_the_interpreters_command_budget_and_nested_event_ids() {
    let (mut app, _) = app();
    let mut commands = vec![cmd(10220, 0, vec![0, 20, 20, 1, 0, 1]); 9_998];
    commands.push(cmd(10850, 0, vec![0, 0, 4, 7, 6]));
    commands.extend([switch_cmd(30, 0, 0), switch_cmd(31, 0, 0)]);
    {
        let mut running = app.world_mut().resource_mut::<RunningEvent>();
        running.start(7, vec![cmd(0, 0, vec![])]);
        assert!(running.frame.call(commands, 9));
    }
    super::super::driver::foreground(app.world_mut());
    assert_eq!(app.world().resource::<Variables>().get(20), 9_998);
    assert!(switch_on(&app, 30));
    assert!(!switch_on(&app, 31));
    let running = app.world().resource::<RunningEvent>();
    assert_eq!(running.frame.event_id, 9);
    assert_eq!(running.frame.call_stack[0].event_id, 7);
    super::super::driver::foreground(app.world_mut());
    assert!(switch_on(&app, 31));
}

#[test]
fn quick_transfer_keeps_a_reserved_ordinary_target_and_the_erased_screen() {
    let (mut app, _) = app();
    app.world_mut()
        .resource_mut::<PendingTeleport>()
        .reload(3, 1, 2);
    app.world_mut()
        .resource_mut::<PendingTeleport>()
        .quick(4, 7, 6);
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .event_erased = true;
    crate::teleport::flush_quick(app.world_mut());
    assert_eq!(app.world().resource::<PendingTeleport>().0, Some((3, 1, 2)));
    assert_eq!(app.world().resource::<MapData>().map_id, 4);
    assert!(
        app.world()
            .resource::<crate::transitions::Transition>()
            .event_erased
    );
    assert!(!app.world().resource::<Fade>().busy());
    assert!(
        app.world()
            .resource::<Messages<crate::world::MapEffectsReset>>()
            .is_empty()
    );
}

#[test]
fn destination_music_precedes_the_quick_transfer_tail_and_does_not_repeat_next_frame() {
    let (mut app, _) = app();
    app.insert_resource(crate::map_bgm::MapInfoData(vec![
        amnezia_data::MapInfoDef {
            id: 4,
            parent: 0,
            music_type: 2,
            music: amnezia_data::MusicDef {
                name: "Destination".into(),
                volume: 80,
                tempo: 100,
                balance: 50,
                fadein: 0,
            },
        },
    ]));
    app.world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .clear();
    let mut music = cmd(11510, 0, vec![0, 100, 100, 50]);
    music.string = "After arrival".into();
    app.world_mut().resource_mut::<RunningEvent>().start(
        7,
        vec![
            cmd(10850, 0, vec![0, 0, 4, 7, 6]),
            music,
            cmd(11410, 0, vec![10]),
        ],
    );
    super::super::driver::foreground(app.world_mut());
    let names = app
        .world_mut()
        .resource_mut::<Messages<AudioRequest>>()
        .drain()
        .filter_map(|request| match request {
            AudioRequest::Bgm { name, .. } => Some(name),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(names, ["Destination", "After arrival"]);
    app.update();
    assert!(app.world().resource::<Messages<AudioRequest>>().is_empty());
}

#[test]
fn quick_common_transfer_discards_old_map_pages_before_visiting_the_destination() {
    let (mut app, _) = app();
    app.insert_resource(MapEvents {
        events: vec![map_event(8, 4, vec![cmd(11410, 0, vec![10])])],
    });
    super::super::parallel::run_parallel(app.world_mut());
    assert_eq!(app.world().resource::<ParallelPool>().count(), 1);
    app.insert_resource(CommonEvents(vec![common(
        1,
        4,
        0,
        vec![cmd(10850, 0, vec![0, 0, 4, 7, 6]), cmd(11410, 0, vec![10])],
    )]));
    super::super::parallel::run_parallel(app.world_mut());
    assert_eq!(app.world().resource::<MapData>().map_id, 4);
    assert_eq!(app.world().resource::<ParallelPool>().count(), 1);
}
