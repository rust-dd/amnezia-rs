use super::super::*;
use crate::font::bitmap::Run;
use crate::shop::{Mode, NumberState};

fn data() -> GameData {
    let mut app = App::new();
    app.add_plugins(crate::gamedata::GameDataPlugin);
    app.world_mut().remove_resource::<GameData>().unwrap()
}

#[test]
fn every_item_uses_the_original_buy_price_and_two_column_sell_count_positions() {
    let data = data();
    let font = BitmapFont::from_id(0);
    let mut inventory = Inventory::default();
    inventory.add_gold(50);
    for item in &data.items {
        inventory.add_item(item.id, item.id % 99 + 1);
    }
    let ids = data.items.iter().map(|item| item.id).collect::<Vec<_>>();
    for buying in [false, true] {
        let content = text::entries(&ids, buying, &data, &inventory, &font);
        assert_eq!(content.runs.len(), ids.len() * 3);
        assert_eq!(content.size.x, if buying { 168 } else { 304 });
        for (index, item) in data.items.iter().enumerate() {
            let columns = if buying { 1 } else { 2 };
            let x = (index % columns * 160) as i32;
            let y = (index / columns * 16 + 2) as i32;
            let enabled = if buying {
                item.price <= 50 && inventory.count(item.id) < 99
            } else {
                item.price > 0
            };
            let color = if enabled { 0 } else { 3 };
            assert_eq!(
                content.runs[index * 3],
                Run::clear(x, y, if buying { 168 } else { 144 }, 12)
            );
            assert_eq!(
                content.runs[index * 3 + 1],
                Run::new(&item.name, x, y, color)
            );
            let value = if buying {
                item.price.to_string()
            } else {
                format!(":{:>3}", inventory.count(item.id))
            };
            assert_eq!(
                content.runs[index * 3 + 2],
                Run::new(
                    &value,
                    if buying {
                        168 - value.len() as i32 * 6
                    } else {
                        x + 120
                    },
                    y,
                    color
                )
            );
        }
    }
}

#[test]
fn quantity_currency_and_status_use_original_bitmap_baselines_without_added_spaces() {
    let data = data();
    let font = BitmapFont::from_id(0);
    let mut terms = Terms::default();
    terms.0.gold = "G".into();
    terms.0.possessed_items = "Owned".into();
    terms.0.equipped_items = "Equipped".into();
    let number = NumberState {
        mode: Mode::Buy,
        item_id: 7,
        count: 15,
        max: 99,
        unit_price: 1200,
        origin: 0,
    };
    assert_eq!(
        text::quantity(&number, &data, &terms, &font).runs,
        [
            Run::new(&data.item(7).unwrap().name, 0, 34, 0),
            Run::new("x", 132, 34, 0),
            Run::new("15", 150, 34, 0),
            Run::new("G", 162, 66, 1),
            Run::new("18000", 132, 66, 0),
        ]
    );
    assert_eq!(
        text::status(99, 4, &terms, &font).runs,
        [
            Run::new("Owned", 0, 2, 1),
            Run::new("99", 108, 2, 0),
            Run::new("Equipped", 0, 18, 1),
            Run::new("4", 114, 18, 0),
        ]
    );
}

#[test]
fn portrait_offsets_all_four_command_lines_and_every_style_keeps_its_terms() {
    let mut app = App::new();
    app.add_plugins(crate::terms::TermsPlugin);
    let terms = app.world().resource::<Terms>();
    for style in 0..3 {
        for regreet in [false, true] {
            let state = ShopState {
                items: vec![],
                allow_buy: true,
                allow_sell: true,
                shop_type: style,
                phase: Phase::Command { cursor: 0, regreet },
                scene: default(),
            };
            let labels = terms.0.shop_set(style);
            let expected = [
                if regreet {
                    labels.regreeting
                } else {
                    labels.greeting
                },
                labels.buy,
                labels.sell,
                labels.leave,
            ];
            for face in [false, true] {
                let message = text::message(&state, face, terms);
                for (index, run) in message.runs.iter().enumerate() {
                    assert_eq!(run.text, expected[index]);
                    assert_eq!(
                        run.position,
                        IVec2::new(
                            if face { 72 } else { 0 } + if index > 0 { 12 } else { 0 },
                            2 + index as i32 * 16
                        )
                    );
                }
            }
        }
    }
}

#[test]
fn phase_visibility_has_no_map_gaps_or_overlapping_central_windows() {
    for (phase, expected) in [
        (
            Phase::Command {
                cursor: 0,
                regreet: false,
            },
            vec![Window::Help, Window::Message, Window::EmptyCenter],
        ),
        (
            Phase::Buy { cursor: 0 },
            vec![
                Window::Help,
                Window::Buy,
                Window::Party,
                Window::Status,
                Window::Gold,
                Window::Message,
            ],
        ),
        (
            Phase::Sell { cursor: 0 },
            vec![Window::Help, Window::Sell, Window::Message],
        ),
        (
            Phase::Number(NumberState {
                mode: Mode::Sell,
                item_id: 7,
                count: 1,
                max: 9,
                unit_price: 600,
                origin: 0,
            }),
            vec![
                Window::Help,
                Window::Number,
                Window::Party,
                Window::Status,
                Window::Gold,
                Window::Message,
            ],
        ),
        (
            Phase::Sold {
                remaining: 60,
                cursor: 0,
                item_id: 7,
            },
            vec![
                Window::Help,
                Window::Party,
                Window::Status,
                Window::Gold,
                Window::Message,
                Window::EmptyLeft,
            ],
        ),
    ] {
        let mut coverage = vec![0; 320 * 240];
        for (id, x, y, width, height) in layout::WINDOWS {
            assert_eq!(layout::shown(id, &phase), expected.contains(&id));
            if layout::shown(id, &phase) {
                for row in y as u32..y as u32 + height {
                    for col in x as u32..x as u32 + width {
                        coverage[(row * 320 + col) as usize] += 1;
                    }
                }
            }
        }
        assert!(coverage.into_iter().all(|value| value == 1));
    }
}
