use super::*;
use crate::text::HeroName;

fn scaffold() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .insert_resource(GameFont(Handle::default()))
        .add_systems(Startup, spawn_ui);
    app.update();
    app
}

fn position_in_window(world: &World, mut entity: Entity) -> Vec2 {
    let mut position = Vec2::ZERO;
    loop {
        let node = world.get::<Node>(entity).unwrap();
        if let (Val::Px(x), Val::Px(y)) = (node.left, node.top) {
            position += Vec2::new(x, y);
        }
        entity = world.get::<ChildOf>(entity).unwrap().parent();
        if world.get::<MenuWindow>(entity).is_some() {
            return position;
        }
    }
}

#[test]
fn main_menu_portraits_and_text_use_eight_native_pixel_content_margins() {
    let mut app = scaffold();
    let world = app.world_mut();
    for (entity, slot, node) in world.query::<(Entity, &MenuFace, &Node)>().iter(world) {
        assert_eq!(
            position_in_window(world, entity),
            Vec2::new(24.0, 24.0 + slot.0 as f32 * 174.0)
        );
        assert_eq!((node.width, node.height), (Val::Px(144.0), Val::Px(144.0)));
    }
    for (entity, slot) in world.query::<(Entity, &MenuText)>().iter(world) {
        let (x, y) = match slot.0 {
            TextSlot::Command(index) => (24.0, 30.0 + index as f32 * 48.0),
            TextSlot::Gold => (24.0, 30.0),
            TextSlot::Member { slot, field } => {
                let (x, y) = match field {
                    MemberField::Name => (192.0, 30.0),
                    MemberField::Title => (456.0, 30.0),
                    MemberField::Level => (192.0, 78.0),
                    MemberField::Condition => (318.0, 78.0),
                    MemberField::Hp => (510.0, 78.0),
                    MemberField::Exp => (192.0, 126.0),
                    MemberField::Sp => (510.0, 126.0),
                };
                (x, y + slot as f32 * 174.0)
            }
            TextSlot::Content => continue,
        };
        assert_eq!(position_in_window(world, entity), Vec2::new(x, y));
    }
}

#[test]
fn status_contents_clip_long_labels_before_the_window_border() {
    let mut app = scaffold();
    let world = app.world_mut();
    for parent in world
        .query_filtered::<&ChildOf, With<MenuFace>>()
        .iter(world)
    {
        let node = world.get::<Node>(parent.parent()).unwrap();
        assert_eq!(
            (node.left, node.top, node.right, node.bottom),
            (Val::Px(24.0), Val::Px(24.0), Val::Px(24.0), Val::Px(24.0))
        );
        assert_eq!(node.overflow, Overflow::clip());
    }
}

#[test]
fn command_and_member_selection_rectangles_match_the_original_contents_coordinates() {
    let mut app = scaffold();
    let world = app.world_mut();
    for (cursor, node) in world.query::<(&MenuCursor, &Node)>().iter(world) {
        let (left, width, height) = match cursor.0 {
            CursorId::Command => (12.0, 240.0, 48.0),
            CursorId::Status => (180.0, 504.0, 144.0),
            CursorId::Content => continue,
        };
        assert_eq!(node.left, Val::Px(left));
        assert_eq!((node.width, node.height), (Val::Px(width), Val::Px(height)));
    }
    assert_eq!(CMD_ROW_TOP, 24.0);
    assert_eq!(MEMBER_TOP, 24.0);
}

#[test]
fn main_menu_frame_layers_background_below_eight_native_pixel_corners() {
    let mut app = scaffold();
    let world = app.world_mut();
    let children = world
        .query::<(&MenuWindow, &Children)>()
        .iter(world)
        .find(|(window, _)| window.0 == WindowId::Status)
        .unwrap()
        .1;
    let background = world.get::<ImageNode>(children[0]).unwrap();
    assert_eq!(background.rect, Some(Rect::new(0.0, 0.0, 32.0, 32.0)));
    let corners = children
        .iter()
        .filter_map(|entity| Some((world.get::<Node>(entity)?, world.get::<ImageNode>(entity)?)))
        .filter(|(_, image)| {
            image
                .rect
                .is_some_and(|rect| rect.size() == Vec2::splat(8.0))
        })
        .collect::<Vec<_>>();
    assert_eq!(corners.len(), 4);
    for (node, _) in corners {
        assert_eq!((node.width, node.height), (Val::Px(24.0), Val::Px(24.0)));
    }
}

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
