use super::*;

pub(crate) fn save_resources(location: PathBuf) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Dialogue>()
        .init_resource::<Fade>()
        .init_resource::<PendingTeleport>()
        .init_resource::<Switches>()
        .init_resource::<Variables>()
        .init_resource::<Party>()
        .init_resource::<Inventory>()
        .init_resource::<LoadRequest>()
        .init_resource::<LoadOutcome>()
        .init_resource::<SaveRequest>()
        .init_resource::<EventSaveRequest>()
        .init_resource::<Vitals>()
        .init_resource::<Progression>()
        .init_resource::<Equipment>()
        .init_resource::<Weather>()
        .init_resource::<WeatherStrength>()
        .init_resource::<TintState>()
        .init_resource::<PlayTime>()
        .init_resource::<GameClock>()
        .init_resource::<crate::timing::GameFrames>();
    app.insert_resource(SaveLocation(location));
    app.insert_resource(HeroName("Ron".into()));
    app
}
