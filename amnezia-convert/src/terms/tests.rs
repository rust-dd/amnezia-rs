use super::*;

#[test]
#[ignore = "requires the untracked original/RPG_RT.ldb"]
fn every_original_term_survives_conversion_including_intentional_blanks() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let original =
        lcf::parse_terms(&std::fs::read(root.join("original/RPG_RT.ldb")).unwrap()).unwrap();
    let converted =
        ron::from_str::<TermsDef>(&std::fs::read_to_string(root.join("assets/terms.ron")).unwrap())
            .unwrap();
    macro_rules! compare {
        ($($field:ident),+ $(,)?) => {
            $(assert_eq!(converted.$field, original.$field, stringify!($field));)+
        };
    }
    compare!(
        encounter,
        special_combat,
        escape_success,
        escape_failure,
        victory,
        defeat,
        exp_received,
        gold_recieved_a,
        gold_recieved_b,
        item_recieved,
        attacking,
        enemy_critical,
        actor_critical,
        defending,
        observing,
        focus,
        autodestruction,
        enemy_escape,
        enemy_transform,
        enemy_damaged,
        enemy_undamaged,
        actor_damaged,
        actor_undamaged,
        skill_failure_a,
        skill_failure_b,
        skill_failure_c,
        dodge,
        use_item,
        hp_recovery,
        parameter_increase,
        parameter_decrease,
        enemy_hp_absorbed,
        actor_hp_absorbed,
        resistance_increase,
        resistance_decrease,
        level_up,
        skill_learned,
        battle_start,
        miss,
        shop_greeting1,
        shop_regreeting1,
        shop_buy1,
        shop_sell1,
        shop_leave1,
        shop_buy_select1,
        shop_buy_number1,
        shop_purchased1,
        shop_sell_select1,
        shop_sell_number1,
        shop_sold1,
        shop_greeting2,
        shop_regreeting2,
        shop_buy2,
        shop_sell2,
        shop_leave2,
        shop_buy_select2,
        shop_buy_number2,
        shop_purchased2,
        shop_sell_select2,
        shop_sell_number2,
        shop_sold2,
        shop_greeting3,
        shop_regreeting3,
        shop_buy3,
        shop_sell3,
        shop_leave3,
        shop_buy_select3,
        shop_buy_number3,
        shop_purchased3,
        shop_sell_select3,
        shop_sell_number3,
        shop_sold3,
        inn_a_greeting_1,
        inn_a_greeting_2,
        inn_a_greeting_3,
        inn_a_accept,
        inn_a_cancel,
        inn_b_greeting_1,
        inn_b_greeting_2,
        inn_b_greeting_3,
        inn_b_accept,
        inn_b_cancel,
        possessed_items,
        equipped_items,
        gold,
        battle_fight,
        battle_auto,
        battle_escape,
        command_attack,
        command_defend,
        command_item,
        command_skill,
        menu_equipment,
        menu_save,
        menu_quit,
        new_game,
        load_game,
        exit_game,
        status,
        row,
        order,
        wait_on,
        wait_off,
        level,
        health_points,
        spirit_points,
        normal_status,
        exp_short,
        lvl_short,
        hp_short,
        sp_short,
        sp_cost,
        attack,
        defense,
        spirit,
        agility,
        weapon,
        shield,
        armor,
        helmet,
        accessory,
        save_game_message,
        load_game_message,
        file,
        exit_game_message,
        yes,
        no,
    );
}

#[test]
fn older_terms_files_load_and_new_text_keeps_spaces_accents_and_empty_strings() {
    let mut terms = ron::from_str::<TermsDef>(r#"(command_attack:"Támadás")"#).unwrap();
    assert_eq!(terms.command_attack, "Támadás");
    assert!(terms.skill_failure_a.is_empty());
    assert!(terms.save_game_message.is_empty());
    terms.skill_failure_a = " már nincs hatással!".into();
    terms.hp_recovery = "  ponttal növekedett".into();
    let text = ron::to_string(&terms).unwrap();
    assert_eq!(ron::from_str::<TermsDef>(&text).unwrap(), terms);
}
