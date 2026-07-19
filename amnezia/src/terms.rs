//! The real RM2000 vocabulary loaded as a shared resource: the menu command
//! labels, status / equipment labels, the currency term, and the battle / shop /
//! inn message terms, read from `terms.ron` (produced by `amnezia-convert` from
//! the original `RPG_RT.ldb` Terms section). The menu, battle, and shop chrome
//! read these in place of the invented Hungarian placeholders and route each
//! through [`crate::i18n::tr`], so they localise to English like the rest of the
//! Hungarian source text and stay live under the F2 toggle.

use crate::assets::asset_root;
use crate::i18n;
use amnezia_data::TermsDef;
use bevy::prelude::*;

/// The game's term vocabulary, loaded once at startup. Wraps the converted
/// [`TermsDef`]; the chrome reads its fields through [`Terms::label`].
#[derive(Resource, Default)]
pub struct Terms(pub TermsDef);

impl Terms {
    /// The localised display string for a parsed `term`, falling back to
    /// `fallback` (the faithful Hungarian placeholder) when the original database
    /// left the term blank or `terms.ron` failed to load. Both paths go through
    /// [`i18n::tr`], so English mode localises whichever string is shown.
    pub fn label(&self, term: &str, fallback: &str) -> String {
        i18n::tr(if term.is_empty() { fallback } else { term })
    }
}

pub struct TermsPlugin;

impl Plugin for TermsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(load_terms());
    }
}

/// Load `terms.ron` into a [`Terms`] resource, falling back to an empty
/// vocabulary (so the chrome shows its Hungarian placeholders) on any error — a
/// missing or malformed terms file never blocks startup.
fn load_terms() -> Terms {
    let path = format!("{}/terms.ron", asset_root());
    let terms = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| ron::from_str::<TermsDef>(&text).ok())
        .unwrap_or_default();
    Terms(terms)
}
