use super::*;
use crate::font::bitmap::{DEFAULT, DISABLED, Run};
use crate::i18n::tr;
use crate::shop::{Mode, NumberState, messages};

fn text(width: u32, height: u32, runs: Vec<Run>) -> PixelText {
    PixelText {
        size: UVec2::new(width, height),
        runs,
    }
}

pub(super) fn help(id: u32, data: &GameData) -> PixelText {
    text(
        304,
        16,
        vec![Run::new(
            data.item(id)
                .map(|item| tr(&item.description))
                .unwrap_or_default(),
            0,
            2,
            DEFAULT,
        )],
    )
}

pub(super) fn entries(
    ids: &[u32],
    buying: bool,
    data: &GameData,
    inventory: &Inventory,
    font: &BitmapFont,
) -> PixelText {
    let columns = if buying { 1 } else { 2 };
    let width = if buying { 168 } else { 304 };
    let mut runs = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        let item = data.item(*id).unwrap();
        let x = (index % columns * 160) as i32;
        let y = (index / columns * 16 + 2) as i32;
        let enabled = if buying {
            crate::shop::logic::can_buy(item.price, inventory.gold(), inventory.count(*id))
        } else {
            item.price > 0
        };
        let color = if enabled { DEFAULT } else { DISABLED };
        runs.push(Run::clear(x, y, if buying { 168 } else { 144 }, 12));
        runs.push(Run::new(tr(&item.name), x, y, color));
        let value = if buying {
            item.price.to_string()
        } else {
            format!(":{:>3}", inventory.count(*id))
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
    text(width, ids.len().div_ceil(columns).max(7) as u32 * 16, runs)
}

pub(super) fn currency(
    amount: i32,
    edge: i32,
    y: i32,
    terms: &Terms,
    font: &BitmapFont,
) -> Vec<Run> {
    let label = tr(&terms.0.gold);
    let amount = amount.to_string();
    vec![
        Run::new(&label, edge - font.width(&label), y, 1),
        Run::new(
            &amount,
            edge - font.width(&label) - font.width(&amount),
            y,
            DEFAULT,
        ),
    ]
}

pub(super) fn gold(amount: i32, terms: &Terms, font: &BitmapFont) -> PixelText {
    text(120, 16, currency(amount, 120, 2, terms, font))
}

pub(super) fn quantity(
    number: &NumberState,
    data: &GameData,
    terms: &Terms,
    font: &BitmapFont,
) -> PixelText {
    let mut runs = vec![
        Run::new(
            data.item(number.item_id)
                .map(|item| tr(&item.name))
                .unwrap_or_default(),
            0,
            34,
            DEFAULT,
        ),
        Run::new("x", 132, 34, DEFAULT),
        Run::new(
            number.count.to_string(),
            162 - font.width(&number.count.to_string()),
            34,
            DEFAULT,
        ),
    ];
    runs.extend(currency(
        number.unit_price * number.count as i32,
        168,
        66,
        terms,
        font,
    ));
    text(168, 112, runs)
}

pub(super) fn status(owned: u32, equipped: usize, terms: &Terms, font: &BitmapFont) -> PixelText {
    let mut runs = Vec::new();
    for (y, label, value) in [
        (2, &terms.0.possessed_items, owned.to_string()),
        (18, &terms.0.equipped_items, equipped.to_string()),
    ] {
        runs.push(Run::new(tr(label), 0, y, 1));
        runs.push(Run::new(&value, 120 - font.width(&value), y, DEFAULT));
    }
    text(120, 32, runs)
}

pub(super) fn message(state: &ShopState, face: bool, terms: &Terms) -> PixelText {
    let labels = messages::shop_vocab(state.shop_type, terms);
    let x = if face { 72 } else { 0 };
    let runs = match &state.phase {
        Phase::Command { regreet, .. } => vec![
            Run::new(
                if *regreet {
                    labels.regreeting
                } else {
                    labels.greeting
                },
                x,
                2,
                DEFAULT,
            ),
            Run::new(labels.buy, x + 12, 18, DEFAULT),
            Run::new(labels.sell, x + 12, 34, DEFAULT),
            Run::new(labels.leave, x + 12, 50, DEFAULT),
        ],
        phase => vec![Run::new(
            match phase {
                Phase::Buy { .. } => labels.buy_select,
                Phase::Sell { .. } => labels.sell_select,
                Phase::Number(number) if number.mode == Mode::Buy => labels.number,
                Phase::Number(_) => labels.sell_number,
                Phase::Bought { .. } => labels.purchased,
                Phase::Sold { .. } => labels.sold,
                Phase::Command { .. } => unreachable!(),
            },
            x,
            2,
            DEFAULT,
        )],
    };
    text(304, 64, runs)
}
