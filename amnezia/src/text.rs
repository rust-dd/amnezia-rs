//! Control-code expansion into glyphs and reveal markers; [`substitute`] keeps
//! only plain text for surfaces without typewriter timing.

use crate::state::Variables;
use bevy::prelude::Resource;

/// Actor 1's runtime name, shared by messages, menus, battles and saves.
#[derive(Resource)]
pub struct HeroName(pub String);

impl HeroName {
    pub(crate) fn actor<'a>(&'a self, actor: &'a amnezia_data::ActorDef) -> &'a str {
        if actor.id == 1 { &self.0 } else { &actor.name }
    }
}

/// Legacy message segment with names/variables expanded. This parser drops colour
/// and unknown codes; the dialogue typewriter has its own token handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// A glyph to reveal (a literal char, or one expanded from `\N`/`\V`/`\_`).
    Char(char),
    /// `\s[n]`: set the reveal speed (clamped to RM2000's `1..=20`).
    Speed(u8),
    /// `\.`: pause the reveal for a quarter second (RM2000 waits 16 frames).
    QuarterPause,
    /// `\|`: pause the reveal for one second (RM2000 waits 61 frames).
    FullPause,
    /// `\!`: pause mid-text until the player presses the confirm key.
    WaitKey,
    /// `\^`: close the page automatically once revealed, without waiting for a key.
    KillPage,
    /// `\>`: reveal the following text instantly until [`Segment::InstantOff`].
    InstantOn,
    /// `\<`: end an instant-speed run started by [`Segment::InstantOn`].
    InstantOff,
}

/// Parse `raw` into its [`Segment`] stream, expanding `\N[k]` to `hero` and
/// `\V[k]` to variable `k`'s value. `\\` becomes a single backslash; `\_` a
/// space; the reveal-timing codes become their markers. `\C[..]` (colour) and any
/// other unrecognised `\X`/`\X[..]` are consumed and dropped. A dangling trailing
/// backslash is dropped.
pub fn parse_segments(raw: &str, hero: &str, variables: &Variables) -> Vec<Segment> {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '\\' {
            out.push(Segment::Char(c));
            i += 1;
            continue;
        }
        let Some(&code) = chars.get(i + 1) else {
            break;
        };
        if code == '\\' {
            out.push(Segment::Char('\\'));
            i += 2;
            continue;
        }
        i += 2;
        match code {
            'N' | 'n' => {
                let _ = read_bracket(&chars, &mut i);
                out.extend(hero.chars().map(Segment::Char));
            }
            'V' | 'v' => {
                let arg = read_bracket(&chars, &mut i);
                let id = arg.and_then(|a| a.trim().parse::<u32>().ok()).unwrap_or(0);
                out.extend(variables.get(id).to_string().chars().map(Segment::Char));
            }
            'S' | 's' => {
                let arg = read_bracket(&chars, &mut i);
                let n = arg.and_then(|a| a.trim().parse::<u8>().ok()).unwrap_or(1);
                out.push(Segment::Speed(n.clamp(1, 20)));
            }
            'C' | 'c' => {
                let _ = read_bracket(&chars, &mut i);
            }
            '.' => out.push(Segment::QuarterPause),
            '|' => out.push(Segment::FullPause),
            '!' => out.push(Segment::WaitKey),
            '^' => out.push(Segment::KillPage),
            '>' => out.push(Segment::InstantOn),
            '<' => out.push(Segment::InstantOff),
            '_' => out.push(Segment::Char(' ')),
            _ => {
                let _ = read_bracket(&chars, &mut i);
            }
        }
    }
    out
}

/// Plain-text projection with name/variable expansion but no reveal-timing markers.
pub fn substitute(raw: &str, hero: &str, variables: &Variables) -> String {
    parse_segments(raw, hero, variables)
        .into_iter()
        .filter_map(|s| match s {
            Segment::Char(c) => Some(c),
            _ => None,
        })
        .collect()
}

