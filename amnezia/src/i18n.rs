//! Hungarian-to-English lookup from `i18n/en.ron`, toggled with F2; missing entries
//! keep the source text. Process-global state avoids threading a resource through every UI.

use crate::assets::asset_root;
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// The Hungarian→English table, loaded once at startup.
static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
/// Whether English is active (`false` = Hungarian, the source language).
static ENGLISH: AtomicBool = AtomicBool::new(false);

/// Translate `source` (Hungarian) for display: the English rendering when English
/// is active and a translation exists, otherwise the Hungarian source verbatim.
pub fn tr(source: &str) -> String {
    if ENGLISH.load(Ordering::Relaxed) {
        TABLE
            .get()
            .and_then(|table| table.get(source))
            .cloned()
            .unwrap_or_else(|| source.to_owned())
    } else {
        source.to_owned()
    }
}

pub fn is_english() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}

/// Switch the display language (`true` = English, `false` = Hungarian).
pub fn set_english(on: bool) {
    ENGLISH.store(on, Ordering::Relaxed);
}

pub struct I18nPlugin;

impl Plugin for I18nPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_table)
            .add_systems(Update, toggle_language);
    }
}

/// Load `i18n/en.ron` into the global table, staying empty (Hungarian-only) on
/// any error so a missing or malformed table never blocks startup.
fn load_table() {
    let path = format!("{}/i18n/en.ron", asset_root());
    let table = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| ron::from_str::<HashMap<String, String>>(&text).ok())
        .unwrap_or_default();
    if table.is_empty() {
        warn!("no translations loaded from {path}; display stays Hungarian");
    }
    let _ = TABLE.set(table);
}

/// F2 toggles the display language at any time. Shown text re-localises on its
/// next refresh (the next message, the next menu redraw).
fn toggle_language(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::F2) {
        let english = !is_english();
        set_english(english);
        info!("language: {}", if english { "English" } else { "Magyar" });
    }
}
