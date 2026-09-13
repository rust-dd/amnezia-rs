use super::*;
use crate::interpreter::frame::{CallFrame, Frame, MAX_CALL_DEPTH};

#[test]
fn every_call_level_has_fresh_results_and_unwinds_its_own_scope() {
    let mut frame = Frame::default();
    frame.start(0, vec![cmd(0, 0, vec![])]);
    for depth in 0..MAX_CALL_DEPTH {
        frame.choices.insert(0, depth as i32);
        frame.battle_outcome = Some(BattleOutcome::Victory);
        frame.shop_transacted = Some(depth % 2 == 0);
        assert!(frame.call(vec![cmd(0, 0, vec![])], depth as u32 + 1));
        assert!(frame.choices.is_empty());
        assert_eq!(frame.battle_outcome, None);
        assert_eq!(frame.shop_transacted, None);
    }
    let full = frame.clone();
    assert!(!frame.call(vec![switch_cmd(9998, 0, 0)], 999));
    assert_eq!(frame, full);
    for depth in (0..MAX_CALL_DEPTH).rev() {
        assert!(frame.return_to_caller());
        assert_eq!(frame.event_id, depth as u32);
        assert_eq!(frame.choices.get(&0), Some(&(depth as i32)));
        assert_eq!(frame.battle_outcome, Some(BattleOutcome::Victory));
        assert_eq!(frame.shop_transacted, Some(depth % 2 == 0));
    }
    assert!(!frame.return_to_caller());
    frame.stop();
    assert_eq!(frame, Frame::default());
}

#[test]
fn an_older_caller_without_recorded_results_has_an_empty_local_scope() {
    let caller = ron::from_str::<CallFrame>("(commands: [], ip: 0, event_id: 7)").unwrap();
    assert!(caller.choices.is_empty());
    assert_eq!(caller.battle_outcome, None);
    assert_eq!(caller.shop_transacted, None);
    let mut frame = Frame::default();
    frame.choices.insert(0, 1);
    frame.battle_outcome = Some(BattleOutcome::Defeat);
    frame.shop_transacted = Some(true);
    frame.call_stack.push(caller);
    assert!(frame.return_to_caller());
    assert!(frame.choices.is_empty());
    assert_eq!(frame.battle_outcome, None);
    assert_eq!(frame.shop_transacted, None);
}
