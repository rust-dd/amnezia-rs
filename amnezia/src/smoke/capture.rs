use super::{offscreen, ui_layers};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

pub(super) fn capture(world: &mut World, label: &str) {
    let target = world.get_resource::<offscreen::Target>();
    let prefix = if target.is_some() {
        "amnezia-smoke-offscreen"
    } else {
        "amnezia-smoke"
    };
    let screenshot = target.map_or_else(Screenshot::primary_window, |target| {
        Screenshot::image(target.0.clone())
    });
    let path = std::env::temp_dir().join(format!("{prefix}-{label}.png"));
    info!("smoke screenshot: {}", path.display());
    let picture_pixels = crate::picture::smoke::expected_pixels(world, label);
    let display_snapshot = crate::display::smoke::capture_native(world, label, prefix);
    let animation_snapshot = crate::animation::smoke::snapshot(world, label);
    let water_snapshot = crate::world::water_smoke::snapshot(world, label);
    let transition_snapshot = crate::transitions::smoke::snapshot(world, label);
    let gameover_snapshot = crate::gameover::smoke::snapshot(world, label);
    let font_snapshot = crate::font::bitmap::smoke::snapshot(world, label);
    let arrow_snapshot = crate::battle::hud::arrow_snapshot(world, label);
    let cursor_snapshot = crate::battle::hud::cursor_snapshot(world, label);
    let movement_snapshot = crate::battle::hud::movement_snapshot(world, label);
    let battler_snapshot = crate::battle::battler_snapshot(world, label);
    let actor_snapshot = crate::appearance::smoke::snapshot(world, label);
    let map_animation_snapshot = crate::animation::map_smoke::snapshot(world, label);
    let world_tone_snapshot = crate::legacy_colors::world_smoke::snapshot(world, label);
    let map_flash_snapshot = crate::animation::map_flash_smoke::snapshot(world, label);
    let ui_layer_snapshot = ui_layers::snapshot(world, label);
    let menu_snapshot = crate::menu::layout_smoke::snapshot(world, label);
    let save_selector_snapshot = crate::menu::save_files::smoke::snapshot(world, label);
    let menu_font_snapshot = crate::menu::font_smoke::snapshot(world, label);
    let end_game_snapshot = crate::menu::end_smoke::snapshot(world, label);
    let title_snapshot = crate::title::smoke::snapshot(world, label);
    let saved_message_snapshot = crate::save::music_smoke::message::snapshot(world, label);
    let saved_erasure_snapshot = crate::save::music_smoke::erasure::snapshot(world, label);
    let saved_picture_snapshot = crate::save::picture_smoke::snapshot(world, label);
    let screen_flash_snapshot = crate::screenfx::flash_smoke::snapshot(world, label);
    let saved_screen_snapshot = crate::screenfx::saved::smoke::snapshot(world, label);
    let weather_snapshot = crate::screenfx::weather_smoke::snapshot(world, label);
    let saved_animation_snapshot = crate::animation::saved::smoke::snapshot(world, label);
    let dialogue_snapshot = crate::dialogue::font_smoke::snapshot(world, label);
    let dialogue_arrow_snapshot = crate::dialogue::timing_smoke::arrow_snapshot(world, label);
    let saved_npc_snapshot = crate::world::saved::smoke::snapshot(world, label);
    let saved_hero_snapshot = crate::save::hero_smoke::pixels::snapshot(world, label);
    let saved_vehicle_snapshot = crate::save::vehicle_smoke::pixels::snapshot(world, label);
    let label = label.to_owned();
    world.spawn(screenshot).observe(save_to_disk(path)).observe(
        move |capture: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
            verify_content(&capture.image, &label);
            if let Some(snapshot) = &save_selector_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &saved_erasure_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &saved_vehicle_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &saved_hero_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &saved_npc_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &dialogue_arrow_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &dialogue_snapshot {
                snapshot.verify(&capture.image);
            }
            crate::battle::smoke::verify_skin(&capture.image, &label);
            crate::picture::smoke::verify_image(&capture.image, &label, &picture_pixels);
            crate::legacy_colors::smoke::verify(&capture.image, &label);
            if let Some(snapshot) = &display_snapshot {
                snapshot.submit(&capture.image, false);
            }
            if let Some(snapshot) = &animation_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &water_snapshot {
                snapshot.verify(&capture.image, &label);
            }
            if let Some(snapshot) = &transition_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &gameover_snapshot {
                crate::gameover::smoke::verify_image(snapshot, &capture.image);
            }
            if let Some(snapshot) = &font_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &arrow_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &cursor_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &movement_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &battler_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &actor_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &map_animation_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &world_tone_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &map_flash_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &ui_layer_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &menu_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &menu_font_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &end_game_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &title_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &saved_message_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &saved_picture_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &screen_flash_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &saved_screen_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &weather_snapshot {
                snapshot.verify(&capture.image);
            }
            if let Some(snapshot) = &saved_animation_snapshot {
                snapshot.verify(&capture.image);
            }
        },
    );
}

pub(super) fn verify_content(image: &Image, label: &str) {
    if !matches!(label, "title" | "message-auto-top" | "message-fixed-bottom") {
        return;
    }
    let pixels = image.data.as_deref().expect("screenshot has no CPU pixels");
    assert!(
        has_visible_content(pixels),
        "{label} screenshot is blank; verify the rendering and desktop session before accepting this smoke test"
    );
}

fn has_visible_content(pixels: &[u8]) -> bool {
    let Some(background) = pixels.get(..3) else {
        return false;
    };
    pixels
        .chunks_exact(4)
        .filter(|pixel| &pixel[..3] != background && pixel[..3].iter().any(|&v| v > 16))
        .count()
        > pixels.len() / 400
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_black_and_solid_captures_cannot_pass_visual_smoke_checks() {
        assert!(!has_visible_content(&[]));
        assert!(!has_visible_content(&[0, 0, 0, 255].repeat(100)));
        assert!(!has_visible_content(&[255; 400]));
        let mut pixels = [0, 0, 0, 255].repeat(100);
        pixels[40..80].fill(255);
        assert!(has_visible_content(&pixels));
    }
}
