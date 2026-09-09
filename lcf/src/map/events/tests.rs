use super::*;

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
