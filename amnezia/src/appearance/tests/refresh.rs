use super::*;
use crate::gamedata::{GameData, GameDataPlugin};
use crate::world::RouteStepper;

fn app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, GameDataPlugin, AppearancePlugin))
        .init_resource::<Party>();
    let hero = app
        .world_mut()
        .spawn((
            Player {
                tile_x: 9,
                tile_y: 4,
                dir: 2,
                frame: 1,
                charset: "Chara1".into(),
                index: 0,
            },
            RouteStepper::default(),
            Sprite::default(),
        ))
        .id();
    app.update();
    (app, hero)
}

fn assert_graphic(app: &App, hero: Entity, expected: (&str, u32)) {
    let player = app.world().get::<Player>(hero).unwrap();
    assert_eq!((player.charset.as_str(), player.index), expected);
    assert_eq!(
        (player.tile_x, player.tile_y, player.dir, player.frame),
        (9, 4, 2, 1)
    );
}

fn pose(app: &mut App, hero: Entity) {
    app.world_mut()
        .get_mut::<Player>(hero)
        .unwrap()
        .set_graphic("Poses2".into(), 4);
}

#[test]
fn roster_refresh_uses_each_original_actor_graphic_and_restored_overrides() {
    let (mut app, hero) = app();
    let original = app.world().resource::<GameData>().actors.clone();
    for actor in &original {
        app.world_mut()
            .resource_mut::<Party>()
            .restore(vec![actor.id]);
        app.update();
        assert_graphic(&app, hero, (&actor.character_name, actor.character_index));
    }
    let mut saved = Appearance::default();
    saved.set(1, "Poses2".into(), 4);
    let encoded = ron::to_string(&saved).unwrap();
    app.insert_resource(ron::from_str::<Appearance>(&encoded).unwrap());
    app.world_mut().resource_mut::<Party>().restore(vec![1, 2]);
    app.update();
    assert_graphic(&app, hero, ("Poses2", 4));
    app.world_mut().resource_mut::<Party>().remove(1);
    app.update();
    assert_graphic(&app, hero, ("Chara1", 1));
    app.world_mut().resource_mut::<Party>().restore(vec![1, 2]);
    app.update();
    assert_graphic(&app, hero, ("Poses2", 4));
}

#[test]
fn only_real_party_changes_reset_temporary_route_graphics() {
    let (mut app, hero) = app();
    pose(&mut app, hero);
    app.update();
    assert_graphic(&app, hero, ("Poses2", 4));
    app.world_mut().resource_mut::<Party>().add(1);
    app.world_mut().resource_mut::<Party>().remove(999);
    app.update();
    assert_graphic(&app, hero, ("Poses2", 4));
    app.world_mut().resource_mut::<Party>().add(2);
    app.world_mut().resource_mut::<Party>().remove(2);
    app.update();
    assert_graphic(&app, hero, ("Chara1", 0));
    pose(&mut app, hero);
    app.world_mut().write_message(SpriteChange {
        actor_id: 2,
        charset: "Chara4".into(),
        index: 3,
    });
    app.update();
    assert_graphic(&app, hero, ("Chara1", 0));
    pose(&mut app, hero);
    app.world_mut().write_message(SpriteChange {
        actor_id: 999,
        charset: "Chara4".into(),
        index: 3,
    });
    app.update();
    assert_graphic(&app, hero, ("Poses2", 4));
}

#[test]
fn removing_the_last_actor_clears_the_player_graphic_without_erasing_actor_skins() {
    let (mut app, hero) = app();
    app.world_mut().write_message(SpriteChange {
        actor_id: 1,
        charset: "Poses2".into(),
        index: 4,
    });
    app.update();
    app.world_mut().resource_mut::<Party>().remove(1);
    app.update();
    assert_graphic(&app, hero, ("", 0));
    assert_eq!(
        app.world().resource::<Appearance>().get(1),
        Some(("Poses2", 4))
    );
    app.world_mut().resource_mut::<Party>().add(1);
    app.update();
    assert_graphic(&app, hero, ("Poses2", 4));
}

#[test]
fn map_transfers_reset_route_graphics_and_transparency_without_changing_speed() {
    let (mut app, hero) = app();
    let map = crate::assets::load_ron::<amnezia_data::Map>(&format!(
        "{}/maps/map_0001.ron",
        crate::assets::asset_root()
    ));
    let mut page = map.events[0].pages[0].clone();
    page.translucent = true;
    page.move_speed = 5;
    app.world_mut()
        .entity_mut(hero)
        .insert(RouteStepper::from_event_page(Some(&page)));
    app.world_mut().get_mut::<Sprite>(hero).unwrap().color = Color::WHITE.with_alpha(0.5);
    pose(&mut app, hero);
    app.update();
    assert_graphic(&app, hero, ("Poses2", 4));
    app.world_mut().write_message(MapChanged);
    app.update();
    assert_graphic(&app, hero, ("Chara1", 0));
    let route = app.world().get::<RouteStepper>(hero).unwrap();
    assert_eq!(route.speed(), 5);
    assert_eq!(route.alpha(), 1.0);
    assert_eq!(app.world().get::<Sprite>(hero).unwrap().color.alpha(), 1.0);
}
