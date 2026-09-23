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
    pub greetings: [String; 2],
    pub accept: String,
    pub cancel: String,
}

/// The two original inn styles retain empty terms and the runtime's spacing.
pub fn inn_vocab(inn_type: u32, cost: i32, terms: &Terms) -> InnVocab {
    let t = &terms.0;
    let [first, second, third, accept, cancel] = match inn_type {
        0 => [
            &t.inn_a_greeting_1,
            &t.inn_a_greeting_2,
            &t.inn_a_greeting_3,
            &t.inn_a_accept,
            &t.inn_a_cancel,
        ]
        .map(String::as_str),
        1 => [
            &t.inn_b_greeting_1,
            &t.inn_b_greeting_2,
            &t.inn_b_greeting_3,
            &t.inn_b_accept,
            &t.inn_b_cancel,
        ]
        .map(String::as_str),
        _ => [""; 5],
    }
    .map(crate::i18n::tr);
    InnVocab {
        greetings: [format!("{first} {cost}{} {second}", currency(terms)), third],
        accept,
        cancel,
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
        assert_eq!(inn_vocab(0, 30, &terms).accept, "");
        assert_eq!(currency(&terms), "");
    }

    #[test]
    fn inn_wording_uses_both_original_sets_and_preserves_runtime_spacing() {
        let terms = Terms(crate::assets::load_ron(&format!(
            "{}/terms.ron",
            crate::assets::asset_root()
        )));
        for cost in [30, 50, 200] {
            let first = inn_vocab(0, cost, &terms);
            assert_eq!(
                first.greetings,
                [
                    format!("Mindössze  {cost}GP  egy éjszaka!"),
                    "Itt alszik?".into()
                ]
            );
            let second = inn_vocab(1, cost, &terms);
            assert_eq!(
                second.greetings,
                [
                    format!("Egy éjszaka  {cost}GP !"),
                    "Bejegyezhetem mára?".into()
                ]
            );
            assert_eq!(
                (second.accept.as_str(), second.cancel.as_str()),
                ("Igen", "Nem")
            );
        }
    }

    #[test]
    fn blank_inn_terms_do_not_invent_yes_or_no() {
        for style in [0, 1, 2] {
            let vocabulary = inn_vocab(style, 0, &Terms::default());
            assert_eq!(vocabulary.greetings, [" 0 ", ""]);
            assert!(vocabulary.accept.is_empty());
            assert!(vocabulary.cancel.is_empty());
        }
    }
}
