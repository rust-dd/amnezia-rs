use super::{MoveCommandDef, MoveRouteDef, RouteStepper, StopClock, decode, stop_clock};
use amnezia_data::EventPage;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Suspended {
    commands: Vec<MoveCommandDef>,
    pub(super) index: usize,
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
            route_through: Some(false),
            overlap_forbidden: None,
            transparency: 0,
            stop: Some(StopClock {
                count: if forced { 0xFFFF } else { 0 },
                maximum: 0,
            }),
            legacy_timer: 0.0,
            rng: 0x9E37_79B9,
            facing_lock: None,
            direction: None,
            suspended: None,
            page_present: true,
            first_pass_complete: false,
            moving: false,
            legacy_autonomy_reset: false,
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
            route.overlap_forbidden = Some(page.overlap_forbidden);
            route.transparency = if page.translucent { 3 } else { 0 };
            route.active &= page.move_type == 6;
            route.set_stop_maximum(if page.move_type == 6 {
                stop_clock::turn(route.frequency)
            } else {
                stop_clock::step(route.frequency)
            });
            route
        } else {
            Self {
                page_present: false,
                through: true,
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
        let original_frequency = self
            .suspended
            .as_ref()
            .map_or(self.frequency, |s| s.frequency);
        self.commands = std::mem::take(&mut route.commands);
        self.index = 0;
        self.repeat = route.repeat;
        self.skippable = route.skippable;
        self.frequency = route.frequency;
        self.set_stop_count(0xFFFF);
        if self.frequency != original_frequency {
            self.set_stop_maximum(stop_clock::step(self.frequency));
        }
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

    pub(crate) fn page_present(&self) -> bool {
        self.page_present
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

    pub(in crate::world) fn overlap_forbidden(&self) -> Option<bool> {
        self.overlap_forbidden
    }

    pub(crate) fn frequency(&self) -> u32 {
        self.frequency
    }

    pub(crate) fn set_speed(&mut self, speed: u32) {
        self.speed = speed.clamp(1, 6);
    }

    pub(super) fn cancel_forced(&mut self) {
        self.forced = false;
        self.active = false;
        self.moving = false;
        if let Some(suspended) = self.suspended.take() {
            suspended.restore(self);
        }
        self.set_stop_maximum(stop_clock::step(self.frequency));
    }

    pub(crate) fn cancel_for_rider(&mut self) {
        if self.forced {
            self.cancel_forced();
        }
        self.first_pass_complete = false;
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
    /// additional route-frequency delay. Turns and waits finish after their threshold.
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
        let route_through = *self.route_through.get_or_insert(self.through);
        self.page_present = page.is_some();
        let Some(page) = page else {
            self.through = true;
            return;
        };
        self.through = route_through;
        self.overlap_forbidden = Some(page.overlap_forbidden);
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
        self.set_stop_maximum(next.stop_maximum());
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
