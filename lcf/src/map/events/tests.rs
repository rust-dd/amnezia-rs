use super::*;

#[test]
fn omitted_condition_ids_use_the_original_first_database_entries() {
    let condition = parse_condition(&[0]).unwrap();
    assert_eq!(condition.flags, 0);
    assert_eq!(condition.switch_a, 1);
    assert_eq!(condition.switch_b, 1);
    assert_eq!(condition.variable_id, 1);
    assert_eq!(condition.variable_value, 0);
    assert_eq!(condition.item_id, 1);
    assert_eq!(condition.actor_id, 1);
}

#[test]
fn an_omitted_variable_id_still_tests_the_first_story_counter() {
    let condition = parse_condition(&[1, 1, 4, 5, 1, 1, 0]).unwrap();
    assert_eq!(condition.flags, 4);
    assert_eq!((condition.variable_id, condition.variable_value), (1, 1));
}

#[test]
fn explicit_zero_condition_ids_are_preserved() {
    let condition = parse_condition(&[2, 1, 0, 3, 1, 0, 4, 1, 0, 6, 1, 0, 7, 1, 0, 0]).unwrap();
    assert_eq!(condition.switch_a, 0);
    assert_eq!(condition.switch_b, 0);
    assert_eq!(condition.variable_id, 0);
    assert_eq!(condition.item_id, 0);
    assert_eq!(condition.actor_id, 0);
}

#[test]
fn command_terminators_are_not_executable_commands() {
    assert!(parse_commands(&[0, 0, 0, 0]).unwrap().is_empty());
    let commands = parse_commands(&[10, 0, 0, 0, 0, 0, 0, 0]).unwrap();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].code, 10);
    assert!(parse_commands(&[0]).is_err());
}

#[test]
fn overlap_forbidden_defaults_to_false_and_preserves_explicit_values() {
    assert!(!parse_pages(&[1, 1, 0]).unwrap()[0].overlap_forbidden);
    assert!(!parse_pages(&[1, 1, 0x23, 1, 0, 0]).unwrap()[0].overlap_forbidden);
    assert!(parse_pages(&[1, 1, 0x23, 1, 1, 0]).unwrap()[0].overlap_forbidden);
}

#[test]
fn page_translucency_defaults_to_opaque_and_preserves_both_explicit_values() {
    assert!(!parse_pages(&[1, 1, 0]).unwrap()[0].translucent);
    assert!(!parse_pages(&[1, 1, 0x19, 1, 0, 0]).unwrap()[0].translucent);
    assert!(parse_pages(&[1, 1, 0x19, 1, 1, 0]).unwrap()[0].translucent);
}

#[test]
fn animation_modes_preserve_each_value_and_default_to_normal_walking() {
    assert_eq!(parse_pages(&[1, 1, 0]).unwrap()[0].animation_type, 0);
    for mode in 0..=6 {
        assert_eq!(
            parse_pages(&[1, 1, 0x24, 1, mode, 0]).unwrap()[0].animation_type,
            u32::from(mode)
        );
    }
}
