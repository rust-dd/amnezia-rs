#[derive(Debug)]
pub(super) struct Cadence {
    pub(super) speed: u8,
    pub(super) instant: bool,
    line_count: u32,
    printable: bool,
}

impl Default for Cadence {
    fn default() -> Self {
        Self {
            speed: 1,
            instant: false,
            line_count: 0,
            printable: false,
        }
    }
}

impl Cadence {
    fn count(&mut self) {
        if self.speed == 1 {
            self.line_count += 1;
        } else {
            self.line_count = (self.line_count & !1) + 2;
        }
    }

    pub(super) fn character(&mut self, last_line: bool, last_page: bool) -> u32 {
        let delay = if self.instant {
            0
        } else if last_page {
            2
        } else if self.speed > 1 {
            u32::from(self.speed) / 2 + 1
        } else {
            u32::from(self.line_count % 2 == 1 || last_line)
        };
        self.printable = true;
        self.count();
        delay
    }

    pub(super) fn control(&mut self, frames: u32) -> u32 {
        let delay = if self.instant {
            0
        } else {
            frames + u32::from(self.speed == 1) * (self.line_count & 1)
        };
        self.printable = false;
        if self.line_count > 0 {
            self.count();
        }
        delay
    }

    pub(super) fn newline(&mut self, last_page: bool) -> u32 {
        let delay = if self.instant {
            u32::from(last_page) * 2
        } else if !self.printable {
            1 + u32::from(last_page)
        } else {
            0
        };
        self.line_count = 0;
        self.printable = false;
        self.instant = false;
        delay
    }
}
