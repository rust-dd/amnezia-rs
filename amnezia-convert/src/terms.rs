//! Conversion of the `RPG_RT.ldb` Terms (vocabulary) section into its clean RON
//! asset: the real Hungarian menu command labels, status / equipment labels, the
//! currency term, and the battle / shop / inn message terms the game's chrome
//! shows in place of the invented Hungarian placeholders.

use amnezia_data::TermsDef;
use anyhow::{Context, Result};
use std::path::Path;

/// Convert the Terms section in `input/RPG_RT.ldb` into `output/terms.ron` (the
/// chrome-facing subset of the RM2000 vocabulary). The menu, battle, and shop
/// screens read it back and route each term through `i18n::tr()`.
pub fn convert_terms(input: &Path, output: &Path) -> Result<()> {
    if !input.is_dir() {
        anyhow::bail!("input directory not found: {}", input.display());
    }
    let ldb = input.join("RPG_RT.ldb");
    let bytes = std::fs::read(&ldb).with_context(|| format!("reading {}", ldb.display()))?;
    let t = lcf::parse_terms(&bytes).with_context(|| format!("parsing {}", ldb.display()))?;
    let terms = TermsDef {
        command_attack: t.command_attack,
        command_defend: t.command_defend,
        command_item: t.command_item,
        command_skill: t.command_skill,
        menu_equipment: t.menu_equipment,
        menu_save: t.menu_save,
        menu_quit: t.menu_quit,
        battle_fight: t.battle_fight,
        battle_auto: t.battle_auto,
        battle_escape: t.battle_escape,
        status: t.status,
        row: t.row,
        level: t.level,
        health_points: t.health_points,
        spirit_points: t.spirit_points,
        normal_status: t.normal_status,
        exp_short: t.exp_short,
        lvl_short: t.lvl_short,
        hp_short: t.hp_short,
        sp_short: t.sp_short,
        attack: t.attack,
        defense: t.defense,
        spirit: t.spirit,
        agility: t.agility,
        weapon: t.weapon,
        shield: t.shield,
        armor: t.armor,
        helmet: t.helmet,
        accessory: t.accessory,
        gold: t.gold,
        attacking: t.attacking,
        defending: t.defending,
        dodge: t.dodge,
        miss: t.miss,
        enemy_critical: t.enemy_critical,
        actor_critical: t.actor_critical,
        enemy_damaged: t.enemy_damaged,
        actor_damaged: t.actor_damaged,
        victory: t.victory,
        defeat: t.defeat,
        escape_success: t.escape_success,
        escape_failure: t.escape_failure,
        exp_received: t.exp_received,
        gold_recieved_a: t.gold_recieved_a,
        gold_recieved_b: t.gold_recieved_b,
        item_recieved: t.item_recieved,
        level_up: t.level_up,
        shop_greeting1: t.shop_greeting1,
        shop_regreeting1: t.shop_regreeting1,
        shop_buy1: t.shop_buy1,
        shop_sell1: t.shop_sell1,
        shop_leave1: t.shop_leave1,
        shop_buy_select1: t.shop_buy_select1,
        shop_sell_select1: t.shop_sell_select1,
        shop_buy_number1: t.shop_buy_number1,
        shop_purchased1: t.shop_purchased1,
        shop_sold1: t.shop_sold1,
        shop_greeting2: t.shop_greeting2,
        shop_regreeting2: t.shop_regreeting2,
        shop_buy2: t.shop_buy2,
        shop_sell2: t.shop_sell2,
        shop_leave2: t.shop_leave2,
        shop_buy_select2: t.shop_buy_select2,
        shop_sell_select2: t.shop_sell_select2,
        shop_buy_number2: t.shop_buy_number2,
        shop_purchased2: t.shop_purchased2,
        shop_sold2: t.shop_sold2,
        shop_greeting3: t.shop_greeting3,
        shop_regreeting3: t.shop_regreeting3,
        shop_buy3: t.shop_buy3,
        shop_sell3: t.shop_sell3,
        shop_leave3: t.shop_leave3,
        shop_buy_select3: t.shop_buy_select3,
        shop_sell_select3: t.shop_sell_select3,
        shop_buy_number3: t.shop_buy_number3,
        shop_purchased3: t.shop_purchased3,
        shop_sold3: t.shop_sold3,
        inn_a_greeting_1: t.inn_a_greeting_1,
        inn_a_greeting_2: t.inn_a_greeting_2,
        inn_a_greeting_3: t.inn_a_greeting_3,
        inn_a_accept: t.inn_a_accept,
        inn_a_cancel: t.inn_a_cancel,
        inn_b_greeting_1: t.inn_b_greeting_1,
        inn_b_greeting_2: t.inn_b_greeting_2,
        inn_b_greeting_3: t.inn_b_greeting_3,
        inn_b_accept: t.inn_b_accept,
        inn_b_cancel: t.inn_b_cancel,
    };
    let serialised = ron::to_string(&terms).context("serialising terms to RON")?;
    std::fs::create_dir_all(output).with_context(|| format!("creating {}", output.display()))?;
    std::fs::write(output.join("terms.ron"), serialised)
        .with_context(|| format!("writing {}", output.join("terms.ron").display()))?;
    Ok(())
}
