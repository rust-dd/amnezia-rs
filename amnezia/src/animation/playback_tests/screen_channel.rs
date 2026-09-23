use super::*;
use crate::screenfx::{FlashOverlay, Fx, ScreenEffect, flash::channel};

fn setup() -> App {
    let mut app = app(60);
    app.init_resource::<Fx>()
        .init_resource::<channel::Inbox>()
        .add_message::<ScreenEffect>()
        .add_systems(Update, channel::receive.after(step_animations))
        .add_systems(PostUpdate, (channel::paint, cells::sync_flash));
    app
}

fn event_flash(app: &mut App) {
    app.world_mut().write_message(ScreenEffect::Flash {
        r: 0,
        g: 31,
        b: 0,
        intensity: 20,
        secs: 1.0,
    });
}

fn pixels(app: &mut App) -> [u8; 4] {
    app.world_mut()
        .query_filtered::<&Sprite, With<FlashOverlay>>()
        .single(app.world())
        .unwrap()
        .color
        .to_srgba()
        .to_u8_array()
}

#[test]
fn foreground_event_replaces_the_animation_without_stacking_or_resuming_it() {
    let mut app = setup();
    app.world_mut().resource_mut::<AnimationLibrary>().0[0].timings = vec![tests::flash_timing(2)];
    app.update();
    assert_eq!(pixels(&mut app), [248, 160, 80, 248]);
    event_flash(&mut app);
    app.update();
    assert_eq!(pixels(&mut app), [0, 248, 0, 160]);
    let handles = app
        .world_mut()
        .query::<&MeshMaterial2d<cells::CellMaterial>>()
        .iter(app.world())
        .map(|handle| handle.0.clone())
        .collect::<Vec<_>>();
    assert!(!handles.is_empty());
    let materials = app.world().resource::<Assets<cells::CellMaterial>>();
    assert!(
        handles
            .iter()
            .all(|handle| materials.get(handle).unwrap().flash
                == Vec4::new(248.0, 160.0, 80.0, 248.0))
    );
    assert_eq!(
        app.world_mut()
            .query::<&render::FlashQuad>()
            .iter(app.world())
            .count(),
        0
    );
    app.update();
    assert_eq!(pixels(&mut app), [248, 160, 80, 248]);
    assert_eq!(
        channel::event_color(app.world().get_resource::<Fx>()),
        Color::NONE
    );
    for _ in 0..12 {
        app.update();
    }
    assert_eq!(pixels(&mut app), [0; 4]);
    assert_eq!(
        app.world_mut()
            .query::<&FlashOverlay>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn an_animation_without_a_screen_timing_still_overwrites_the_event_channel() {
    let mut app = setup();
    event_flash(&mut app);
    app.update();
    assert_eq!(pixels(&mut app), [0, 248, 0, 160]);
    app.update();
    assert_eq!(pixels(&mut app), [0; 4]);
    assert_eq!(
        channel::event_color(app.world().get_resource::<Fx>()),
        Color::NONE
    );
}

#[test]
fn screen_updates_follow_map_party_enemy_order_regardless_of_entity_order() {
    let mut app = setup();
    let mut library = app.world_mut().resource_mut::<AnimationLibrary>();
    library.0[0].timings = vec![tests::flash_timing(2)];
    let mut second = library.0[0].clone();
    second.id = 2;
    second.timings[0].flash_red = 0;
    second.timings[0].flash_green = 31;
    second.timings[0].flash_blue = 0;
    library.0.push(second);
    for (slot, anim_id) in [(AnimationSlot::Enemies, 2), (AnimationSlot::Party, 1)] {
        app.world_mut().write_message(PlayAnimation {
            slot,
            anim_id,
            map_target: None,
            targets: Vec::new(),
            screen_center: Vec2::ZERO,
            global: false,
            sound_only: false,
        });
    }
    app.update();
    app.update();
    assert_eq!(pixels(&mut app), [0, 248, 0, 248]);
    assert_eq!(
        app.world_mut()
            .query::<&FlashOverlay>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn ending_a_short_cast_does_not_leave_an_independently_decaying_flash() {
    let mut app = setup();
    app.add_systems(Update, fade_flashes.before(step_animations));
    app.world_mut().resource_mut::<AnimationLibrary>().0[0].timings = vec![tests::flash_timing(2)];
    app.world_mut().resource_mut::<AnimationLibrary>().0[0]
        .frames
        .truncate(1);
    app.world_mut().write_message(PlayAnimation {
        slot: AnimationSlot::Map,
        anim_id: 1,
        map_target: None,
        targets: Vec::new(),
        screen_center: Vec2::ZERO,
        global: false,
        sound_only: false,
    });
    app.update();
    app.update();
    assert_ne!(pixels(&mut app), [0; 4]);
    app.update();
    assert_ne!(pixels(&mut app), [0; 4]);
    assert_eq!(app.world().resource::<ActiveAnimations>().total, 0);
    app.update();
    assert_eq!(pixels(&mut app), [0; 4]);
}
