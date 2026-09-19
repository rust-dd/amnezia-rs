use super::SaveFiles;
use crate::menu::MenuOpen;
use crate::save::EventSaveRequest;
use crate::teleport::Fade;
use crate::transitions::Transition;
use bevy::prelude::*;

pub(super) fn update(
    mut files: ResMut<SaveFiles>,
    open: Res<MenuOpen>,
    request: Res<EventSaveRequest>,
    fade: Res<Fade>,
    transition: Res<Transition>,
) {
    if fade.busy() || transition.busy() {
        return;
    }
    if request.0 && !files.active() {
        files.event_menu = Some(open.0);
        files.request();
    }
}
