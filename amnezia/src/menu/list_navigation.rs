use bevy::prelude::*;

const KEYS: [KeyCode; 4] = [
    KeyCode::ArrowDown,
    KeyCode::ArrowUp,
    KeyCode::ArrowRight,
    KeyCode::ArrowLeft,
];

#[derive(Default)]
pub(super) struct Navigation<const ROWS: usize> {
    pub index: usize,
    pub offset: i32,
    pub cursor_index: usize,
    pub cursor_y: i32,
    pub help_index: usize,
    pub cursor_frame: u32,
    pub arrow_frame: u32,
    pub arrows: [bool; 2],
    count: usize,
    movement: Option<(i32, u32)>,
    held: [u32; 4],
}

impl<const ROWS: usize> Navigation<ROWS> {
    pub(super) fn new(index: usize, count: usize) -> Self {
        let mut nav = Self::default();
        nav.refresh(index, count);
        nav
    }

    pub(super) fn refresh(&mut self, index: usize, count: usize) {
        self.count = count.max(1);
        self.index = index.min(self.count - 1);
        self.movement = None;
        self.update_cursor();
    }

    pub(super) fn count(&self) -> usize {
        self.count
    }

    pub(super) fn suspend(&mut self) {
        self.held = [0; 4];
    }

    pub(super) fn tick(
        &mut self,
        keys: &ButtonInput<KeyCode>,
        triggered: bool,
        timed: bool,
    ) -> u32 {
        self.cursor_frame = (self.cursor_frame + u32::from(timed)) % 21;
        let repeated = std::array::from_fn::<_, 4, _>(|i| {
            self.held[i] = if keys.pressed(KEYS[i]) {
                self.held[i].saturating_add(u32::from(timed))
            } else {
                0
            };
            (triggered && keys.just_pressed(KEYS[i]))
                || (timed && self.held[i] >= 24 && self.held[i].is_multiple_of(4))
        });
        if let Some((direction, age)) = self.movement {
            if !timed {
                return 0;
            }
            self.offset += direction * 4;
            self.update_arrows(true);
            if age + 1 < 4 {
                self.movement = Some((direction, age + 1));
                return 0;
            }
            self.movement = None;
            self.update_cursor();
        }
        let old_index = self.index;
        let mut moves = 0;
        for (action, repeated) in repeated.into_iter().enumerate() {
            if !repeated {
                continue;
            }
            let next = match action {
                0 if self.index < self.count.saturating_sub(2) => self.index + 2,
                1 if self.index >= 2 => self.index - 2,
                2 if self.index + 1 < self.count => self.index + 1,
                3 if self.index > 0 => self.index - 1,
                _ => self.index,
            };
            moves += u32::from(next != self.index);
            self.index = next;
        }
        if self.index.abs_diff(old_index) <= 2 {
            let row = self.index / 2;
            let top = self.offset as usize / 16;
            let direction = if row < top && old_index + 1 < self.count {
                -1
            } else if row >= top + ROWS && old_index > 0 {
                1
            } else {
                0
            };
            if direction != 0 {
                self.movement = Some((direction, 0));
                return moves;
            }
        }
        self.update_cursor();
        // Finishing a scroll also runs the normal window update's arrow step.
        self.update_arrows(timed);
        moves
    }

    fn update_cursor(&mut self) {
        let row = self.index / 2;
        let top = self.offset.max(0) as usize / 16;
        let top = top.max(row.saturating_sub(ROWS - 1)).min(row);
        self.offset = top as i32 * 16;
        self.cursor_index = self.index;
        self.cursor_y = row as i32 * 16 - self.offset;
        self.help_index = self.index;
    }

    fn update_arrows(&mut self, timed: bool) {
        let top = self.offset as usize / 16;
        let enabled = [top > 0, top + ROWS < self.count.div_ceil(2)];
        if timed && enabled.iter().any(|enabled| *enabled) {
            self.arrow_frame = (self.arrow_frame + 1) % 40;
        }
        self.arrows = enabled.map(|enabled| enabled && self.arrow_frame < 20);
    }
}

#[cfg(test)]
mod tests;
