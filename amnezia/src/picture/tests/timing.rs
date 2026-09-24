use super::*;
use crate::interpreter::{InterpreterStep, ParallelStep};

#[derive(Resource, Default)]
struct Parallel(Vec<PictureCommand>);

#[derive(Resource, Default)]
struct Foreground(Vec<PictureCommand>);

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<render::PictureMaterial>()
        .init_resource::<Parallel>()
        .init_resource::<Foreground>()
        .add_message::<PictureCommand>()
        .add_message::<MapRebuilt>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .add_systems(Startup, render::setup_picture_mesh)
        .configure_sets(Update, ParallelStep.before(InterpreterStep))
        .add_systems(
            Update,
            (
                (|mut pending: ResMut<Parallel>, mut out: MessageWriter<PictureCommand>| {
                    out.write_batch(pending.0.drain(..));
                })
                .in_set(ParallelStep),
                (|mut pending: ResMut<Foreground>, mut out: MessageWriter<PictureCommand>| {
                    out.write_batch(pending.0.drain(..));
                })
                .in_set(InterpreterStep),
            ),
        );
    register_timeline(&mut app);
    app.world_mut()
        .spawn((crate::world::MainCamera, Transform::default()));
    app.update();
    app
}

fn show(x: f32, fixed: bool) -> PictureCommand {
    PictureCommand::Show {
        id: 1,
        name: "Cross".into(),
        x,
        y: 120.0,
        fixed_to_map: fixed,
        use_transparent_color: true,
        transparency: 0.0,
        zoom: 100.0,
        tone: Tone::NEUTRAL,
        effect: Effect::default(),
    }
}

fn move_to(x: f32, secs: f32) -> PictureCommand {
    PictureCommand::Move {
        id: 1,
        x,
        y: 120.0,
        transparency: 0.0,
        zoom: 100.0,
        tone: Tone::NEUTRAL,
        effect: Effect::default(),
        secs,
    }
}

fn picture_x(app: &mut App) -> f32 {
    let world = app.world_mut();
    world.query::<&Picture>().single(world).unwrap().x
}

#[test]
fn picture_tweens_distinguish_parallel_and_foreground_command_ticks() {
    for parallel in [false, true] {
        let mut app = app();
        let commands = vec![show(0.0, false), move_to(60.0, 1.0)];
        if parallel {
            app.world_mut().resource_mut::<Parallel>().0 = commands;
        } else {
            app.world_mut().resource_mut::<Foreground>().0 = commands;
        }
        app.update();
        assert_eq!(picture_x(&mut app), if parallel { 1.0 } else { 0.0 });
        app.update();
        assert_eq!(picture_x(&mut app), if parallel { 2.0 } else { 1.0 });
    }
}

#[test]
fn the_early_picture_pass_is_not_replayed_over_later_foreground_changes() {
    let mut app = app();
    app.world_mut().resource_mut::<Parallel>().0 = vec![show(20.0, false)];
    app.world_mut().resource_mut::<Foreground>().0 = vec![move_to(30.0, 0.0)];
    app.update();
    assert_eq!(picture_x(&mut app), 30.0);
    app.update();
    assert_eq!(picture_x(&mut app), 30.0);
}

#[test]
fn parallel_map_fixed_pictures_are_anchored_before_the_player_scrolls() {
    for parallel in [false, true] {
        let mut app = app();
        app.add_systems(
            Update,
            (|mut cameras: Query<&mut Transform, With<crate::world::MainCamera>>| {
                cameras.single_mut().unwrap().translation.x += 2.0;
            })
            .in_set(crate::player::PlayerStep)
            .after(ParallelStep)
            .before(InterpreterStep),
        );
        if parallel {
            app.world_mut().resource_mut::<Parallel>().0 = vec![show(160.0, true)];
        } else {
            app.world_mut().resource_mut::<Foreground>().0 = vec![show(160.0, true)];
        }
        app.update();
        let world = app.world_mut();
        let picture = world.query::<&Picture>().single(world).unwrap();
        assert_eq!(
            picture.world_anchor,
            Some(Vec2::new(if parallel { 0.0 } else { 2.0 }, 0.0))
        );
    }
}

#[test]
fn picture_positions_use_the_unshaken_camera_for_both_anchor_modes() {
    for fixed in [false, true] {
        let mut app = render_app();
        let mut pan = crate::player::CameraPan::default();
        pan.position = Some(Vec2::new(40.0, -8.0));
        app.insert_resource(pan);
        let world = app.world_mut();
        world
            .query_filtered::<&mut Transform, With<crate::world::MainCamera>>()
            .single_mut(world)
            .unwrap()
            .translation = Vec3::new(43.0, -8.0, 0.0);
        world.write_message(show(160.0, fixed));
        app.update();
        let world = app.world_mut();
        let (picture, transform) = world
            .query::<(&Picture, &Transform)>()
            .single(world)
            .unwrap();
        assert_eq!(picture.world_anchor, fixed.then_some(Vec2::new(40.0, -8.0)));
        assert_eq!(transform.translation.truncate(), Vec2::new(40.0, -8.0));
        world
            .query_filtered::<&mut Transform, With<crate::world::MainCamera>>()
            .single_mut(world)
            .unwrap()
            .translation
            .x = 36.0;
        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<&Transform, With<Picture>>()
                .single(world)
                .unwrap()
                .translation
                .truncate(),
            Vec2::new(40.0, -8.0)
        );
    }
}
