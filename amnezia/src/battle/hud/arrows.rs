use super::*;

#[derive(Component)]
pub(super) struct Arrow {
    panel: Panel,
    up: bool,
}

pub(super) fn spawn(parent: &mut ChildSpawnerCommands, system: &Handle<Image>, panel: Panel) {
    if !matches!(panel, Panel::Command | Panel::Option) {
        return;
    }
    for up in [true, false] {
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: px(48),
                height: px(24),
                top: if up { px(0) } else { Val::Auto },
                bottom: if up { Val::Auto } else { px(0) },
                ..default()
            },
            ImageNode {
                image: system.clone(),
                rect: Some(Rect::new(
                    40.0,
                    if up { 8.0 } else { 16.0 },
                    56.0,
                    if up { 16.0 } else { 24.0 },
                )),
                image_mode: NodeImageMode::Stretch,
                ..default()
            },
            Arrow { panel, up },
            Visibility::Hidden,
        ));
    }
}

pub(super) fn update(
    battle: Res<Battle>,
    scroll: Res<ListScroll>,
    clocks: Res<clocks::WindowClocks>,
    mut arrows: Query<(&Arrow, &mut Node, &mut Visibility)>,
) {
    for (arrow, mut node, mut visibility) in &mut arrows {
        let Some((_, _, width, _)) = layout::rectangle(arrow.panel, &battle) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let shown = clocks.arrows(arrow.panel, &battle, scroll.first)[usize::from(!arrow.up)];
        *visibility = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        node.left = px((width / 2.0 - 8.0) * 3.0);
    }
}

pub(in crate::battle) fn verify(world: &mut World) {
    let visible = world
        .query::<(&Arrow, &InheritedVisibility)>()
        .iter(world)
        .filter(|(_, v)| v.get())
        .map(|(a, _)| a.up)
        .collect::<Vec<_>>();
    assert_eq!(visible, [true, false]);
}

pub(crate) struct Snapshot(Vec<(u32, u32, [u8; 4])>);

pub(crate) fn snapshot(world: &mut World, label: &str) -> Option<Snapshot> {
    if label != "battle-scroll-arrows" {
        return None;
    }
    let sources = world
        .query::<(&Arrow, &ImageNode)>()
        .iter(world)
        .filter(|(a, _)| a.panel == Panel::Command)
        .map(|(a, image)| (a.up, image.image.clone()))
        .collect::<Vec<_>>();
    let mut pixels = Vec::new();
    for (up, handle) in sources {
        let image = world.resource::<Assets<Image>>().get(&handle).unwrap();
        for y in 0..8 {
            for x in 0..16 {
                let expected = image
                    .get_color_at(40 + x, (if up { 8 } else { 16 }) + y)
                    .unwrap()
                    .to_srgba()
                    .to_u8_array();
                if expected[3] == 255 {
                    pixels.push((152 + x, (if up { 160 } else { 232 }) + y, expected));
                }
            }
        }
    }
    assert!(!pixels.is_empty());
    Some(Snapshot(pixels))
}

impl Snapshot {
    pub(crate) fn verify(&self, image: &Image) {
        for &(x, y, expected) in &self.0 {
            let actual = image
                .get_color_at(x * image.width() / 320, y * image.height() / 240)
                .unwrap()
                .to_srgba()
                .to_u8_array();
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "scroll arrow ({x},{y}): expected {expected:?}, got {actual:?}"
            );
        }
        info!(
            "scroll arrows: {} source-asset GPU pixels verified",
            self.0.len()
        );
    }
}
