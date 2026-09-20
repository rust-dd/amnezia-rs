#[derive(Default)]
pub(super) struct List<const COLUMNS: usize> {
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
}

impl<const COLUMNS: usize> List<COLUMNS> {
    pub(super) fn refresh(&mut self, index: usize, count: usize) {
        self.count = count;
        self.index = index.min(self.count.saturating_sub(1));
        self.update_cursor();
    }

    pub(super) fn tick(
        &mut self,
        repeated: [bool; 6],
        triggered: [bool; 2],
        active: bool,
        timed: bool,
    ) -> u32 {
        let mut moves = 0;
        if active {
            self.cursor_frame = (self.cursor_frame + u32::from(timed)) % 21;
        }
        if active && self.count > 0 {
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
                self.help_index = self.index;
            }
            let old = self.index;
            for action in [0, 1, 4, 5, 2, 3] {
                if !repeated[action] {
                    continue;
                }
                let next = match action {
                    0 if COLUMNS == 1 && (triggered[0] || self.index + 1 < self.count) => {
                        Some((self.index + 1) % self.count)
                    }
                    1 if COLUMNS == 1 && (triggered[1] || self.index > 0) => {
                        Some((self.index + self.count - 1) % self.count)
                    }
                    0 if self.index + COLUMNS < self.count => Some(self.index + COLUMNS),
                    1 if self.index >= COLUMNS => Some(self.index - COLUMNS),
                    4 if COLUMNS == 1 && self.index + 1 < self.count => {
                        Some((self.index + 7).min(self.count - 1))
                    }
                    5 if COLUMNS == 1 && self.index > 0 => Some(self.index.saturating_sub(7)),
                    2 if COLUMNS >= 2 && self.index + 1 < self.count => Some(self.index + 1),
                    3 if COLUMNS >= 2 && self.index > 0 => Some(self.index - 1),
                    _ => None,
                };
                if let Some(next) = next {
                    self.index = next;
                    moves += 1;
                }
            }
            if self.index.abs_diff(old) <= COLUMNS {
                let row = self.index / COLUMNS;
                let top = self.offset as usize / 16;
                let direction = if row < top && old + 1 < self.count {
                    -1
                } else if row >= top + 7 && old > 0 {
                    1
                } else {
                    0
                };
                if direction != 0 {
                    self.movement = Some((direction, 0));
                    return moves;
                }
            }
            self.help_index = self.index;
        }
        self.update_cursor();
        self.update_arrows(timed);
        moves
    }

    fn update_cursor(&mut self) {
        let row = self.index / COLUMNS;
        let top = self.offset.max(0) as usize / 16;
        let top = top.max(row.saturating_sub(6)).min(row);
        if row < self.offset.max(0) as usize / 16 || row >= self.offset.max(0) as usize / 16 + 7 {
            self.offset = top as i32 * 16;
        }
        self.cursor_index = self.index;
        self.cursor_y = row as i32 * 16 - self.offset;
    }

    fn update_arrows(&mut self, timed: bool) {
        let top = self.offset.max(0) as usize / 16;
        let enabled = [top > 0, top + 7 < self.count.div_ceil(COLUMNS)];
        if timed && enabled.iter().any(|enabled| *enabled) {
            self.arrow_frame = (self.arrow_frame + 1) % 40;
        }
        self.arrows = enabled.map(|enabled| enabled && self.arrow_frame < 20);
    }
}

#[cfg(test)]
mod tests;
