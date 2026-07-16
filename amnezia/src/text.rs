//! Expansion of RM2000 message control codes into display text. Message
//! strings embed codes like `\N[1]` (insert an actor's name) or `\V[3]` (insert
//! a variable's value); this module turns them into the text the box shows.

use crate::state::Variables;
use bevy::prelude::Resource;

/// The hero's name, loaded from `hero.ron` at startup and inserted into the
/// `\N[k]` control code by [`substitute`].
#[derive(Resource)]
pub struct HeroName(pub String);

/// Expand RM2000 message control codes in `raw` into display text.
///
/// `\N[k]` inserts the hero's name (the game has only actor 1, so every `k`
/// yields `hero`); `\V[k]` inserts variable `k`'s current value; `\\` becomes a
/// single backslash. The text-speed (`\S[..]`) and full-second pause (`\|`)
/// codes have no analogue in this remake and are stripped, as is any other
/// unrecognised `\X` or `\X[..]`.
pub fn substitute(raw: &str, hero: &str, variables: &Variables) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '\\' {
            out.push(c);
            i += 1;
            continue;
        }
        let Some(&code) = chars.get(i + 1) else {
            break;
        };
        if code == '\\' {
            out.push('\\');
            i += 2;
            continue;
        }
        i += 2;
        let arg = read_bracket(&chars, &mut i);
        match code.to_ascii_uppercase() {
            'N' => out.push_str(hero),
            'V' => {
                let id = arg.and_then(|a| a.trim().parse::<u32>().ok()).unwrap_or(0);
                out.push_str(&variables.get(id).to_string());
            }
            _ => {}
        }
    }
    out
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
        assert_eq!(substitute("Beszélt Adéllal", "Ron", &vars(&[])), "Beszélt Adéllal");
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
}
