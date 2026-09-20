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

fn viewing(screen: MenuScreen) -> App {
    let mut app = scaffold();
    app.insert_resource(MenuOpen(true))
        .insert_resource(MenuState { cursor: 4, screen })
        .insert_resource(super::super::testkit::data())
        .init_resource::<Party>()
        .init_resource::<Progression>()
        .init_resource::<Inventory>()
        .init_resource::<Vitals>()
        .init_resource::<Equipment>()
        .init_resource::<Terms>()
        .insert_resource(crate::font::bitmap::BitmapFont::from_id(0))
        .init_resource::<crate::save::SaveAccess>()
        .insert_resource(HeroName("Ron".into()))
        .init_resource::<crate::timing::GameFrames>()
        .init_resource::<clocks::Clock>()
        .add_systems(Update, (update_ui, clocks::update));
    app.update();
    app
}

#[test]
fn end_game_does_not_use_the_legacy_full_screen_text_panel() {
    let mut app = viewing(MenuScreen::EndGame { cursor: 1 });
    let world = app.world_mut();
    let (_, visibility) = world
        .query::<(&MenuWindow, &Visibility)>()
        .iter(world)
        .find(|(window, _)| window.0 == WindowId::Content)
        .unwrap();
    assert_eq!(*visibility, Visibility::Hidden);
}

#[test]
fn item_list_does_not_use_the_legacy_full_screen_text_panel() {
    let mut app = viewing(MenuScreen::ItemList { cursor: 0 });
    let world = app.world_mut();
    let (_, visibility) = world
        .query::<(&MenuWindow, &Visibility)>()
        .iter(world)
        .find(|(window, _)| window.0 == WindowId::Content)
        .unwrap();
    assert_eq!(*visibility, Visibility::Hidden);
}

#[test]
fn skill_list_does_not_use_the_legacy_full_screen_text_panel() {
    let mut app = viewing(MenuScreen::SkillList {
        member: 0,
        cursor: 0,
    });
    let world = app.world_mut();
    let (_, visibility) = world
        .query::<(&MenuWindow, &Visibility)>()
        .iter(world)
        .find(|(window, _)| window.0 == WindowId::Content)
        .unwrap();
    assert_eq!(*visibility, Visibility::Hidden);
}

#[test]
fn target_screens_do_not_use_the_legacy_full_screen_text_panel() {
    for screen in [
        MenuScreen::ItemTarget {
            item_id: 5,
            cursor: 0,
        },
        MenuScreen::SkillTarget {
            member: 0,
            skill_id: 1,
            cursor: 0,
        },
    ] {
        let mut app = viewing(screen);
        let world = app.world_mut();
        let (_, visibility) = world
            .query::<(&MenuWindow, &Visibility)>()
            .iter(world)
            .find(|(window, _)| window.0 == WindowId::Content)
            .unwrap();
        assert_eq!(*visibility, Visibility::Hidden, "{screen:?}");
    }
}

#[test]
fn inactive_command_cursor_remains_visible_during_member_selection() {
    let mut app = viewing(MenuScreen::MemberSelect {
        action: crate::menu::MemberAction::Skill,
        cursor: 0,
    });
    let world = app.world_mut();
    for (cursor, visibility) in world.query::<(&MenuCursor, &Visibility)>().iter(world) {
        if matches!(cursor.0, CursorId::Command | CursorId::Status) {
            assert_eq!(*visibility, Visibility::Inherited);
        }
    }
}

#[test]
fn main_menu_cursor_reaches_the_second_skin_phase_after_twelve_logical_frames() {
    let mut app = viewing(MenuScreen::Command);
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = 12;
    app.update();
    let world = app.world_mut();
    let (_, pieces) = world
        .query::<(&MenuCursor, &Children)>()
        .iter(world)
        .find(|(cursor, _)| matches!(cursor.0, CursorId::Command))
        .unwrap();
    assert_eq!(pieces.len(), 9);
    for entity in pieces {
        let rect = world.get::<ImageNode>(*entity).unwrap().rect.unwrap();
        assert!(rect.min.x >= 96.0 && rect.max.x <= 128.0, "{rect:?}");
    }
}

#[test]
fn main_menu_cursor_pauses_while_the_save_selector_owns_the_scene() {
    let mut app = viewing(MenuScreen::Command);
    let mut files = crate::menu::save_files::SaveFiles::default();
    files.request();
    app.insert_resource(files);
    app.world_mut()
        .resource_mut::<crate::timing::GameFrames>()
        .frame = 12;
    app.update();
    assert_eq!(
        app.world()
            .resource::<clocks::Clock>()
            .source_x(CursorId::Command),
        64.0
    );
}

#[test]
fn field_scene_background_uses_the_original_system_color_pixel() {
    let mut app = scaffold();
    let world = app.world_mut();
    let (_, image) = world
        .query::<(&MenuWindow, Option<&ImageNode>)>()
        .iter(world)
        .find(|(window, _)| window.0 == WindowId::Panel)
        .unwrap();
    let image = image.expect("the field menu must cover the map between its windows");
    assert_eq!(image.rect, Some(Rect::new(0.0, 32.0, 1.0, 33.0)));
    assert_eq!(
        image.image,
        world
            .resource::<AssetServer>()
            .load("graphics/System/System.png")
    );
    assert!(matches!(image.image_mode, NodeImageMode::Stretch));
}

#[test]
fn all_main_menu_fields_use_original_bitmap_text() {
    let mut app = scaffold();
    let world = app.world_mut();
    let count = world
        .query_filtered::<&crate::font::bitmap::PixelText, With<MenuText>>()
        .iter(world)
        .count();
    assert_eq!(count, 5 + 1 + 4 * 7);
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
        .insert_resource(crate::font::bitmap::BitmapFont::from_id(0))
        .init_resource::<crate::save::SaveAccess>()
        .insert_resource(HeroName("Ron".into()))
        .add_systems(Update, update_ui);
    let name = app
        .world_mut()
        .spawn((
            MenuText(TextSlot::Member {
                slot: 0,
                field: MemberField::Name,
            }),
            PixelText::default(),
            Visibility::Inherited,
        ))
        .id();
    app.update();
    assert_eq!(
        app.world().get::<PixelText>(name).unwrap().runs[0].text,
        "Ron"
    );
    app.update();
    app.world_mut().resource_mut::<HeroName>().0 = "Áron".into();
    app.update();
    assert_eq!(
        app.world().get::<PixelText>(name).unwrap().runs[0].text,
        "Áron"
    );
    assert_eq!(app.world().resource::<MenuState>().cursor, 0);
}
