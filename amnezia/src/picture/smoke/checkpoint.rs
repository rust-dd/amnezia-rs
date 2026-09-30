use super::*;

pub(crate) fn apply_original_ship_fog(world: &mut World, pixels: &mut [(u32, u32, [u8; 4])]) {
    let picture = world.query::<&Picture>().single(world).unwrap();
    assert_eq!((picture.id, picture.name.as_str()), (1, "Fog"));
    assert_eq!(
        (picture.x, picture.y, picture.zoom, picture.transparency),
        (160.0, 120.0, 120.0, 60.0)
    );
    assert!(!picture.fixed_to_map && !picture.use_transparent_color);
    let phase = picture
        .effect
        .wave(Vec2::new(320.0, 242.0), 1.2, Vec2::new(160.0, 120.0))
        .unwrap()
        .uniform
        .x as f64;
    let handle = world
        .resource::<AssetServer>()
        .load::<Image>(crate::assets::resolve_png("Picture", "Fog"));
    let source = world.resource::<Assets<Image>>().get(&handle).unwrap();
    for (x, y, color) in pixels {
        let offset = (4.8 * (phase + f64::from(*y) * std::f64::consts::TAU / 38.4).sin()).trunc();
        let inverse_zoom = 54613.0 / 65536.0;
        let sx = ((f64::from(*x) + 32.5 - offset) * inverse_zoom).ceil() as u32 - 1;
        let sy = ((f64::from(*y) + 25.5) * inverse_zoom).ceil() as u32 - 1;
        let fog = source
            .get_color_at(sx, sy)
            .unwrap()
            .to_srgba()
            .to_u8_array();
        for channel in 0..3 {
            color[channel] =
                (f32::from(color[channel]) * 0.6 + f32::from(fog[channel]) * 0.4).round() as u8;
        }
    }
}
