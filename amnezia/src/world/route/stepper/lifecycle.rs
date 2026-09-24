use super::{MoveCommandDef, MoveRouteDef, RouteStepper, decode, step_delay_secs, turn_delay_secs};
use amnezia_data::EventPage;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Suspended {
    commands: Vec<MoveCommandDef>,
    index: usize,
    repeat: bool,
    skippable: bool,
    frequency: u32,
    active: bool,
}

impl Suspended {
    pub(super) fn valid(&self) -> bool {
        self.index <= self.commands.len().saturating_add(1) && (1..=8).contains(&self.frequency)
    }

    fn take(route: &mut RouteStepper) -> Self {
        Self {
            commands: std::mem::take(&mut route.commands),
            index: route.index,
            repeat: route.repeat,
            skippable: route.skippable,
            frequency: route.frequency,
            active: route.active,
        }
    }

    fn restore(self, route: &mut RouteStepper) {
        route.commands = self.commands;
        route.index = self.index;
        route.repeat = self.repeat;
        route.skippable = self.skippable;
        route.frequency = self.frequency;
        route.active = self.active;
    }
}

impl RouteStepper {
    pub(super) fn new(
        commands: Vec<MoveCommandDef>,
        repeat: bool,
        skippable: bool,
        speed: u32,
        frequency: u32,
        forced: bool,
    ) -> Self {
        Self {
            active: !commands.is_empty(),
            animation: Default::default(),
            forced: forced && !commands.is_empty(),
            commands,
            index: 0,
            repeat,
            skippable,
            speed: speed.clamp(1, 6),
            frequency: frequency.clamp(1, 8),
            through: false,
            transparency: 0,
            timer: 0.0,
            rng: 0x9E37_79B9,
            facing_lock: None,
            direction: None,
            suspended: None,
            page_present: true,
            first_pass_complete: false,
            moving: false,
            autonomy_reset: false,
        }
    }

    pub fn speed(&self) -> u32 {
        self.speed
    }

    pub(crate) fn alpha(&self) -> f32 {
        crate::tiles::character_alpha(self.transparency)
    }

    pub(crate) fn reset_transparency(&mut self) {
        self.transparency = 0;
    }

    #[cfg(test)]
    pub fn with_speed(mut self, speed: u32) -> Self {
        self.set_speed(speed);
        self
    }

    /// Arm a page's custom route at its speed and frequency.
    pub fn from_page(route: &MoveRouteDef, speed: u32, frequency: u32) -> Self {
        Self::new(
            route.commands.clone(),
            route.repeat,
            route.skippable,
            speed,
            frequency,
            false,
        )
    }

    pub(crate) fn from_event_page(page: Option<&EventPage>) -> Self {
        if let Some(page) = page {
            let mut route = Self::from_page(&page.move_route, page.move_speed, page.move_frequency);
            route.animation.mode = page.animation_type;
            route.direction = Some(page.direction);
            route.transparency = if page.translucent { 3 } else { 0 };
            route.active &= page.move_type == 6;
            route.timer = if page.move_type == 6 {
                turn_delay_secs(route.frequency)
            } else {
                step_delay_secs(route.frequency)
            };
            route
        } else {
            Self {
                page_present: false,
                ..Self::default()
            }
        }
    }

    /// MoveEvent parameters are `[character, frequency, repeat, skip, packed bytes…]`.
    /// The command carries no speed; applying it preserves the character's live speed.
    pub fn from_move_event(params: &[i32]) -> Self {
        let freq = match params.get(1).copied().unwrap_or(6) {
            f @ 1..=8 => f as u32,
            _ => 6,
        };
        let repeat = params.get(2).copied().unwrap_or(0) != 0;
        let skippable = params.get(3).copied().unwrap_or(0) != 0;
        let commands = decode::commands(params.get(4..).unwrap_or(&[])).unwrap_or_else(|| {
            bevy::log::warn!("Ignoring malformed MoveEvent route");
            Vec::new()
        });
        Self::new(commands, repeat, skippable, 4, freq, true)
    }

    /// Suspend only the page program. Graphic, direction lock, transparency,
    /// through and speed belong to the character, not to an individual program.
    pub fn force_route(&mut self, mut route: Self) {
        if !self.forced {
            self.suspended = Some(Suspended::take(self));
        }
        self.commands = std::mem::take(&mut route.commands);
        self.index = 0;
        self.repeat = route.repeat;
        self.skippable = route.skippable;
        self.frequency = route.frequency;
        self.timer = 0.0;
        self.forced = true;
        self.active = !self.commands.is_empty();
        self.first_pass_complete = false;
        self.moving = false;
        if !self.active {
            self.cancel_forced();
        }
    }

    pub fn active(&self) -> bool {
        self.active && self.page_present
    }

    /// Forced routes keep advancing during an event or message; page routes pause.
    pub fn forced(&self) -> bool {
        self.forced
    }

    /// ProceedWithMovement waits for the first completed pass, even if the route repeats.
    pub(crate) fn pending(&self) -> bool {
        self.forced && !self.first_pass_complete
    }

    pub(crate) fn through(&self) -> bool {
        self.through
    }

    pub(crate) fn frequency(&self) -> u32 {
        self.frequency
    }

    pub(crate) fn take_autonomy_reset(&mut self) -> bool {
        std::mem::take(&mut self.autonomy_reset)
    }

    pub(crate) fn set_speed(&mut self, speed: u32) {
        self.speed = speed.clamp(1, 6);
    }

    fn cancel_forced(&mut self) {
        self.forced = false;
        self.active = false;
        self.moving = false;
        self.autonomy_reset = true;
        if let Some(suspended) = self.suspended.take() {
            suspended.restore(self);
        }
        self.timer = step_delay_secs(self.frequency);
    }

    pub(super) fn finish_pass(&mut self) {
        self.first_pass_complete = true;
        if self.repeat && !self.commands.is_empty() {
            self.index = 0;
        } else if self.forced {
            self.cancel_forced();
        } else {
            self.active = false;
        }
    }

    /// The last move ends the forced route when its tween lands, without an
    /// additional route-frequency delay. Turns and waits still finish on their timer.
    pub(crate) fn settle_movement(&mut self) -> bool {
        if self.moving {
            self.moving = false;
            if self.index >= self.commands.len() {
                self.finish_pass();
                return true;
            }
        }
        false
    }

    pub(crate) fn refresh_page(&mut self, page: Option<&EventPage>) {
        self.page_present = page.is_some();
        let Some(page) = page else {
            return;
        };
        let mut next = Self::from_event_page(Some(page));
        let previous = if self.forced {
            self.suspended.take()
        } else {
            Some(Suspended::take(self))
        };
        if let Some(previous) = previous
            && previous
                .commands
                .iter()
                .map(|c| c.code)
                .eq(next.commands.iter().map(|c| c.code))
        {
            next.index = previous.index;
        }
        self.speed = next.speed;
        self.animation.mode = next.animation.mode;
        self.frequency = next.frequency;
        self.timer = next.timer;
        self.transparency = next.transparency;
        self.facing_lock = None;
        if self.forced {
            self.suspended = Some(Suspended::take(&mut next));
        } else {
            Suspended::take(&mut next).restore(self);
        }
    }
}

#[cfg(test)]
mod tests;
