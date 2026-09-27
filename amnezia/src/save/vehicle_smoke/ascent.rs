use super::*;

#[derive(Resource, Default)]
struct Probe {
    remaining: Option<u16>,
    ticks: u16,
    expected_ticks: u16,
}

impl Probe {
    fn arm(&mut self, remaining: u16) {
        if self.remaining.is_none() {
            self.remaining = Some(remaining);
            self.expected_ticks = remaining.div_ceil(8);
        }
    }

    fn check(&mut self, paused: bool, actual: u16, altitude: f32) {
        let Some(remaining) = self.remaining.as_mut() else {
            return;
        };
        if !paused && *remaining > 0 {
            *remaining = remaining.saturating_sub(8);
            self.ticks += 1;
        }
        assert_eq!(
            actual, *remaining,
            "saved ascent must consume eight subpixels per live update"
        );
        assert_eq!(altitude, f32::from((256 - *remaining) / 16));
        if *remaining == 0 {
            assert_eq!(self.ticks, self.expected_ticks);
        }
    }
}

pub(super) fn configure(app: &mut App) {
    app.init_resource::<Probe>().add_systems(
        Update,
        observe
            .after(crate::vehicles::VehicleStep)
            .before(crate::dialogue::MessageUpdate),
    );
}

pub(super) fn arm(world: &mut World) {
    let remaining = world.resource::<Vehicles>().airship_ascent_remaining();
    world.resource_mut::<Probe>().arm(remaining);
}

fn observe(guards: crate::world::MoveGuards, vehicles: Res<Vehicles>, mut probe: ResMut<Probe>) {
    probe.check(
        guards.forced_route_paused(),
        vehicles.airship_ascent_remaining(),
        vehicles.airship_altitude(),
    );
}

pub(super) fn finish(world: &mut World) {
    let mut probe = world.resource_mut::<Probe>();
    assert_eq!(probe.remaining, Some(0));
    assert!(probe.ticks > 0);
    assert_eq!(probe.ticks, probe.expected_ticks);
    info!(
        "saved vehicle ascent: {} exact live updates and paused GPU handoffs verified",
        probe.ticks
    );
    *probe = default();
    let vehicles = world.resource::<Vehicles>();
    assert!(!vehicles.airship_transitioning());
    assert_eq!(vehicles.airship_altitude(), 16.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascent_probe_retains_the_exact_tick_count_through_variable_snapshot_waits() {
        for remaining in [8, 144, 256] {
            for pause in [0, 1, 2, 20, 50] {
                let mut probe = Probe::default();
                probe.arm(remaining);
                let mut actual = remaining;
                for _ in 0..pause {
                    probe.check(true, actual, f32::from((256 - actual) / 16));
                }
                while actual > 0 {
                    actual = actual.saturating_sub(8);
                    probe.check(false, actual, f32::from((256 - actual) / 16));
                    probe.check(true, actual, f32::from((256 - actual) / 16));
                }
                assert_eq!(probe.ticks, remaining / 8);
            }
        }
    }

    #[test]
    #[should_panic(expected = "saved ascent must consume eight subpixels per live update")]
    fn ascent_probe_rejects_a_missed_live_update() {
        let mut probe = Probe::default();
        probe.arm(144);
        probe.check(false, 144, 7.0);
    }
}
