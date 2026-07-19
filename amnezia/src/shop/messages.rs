//! The merchant vocabulary: the shopkeeper's greeting / prompts / confirmations
//! and the inn's prompt, plus the currency term.
//!
//! These are faithful Hungarian *placeholders*, not the game's own strings. The
//! real RM2000 wording lives in the `RPG_RT.ldb` Terms section
//! (`shop_greeting1`, `shop_buy_select1`, `inn_a_accept`, `gold`, …), which the
//! `lcf` crate does not yet parse and the conversion does not emit — wiring the
//! true terms means extending the convert pipeline (a separate task). Until then
//! the panel reads naturally in the game's language.

/// The wallet label shown on the shop panel (placeholder for `terms.gold`).
pub const GOLD_LABEL: &str = "Pénz";

/// The suffix appended to a price/total (placeholder for `terms.gold`).
pub const GOLD_UNIT: &str = " pénz";

/// The shopkeeper's lines for one shop, chosen by RM2000 shop type. The type
/// picks one of three term sets in RPG_RT; the true per-type wording is deferred
/// with the rest of the LDB terms, so every type currently shares one set.
pub struct ShopVocab {
    /// First greeting, shown on the Buy/Sell/Leave menu.
    pub greeting: &'static str,
    /// Shown on the menu after returning from a buy or sell ("anything else?").
    pub regreeting: &'static str,
    pub buy: &'static str,
    pub sell: &'static str,
    pub leave: &'static str,
    pub buy_select: &'static str,
    pub sell_select: &'static str,
    /// Prompt over the quantity window.
    pub number: &'static str,
    pub purchased: &'static str,
    pub sold: &'static str,
}

/// The vocabulary for `shop_type`. All three RM2000 types share one faithful
/// Hungarian set until the real per-type terms are wired.
pub fn shop_vocab(_shop_type: u32) -> ShopVocab {
    ShopVocab {
        greeting: "Üdvözöllek! Mit szeretnél?",
        regreeting: "Van még valami?",
        buy: "Vásárlás",
        sell: "Eladás",
        leave: "Távozás",
        buy_select: "Mit szeretnél venni?",
        sell_select: "Mit adsz el?",
        number: "Hányat?",
        purchased: "Köszönöm a vásárlást!",
        sold: "Megvettem, tessék a pénzed.",
    }
}

/// The inn keeper's lines.
pub struct InnVocab {
    pub accept: &'static str,
    pub cancel: &'static str,
    /// Suffix marking the accept option as unaffordable.
    pub broke: &'static str,
    pub rested: &'static str,
}

/// The inn vocabulary. Faithful Hungarian placeholder for the LDB `inn_a_*`
/// terms (see the module note).
pub fn inn_vocab() -> InnVocab {
    InnVocab {
        accept: "Igen",
        cancel: "Nem",
        broke: "(nincs elég pénzed)",
        rested: "Kipihenten ébredsz.",
    }
}
