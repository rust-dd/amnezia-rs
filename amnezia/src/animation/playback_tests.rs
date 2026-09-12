use super::playback::LiveAnimation;
use super::*;
use crate::timing::{GameFrames, TimingPlugin};

mod map_flashes;
mod map_targets;
mod scenes;
mod slots;
mod transfers;

fn app(fps: u32) -> App {
    fixture(fps, false)
}

fn fixture(fps: u32, sound_only: bool) -> App {
    let mut app = App::new();
    app.register_required_components::<Mesh2d, Visibility>();
    map::flash::register(&mut app);
    scene::register(&mut app);
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), TimingPlugin))
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<cells::CellMaterial>()
        .add_message::<PlayAnimation>()
        .add_message::<AudioRequest>()
        .add_message::<BattlerFlash>()
        .init_resource::<ActiveAnimations>()
        .add_systems(Startup, cells::setup_mesh)
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / fps as f64),
        ))
        .insert_resource(AnimationLibrary(vec![AnimationDef {
            id: 1,
            name: "Clock fixture".into(),
            animation_name: "Sword1".into(),
            scope: 0,
            position: 1,
            frames: vec![
                amnezia_data::AnimationFrameDef {
                    cells: vec![amnezia_data::AnimationCellDef {
                        valid: true,
                        cell_id: 0,
                        x: 0,
                        y: 0,
                        scale: 100,
                        tone_red: 100,
                        tone_green: 100,
                        tone_blue: 100,
                        tone_gray: 100,
                        transparency: 0,
                    }],
                };
                30
            ],
            timings: (1..=30)
                .map(|frame| AnimationTimingDef {
                    frame,
                    ..tests::se_timing("Punch")
                })
                .collect(),
        }]))
        .add_systems(
            Update,
            (start_animations, step_animations, track_active_animations).chain(),
        );
    app.update();
    app.world_mut().write_message(PlayAnimation {
        map_target: None,
        slot: if sound_only {
            AnimationSlot::Party
        } else {
            AnimationSlot::Map
        },
        anim_id: 1,
        targets: vec![AnimAnchor {
            pos: Vec2::ZERO,
            height: 48.0,
        }],
        screen_center: Vec2::ZERO,
        global: false,
        sound_only,
    });
    app.update();
    app
}

#[test]
fn starting_an_animation_does_not_spend_the_preceding_render_delta() {
    for fps in [15, 30, 60, 144] {
        let mut app = app(fps);
        let frame = app
            .world_mut()
            .query::<&LiveAnimation>()
            .single(app.world())
            .unwrap()
            .frame;
        assert_eq!(frame, 0, "{fps} FPS");
    }
}

#[test]
fn all_data_frames_and_sounds_finish_on_the_same_logical_tick_at_every_fps() {
    for fps in [15, 30, 60, 144] {
        let mut app = app(fps);
        let start = app.world().resource::<GameFrames>().frame;
        let mut sounds = app
            .world()
            .resource::<Messages<AudioRequest>>()
            .get_cursor();
        let mut count = sounds
            .read(app.world().resource::<Messages<AudioRequest>>())
            .count();
        for _ in 0..fps {
            app.update();
            count += sounds
                .read(app.world().resource::<Messages<AudioRequest>>())
                .count();
            let age = app
                .world()
                .resource::<GameFrames>()
                .frame
                .wrapping_sub(start);
            let current = app
                .world_mut()
                .query::<&LiveAnimation>()
                .iter(app.world())
                .next()
                .map(|a| a.frame);
            assert_eq!(
                current,
                (age < 60).then_some(age as usize / 2),
                "{fps} FPS at {age}"
            );
            assert_eq!(
                app.world().resource::<ActiveAnimations>().total,
                usize::from(age < 60)
            );
        }
        assert_eq!(count, 30, "each timing fires exactly once at {fps} FPS");
    }
}

#[test]
fn first_sound_waits_for_the_first_update_and_skipped_flashes_keep_their_age() {
    let mut app = app(15);
    let sounds = app.world().resource::<Messages<AudioRequest>>();
    assert_eq!(sounds.get_cursor().read(sounds).count(), 0);
    app.update();
    let flashes = app.world().resource::<Messages<BattlerFlash>>();
    let values = flashes
        .get_cursor()
        .read(flashes)
        .map(|flash| (flash.age, flash.rgb))
        .collect::<Vec<_>>();
    assert_eq!(values, [(3, [248; 3]), (1, [248; 3])]);
}

#[test]
fn a_transition_pauses_animation_time_without_a_catch_up_jump() {
    let mut app = app(60);
    app.update();
    let mut transition = crate::transitions::Transition::default();
    transition.start(crate::transitions::Kind::Fade, true, 0, IVec2::ZERO);
    app.insert_resource(transition);
    for _ in 0..120 {
        app.update();
    }
    assert_eq!(
        app.world_mut()
            .query::<&LiveAnimation>()
            .single(app.world())
            .unwrap()
            .frame,
        0
    );
    app.world_mut()
        .remove_resource::<crate::transitions::Transition>();
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&LiveAnimation>()
            .single(app.world())
            .unwrap()
            .frame,
        1
    );
}

#[test]
fn sound_only_plays_twenty_timings_without_cells_or_flashes_and_stops_at_forty_ticks() {
    for fps in [15, 30, 60, 144] {
        let mut app = fixture(fps, true);
        for timing in &mut app.world_mut().resource_mut::<AnimationLibrary>().0[0].timings {
            timing.flash_scope = if timing.frame.is_multiple_of(2) { 2 } else { 1 };
        }
        let start = app.world().resource::<GameFrames>().frame;
        let mut sounds = app
            .world()
            .resource::<Messages<AudioRequest>>()
            .get_cursor();
        let mut count = 0;
        for _ in 0..fps {
            app.update();
            count += sounds
                .read(app.world().resource::<Messages<AudioRequest>>())
                .count();
            assert_eq!(
                app.world_mut().query::<&Mesh2d>().iter(app.world()).count(),
                0
            );
            assert_eq!(
                app.world_mut()
                    .query::<&render::FlashQuad>()
                    .iter(app.world())
                    .count(),
                0
            );
            let flashes = app.world().resource::<Messages<BattlerFlash>>();
            assert_eq!(flashes.get_cursor().read(flashes).count(), 0);
            let age = app
                .world()
                .resource::<GameFrames>()
                .frame
                .wrapping_sub(start);
            assert_eq!(
                app.world().resource::<ActiveAnimations>().total,
                usize::from(age < 40)
            );
        }
        assert_eq!(count, 20);
    }
}
