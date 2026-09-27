use super::*;
use crate::player::{CameraPan, Player};
use crate::world::{Character, RouteStepper};
use bevy::ecs::system::RunSystemOnce;

pub(super) fn relative_positions(world: &mut World) -> Vec<Vec2> {
    let camera = world.resource::<CameraPan>().position.unwrap();
    let mut pictures = world
        .query::<(&Picture, &Transform)>()
        .iter(world)
        .collect::<Vec<_>>();
    pictures.sort_by_key(|(picture, _)| picture.id);
    pictures
        .into_iter()
        .map(|(_, transform)| transform.translation.truncate() - camera)
        .collect()
}

pub(super) fn fixture() -> (App, Entity) {
    let (mut app, hero) = crate::world::test_support::camera_app((20, 15));
    app.init_asset::<Mesh>()
        .init_asset::<render::PictureMaterial>()
        .add_message::<PictureCommand>()
        .add_message::<MapRebuilt>()
        .add_systems(PostUpdate, render::place_pictures);
    app.world_mut()
        .run_system_once(render::setup_picture_mesh)
        .unwrap();
    register_timeline(&mut app);
    (app, hero)
}

fn show(app: &mut App) {
    for id in 1..=2 {
        app.world_mut().write_message(PictureCommand::Show {
            id,
            name: "Cross".into(),
            x: 160.0,
            y: 120.0,
            fixed_to_map: id == 1,
            use_transparent_color: true,
            transparency: 0.0,
            zoom: 100.0,
            tone: Tone::NEUTRAL,
            effect: Effect::default(),
        });
    }
    app.update();
}

#[test]
fn jump_landing_rounding_does_not_scroll_fixed_or_screen_pictures() {
    let (mut app, hero) = fixture();
    app.world_mut()
        .resource_mut::<CameraPan>()
        .command(&[2, 1, 1, 1, 0]);
    for _ in 0..33 {
        app.update();
    }
    show(&mut app);
    let before = relative_positions(app.world_mut());
    let mut route = app.world_mut().get_mut::<RouteStepper>(hero).unwrap();
    route.set_speed(6);
    route.force_route(RouteStepper::from_move_event(&[10001, 8, 0, 0, 24, 25]));
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        relative_positions(app.world_mut()),
        vec![before[0] - Vec2::X, before[1]]
    );
}

#[test]
fn same_map_relocation_keeps_map_fixed_pictures_at_their_screen_coordinates() {
    let (mut app, hero) = fixture();
    show(&mut app);
    let before = relative_positions(app.world_mut());
    app.world_mut()
        .get_mut::<Player>(hero)
        .unwrap()
        .set_tile(25, 18);
    app.world_mut().resource_mut::<CameraPan>().recenter(false);
    app.update();
    assert_eq!(relative_positions(app.world_mut()), before);
}
