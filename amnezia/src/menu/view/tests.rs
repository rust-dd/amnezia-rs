use super::*;
use crate::text::HeroName;

#[test]
fn renamed_hero_refreshes_an_already_open_menu_without_moving_the_cursor() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .insert_resource(MenuOpen(true))
        .init_resource::<MenuState>()
        .insert_resource(super::super::testkit::data())
        .init_resource::<Party>()
        .init_resource::<Progression>()
        .init_resource::<Inventory>()
        .init_resource::<Vitals>()
        .init_resource::<Equipment>()
        .init_resource::<Terms>()
        .insert_resource(HeroName("Ron".into()))
        .add_systems(Update, update_ui);
    let name = app
        .world_mut()
        .spawn((
            MenuText(TextSlot::Member {
                slot: 0,
                field: MemberField::Name,
            }),
            Text::default(),
            Visibility::Inherited,
        ))
        .id();
    app.update();
    assert_eq!(app.world().get::<Text>(name).unwrap().0, "Ron");
    app.update();
    app.world_mut().resource_mut::<HeroName>().0 = "Áron".into();
    app.update();
    assert_eq!(app.world().get::<Text>(name).unwrap().0, "Áron");
    assert_eq!(app.world().resource::<MenuState>().cursor, 0);
}
