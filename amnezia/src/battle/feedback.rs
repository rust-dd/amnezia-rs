use super::model::Battle;
use bevy::prelude::*;

fn drain_hit_reports(mut battle: ResMut<Battle>) {
    if battle.hit_reports.is_empty() {
        return;
    }
    // Front-view RPG2000 reports results in its message window, not floating text.
    for hit in battle.hit_reports.drain(..) {
        trace!(pos = ?hit.pos, value = hit.text, kind = ?hit.kind, "Battle hit");
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        drain_hit_reports.after(super::systems::resolve_tick),
    );
}

#[cfg(test)]
mod tests {
    use super::super::model::{HitKind, HitReport};
    use super::*;

    #[test]
    fn damage_healing_and_misses_do_not_spawn_text_in_front_view() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).init_resource::<Battle>();
        register(&mut app);
        for (text, kind) in [
            ("42", HitKind::Damage),
            ("17", HitKind::Heal),
            ("Miss", HitKind::Miss),
        ] {
            app.world_mut()
                .resource_mut::<Battle>()
                .hit_reports
                .push(HitReport {
                    pos: (0.0, -20.0),
                    text: text.into(),
                    kind,
                });
        }
        app.update();
        assert!(app.world().resource::<Battle>().hit_reports.is_empty());
        assert_eq!(
            app.world_mut().query::<&Text2d>().iter(app.world()).count(),
            0
        );
        assert_eq!(
            app.world_mut().query::<&Sprite>().iter(app.world()).count(),
            0
        );
    }
}
