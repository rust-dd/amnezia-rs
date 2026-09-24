#[derive(Clone)]
pub(crate) struct List {
    pub index: usize,
    pub offset: i32,
    pub cursor_index: usize,
    pub cursor_y: i32,
    pub help_index: usize,
    pub cursor_frame: u32,
    pub arrow_frame: u32,
    pub arrows: [bool; 2],
    columns: usize,
    rows: usize,
    endless: bool,
    count: usize,
    movement: Option<(i32, u32)>,
}

impl List {
    pub(crate) fn new(columns: usize, rows: usize, endless: bool) -> Self {
        assert!(columns > 0 && rows > 0);
        Self {
            index: 0,
            offset: 0,
            cursor_index: 0,
            cursor_y: 0,
            help_index: 0,
            cursor_frame: 0,
            arrow_frame: 0,
            arrows: [false; 2],
            columns,
            rows,
            endless,
            count: 0,
            movement: None,
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.count
    }

    pub(crate) fn cursor_x(&self) -> f32 {
        if self.cursor_frame <= 10 { 64.0 } else { 96.0 }
    }

    pub(crate) fn refresh(&mut self, index: usize, count: usize) {
        self.count = count;
        self.index = index.min(self.count.saturating_sub(1));
        self.update_cursor();
    }

    pub(crate) fn tick(
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
                    0 if self.columns == 1
                        && (self.endless || triggered[0] || self.index + 1 < self.count) =>
                    {
                        Some((self.index + 1) % self.count)
                    }
                    1 if self.columns == 1 && (self.endless || triggered[1] || self.index > 0) => {
                        Some((self.index + self.count - 1) % self.count)
                    }
                    0 if self.index + self.columns < self.count => Some(self.index + self.columns),
                    1 if self.index >= self.columns => Some(self.index - self.columns),
                    4 if self.columns == 1 && self.index + 1 < self.count => {
                        Some((self.index + self.rows).min(self.count - 1))
                    }
                    5 if self.columns == 1 && self.index > 0 => {
                        Some(self.index.saturating_sub(self.rows))
                    }
                    2 if self.columns >= 2 && self.index + 1 < self.count => Some(self.index + 1),
                    3 if self.columns >= 2 && self.index > 0 => Some(self.index - 1),
                    _ => None,
                };
                if let Some(next) = next {
                    self.index = next;
                    moves += 1;
                }
            }
            if self.index.abs_diff(old) <= self.columns {
                let row = self.index / self.columns;
                let top = self.offset as usize / 16;
                let direction = if row < top && old + 1 < self.count {
                    -1
                } else if row >= top + self.rows && old > 0 {
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
        let row = self.index / self.columns;
        let top = self.offset.max(0) as usize / 16;
        let top = top.max(row.saturating_sub(self.rows - 1)).min(row);
        if row < self.offset.max(0) as usize / 16
            || row >= self.offset.max(0) as usize / 16 + self.rows
        {
            self.offset = top as i32 * 16;
        }
        self.cursor_index = self.index;
        self.cursor_y = row as i32 * 16 - self.offset;
    }

    fn update_arrows(&mut self, timed: bool) {
        let top = self.offset.max(0) as usize / 16;
        let enabled = [top > 0, top + self.rows < self.count.div_ceil(self.columns)];
        if timed && enabled.iter().any(|enabled| *enabled) {
            self.arrow_frame = (self.arrow_frame + 1) % 40;
        }
        self.arrows = enabled.map(|enabled| enabled && self.arrow_frame < 20);
    }
}

#[cfg(test)]
mod tests;
