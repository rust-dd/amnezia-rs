use bevy::prelude::*;
use capture::capture;
use input::input;

mod camera;
mod capture;
pub(crate) mod completion;
mod input;
mod looping;
mod message_options;
pub(crate) mod offscreen;
mod scenarios;
mod ui_layers;
mod world_image;

pub struct SmokePlugin;

#[derive(Resource)]
struct SmokeRun {
    frame: u32,
    scenario: &'static str,
    escaped_cast: u8,
    finish_at: Option<u32>,
    save_menu_frames: u32,
}

impl Plugin for SmokePlugin {
    fn build(&self, app: &mut App) {
        if !cfg!(debug_assertions) || !std::env::args().any(|arg| arg == "--smoke-test") {
            return;
        }
        offscreen::configure(app);
        let scenario = scenarios::selected();
        if scenario == "save-slots" {
            crate::menu::save_files::smoke::configure(app);
        } else if scenario == "inn" {
            crate::shop::inn::smoke::configure(app);
        } else if scenario == "load-slots" {
            crate::menu::save_files::smoke::load::configure(app);
        } else if scenario == "save-music" {
            crate::save::music_smoke::configure(app);
        } else if scenario == "save-camera" {
            crate::save::camera_smoke::configure(app);
        } else if scenario == "save-npcs" {
            crate::save::npc_smoke::configure(app);
        } else if scenario == "save-hero" {
            crate::save::hero_smoke::configure(app);
        } else if scenario == "save-vehicles" {
            crate::save::vehicle_smoke::configure(app);
        } else if scenario == "save-pictures" {
            crate::save::picture_smoke::configure(app);
        } else if scenario == "save-screen" {
            crate::save::screen_smoke::configure(app);
        } else if scenario == "save-weather" {
            crate::save::weather_smoke::configure(app);
        } else if scenario == "save-animations" {
            crate::save::animation_smoke::configure(app);
        }
        app.insert_resource(completion::Completion::new(scenario));
        app.insert_resource(SmokeRun {
            frame: 0,
            scenario,
            escaped_cast: 0,
            finish_at: None,
            save_menu_frames: 0,
        })
        .init_resource::<input::ScriptedKeys>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ))
        .add_systems(
            PreUpdate,
            input
                .after(bevy::input::InputSystems)
                .before(crate::vehicles::VehicleInput),
        )
        .add_systems(
            PostUpdate,
            drive
                .after(bevy::transform::TransformSystems::Propagate)
                .after(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate)
                .after(crate::battle::EffectsSet)
                .after(crate::legacy_colors::hue::HueSet),
        );
    }
}

