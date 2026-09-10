use super::*;
use crate::world::{Character, MapData, MoveQueue};
use bevy::ecs::system::SystemParam;

pub(super) mod flash;
pub(crate) mod flash_smoke;
pub(crate) mod smoke;

#[derive(SystemParam)]
pub(super) struct Targets<'w, 's> {
    camera: Query<'w, 's, &'static Transform, With<MainCamera>>,
    hero: Query<
        'w,
        's,
        (
            &'static Player,
            &'static Transform,
            Option<&'static MoveQueue>,
        ),
    >,
    events: Query<
        'w,
        's,
        (
            &'static EventSprite,
            &'static Transform,
            Option<&'static MoveQueue>,
        ),
    >,
    map: Option<Res<'w, MapData>>,
    vehicles: Option<Res<'w, crate::vehicles::Vehicles>>,
}

impl Targets<'_, '_> {
    pub(super) fn anchor(&self, target: AnimTarget) -> Option<AnimAnchor> {
        let camera = self.camera.single().ok()?.translation.truncate();
        let ground = match target {
            AnimTarget::Hero => {
                let (hero, transform, queue) = self.hero.single().ok()?;
                self.ground(
                    hero,
                    transform,
                    queue.filter(|_| !self.vehicles.as_ref().is_some_and(|v| v.riding())),
                )
            }
            AnimTarget::Event(id) => {
                let (event, transform, queue) =
                    self.events.iter().find(|(event, _, _)| event.id == id)?;
                self.ground(event, transform, queue)
            }
        };
        let ground = self
            .map
            .as_ref()
            .map_or(ground, |map| map.world_near(ground, camera));
        // GetScreenY(false) anchors a 24-pixel character above its ground-level feet.
        let pos = target_screen_offset(
            Vec2::new(ground.x.floor(), ground.y.ceil()),
            Vec2::new(camera.x.floor(), camera.y.ceil()),
        ) + Vec2::new(0.0, -4.0);
        Some(AnimAnchor {
            pos,
            height: MAP_CHARACTER_HEIGHT,
        })
    }

    fn ground<C: Character>(
        &self,
        character: &C,
        transform: &Transform,
        queue: Option<&MoveQueue>,
    ) -> Vec2 {
        if let Some((queue, map)) = queue.zip(self.map.as_deref()) {
            queue.ground_position(character, map)
        } else {
            transform.translation.truncate() - Vec2::Y * character.y_offset()
        }
    }
}

pub(super) fn resolve_map_animation(
    mut requests: MessageReader<ShowMapAnimation>,
    mut plays: MessageWriter<PlayAnimation>,
    targets: Targets,
    library: Res<AnimationLibrary>,
    mut commands: Commands,
    mut animations: Query<(Entity, &mut playback::LiveAnimation)>,
) {
    let Some(request) = requests
        .read()
        .filter(|request| library.0.iter().any(|def| def.id == request.anim_id))
        .last()
    else {
        return;
    };
    let Some(anchor) = targets.anchor(request.target) else {
        for (entity, mut animation) in &mut animations {
            if animation.slot == AnimationSlot::Map {
                playback::cancel(&mut commands, entity, &mut animation);
            }
        }
        return;
    };
    plays.write(PlayAnimation {
        slot: AnimationSlot::Map,
        map_target: Some(request.target),
        anim_id: request.anim_id,
        targets: vec![anchor],
        screen_center: MAP_SCREEN_CENTER,
        global: request.global,
        sound_only: false,
    });
}
