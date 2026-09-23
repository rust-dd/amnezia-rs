use super::*;
use crate::battle::{Battle, BattleActive, Phase};

fn app() -> App {
    let mut app = App::new();
    let mut transition = crate::transitions::Transition::default();
    transition.start(crate::transitions::Kind::Mosaic, true, 0, IVec2::ZERO);
    app.add_plugins(MinimalPlugins)
        .init_resource::<Fx>()
        .init_resource::<crate::screenfx::flash::channel::Inbox>()
        .init_resource::<Battle>()
        .init_resource::<BattleActive>()
        .init_resource::<TintState>()
        .insert_resource(transition)
        .add_message::<ScreenEffect>()
        .add_systems(Update, step_effects)
        .add_systems(PostUpdate, flash::channel::paint);
    flash::channel::spawn_overlay(app.world_mut());
    flash(&mut app);
    app
}

fn flash(app: &mut App) {
    app.world_mut()
        .write_message(ScreenEffect::flash(&[31, 5, 10, 31, 600, 0]));
    app.update();
}

fn enter(app: &mut App, generation: u64) {
    let mut battle = app.world_mut().resource_mut::<Battle>();
    battle.generation = generation;
    battle.phase = Phase::PartyCommand;
}

#[test]
fn only_the_actual_battle_scene_clears_the_map_flash_and_its_overlay() {
    let mut app = app();
    app.world_mut().resource_mut::<BattleActive>().0 = true;
    app.update();
    assert!(app.world().resource::<Fx>().flash.is_some());
    enter(&mut app, 1);
    app.update();
    assert!(app.world().resource::<Fx>().flash.is_none());
    let color = app
        .world_mut()
        .query_filtered::<&Sprite, With<FlashOverlay>>()
        .single(app.world())
        .unwrap()
        .color;
    assert_eq!(color, Color::NONE);
}

#[test]
fn battle_entry_keeps_the_tone_and_the_exact_shake_continuation() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<TintState>()
        .set_tone([80.0, 90.0, 110.0, 50.0]);
    let mut expected = ShakeState::default();
    expected.start(3, 5, 8.0 / 60.0);
    let offset = expected.step(1.0 / 60.0);
    {
        let mut fx = app.world_mut().resource_mut::<Fx>();
        fx.shake.start(3, 5, 8.0 / 60.0);
        step_shake(&mut fx, 1.0 / 60.0);
    }
    enter(&mut app, 1);
    app.update();
    assert_eq!(
        app.world().resource::<TintState>().tone(),
        [80.0, 90.0, 110.0, 50.0]
    );
    let mut fx = app.world_mut().resource_mut::<Fx>();
    assert!(fx.flash.is_none());
    assert_eq!(fx.shake_offset, Vec2::new(offset, 0.0));
    assert_eq!(fx.shake.step(1.0 / 60.0), expected.step(1.0 / 60.0));
}

#[test]
fn subsequent_battle_flashes_survive_phase_changes_and_the_return_to_the_map() {
    let mut app = app();
    enter(&mut app, 1);
    app.update();
    flash(&mut app);
    for phase in [
        Phase::Command,
        Phase::Resolve,
        Phase::Outcome,
        Phase::Inactive,
    ] {
        app.world_mut().resource_mut::<Battle>().phase = phase;
        app.update();
        assert!(app.world().resource::<Fx>().flash.is_some());
    }
    enter(&mut app, 1);
    app.update();
    assert!(app.world().resource::<Fx>().flash.is_none());
    flash(&mut app);
    enter(&mut app, 2);
    app.update();
    assert!(app.world().resource::<Fx>().flash.is_none());
}

#[test]
fn an_action_started_after_the_screen_update_does_not_spend_its_first_shake_tick_early() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<crate::transitions::Transition>()
        .clear();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));
    app.add_systems(
        Update,
        (
            |mut once: Local<bool>, mut effects: MessageWriter<ScreenEffect>| {
                if !*once {
                    *once = true;
                    effects.write(ScreenEffect::Shake {
                        power: 3,
                        speed: 5,
                        secs: 8.0 / 60.0,
                    });
                }
            },
            flash::channel::receive,
        )
            .chain()
            .after(step_effects),
    );
    for x in [0.0, 5.0, 5.0, 2.0, -2.0, -6.0, -6.0, -4.0, 0.0] {
        app.update();
        assert_eq!(app.world().resource::<Fx>().shake_offset, Vec2::new(x, 0.0));
    }
}
