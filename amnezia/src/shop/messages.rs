//! The merchant vocabulary sourced from the real RM2000 Terms: the shopkeeper's
//! greeting / prompts / confirmations (one of three per-type term sets, chosen by
//! the shop's RM2000 `shop_type` exactly as EasyRPG `Window_ShopParty` does), the
//! inn's Yes/No prompt, and the currency term. Each routes through
//! [`crate::i18n::tr`] and falls back to a faithful Hungarian placeholder when the
//! original database left the term blank.

use crate::terms::Terms;

/// The localised currency word (RM2000 `gold` term), shown after an amount as in
/// `Window_Gold::DrawCurrencyValue`.
pub fn currency(terms: &Terms) -> String {
    terms.label(&terms.0.gold, "arany")
}

/// The shopkeeper's lines for one shop, chosen by the RM2000 `shop_type` (0/1/2):
/// each of the three merchant styles keys its own greeting / prompt / confirmation
/// terms.
pub struct ShopVocab {
    /// First greeting, shown on the Buy/Sell/Leave menu.
    pub greeting: String,
    /// Shown on the menu after returning from a buy or sell ("anything else?").
    pub regreeting: String,
    pub buy: String,
    pub sell: String,
    pub leave: String,
    pub buy_select: String,
    pub sell_select: String,
    /// Prompt over the quantity window.
    pub number: String,
    pub purchased: String,
    pub sold: String,
}

/// The vocabulary for `shop_type`, resolved from the real per-type Terms with a
/// Hungarian placeholder fallback for any blank term.
pub fn shop_vocab(shop_type: u32, terms: &Terms) -> ShopVocab {
    let s = terms.0.shop_set(shop_type);
    ShopVocab {
        greeting: terms.label(s.greeting, "Üdvözöllek! Mit szeretnél?"),
        regreeting: terms.label(s.regreeting, "Van még valami?"),
        buy: terms.label(s.buy, "Vásárlás"),
        sell: terms.label(s.sell, "Eladás"),
        leave: terms.label(s.leave, "Távozás"),
        buy_select: terms.label(s.buy_select, "Mit szeretnél venni?"),
        sell_select: terms.label(s.sell_select, "Mit adsz el?"),
        number: terms.label(s.number, "Hányat?"),
        purchased: terms.label(s.purchased, "Köszönöm a vásárlást!"),
        sold: terms.label(s.sold, "Megvettem, tessék a pénzed."),
    }
}

/// The inn keeper's lines.
pub struct InnVocab {
    pub accept: String,
    pub cancel: String,
    /// Suffix marking the accept option as unaffordable.
    pub broke: String,
    pub rested: String,
}

/// The inn vocabulary: Yes/No from the real RM2000 `inn_a_accept` / `inn_a_cancel`
/// terms. The "no funds" and "rested" lines have no RM2000 term, so they stay
/// faithful Hungarian.
pub fn inn_vocab(terms: &Terms) -> InnVocab {
    InnVocab {
        accept: terms.label(&terms.0.inn_a_accept, "Igen"),
        cancel: terms.label(&terms.0.inn_a_cancel, "Nem"),
        broke: "(nincs elég pénzed)".to_string(),
        rested: "Kipihenten ébredsz.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shop_vocab_resolves_the_per_type_terms() {
        // Two distinct merchant styles pick their own parsed term sets.
        let mut terms = Terms::default();
        terms.0.shop_greeting1 = "Miben segíthetek?".into();
        terms.0.shop_greeting2 = "Har!".into();
        terms.0.shop_buy1 = "Vásárlás".into();
        assert_eq!(shop_vocab(0, &terms).greeting, "Miben segíthetek?");
        assert_eq!(shop_vocab(1, &terms).greeting, "Har!");
        assert_eq!(shop_vocab(0, &terms).buy, "Vásárlás");
    }

    #[test]
    fn blank_terms_fall_back_to_the_hungarian_placeholders() {
        let terms = Terms::default();
        let vocab = shop_vocab(0, &terms);
        assert_eq!(vocab.buy, "Vásárlás");
        assert_eq!(vocab.sell, "Eladás");
        assert_eq!(inn_vocab(&terms).accept, "Igen");
        assert_eq!(currency(&terms), "arany");
    }
}
