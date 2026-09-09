use bevy::prelude::*;

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
