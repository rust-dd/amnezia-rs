use super::PictureCommand;
use bevy::prelude::*;

pub(crate) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 260 {
        for (id, x, keyed) in [(1, 230.0, 0), (2, 280.0, 1)] {
            world.write_message(PictureCommand::show(
                id,
                "Cross",
                x,
                60.0,
                &[0, 0, 0, 0, 0, 800, 0, keyed, 100, 100, 100, 100, 0, 60],
            ));
        }
        world.write_message(PictureCommand::show(
            3,
            "Staff1",
            74.0,
            120.0,
            &[3, 0, 74, 120, 0, 100, 0, 0, 100, 100, 100, 100, 0, 5497976],
        ));
    }
    (frame == 320).then_some("pictures-color-key")
}

pub(crate) fn verify_image(image: &Image, label: &str) {
    if label != "pictures-color-key" {
        return;
    }
    let at = |x, y| {
        image
            .get_color_at(x * image.width() / 320, y * image.height() / 240)
            .unwrap()
            .to_srgba()
            .to_u8_array()
    };
    let opaque = at(220, 50);
    let keyed = at(270, 50);
    for (actual, expected) in opaque[..3].iter().zip([32, 156, 0]) {
        assert!(
            actual.abs_diff(expected) <= 1,
            "opaque color key: {opaque:?}"
        );
    }
    assert_ne!(
        &keyed[..3],
        &opaque[..3],
        "keyed pixels must reveal the map"
    );
    assert_eq!(&at(10, 10)[..3], &[0, 0, 0], "opaque credits background");
    info!("picture color-key GPU assertions passed");
}
