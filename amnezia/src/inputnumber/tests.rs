use super::*;
use crate::audio::{AudioRequest, SystemSounds};
use crate::timing::GameFrames;

mod navigation;

fn tap(app: &mut App, key: KeyCode) {
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.clear();
        keys.press(key);
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(key);
}

fn headless() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<InputNumber>();
    app.init_resource::<GameFrames>()
        .init_resource::<crate::menu::DirectionInput>()
        .init_resource::<crate::menu::MenuOpen>()
        .insert_resource(SystemSounds {
            cursor: amnezia_data::SoundDef {
                name: "CURSOR".into(),
                volume: 100,
                tempo: 100,
                ..default()
            },
            decision: amnezia_data::SoundDef {
                name: "DECISION".into(),
                volume: 100,
                tempo: 100,
                ..default()
            },
            ..default()
        });
    app.add_message::<AudioRequest>();
    app.add_systems(
        Update,
        (crate::menu::update_directions, input_number_input).chain(),
    );
    app.update();
    app
}

#[test]
fn enters_two_digit_number() {
    let mut app = headless();
    app.world_mut().resource_mut::<InputNumber>().open(2, 20);

    for _ in 0..2 {
        tap(&mut app, KeyCode::ArrowUp);
    }
    tap(&mut app, KeyCode::ArrowRight);
    for _ in 0..4 {
        tap(&mut app, KeyCode::ArrowUp);
    }
    tap(&mut app, KeyCode::Enter);

    let input = app.world().resource::<InputNumber>();
    assert_eq!(input.result, Some(42));
    assert!(!input.active());
    assert_eq!(input.var_id, 20);
}

#[test]
fn arrow_down_wraps_to_nine() {
    let mut app = headless();
    app.world_mut().resource_mut::<InputNumber>().open(1, 7);

    tap(&mut app, KeyCode::ArrowDown);
    tap(&mut app, KeyCode::Space);

    let input = app.world().resource::<InputNumber>();
    assert_eq!(input.result, Some(9));
    assert!(!input.active());
}
