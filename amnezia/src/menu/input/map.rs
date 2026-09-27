use super::*;

#[derive(Resource, Default)]
pub(crate) struct Calling {
    pending: bool,
    requested: bool,
}

impl Calling {
    pub(crate) fn consume(&mut self) -> bool {
        if std::mem::take(&mut self.pending) {
            self.requested = true;
            return true;
        }
        false
    }

    pub(crate) fn cancel(&mut self) {
        self.pending = false;
    }

    #[cfg(test)]
    pub(crate) fn pending(&self) -> bool {
        self.pending
    }
}

pub(in crate::menu) fn register(app: &mut App) {
    crate::interpreter::scenes::register(app);
    app.init_resource::<Calling>()
        .init_resource::<crate::menu::SceneFlow>()
        .add_systems(
            Update,
            (capture, request.in_set(crate::menu::MapMenuRequest))
                .chain()
                .after(crate::menu::MenuInput)
                .after(crate::player::CameraFollow)
                .before(crate::dialogue::MessageUpdate),
        );
}

#[allow(clippy::too_many_arguments)]
fn capture(
    keys: Res<ButtonInput<KeyCode>>,
    open: Res<MenuOpen>,
    files: Res<SaveFiles>,
    gates: MenuGates,
    title: Res<TitleActive>,
    blockers: OpenBlockers,
    mut calling: ResMut<Calling>,
) {
    if !open.0
        && !files.active()
        && !title.0
        && !gates.shop.0
        && !gates.battle.0
        && gates.menu_access.0
        && gates.scene.as_ref().is_none_or(|flow| !flow.active())
        && gates.switch.as_ref().is_none_or(|switch| !switch.active())
        && blockers.frame.as_ref().is_none_or(|frame| !frame.0)
        && !blockers.any()
        && keys.just_pressed(KeyCode::Escape)
    {
        calling.pending = true;
    }
}

pub(in crate::menu) fn request(
    mut calling: ResMut<Calling>,
    mut scenes: ResMut<crate::interpreter::scenes::Requests>,
    mut sounds: MenuSfx,
) {
    if std::mem::take(&mut calling.requested) {
        sounds.decision();
        scenes.menu();
    }
}
