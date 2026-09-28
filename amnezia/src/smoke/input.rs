use super::SmokeRun;
use bevy::prelude::*;

#[cfg(test)]
mod tests;

/// Keeps scripted holds independent of native keyboard and focus events.
#[derive(Resource, Default)]
pub(super) struct ScriptedKeys(ButtonInput<KeyCode>);

pub(super) fn input(world: &mut World) {
    world.resource_scope(|world, mut scripted: Mut<ScriptedKeys>| {
        scripted.0.clear();
        *world.resource_mut::<ButtonInput<KeyCode>>() = scripted.0.clone();
        run_script(world);
        scripted.0 = world.resource::<ButtonInput<KeyCode>>().clone();
    });
}

fn run_script(world: &mut World) {
    let frame = world.resource::<SmokeRun>().frame;
    let scenario = world.resource::<SmokeRun>().scenario;
    if scenario == "inn" && crate::shop::inn::smoke::input(world, frame) {
        return;
    }
    if scenario == "shop" && crate::shop::smoke::input(world, frame) {
        return;
    }
    if scenario.starts_with("save-")
        && scenario != "save-slots"
        && crate::menu::save_files::smoke::event_input(world)
    {
        return;
    }
    if scenario == "dialogue-timing" && crate::dialogue::timing_smoke::held_input(world, frame) {
        return;
    }
    if scenario == "items" && crate::menu::target_navigation_smoke::held_input(world, frame) {
        return;
    }
    if scenario == "skills" && crate::menu::skill_smoke::held_input(world, frame) {
        return;
    }
    if scenario == "equipment" && crate::menu::equipment_smoke::held_input(world, frame) {
        return;
    }
    if scenario == "menu" && crate::menu::navigation_smoke::held_input(world, frame) {
        return;
    }
    if scenario == "return-title" && crate::title::smoke::held_input(world, frame) {
        return;
    }
    let requested = if scenario == "save-slots" {
        crate::menu::save_files::smoke::input(frame)
    } else if scenario == "items" {
        crate::menu::item_smoke::input(frame)
    } else if scenario == "equipment" {
        crate::menu::equipment_smoke::input(frame)
    } else if scenario == "skills" {
        crate::menu::skill_smoke::input(frame)
    } else if scenario == "load-slots" {
        crate::menu::save_files::smoke::load::input(frame)
    } else if matches!(scenario, "gameover" | "battle-defeat") {
        crate::gameover::smoke::input(world, frame, scenario == "battle-defeat")
    } else if scenario == "return-title" {
        crate::title::smoke::return_input(frame)
    } else if scenario == "battle-menus" {
        crate::battle::smoke::input(frame)
    } else if scenario == "menu" {
        crate::menu::layout_smoke::input(frame)
    } else if scenario == "save-screen" {
        crate::save::screen_smoke::input(frame)
    } else if scenario == "weather" {
        crate::screenfx::weather_smoke::input(frame)
    } else if scenario == "dialogue-timing" {
        crate::dialogue::timing_smoke::input(frame)
    } else if scenario == "screen-events" {
        crate::screenfx::flash_smoke::input(frame)
    } else {
        None
    };
    let advance = frame > 90
        && (world.resource::<SmokeRun>().scenario != "battle-events" || frame > 360)
        && world.resource::<SmokeRun>().finish_at.is_none()
        && !matches!(
            world.resource::<SmokeRun>().scenario,
            "return-title"
                | "load-slots"
                | "save-slots"
                | "items"
                | "skills"
                | "equipment"
                | "save-music"
                | "save-camera"
                | "save-npcs"
                | "save-hero"
                | "save-vehicles"
                | "save-pictures"
                | "save-screen"
                | "save-weather"
                | "save-animations"
                | "weather"
                | "gameover"
                | "battle-defeat"
                | "font"
                | "panorama"
                | "timer"
                | "battle-menus"
                | "battle-transitions"
                | "message-options"
                | "dialogue-timing"
                | "display"
                | "actor-names"
                | "ui-layers"
                | "normal-transfers"
                | "reserved-transfers"
                | "async-transitions"
                | "async-inns"
        )
        && frame.is_multiple_of(15)
        && (world.resource::<crate::dialogue::Dialogue>().active
            || world.resource::<crate::battle::BattleActive>().0);
    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    *keys = ButtonInput::default();
    if advance {
        keys.press(KeyCode::Enter);
    }
    if let Some(key) = requested {
        keys.press(key);
    }
}
