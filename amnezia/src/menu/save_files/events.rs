use super::SaveFiles;
use crate::menu::MenuOpen;
use crate::save::EventSaveRequest;
use crate::teleport::Fade;
use crate::transitions::Transition;
use bevy::prelude::*;

pub(super) fn update(
    mut files: ResMut<SaveFiles>,
    mut open: ResMut<MenuOpen>,
    mut request: ResMut<EventSaveRequest>,
    fade: Res<Fade>,
    mut transition: Option<ResMut<Transition>>,
) {
    if fade.busy()
        || transition
            .as_ref()
            .is_some_and(|transition| transition.busy())
    {
        return;
    }
    if let Some(was_open) = files.event_menu
        && let Some(save) = files.finished
    {
        request.0 = save;
        open.0 = was_open;
        *files = default();
    } else if request.0 && !files.active() {
        if let Some(transition) = transition.as_deref_mut() {
            transition.clear();
        }
        files.event_menu = Some(open.0);
        files.request();
        open.0 = true;
    }
}
