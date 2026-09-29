use super::model::Battle;
use crate::screenfx::{ScreenEffect, ScreenEffectsSet};
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

fn drain_pending_shake(mut battle: ResMut<Battle>, mut effects: MessageWriter<ScreenEffect>) {
    if battle.pending_shake {
        battle.pending_shake = false;
        effects.write(ScreenEffect::Shake {
            power: 3,
            speed: 5,
            secs: 8.0 / 60.0,
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_message::<ScreenEffect>().add_systems(
        Update,
        (
            drain_hit_reports.after(super::systems::resolve_tick),
            drain_pending_shake
                .after(super::systems::resolve_tick)
                .before(ScreenEffectsSet),
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::super::model::{HitKind, HitReport};
    use super::*;

    #[test]
    fn a_resolved_enemy_hit_emits_one_original_eight_frame_shake() {
        use crate::battle::model::{Action, Command, Source, testkit};
        use crate::screenfx::ScreenEffect;

        let mut battle = testkit::build_1v2();
        battle.states = crate::conditions::definitions().to_vec();
        battle.members[0].states.push((7, 0));
        battle.members[0].stats.defense = 0;
        battle.enemies[0].stats.attack = 30;
        let hp = battle.members[0].hp;
        battle.queue = vec![Action {
            source: Source::Enemy(0),
            kind: Command::Attack { target: 0 },
            agility: 1,
        }];
        assert!(battle.resolve_next_with_items(|_, _| true));
        assert!(battle.members[0].hp < hp);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<ScreenEffect>()
            .insert_resource(battle);
        register(&mut app);
        app.update();
        let effects = app
            .world_mut()
            .resource_mut::<Messages<ScreenEffect>>()
            .drain()
            .collect::<Vec<_>>();
        assert_eq!(
            effects,
            [ScreenEffect::Shake {
                power: 3,
                speed: 5,
                secs: 8.0 / 60.0
            }]
        );
        app.update();
        assert!(app.world().resource::<Messages<ScreenEffect>>().is_empty());
    }

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
