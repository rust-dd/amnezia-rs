use super::*;
use crate::windowskin::motion::Pixels;

#[derive(Component)]
pub(in crate::dialogue) struct AnimatedFrame;

pub(super) fn spawn(panel: &mut ChildSpawnerCommands) {
    panel.spawn((
        fill_node(),
        Pixels {
            size: UVec2::new(320, 80),
            half: 0,
        },
        AnimatedFrame,
        Visibility::Hidden,
    ));
}

#[allow(clippy::type_complexity)]
pub(in crate::dialogue) fn render(
    dialogue: Res<Dialogue>,
    transparent: Res<MessageTransparent>,
    mut nodes: Query<(
        &mut Visibility,
        Option<&mut Pixels>,
        Has<AnimatedFrame>,
        Has<DialogueFrame>,
        Has<DialogueText>,
        Has<DialogueFace>,
        Has<DialogueArrow>,
        Has<super::prompts::Cursor>,
    )>,
) {
    let motion = dialogue.lifecycle.message;
    let animating = motion.visible() && !motion.ready();
    for (mut visibility, pixels, animated, frame, text, face, arrow, cursor) in &mut nodes {
        if animated {
            pixels.unwrap().half = motion.half_height(80);
            *visibility = visible_if(animating && !transparent.0);
        } else if frame && animating {
            *visibility = Visibility::Hidden;
        } else if text {
            *visibility = visible_if(!animating);
        } else if animating && (face || arrow || cursor) {
            *visibility = Visibility::Hidden;
        }
    }
}
