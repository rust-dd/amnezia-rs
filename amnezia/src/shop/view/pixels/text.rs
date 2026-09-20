use super::*;

type Layer = ((u32, u32, u32, u32), PixelText, u32);

pub(super) fn contents(world: &World, state: &ShopState, face: bool) -> Vec<Layer> {
    let data = world.resource::<GameData>();
    let inventory = world.resource::<Inventory>();
    let terms = &world.resource::<Terms>().0;
    let font = world.resource::<BitmapFont>();
    let mut layers = Vec::new();
    let mut add = |rect: (u32, u32, u32, u32), height, offset, runs| {
        layers.push((
            rect,
            PixelText {
                size: UVec2::new(rect.2, height),
                runs,
            },
            offset,
        ));
    };
    let help = data
        .item(state.scene.help_id)
        .map_or("", |item| &item.description);
    add((8, 8, 304, 16), 16, 0, vec![Run::new(help, 0, 2, 0)]);
    let labels = terms.shop_set(state.shop_type);
    let x = if face { 72 } else { 0 };
    let message = match &state.phase {
        Phase::Command { regreet, .. } => [
            if *regreet {
                labels.regreeting
            } else {
                labels.greeting
            },
            labels.buy,
            labels.sell,
            labels.leave,
        ]
        .into_iter()
        .enumerate()
        .map(|(row, value)| {
            Run::new(
                value,
                x + if row > 0 { 12 } else { 0 },
                row as i32 * 16 + 2,
                0,
            )
        })
        .collect(),
        phase => vec![Run::new(
            match phase {
                Phase::Buy { .. } => labels.buy_select,
                Phase::Sell { .. } => labels.sell_select,
                Phase::Number(number) if number.mode == Mode::Buy => labels.number,
                Phase::Number(_) => labels.sell_number,
                Phase::Bought { .. } => labels.purchased,
                Phase::Sold { .. } => labels.sold,
                _ => unreachable!(),
            },
            x,
            2,
            0,
        )],
    };
    add((8, 168, 304, 64), 64, 0, message);
    if matches!(state.phase, Phase::Buy { .. } | Phase::Sell { .. }) {
        let buying = matches!(state.phase, Phase::Buy { .. });
        let items = if buying {
            state
                .items
                .iter()
                .filter_map(|id| data.item(*id))
                .collect::<Vec<_>>()
        } else {
            data.items
                .iter()
                .filter(|item| inventory.count(item.id) > 0)
                .collect()
        };
        let mut runs = Vec::new();
        for (index, item) in items.iter().enumerate() {
            let (x, y) = if buying {
                (0, index as i32 * 16 + 2)
            } else {
                (index as i32 % 2 * 160, index as i32 / 2 * 16 + 2)
            };
            let color = if if buying {
                item.price as i32 > inventory.gold() || inventory.count(item.id) == 99
            } else {
                item.price == 0
            } {
                3
            } else {
                0
            };
            runs.push(Run::clear(x, y, if buying { 168 } else { 144 }, 12));
            runs.push(Run::new(&item.name, x, y, color));
            let value = if buying {
                item.price.to_string()
            } else {
                format!(":{:>3}", inventory.count(item.id))
            };
            runs.push(Run::new(
                &value,
                if buying {
                    168 - font.width(&value)
                } else {
                    x + 120
                },
                y,
                color,
            ));
        }
        let (width, offset, rows) = if buying {
            (168, state.scene.buy.offset, items.len())
        } else {
            (304, state.scene.sell.offset, items.len().div_ceil(2))
        };
        add(
            (8, 40, width, 112),
            rows.max(7) as u32 * 16,
            offset as u32,
            runs,
        );
    }
    if let Phase::Number(number) = &state.phase {
        let name = &data.item(number.item_id).unwrap().name;
        let count = number.count.to_string();
        let mut runs = vec![
            Run::new(name, 0, 34, 0),
            Run::new("x", 132, 34, 0),
            Run::new(&count, 162 - font.width(&count), 34, 0),
        ];
        runs.extend(currency(
            number.unit_price * number.count as i32,
            &terms.gold,
            168,
            66,
            font,
        ));
        add((8, 40, 168, 112), 112, 0, runs);
    }
    if !matches!(state.phase, Phase::Command { .. } | Phase::Sell { .. }) {
        let worn = world.resource::<crate::equipment::Equipment>();
        let equipped = world
            .resource::<Party>()
            .snapshot()
            .iter()
            .filter_map(|id| data.actor(*id))
            .flat_map(|actor| worn.slots(actor))
            .filter(|id| *id != 0 && *id == state.scene.item_id)
            .count();
        let mut runs = Vec::new();
        for (label, value, y) in [
            (
                &terms.possessed_items,
                inventory.count(state.scene.item_id).to_string(),
                2,
            ),
            (&terms.equipped_items, equipped.to_string(), 18),
        ] {
            runs.push(Run::new(label, 0, y, 1));
            runs.push(Run::new(&value, 120 - font.width(&value), y, 0));
        }
        add((192, 88, 120, 32), 32, 0, runs);
        add(
            (192, 136, 120, 16),
            16,
            0,
            currency(inventory.gold(), &terms.gold, 120, 2, font),
        );
    }
    layers
}

fn currency(amount: i32, label: &str, right: i32, y: i32, font: &BitmapFont) -> Vec<Run> {
    let value = amount.to_string();
    vec![
        Run::new(label, right - font.width(label), y, 1),
        Run::new(&value, right - font.width(label) - font.width(&value), y, 0),
    ]
}
