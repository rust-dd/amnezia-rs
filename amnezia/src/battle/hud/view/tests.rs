use super::*;

#[test]
fn every_battle_panel_owns_a_native_background_of_its_original_dimensions() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .add_systems(Startup, spawn);
    app.world_mut().spawn(crate::battle::HudCamera);
    app.update();
    let world = app.world_mut();
    let panels = world
        .query::<(&Panel, &Children)>()
        .iter(world)
        .map(|(panel, children)| (*panel, children.to_vec()))
        .collect::<Vec<_>>();
    assert_eq!(panels.len(), 8);
    for (panel, children) in panels {
        let size = match panel {
            Panel::Option | Panel::Command => (76, 80),
            Panel::Help => (320, 32),
            Panel::Status => (244, 80),
            Panel::Target => (136, 80),
            Panel::Item | Panel::Skill | Panel::Message => (320, 80),
        };
        let rasters = children
            .iter()
            .filter_map(|entity| world.get::<crate::windowskin::background::Pixels>(*entity))
            .collect::<Vec<_>>();
        assert_eq!(rasters.len(), 1, "{panel:?}");
        assert_eq!(rasters[0].0, UVec2::new(size.0, size.1), "{panel:?}");
    }
}
