use super::*;

pub(in crate::battle) const LABELS: [&str; 10] = [
    "battle-scroll-down-start",
    "battle-scroll-down-four",
    "battle-scroll-down-eight",
    "battle-scroll-down-twelve",
    "battle-scroll-down-end",
    "battle-scroll-up-start",
    "battle-scroll-up-twelve",
    "battle-scroll-up-eight",
    "battle-scroll-up-four",
    "battle-scroll-up-end",
];

#[derive(Resource, Default)]
struct Checks(usize);

pub(in crate::battle) fn drive(world: &mut World, frame: u32) -> Option<&'static str> {
    if frame == 632 || frame == 640 {
        let mut battle = world.resource_mut::<Battle>();
        assert!(battle.phase == Phase::Command && battle.menu == MenuLevel::Skill);
        battle.cursor = if frame == 632 { 6 } else { 2 };
    }
    let (label, offset, selected, shown, cursor_y) = if (635..=639).contains(&frame) {
        let age = frame - 635;
        (
            LABELS[age as usize],
            age * 4,
            8,
            if age == 4 { 8 } else { 6 },
            48,
        )
    } else if (643..=647).contains(&frame) {
        let age = frame - 643;
        (
            LABELS[5 + age as usize],
            16 - age * 4,
            0,
            if age == 4 { 0 } else { 2 },
            0,
        )
    } else {
        return None;
    };
    let battle = world.resource::<Battle>();
    let list = world.resource::<Windows>().get(Panel::Skill);
    assert_eq!(
        (battle.cursor, list.index, list.offset),
        (selected, selected, offset as i32),
        "{label}"
    );
    assert_eq!(
        (list.cursor_index, list.help_index, list.cursor_y),
        (shown, shown, cursor_y),
        "{label}"
    );
    world.init_resource::<Checks>();
    world.resource_mut::<Checks>().0 += 1;
    Some(label)
}

pub(in crate::battle) fn verify_finished(world: &World) {
    assert_eq!(world.resource::<Checks>().0, LABELS.len());
}
