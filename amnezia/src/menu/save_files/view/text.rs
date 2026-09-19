use super::*;
use crate::font::bitmap::{DEFAULT, DISABLED, KNOCKOUT};
use crate::i18n;

fn short(value: &str) -> String {
    i18n::tr(value)
        .chars()
        .chain(std::iter::repeat(' '))
        .take(2)
        .collect()
}

pub(super) fn row(
    index: usize,
    contents: &Contents,
    mode: Mode,
    terms: &Terms,
    font: &BitmapFont,
) -> Vec<Run> {
    let label = i18n::tr(&terms.0.file);
    let color = if mode == Mode::Load && !matches!(contents, Contents::Party(_)) {
        DISABLED
    } else {
        DEFAULT
    };
    let mut runs = vec![
        Run::new(&label, 4, 2, color),
        Run::new(
            format!("{:>2}", index + 1),
            4 + font.width(&label) + 3,
            2,
            color,
        ),
    ];
    match contents {
        Contents::Empty => {}
        Contents::Corrupt => runs.push(Run::new("Savegame corrupted", 4, 18, KNOCKOUT)),
        Contents::Party(party) => {
            let level = short(&terms.0.lvl_short);
            let hp = short(&terms.0.hp_short);
            runs.extend([
                Run::new(&party.name, 4, 18, DEFAULT),
                Run::new(&level, 4, 34, 1),
                Run::new(
                    format!("{:>2}", party.level),
                    4 + font.width(&level),
                    34,
                    DEFAULT,
                ),
                Run::new(&hp, 46, 34, 1),
                Run::new(
                    format!("{:>3}", party.hp),
                    46 + font.width(&hp),
                    34,
                    DEFAULT,
                ),
            ]);
        }
    }
    runs
}
