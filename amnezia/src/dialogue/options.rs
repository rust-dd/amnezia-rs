use super::MessagePosition;
use bevy::prelude::*;

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageOptions {
    pub fixed: bool,
    pub continue_events: bool,
}

impl MessageOptions {
    pub(crate) fn apply(&mut self, params: &[i32]) {
        self.fixed = params.get(2).copied().unwrap_or(1) == 0;
        self.continue_events = params.get(3).copied().unwrap_or(0) != 0;
    }

    pub(crate) fn position(
        &self,
        requested: MessagePosition,
        hero_y: i32,
        battle: bool,
    ) -> MessagePosition {
        use MessagePosition::*;
        if battle {
            return Bottom;
        }
        if self.fixed {
            return requested;
        }
        match requested {
            Top => {
                if hero_y > 112 {
                    Top
                } else {
                    Bottom
                }
            }
            Middle => {
                if hero_y <= 112 {
                    Bottom
                } else if hero_y >= 160 {
                    Top
                } else {
                    Middle
                }
            }
            Bottom => {
                if hero_y >= 160 {
                    Top
                } else {
                    Bottom
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use MessagePosition::*;

    #[test]
    fn automatic_placement_matches_both_original_overlap_thresholds() {
        let options = MessageOptions::default();
        for (y, top, middle, bottom) in [
            (0, Bottom, Bottom, Bottom),
            (112, Bottom, Bottom, Bottom),
            (113, Top, Middle, Bottom),
            (159, Top, Middle, Bottom),
            (160, Top, Top, Top),
            (240, Top, Top, Top),
        ] {
            assert_eq!(options.position(Top, y, false), top);
            assert_eq!(options.position(Middle, y, false), middle);
            assert_eq!(options.position(Bottom, y, false), bottom);
        }
    }

    #[test]
    fn fixed_placement_and_battle_override_use_the_original_parameter_polarity() {
        let mut options = MessageOptions::default();
        options.apply(&[0, 0, 0, 1]);
        assert!(options.fixed && options.continue_events);
        for position in [Top, Middle, Bottom] {
            assert_eq!(options.position(position, 0, false), position);
            assert_eq!(options.position(position, 240, false), position);
            assert_eq!(options.position(position, 0, true), Bottom);
        }
        options.apply(&[0, 2, 1, 0]);
        assert_eq!(options, MessageOptions::default());
    }
}
