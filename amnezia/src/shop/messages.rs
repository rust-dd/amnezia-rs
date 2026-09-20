//! Original shop vocabulary and the remaining legacy inn labels.

use crate::terms::Terms;

/// The localised currency word (RM2000 `gold` term), shown after an amount as in
/// `Window_Gold::DrawCurrencyValue`.
pub fn currency(terms: &Terms) -> String {
    crate::i18n::tr(&terms.0.gold)
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
    pub sell_number: String,
    pub purchased: String,
    pub sold: String,
}

/// Empty database terms stay empty, as in the original merchant window.
pub fn shop_vocab(shop_type: u32, terms: &Terms) -> ShopVocab {
    let s = terms.0.shop_set(shop_type);
    ShopVocab {
        greeting: crate::i18n::tr(s.greeting),
        regreeting: crate::i18n::tr(s.regreeting),
        buy: crate::i18n::tr(s.buy),
        sell: crate::i18n::tr(s.sell),
        leave: crate::i18n::tr(s.leave),
        buy_select: crate::i18n::tr(s.buy_select),
        sell_select: crate::i18n::tr(s.sell_select),
        number: crate::i18n::tr(s.number),
        sell_number: crate::i18n::tr(s.sell_number),
        purchased: crate::i18n::tr(s.purchased),
        sold: crate::i18n::tr(s.sold),
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
        let mut terms = Terms::default();
        terms.0.shop_greeting1 = "Miben segíthetek?".into();
        terms.0.shop_greeting2 = "Har!".into();
        terms.0.shop_buy1 = "Vásárlás".into();
        assert_eq!(shop_vocab(0, &terms).greeting, "Miben segíthetek?");
        assert_eq!(shop_vocab(1, &terms).greeting, "Har!");
        assert_eq!(shop_vocab(0, &terms).buy, "Vásárlás");
    }

    #[test]
    fn empty_shop_terms_are_not_replaced_with_invented_labels() {
        let terms = Terms::default();
        let vocab = shop_vocab(0, &terms);
        assert_eq!(vocab.buy, "");
        assert_eq!(vocab.sell, "");
        assert_eq!(inn_vocab(&terms).accept, "Igen");
        assert_eq!(currency(&terms), "");
    }
}