fn drive(world: &mut World) {
    if world.resource::<SmokeRun>().scenario != "save-slots"
        && crate::menu::save_files::smoke::event_active(world)
    {
        if completion::close_early(world, world.resource::<SmokeRun>().frame) {
            return;
        }
        let mut smoke = world.resource_mut::<SmokeRun>();
        smoke.save_menu_frames += 1;
        assert!(
            smoke.save_menu_frames <= 240,
            "save selector did not complete within four seconds"
        );
        return;
    }
    world.resource_mut::<SmokeRun>().save_menu_frames = 0;
    let frame = {
        let mut smoke = world.resource_mut::<SmokeRun>();
        smoke.frame += 1;
        smoke.frame
    };
    if completion::close_early(world, frame) {
        return;
    }
    if frame == 60 {
        capture(world, "title");
    }
    if frame == 90 && world.resource::<SmokeRun>().scenario != "load-slots" {
        world.resource_mut::<crate::title::TitleActive>().0 = false;
        world
            .resource_mut::<crate::session::NewGameRequest>()
            .requested = true;
    }
    let scenario = world.resource::<SmokeRun>().scenario;
    if scenario == "inn" {
        if let Some(label) = crate::shop::inn::smoke::drive(world, frame) {
            capture(world, label);
        }
        if crate::shop::inn::smoke::finished(world)
            && world.resource::<SmokeRun>().finish_at.is_none()
        {
            world.resource_mut::<SmokeRun>().finish_at = Some(frame + 30);
        }
    }
    if scenario == "shop"
        && let Some(label) = crate::shop::smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "items"
        && let Some(label) = crate::menu::item_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "equipment"
        && let Some(label) = crate::menu::equipment_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "skills"
        && let Some(label) = crate::menu::skill_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "load-slots"
        && let Some(label) = crate::menu::save_files::smoke::load::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "save-slots"
        && let Some(label) = crate::menu::save_files::smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "battle-menus"
        && let Some(label) = crate::battle::smoke::show(world, frame)
    {
        capture(world, label);
        if label == "battle-skill-impact" {
            world.resource_mut::<SmokeRun>().finish_at = Some(frame + 45);
        }
    }
    if scenario == "battle-rewards" {
        if let Some(label) = crate::battle::outcome_smoke::drive(world) {
            capture(world, label);
        }
        if crate::battle::outcome_smoke::ready(world)
            && world.resource::<SmokeRun>().finish_at.is_none()
        {
            world.resource_mut::<SmokeRun>().finish_at = Some(frame + 45);
        }
    }
    if frame == 150 && !matches!(scenario, "intro" | "load-slots") {
        scenarios::start(world, scenario);
    }
    if scenario == "looping" {
        looping::drive(world, frame);
    }
    if scenario == "camera" {
        camera::drive(world, frame);
    }
    if scenario == "map-animations"
        && let Some(label) = crate::animation::map_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "world-tones"
        && let Some(label) = crate::legacy_colors::world_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "weather"
        && let Some(label) = crate::screenfx::weather_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "save-weather"
        && let Some(label) = crate::save::weather_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "save-animations"
        && let Some(label) = crate::save::animation_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "map-flashes"
        && let Some(label) = crate::animation::map_flash_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "message-options" {
        message_options::drive(world, frame);
    }
    if scenario == "font"
        && let Some(label) = crate::dialogue::font_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "dialogue-timing"
        && let Some(label) = crate::dialogue::timing_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "ui-layers"
        && let Some(label) = ui_layers::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "actor-graphics"
        && let Some(label) = crate::appearance::smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "actor-names"
        && let Some(label) = crate::menu::name_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "pictures"
        && let Some(label) = crate::picture::smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "colors"
        && let Some(label) = crate::legacy_colors::smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "font-colors"
        && let Some(label) = crate::font::bitmap::smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "display"
        && let Some(label) = crate::display::smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if frame == 350 && scenario == "battle-events" {
        crate::dialogue::verify_battle_layer(world);
        capture(world, "battle-events-message");
    }
    if scenario == "transitions"
        && let Some(label) = crate::transitions::smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "screen-events"
        && let Some(label) = crate::transitions::event_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "save-screen"
        && let Some(label) = crate::save::screen_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "battle-transitions"
        && let Some(label) = crate::battle::flow::smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if matches!(scenario, "gameover" | "battle-defeat")
        && let Some(label) =
            crate::gameover::smoke::drive(world, frame, scenario == "battle-defeat")
    {
        capture(world, label);
    }
    if scenario == "return-title"
        && let Some(label) = crate::title::smoke::return_scene(world, frame)
    {
        capture(world, label);
    }
    if scenario == "save-music"
        && let Some(label) = crate::save::music_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "save-camera"
        && let Some(label) = crate::save::camera_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "save-npcs"
        && let Some(label) = crate::save::npc_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "save-pictures"
        && let Some(label) = crate::save::picture_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "save-hero"
        && let Some(label) = crate::save::hero_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "save-vehicles"
        && let Some(label) = crate::save::vehicle_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "water"
        && let Some(label) = crate::world::water_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "animation-colors"
        && let Some(label) = crate::animation::smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if scenario == "menu"
        && let Some(label) = crate::menu::layout_smoke::drive(world, frame)
    {
        capture(world, label);
    }
    if frame == 900 && scenario == "timer" {
        assert!(world.resource::<crate::battle::BattleActive>().0);
        world.resource_mut::<crate::timer::GameClock>().remaining = 1.0;
    }
    if scenario == "escape" {
        let escaped = scenarios::escaped_airship_cast(world);
        world.resource_mut::<SmokeRun>().escaped_cast |= escaped;
        if world.resource::<crate::world::MapData>().map_id == 86
            && world.resource::<SmokeRun>().finish_at.is_none()
        {
            assert_eq!(world.resource::<SmokeRun>().escaped_cast, 0b11111);
            assert!(!world.resource::<crate::player::HeroHidden>().0);
            // The 60-frame tint resumes after the transfer's 35-frame show.
            world.resource_mut::<SmokeRun>().finish_at = Some(frame + 120);
        }
        if world.resource::<SmokeRun>().finish_at == Some(frame + 10) {
            assert!(!world.resource::<crate::teleport::Fade>().busy());
            assert_eq!(
                world.resource::<crate::screenfx::TintState>().tone(),
                [100.0; 4]
            );
            capture(world, "escape");
        }
    }
    if scenario == "battle-events"
        && world.resource::<SmokeRun>().finish_at.is_none()
        && crate::battle::smoke::events_ready(world)
    {
        capture(world, "battle-events");
        world.resource_mut::<SmokeRun>().finish_at = Some(frame + 45);
    }
    if frame.is_multiple_of(300) {
        let map = world.resource::<crate::world::MapData>().map_id;
        let running = world
            .resource::<crate::interpreter::RunningEvent>()
            .debug_id();
        let hero = world
            .query::<&crate::player::Player>()
            .single(world)
            .map(|p| (p.tile_x, p.tile_y));
        info!("smoke frame={frame} map={map} hero={hero:?} event={running:?}");
    }
    if frame == 1200 && !matches!(scenario, "escape" | "battle-events") {
        capture(world, scenario);
    }
    if frame == 360 && !matches!(scenario, "intro" | "gameover" | "battle-defeat") {
        capture(world, &format!("{scenario}-early"));
    }
    let finish = world.resource::<SmokeRun>().finish_at.unwrap_or(
        if matches!(
            scenario,
            "escape" | "battle-menus" | "battle-events" | "battle-rewards"
        ) {
            2400
        } else if scenario == "items" {
            1480
        } else if scenario == "menu" {
            1520
        } else if scenario == "equipment" {
            1290
        } else if scenario == "animation-colors" {
            1340
        } else if scenario == "dialogue-timing" {
            3000
        } else if scenario == "inn" {
            6000
        } else {
            1260
        },
    );
    if frame >= finish {
        world_image::verify_finished(world, scenario);
        if scenario == "escape" {
            assert!(
                world.resource::<SmokeRun>().finish_at.is_some(),
                "airship escape never reached the next dream scene"
            );
        }
        if scenario == "inn" {
            crate::shop::inn::smoke::verify_finished(world);
        } else if scenario == "shop" {
            crate::shop::smoke::verify_finished(world);
        } else if scenario == "save-slots" {
            crate::menu::save_files::smoke::verify_finished(world);
        } else if scenario == "items" {
            crate::menu::item_smoke::verify_finished(world);
        } else if scenario == "equipment" {
            crate::menu::equipment_smoke::verify_finished(world);
        } else if scenario == "skills" {
            crate::menu::skill_smoke::verify_finished(world);
        } else if scenario == "load-slots" {
            crate::menu::save_files::smoke::load::verify_finished(world);
        } else if scenario == "display" {
            crate::display::smoke::verify_finished(world);
        } else if scenario == "font" {
            crate::dialogue::font_smoke::verify_finished(world);
        } else if scenario == "dialogue-timing" {
            crate::dialogue::timing_smoke::verify_finished(world);
        } else if scenario == "map-animations" {
            crate::animation::map_smoke::verify_finished(world);
        } else if scenario == "world-tones" {
            crate::legacy_colors::world_smoke::verify_finished(world);
        } else if scenario == "weather" {
            crate::screenfx::weather_smoke::verify_finished(world);
        } else if scenario == "map-flashes" {
            crate::animation::map_flash_smoke::verify_finished(world);
        } else if scenario == "ui-layers" {
            ui_layers::verify_finished(world);
        } else if scenario == "menu" {
            crate::menu::layout_smoke::verify_finished(world);
            crate::menu::font_smoke::verify_finished(world);
        } else if scenario == "actor-graphics" {
            crate::appearance::smoke::verify_finished(world);
        } else if scenario == "font-colors" {
            crate::font::bitmap::smoke::verify_finished(world);
        } else if scenario == "animation-colors" {
            crate::animation::smoke::verify_finished(world);
        } else if scenario == "pictures" {
            crate::picture::smoke::verify_finished(world);
        } else if scenario == "water" {
            crate::world::water_smoke::verify_finished(world);
        } else if scenario == "transitions" {
            crate::transitions::smoke::verify_finished(world);
        } else if scenario == "screen-events" {
            crate::transitions::event_smoke::verify_finished(world);
        } else if scenario == "camera" {
            camera::verify_finished(world);
        } else if scenario == "save-screen" {
            crate::save::screen_smoke::verify_finished(world);
        } else if scenario == "save-weather" {
            crate::save::weather_smoke::verify_finished(world);
        } else if scenario == "save-animations" {
            crate::save::animation_smoke::verify_finished(world);
        } else if scenario == "battle-transitions" {
            crate::battle::flow::smoke::verify_finished(world);
        } else if matches!(scenario, "gameover" | "battle-defeat") {
            crate::gameover::smoke::verify_finished(world, scenario == "battle-defeat");
        } else if scenario == "return-title" {
            assert!(crate::title::smoke::ready(world));
            crate::menu::end_smoke::verify_finished(world);
            crate::title::smoke::verify_finished(world);
        } else if scenario == "save-music" {
            crate::save::music_smoke::verify_finished(world);
        } else if scenario == "save-camera" {
            crate::save::camera_smoke::verify_finished(world);
        } else if scenario == "save-npcs" {
            crate::save::npc_smoke::verify_finished(world);
        } else if scenario == "save-hero" {
            crate::save::hero_smoke::verify_finished(world);
        } else if scenario == "save-vehicles" {
            crate::save::vehicle_smoke::verify_finished(world);
        } else if scenario == "save-pictures" {
            crate::save::picture_smoke::verify_finished(world);
        } else if scenario == "intro" {
            assert_eq!(world.resource::<crate::world::MapData>().map_id, 3);
            assert!(
                !world
                    .resource::<crate::interpreter::RunningEvent>()
                    .active()
            );
        } else if scenario == "airship" {
            let (x, y, _) = world
                .resource::<crate::vehicles::Vehicles>()
                .character(10004)
                .unwrap();
            assert_eq!((x, y), (28, 101));
        } else if scenario == "panorama" {
            crate::panorama::smoke::verify_finished(world);
            assert_eq!(world.resource::<crate::world::MapData>().map_id, 94);
            assert!(world.resource::<crate::player::HeroHidden>().0);
            let (hero, visibility) = world
                .query::<(&crate::player::Player, &Visibility)>()
                .single(world)
                .unwrap();
            assert_eq!((hero.tile_x, hero.tile_y), (9, 4));
            assert_eq!(*visibility, Visibility::Hidden);
            scenarios::verify_airship_staging(world);
        } else if scenario == "timer" {
            let clock = world.resource::<crate::timer::GameClock>();
            assert_eq!(clock.seconds(), 0);
            assert!(!clock.running && !clock.visible);
            assert!(!world.resource::<crate::battle::BattleActive>().0);
            assert!(!world.resource::<crate::gameover::GameOverActive>().0);
            assert_eq!(
                world
                    .resource::<crate::audio::CurrentBgm>()
                    .track()
                    .map(|t| t.name),
                Some("House".to_string()),
                "the battle must restore music requested immediately before the encounter"
            );
        } else if scenario == "battle-events" {
            crate::battle::smoke::verify_events(world);
        } else if scenario == "battle-rewards" {
            crate::battle::outcome_smoke::verify_finished(world);
        } else if scenario == "battle-menus" {
            crate::battle::smoke::verify_finished(world);
            crate::battle::hud::verify_cursors(world);
        } else if scenario == "actor-names" {
            crate::battle::smoke::verify_actor_names(world);
        }
        world.resource::<completion::Completion>().mark();
        world.write_message(AppExit::Success);
    }
}
