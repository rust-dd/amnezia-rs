use super::*;

mod bitmap_text;

#[test]
fn merchant_uses_original_window_sizes_and_bitmap_text() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .add_systems(Startup, spawn_ui);
    app.update();
    let world = app.world_mut();
    let mut sizes = world
        .query::<&crate::windowskin::background::Pixels>()
        .iter(world)
        .map(|pixels| (pixels.0.x, pixels.0.y))
        .collect::<Vec<_>>();
    sizes.sort();
    assert_eq!(
        sizes,
        [
            (136, 32),
            (136, 48),
            (136, 48),
            (184, 128),
            (184, 128),
            (184, 128),
            (320, 32),
            (320, 80),
            (320, 128),
            (320, 128)
        ]
    );
    assert_eq!(world.query::<&Text>().iter(world).count(), 0);
    assert_eq!(
        world
            .query::<&crate::font::bitmap::PixelText>()
            .iter(world)
            .count(),
        7
    );
}

#[test]
fn selling_uses_its_own_original_quantity_prompt() {
    let mut terms = Terms::default();
    terms.0.shop_buy_number1 = "BUY COUNT".into();
    terms.0.shop_sell_number1 = "SELL COUNT".into();
    let state = ShopState {
        items: vec![],
        allow_buy: true,
        allow_sell: true,
        shop_type: 0,
        scene: default(),
        phase: Phase::Number(super::super::NumberState {
            mode: crate::shop::Mode::Sell,
            item_id: 1,
            count: 1,
            max: 9,
            unit_price: 5,
            origin: 0,
        }),
    };
    let text = text::message(&state, false, &terms);
    assert_eq!(text.runs.len(), 1);
    assert_eq!(text.runs[0].text, "SELL COUNT");
}
