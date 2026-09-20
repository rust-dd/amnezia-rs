use super::*;

pub(super) fn spawn(panel: &mut ChildSpawnerCommands, system: &Handle<Image>) {
    spawn_command_window(panel, system);
    spawn_gold_window(panel, system);
    spawn_status_window(panel, system);
}

/// The command window (RM2000 0,0,88,96 → 0,0,264,288): five bare command rows
/// under a windowskin cursor.
fn spawn_command_window(panel: &mut ChildSpawnerCommands, system: &Handle<Image>) {
    panel
        .spawn((
            window_node(0.0, 0.0, 264.0, 288.0),
            Visibility::Hidden,
            MenuWindow(WindowId::Command),
        ))
        .with_children(|w| {
            crate::windowskin::fixed_frame(w, system, UVec2::new(88, 96));
            cursor_sprite(w, system, CursorId::Command, 12.0, 240.0, 48.0);
            for i in 0..command::COMMANDS.len() {
                w.spawn((
                    main_text::at(24.0, CMD_ROW_TOP + 6.0 + i as f32 * CMD_ROW_PITCH, 72),
                    MenuText(TextSlot::Command(i)),
                ));
            }
        });
}

/// Right-aligned gold and currency, with independent palette colours.
fn spawn_gold_window(panel: &mut ChildSpawnerCommands, system: &Handle<Image>) {
    panel
        .spawn((
            window_node(0.0, 624.0, 264.0, 96.0),
            Visibility::Hidden,
            MenuWindow(WindowId::Gold),
        ))
        .with_children(|w| {
            crate::windowskin::fixed_frame(w, system, UVec2::new(88, 32));
            w.spawn((main_text::at(24.0, 30.0, 72), MenuText(TextSlot::Gold)));
        });
}

/// The status window (RM2000 88,0,232,240 → 264,0,696,720): a portrait plus the
/// member's identity and vitals per row, under the member-select cursor. Field
/// offsets are `Window_MenuStatus::Refresh` values ×3.
fn spawn_status_window(panel: &mut ChildSpawnerCommands, system: &Handle<Image>) {
    panel
        .spawn((
            window_node(264.0, 0.0, 696.0, 720.0),
            Visibility::Hidden,
            MenuWindow(WindowId::Status),
        ))
        .with_children(|w| {
            crate::windowskin::fixed_frame(w, system, UVec2::new(232, 240));
            cursor_sprite(w, system, CursorId::Status, 180.0, 504.0, 144.0);
            w.spawn(Node {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                top: Val::Px(24.0),
                right: Val::Px(24.0),
                bottom: Val::Px(24.0),
                overflow: Overflow::clip(),
                ..default()
            })
            .with_children(|w| {
                for slot in 0..MAX_SLOTS {
                    let top = slot as f32 * MEMBER_PITCH;
                    w.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(top),
                            width: Val::Px(144.0),
                            height: Val::Px(144.0),
                            ..default()
                        },
                        ImageNode::default(),
                        Visibility::Hidden,
                        MenuFace(slot),
                    ));
                    let field = |f, x, y| {
                        (
                            main_text::at(x, y, 216 - (x / 3.0) as u32),
                            MenuText(TextSlot::Member { slot, field: f }),
                        )
                    };
                    w.spawn(field(MemberField::Name, 168.0, top + 6.0));
                    w.spawn(field(MemberField::Title, 432.0, top + 6.0));
                    w.spawn(field(MemberField::Level, 168.0, top + 54.0));
                    w.spawn(field(MemberField::Condition, 294.0, top + 54.0));
                    w.spawn(field(MemberField::Hp, 486.0, top + 54.0));
                    w.spawn(field(MemberField::Exp, 168.0, top + 102.0));
                    w.spawn(field(MemberField::Sp, 486.0, top + 102.0));
                }
            });
        });
}
