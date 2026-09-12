use super::*;
use crate::menu::MemberAction;

const MEMBERS: MenuScreen = MenuScreen::MemberSelect {
    action: MemberAction::Skill,
    cursor: 0,
};

#[test]
fn menu_cursor_clocks_follow_logical_frames_at_every_render_rate() {
    for fps in [15, 30, 60, 144] {
        let mut frames = GameFrames::default();
        let mut clock = Clock::default();
        clock.advance(0, true, MenuScreen::Command, false);
        for _ in 0..fps * 3 {
            frames.advance(1.0 / fps as f64);
            clock.advance(frames.frame, true, MenuScreen::Command, false);
            assert_eq!(clock.phases, [frames.frame % 21, 0]);
            assert_eq!(
                clock.source_x(CursorId::Command),
                if frames.frame % 21 <= 10 { 64.0 } else { 96.0 }
            );
        }
    }
}

#[test]
fn inactive_windows_and_subscenes_preserve_their_independent_cursor_phases() {
    let mut clock = Clock::default();
    clock.advance(0, true, MenuScreen::Command, false);
    clock.advance(12, true, MEMBERS, false);
    assert_eq!(clock.phases, [12, 0]);
    clock.advance(32, true, MEMBERS, false);
    assert_eq!(clock.phases, [12, 20]);
    clock.advance(
        33,
        true,
        MenuScreen::SkillList {
            member: 0,
            cursor: 0,
        },
        false,
    );
    assert_eq!(clock.phases, [12, 0]);
    clock.advance(500, true, MEMBERS, false);
    assert_eq!(clock.phases, [12, 0]);
    clock.advance(501, true, MenuScreen::Command, false);
    assert_eq!(clock.phases, [12, 1]);
    clock.advance(502, true, MenuScreen::Command, false);
    assert_eq!(clock.phases, [13, 1]);
}

#[test]
fn paused_frames_are_discarded_and_reopening_resets_both_window_clocks() {
    let mut clock = Clock::default();
    clock.advance(u32::MAX - 10, true, MenuScreen::Command, false);
    clock.advance(1, true, MenuScreen::Command, false);
    assert_eq!(clock.phases, [12, 0]);
    clock.advance(1001, true, MenuScreen::Command, true);
    assert_eq!(clock.phases, [12, 0]);
    clock.advance(1002, true, MenuScreen::Command, false);
    assert_eq!(clock.phases, [13, 0]);
    clock.advance(1003, false, MenuScreen::Command, false);
    clock.advance(4000, true, MEMBERS, false);
    assert_eq!(clock.phases, [0, 0]);
    clock.advance(4012, true, MEMBERS, false);
    assert_eq!(clock.phases, [0, 12]);
}
