use super::*;

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
