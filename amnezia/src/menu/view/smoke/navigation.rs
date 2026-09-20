use super::*;

pub(super) fn snapshot(world: &World, label: &str) -> Option<Snapshot> {
    let (rect, source) = match label {
        "menu-navigation-command-last" => ((4, 72, 80, 16), 64),
        "menu-navigation-end-yes" => ((144, 128, 32, 16), 96),
        "menu-navigation-member-last" => ((148, 182, 168, 48), 96),
        "menu-navigation-single-member" => ((148, 8, 168, 48), 96),
        _ => return None,
    };
    let handle = world
        .resource::<AssetServer>()
        .load("graphics/System/System.png");
    let skin = world.resource::<Assets<Image>>().get(&handle).unwrap();
    let mut pixels = Vec::new();
    border(&mut pixels, skin, rect, source, true);
    Some(Snapshot {
        pixels,
        checks: crate::menu::navigation_smoke::pictures(world),
    })
}
