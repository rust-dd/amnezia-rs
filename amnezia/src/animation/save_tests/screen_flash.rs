use super::*;
use bevy::ecs::system::RunSystemOnce;

fn start_flash(app: &mut App) {
    let frame = app.world().resource::<GameFrames>().frame;
    app.world_mut()
        .run_system_once(move |mut commands: Commands| {
            spawn_screen_flash(
                &mut commands,
                [248, 80, 40],
                27,
                FlashStamp { age: 6, frame },
            );
        })
        .unwrap();
}

#[test]
fn saved_flashes_restore_but_the_next_live_cast_update_owns_the_channel() {
    for active in [false, true] {
        let (mut app, path) = app(&format!("flash_{active}"));
        if active {
            play(&mut app);
        }
        start_flash(&mut app);
        let expected = saved::snapshot(app.world_mut());
        assert!(expected.screen_flash.is_some());
        save(&mut app);
        reload(&mut app);
        std::fs::remove_file(path).unwrap();
        assert_eq!(saved::snapshot(app.world_mut()), expected);
        for ticks in 1..=5 {
            step(&mut app, 1);
            let flash = app
                .world_mut()
                .query_filtered::<&Sprite, With<render::FlashQuad>>()
                .iter(app.world())
                .next();
            if !active && ticks < 5 {
                let actual = flash.unwrap().color.to_srgba().to_u8_array();
                assert_eq!(
                    actual,
                    [248, 80, 40, (flash_power_level(6 + ticks, 27) * 8) as u8]
                );
            } else {
                assert!(flash.is_none());
            }
        }
    }
}

#[test]
fn invalid_screen_flash_age_strength_and_channels_cannot_mutate_the_session() {
    for (tag, from, to) in [
        ("age", "elapsed: 6,", "elapsed: 11,"),
        ("strength", "power: 27,", "power: 32,"),
        ("channel", "248,", "255,"),
    ] {
        let (mut app, path) = app(&format!("flash_invalid_{tag}"));
        start_flash(&mut app);
        save(&mut app);
        let text = std::fs::read_to_string(&path).unwrap();
        let invalid = text.replace(from, to);
        assert_ne!(text, invalid);
        std::fs::write(&path, &invalid).unwrap();
        let before = saved::snapshot(app.world_mut());
        app.world_mut().resource_mut::<LoadRequest>().0 = true;
        app.update();
        assert_eq!(
            app.world().resource::<crate::save::LoadOutcome>().0,
            Some(false)
        );
        assert_eq!(saved::snapshot(app.world_mut()), before);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
        std::fs::remove_file(path).unwrap();
    }
}
