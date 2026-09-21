use super::*;
use crate::inputnumber::{InputNumber, InputNumberPlugin};
use crate::timing::GameFrames;
use bevy::input::keyboard::{Key, KeyboardFocusLost, KeyboardInput};
use bevy::input::{ButtonState, InputPlugin, InputSystems};

fn app(scenario: &'static str) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, InputPlugin, InputNumberPlugin))
        .init_resource::<ScriptedKeys>()
        .init_resource::<GameFrames>()
        .init_resource::<crate::menu::DirectionInput>()
        .add_message::<crate::audio::AudioRequest>()
        .insert_resource(SmokeRun {
            frame: 1129,
            scenario,
            escaped_cast: 0,
            finish_at: None,
            save_menu_frames: 0,
        })
        .add_systems(PreUpdate, input.after(InputSystems))
        .add_systems(
            Update,
            crate::menu::update_directions.before(crate::dialogue::PromptInput),
        );
    app
}

fn step(app: &mut App, frame: u32) {
    app.world_mut().resource_mut::<SmokeRun>().frame = frame;
    app.world_mut().resource_mut::<GameFrames>().frame += 1;
    app.update();
}

fn native_key(app: &mut App, key_code: KeyCode, state: ButtonState) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
        state,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

#[test]
fn focus_loss_does_not_restart_a_scripted_number_hold() {
    let mut app = app("dialogue-timing");
    step(&mut app, 1129);
    app.world_mut().resource_mut::<InputNumber>().open(4, 76);
    for frame in 1130..=1190 {
        if frame == 1187 {
            app.world_mut().write_message(KeyboardFocusLost);
        }
        step(&mut app, frame);
        let expected = (1 + (frame + 1).saturating_sub(1150) / 4) as i64 % 10;
        assert_eq!(
            app.world().resource::<InputNumber>().value,
            expected,
            "number after scripted frame {frame}"
        );
    }
}

#[test]
fn native_keyboard_cannot_edit_or_confirm_a_scripted_number() {
    let mut app = app("dialogue-timing");
    step(&mut app, 1129);
    app.world_mut().resource_mut::<InputNumber>().open(4, 76);
    for frame in 1130..=1190 {
        if frame == 1142 {
            native_key(&mut app, KeyCode::ArrowLeft, ButtonState::Pressed);
            native_key(&mut app, KeyCode::Enter, ButtonState::Pressed);
        }
        if frame == 1187 {
            native_key(&mut app, KeyCode::ArrowUp, ButtonState::Released);
        }
        step(&mut app, frame);
        let number = app.world().resource::<InputNumber>();
        assert!(number.active(), "native Enter at frame {frame}");
        assert_eq!(number.cursor(), 3, "native Left at frame {frame}");
        let expected = (1 + (frame + 1).saturating_sub(1150) / 4) as i64 % 10;
        assert_eq!(number.value, expected, "number at frame {frame}");
    }
}

#[test]
fn scripted_shop_hold_triggers_once_and_releases_on_its_own_frame() {
    let mut app = app("shop");
    for frame in 359..=392 {
        step(&mut app, frame);
        let keys = app.world().resource::<ButtonInput<KeyCode>>();
        assert_eq!(keys.just_pressed(KeyCode::ArrowRight), frame == 360);
        assert_eq!(
            keys.pressed(KeyCode::ArrowRight),
            (360..392).contains(&frame)
        );
    }
}
