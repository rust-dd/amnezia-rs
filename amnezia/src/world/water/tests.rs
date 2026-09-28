use super::*;
use crate::timing::GameFrames;

#[test]
fn water_phase_stays_frozen_until_the_frame_after_an_async_transition() {
    use crate::timing::TimingPlugin;
    use crate::transitions::{Kind, Transition, TransitionPlugin};
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, TimingPlugin, TransitionPlugin))
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .insert_resource(WaterStyle {
            fast: true,
            cycle: false,
        })
        .add_systems(PostUpdate, animate_water);
    let entity = app
        .world_mut()
        .spawn((WaterQuarter { id: 0, quarter: 0 }, Sprite::default()))
        .id();
    app.update();
    let first = app.world().get::<Sprite>(entity).unwrap().rect;
    app.world_mut().resource_mut::<Transition>().start_for(
        Kind::Fade,
        true,
        0,
        IVec2::new(160, 120),
        36,
    );
    for frame in 1..=37 {
        app.update();
        assert_eq!(
            app.world().get::<Sprite>(entity).unwrap().rect,
            first,
            "transition frame {frame}"
        );
    }
    assert!(!app.world().resource::<Transition>().busy());
    for _ in 0..12 {
        app.update();
    }
    assert_ne!(app.world().get::<Sprite>(entity).unwrap().rect, first);
}

#[test]
fn original_chipsets_keep_both_animation_speeds_and_the_ping_pong_cycle() {
    let chipsets = crate::assets::load_ron::<Vec<amnezia_data::Chipset>>(&format!(
        "{}/chipsets.ron",
        crate::assets::asset_root(),
    ));
    assert_eq!(chipsets.len(), 20);
    for chipset in &chipsets {
        assert_eq!(
            chipset.animation_speed,
            u32::from([11, 13].contains(&chipset.id))
        );
        assert_eq!(chipset.animation_type, 0);
        let style = WaterStyle::from_chipset(chipset);
        assert_eq!(
            style.frames(12).0,
            u16::from([11, 13].contains(&chipset.id))
        );
    }
}

#[test]
fn water_and_waterfalls_follow_separate_original_frame_tables() {
    for fast in [false, true] {
        let step = if fast { 12 } else { 24 };
        for cycle in [false, true] {
            let style = WaterStyle { fast, cycle };
            let sequence = if cycle {
                vec![0, 1, 2, 0, 1, 2, 0, 1]
            } else {
                vec![0, 1, 2, 1, 0, 1, 2, 1]
            };
            for frame in 0..step * 8 {
                let (water, waterfall) = style.frames(frame);
                assert_eq!(water, sequence[(frame / step) as usize]);
                assert_eq!(waterfall, [0, 1, 2, 3][((frame / 6) % 4) as usize]);
            }
        }
    }
}

#[test]
fn animation_phase_is_independent_of_render_rate_and_large_time_steps() {
    for fps in [15, 30, 60, 120, 144] {
        let mut time = GameFrames::default();
        for _ in 0..fps * 5 {
            time.advance(1.0 / fps as f64);
        }
        assert_eq!(time.frame, 300);
        assert_eq!(WaterStyle::default().frames(time.frame), (0, 2));
        assert_eq!(
            WaterStyle {
                fast: true,
                cycle: false
            }
            .frames(time.frame),
            (1, 2)
        );
    }
    let mut time = GameFrames::default();
    time.advance(5.0);
    assert_eq!(WaterStyle::default().frames(time.frame), (0, 2));
}

#[test]
fn new_tiles_and_chipset_changes_immediately_use_the_current_global_phase() {
    let mut app = App::new();
    app.init_resource::<SceneFrames>()
        .init_resource::<WaterStyle>()
        .add_systems(PostUpdate, animate_water);
    app.world_mut().resource_mut::<SceneFrames>().frame = 30;
    let quarter = app
        .world_mut()
        .spawn((WaterQuarter { id: 0, quarter: 0 }, Sprite::default()))
        .id();
    let cell = app
        .world_mut()
        .spawn((WaterCell { id: 3000 }, Sprite::default()))
        .id();
    app.update();
    let rect = |source: (f32, f32), size| {
        Some(Rect::new(
            source.0,
            source.1,
            source.0 + size,
            source.1 + size,
        ))
    };
    assert_eq!(
        app.world().get::<Sprite>(quarter).unwrap().rect,
        rect(tiles::water_quarters(0, 1)[0].src, 8.0)
    );
    assert_eq!(
        app.world().get::<Sprite>(cell).unwrap().rect,
        rect(tiles::block_c_source(3000, 1), 16.0)
    );
    app.world_mut().insert_resource(WaterStyle {
        fast: true,
        cycle: false,
    });
    let late = app
        .world_mut()
        .spawn((WaterQuarter { id: 0, quarter: 0 }, Sprite::default()))
        .id();
    app.update();
    for entity in [quarter, late] {
        assert_eq!(
            app.world().get::<Sprite>(entity).unwrap().rect,
            rect(tiles::water_quarters(0, 2)[0].src, 8.0)
        );
    }
    assert_eq!(app.world().resource::<SceneFrames>().frame, 30);
}

#[test]
fn loading_original_maps_changes_style_without_resetting_animation_phase() {
    use bevy::ecs::system::RunSystemOnce;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<SceneFrames>();
    app.world_mut().resource_mut::<SceneFrames>().frame = 30;
    for (map_id, expected) in [(213, (2, 1)), (3, (1, 1))] {
        app.world_mut()
            .run_system_once(move |mut commands: Commands, server: Res<AssetServer>| {
                crate::world::load_map(
                    &mut commands,
                    &server,
                    &Default::default(),
                    &Default::default(),
                    &Default::default(),
                    &Default::default(),
                    map_id,
                );
            })
            .unwrap();
        assert_eq!(app.world().resource::<SceneFrames>().frame, 30);
        assert_eq!(app.world().resource::<WaterStyle>().frames(30), expected);
    }
}
