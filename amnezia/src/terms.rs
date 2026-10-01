//! Original RM2000 vocabulary from `terms.ron`, localized through [`crate::i18n::tr`].

use crate::assets::asset_root;
use crate::i18n;
use amnezia_data::TermsDef;
use bevy::prelude::*;

/// The game's term vocabulary, loaded once at startup. Wraps the converted
/// [`TermsDef`]; the chrome reads its fields through [`Terms::label`].
#[derive(Resource, Default)]
pub struct Terms(pub TermsDef);

impl Terms {
    /// Translate the term, or the caller's fallback when it is empty.
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

/// Missing or malformed vocabulary must not block startup.
fn load_terms() -> Terms {
    let path = format!("{}/terms.ron", asset_root());
    let terms = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| ron::from_str::<TermsDef>(&text).ok())
        .unwrap_or_default();
    Terms(terms)
}
