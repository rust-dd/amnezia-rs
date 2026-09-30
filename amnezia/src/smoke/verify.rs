use super::{SmokeRun, camera, completion, scenarios, ui_layers, world_image};
use bevy::prelude::*;

pub(super) fn finished(world: &mut World, scenario: &str) {
    if scenario == "overlap" {
        crate::world::overlap_smoke::verify_finished(world);
    }
    if scenario == "terrain" {
        crate::world::terrain_smoke::verify_finished(world);
    }
    if scenario == "airship-journey" {
        super::journey::verify_finished(world);
    }
    if scenario == "map-passages" {
        crate::world::passage_smoke::verify_finished(world);
    }
    if scenario == "map-scenes" {
        crate::world::scene_smoke::verify_finished(world);
    }
    if scenario == "crystals" {
        crate::save::crystal_smoke::verify_finished(world);
    }
    world_image::verify_finished(world, scenario);
    if scenario == "escape" {
        super::airship::verify_finished(world);
        assert!(
            world.resource::<SmokeRun>().finish_at.is_some(),
            "airship escape never reached the next dream scene"
        );
    }
    if matches!(scenario, "async-transitions" | "async-inns") {
        crate::interpreter::continuation::smoke::verify_finished(world);
    } else if scenario == "inn" {
        crate::shop::inn::smoke::verify_finished(world);
    } else if scenario == "quick-transfers" {
        crate::teleport::smoke::verify_finished(world);
    } else if scenario == "normal-transfers" {
        crate::teleport::normal_smoke::verify_finished(world);
    } else if scenario == "reserved-transfers" {
        crate::teleport::reservation_smoke::verify_finished(world);
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
    } else if scenario == "message-options" {
        super::message_options::verify_finished(world);
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
    } else if scenario == "battle-actions" {
        crate::battle::action_smoke::verify_finished(world);
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
