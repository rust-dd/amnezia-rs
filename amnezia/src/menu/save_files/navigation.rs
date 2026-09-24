use crate::save::slots::COUNT;

#[derive(Default)]
pub(super) struct Navigation {
    pub index: usize,
    pub top: usize,
    pub arrow: u32,
    pub arrows: [bool; 2],
    pub cursors: [u32; COUNT as usize],
    old_top: usize,
    movement: Option<u32>,
}

impl Navigation {
    pub fn new(index: usize) -> Self {
        let mut cursors = [0; COUNT as usize];
        cursors[index] = 1;
        Self {
            index,
            top: index.saturating_sub(2),
            cursors,
            ..Self::default()
        }
    }

    pub fn offset(&self) -> i32 {
        self.movement.map_or(0, |frame| {
            let distance = (self.top as i32 - self.old_top as i32) * 64;
            distance - distance * frame.min(7) as i32 / 7
        })
    }

    pub fn tick(&mut self, repeated: [bool; 6], triggered: [bool; 2], timed: bool) -> u32 {
        if timed {
            self.arrow = (self.arrow + 1) % 40;
            self.arrows = [self.top > 0, self.top + 3 < COUNT as usize]
                .map(|enabled| enabled && self.arrow < 20);
        }
        let moving = self.movement.is_some();
        if timed && let Some(frame) = &mut self.movement {
            *frame += 1;
            if *frame > 7 {
                self.movement = None;
            }
        }
        if moving {
            self.cursors[self.index] = (self.cursors[self.index] + u32::from(timed)) % 21;
            return 0;
        }
        let mut moves = 0;
        for action in [0, 1, 4, 5] {
            if repeated[action] {
                moves += u32::from(
                    self.navigate(action, triggered.get(action).copied().unwrap_or(false)),
                );
            }
        }
        self.old_top = self.top;
        self.top = self.top.max(self.index.saturating_sub(2)).min(self.index);
        if self.old_top != self.top {
            self.movement = Some(u32::from(timed));
        }
        self.cursors[self.index] = (self.cursors[self.index] + u32::from(timed)) % 21;
        moves
    }

    pub fn moving(&self) -> bool {
        self.movement.is_some()
    }

    fn navigate(&mut self, action: usize, triggered: bool) -> bool {
        let last = COUNT as usize - 1;
        self.index = match action {
            0 if triggered || self.index < last => (self.index + 1) % COUNT as usize,
            1 if triggered || self.index > 0 => (self.index + last) % COUNT as usize,
            4 if self.index < last => (self.index + 3).min(last),
            5 if self.index > 0 => self.index.saturating_sub(3),
            _ => return false,
        };
        true
    }
}
