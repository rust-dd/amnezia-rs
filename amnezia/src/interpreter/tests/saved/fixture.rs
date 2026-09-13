use super::*;

pub(super) fn app(tag: &str) -> (App, std::path::PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "amnezia-save-interpreter-{tag}-{}.ron",
        std::process::id()
    ));
    let mut app = interp_app();
    app.add_plugins((
        AssetPlugin::default(),
        SavePlugin,
        crate::gamedata::GameDataPlugin,
    ))
    .init_asset::<Image>()
    .init_resource::<crate::screenfx::TintState>()
    .init_resource::<crate::timer::PlayTime>()
    .insert_resource(SaveLocation(path.clone()))
    .add_systems(Update, rebuild.in_set(crate::teleport::MapTransfer));
    app.world_mut().resource_mut::<MapData>().map_id = 2;
    let world = app.world_mut();
    let entity = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .unwrap();
    world
        .entity_mut(entity)
        .insert((Sprite::default(), Transform::default()));
    (app, path)
}

fn rebuild(world: &mut World) {
    let Some((map_id, x, y)) = world.resource_mut::<PendingTeleport>().0.take() else {
        return;
    };
    world.resource_mut::<MapData>().map_id = map_id;
    let (mut hero, mut queue, mut route) = world
        .query::<(&mut Player, &mut MoveQueue, &mut RouteStepper)>()
        .single_mut(world)
        .unwrap();
    hero.tile_x = x as i32;
    hero.tile_y = y as i32;
    *queue = default();
    *route = default();
    world.write_message(crate::world::MapRebuilt);
    world.write_message(crate::world::MapChanged);
    assert!(
        world
            .resource_mut::<crate::transitions::Transition>()
            .start(
                crate::transitions::Kind::Fade,
                false,
                0,
                IVec2::new(160, 120)
            )
    );
}

pub(super) fn load(app: &mut App) {
    app.world_mut().resource_mut::<LoadRequest>().0 = true;
    app.update();
    assert_eq!(app.world().resource::<LoadOutcome>().0, Some(true));
    assert!(
        app.world()
            .resource::<crate::transitions::Transition>()
            .busy()
    );
}

pub(super) fn resume(app: &mut App) {
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    app.update();
}