/// If a `[...]` group begins at `*i`, consume it (advancing `*i` past the
/// closing `]`, or to the end if unterminated) and return its inner text.
/// Otherwise leave `*i` unchanged and return `None`.
fn read_bracket(chars: &[char], i: &mut usize) -> Option<String> {
    if chars.get(*i) != Some(&'[') {
        return None;
    }
    let mut j = *i + 1;
    let mut inner = String::new();
    while j < chars.len() && chars[j] != ']' {
        inner.push(chars[j]);
        j += 1;
    }
    *i = if j < chars.len() { j + 1 } else { j };
    Some(inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(u32, i32)]) -> Variables {
        let mut v = Variables::default();
        for &(id, value) in pairs {
            v.set(id, value);
        }
        v
    }

    #[test]
    fn expands_hero_name() {
        assert_eq!(substitute("\\N[1]", "Ron", &vars(&[])), "Ron");
    }

    #[test]
    fn expands_hero_name_for_any_actor_index() {
        assert_eq!(substitute("\\N[3]", "Ron", &vars(&[])), "Ron");
    }

    #[test]
    fn expands_variable_value() {
        assert_eq!(substitute("\\V[5]", "Ron", &vars(&[(5, 42)])), "42");
    }

    #[test]
    fn missing_variable_reads_as_zero() {
        assert_eq!(substitute("\\V[9]", "Ron", &vars(&[])), "0");
    }

    #[test]
    fn strips_speed_code() {
        assert_eq!(substitute("a\\S[2]b", "Ron", &vars(&[])), "ab");
    }

    #[test]
    fn strips_pause_code() {
        assert_eq!(substitute("a\\|b", "Ron", &vars(&[])), "ab");
    }

    #[test]
    fn unescapes_double_backslash() {
        assert_eq!(substitute("a\\\\b", "Ron", &vars(&[])), "a\\b");
    }

    #[test]
    fn strips_unknown_code_with_bracket() {
        assert_eq!(substitute("a\\C[2]b", "Ron", &vars(&[])), "ab");
    }

    #[test]
    fn strips_unknown_code_without_bracket() {
        assert_eq!(substitute("a\\$b", "Ron", &vars(&[])), "ab");
    }

    #[test]
    fn leaves_plain_text_untouched() {
        assert_eq!(
            substitute("Beszélt Adéllal", "Ron", &vars(&[])),
            "Beszélt Adéllal"
        );
    }

    #[test]
    fn expands_multiple_codes_in_one_line() {
        let out = substitute("\\N[1] talált \\V[1] aranyat", "Ron", &vars(&[(1, 7)]));
        assert_eq!(out, "Ron talált 7 aranyat");
    }

    #[test]
    fn drops_dangling_backslash() {
        assert_eq!(substitute("done\\", "Ron", &vars(&[])), "done");
    }

    #[test]
    fn parses_full_pause_between_characters() {
        assert_eq!(
            parse_segments("a\\|b", "Ron", &vars(&[])),
            vec![Segment::Char('a'), Segment::FullPause, Segment::Char('b')]
        );
    }

    #[test]
    fn parses_quarter_pause() {
        assert_eq!(
            parse_segments("a\\.b", "Ron", &vars(&[])),
            vec![
                Segment::Char('a'),
                Segment::QuarterPause,
                Segment::Char('b')
            ]
        );
    }

    #[test]
    fn parses_speed_and_kill_page() {
        assert_eq!(
            parse_segments("\\s[5]x\\^", "Ron", &vars(&[])),
            vec![Segment::Speed(5), Segment::Char('x'), Segment::KillPage]
        );
    }

    #[test]
    fn speed_clamps_into_range() {
        assert_eq!(
            parse_segments("\\s[99]", "Ron", &vars(&[])),
            vec![Segment::Speed(20)]
        );
    }

    #[test]
    fn parses_wait_key_and_instant_run() {
        assert_eq!(
            parse_segments("\\>hi\\<\\!", "Ron", &vars(&[])),
            vec![
                Segment::InstantOn,
                Segment::Char('h'),
                Segment::Char('i'),
                Segment::InstantOff,
                Segment::WaitKey,
            ]
        );
    }

    #[test]
    fn expands_name_into_char_segments() {
        assert_eq!(
            parse_segments("\\N[1]!", "Ron", &vars(&[])),
            vec![
                Segment::Char('R'),
                Segment::Char('o'),
                Segment::Char('n'),
                Segment::Char('!'),
            ]
        );
    }
}
