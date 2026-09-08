//! The Game Over screen: raised by the `GameOver` command (12420) or an
//! unrecoverable party wipe, both of which set [`GameOverActive`] on the
//! interpreter's channel. It displays the `GameOver` graphic (with a "Game Over"
//! text fallback beneath) above every other layer, waits for the confirm key,
//! then returns to the title by raising [`TitleActive`].
//!
//! [`GameOverActive`] stays raised for the whole screen so the interpreter's
//! autorun/run guards pause — otherwise an autorun `GameOver` page would
//! re-trigger every frame and the screen could never be dismissed. The title is
//! raised one frame past the confirm keypress so the title's own input can't
//! consume that same press.

use crate::assets::resolve_png;
use crate::audio::{AudioRequest, SystemMusic};
use crate::font::GameFont;
use crate::title::TitleActive;
use bevy::prelude::*;
use bevy::text::FontSource;

/// Whether the Game Over screen owns the display. The interpreter raises it; this
/// module lowers it on the return to the title. While it holds, the interpreter
/// pauses so no map event re-triggers the screen.
#[derive(Resource, Default)]
pub struct GameOverActive(pub bool);

/// Where the Game Over flow is: hidden, showing and awaiting a key, or handing
/// back to the title on the frame after the key.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    #[default]
    Inactive,
    Showing,
    Returning,
}

#[derive(Component)]
struct GameOverRoot;

pub struct GameOverPlugin;

impl Plugin for GameOverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameOverActive>()
            .init_resource::<Phase>()
            .add_systems(Startup, spawn_ui)
            .add_systems(Update, (drive, update_ui).chain());
    }
}

/// The fullscreen Game Over overlay, above the title (z 2000) and every effect,
/// hidden until raised: the `GameOver` graphic over a "Game Over" text fallback
/// on black.
fn spawn_ui(mut commands: Commands, font: Res<GameFont>, asset_server: Res<AssetServer>) {
    let graphic: Handle<Image> = asset_server.load(resolve_png("GameOver", "GameOver"));
    commands
        .spawn((
            fill_node(),
            BackgroundColor(Color::BLACK),
            Visibility::Hidden,
            GlobalZIndex(3000),
            GameOverRoot,
        ))
        .with_children(|root| {
            root.spawn((Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },))
                .with_children(|center| {
                    center.spawn((
                        Text::new("Game Over"),
                        TextFont {
                            font: FontSource::Handle(font.0.clone()),
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            root.spawn((
                fill_node(),
                ImageNode {
                    image: graphic,
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
            ));
        });
}

/// An absolutely-positioned node filling its parent on every side.
fn fill_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        ..default()
    }
}

/// Enter the screen once raised, wait for the confirm key, then hand back to the
/// title on the following frame (dropping [`GameOverActive`] so the world
/// resumes under the now-raised title, and so the title's input never sees the
/// same keypress).
fn drive(
    keys: Res<ButtonInput<KeyCode>>,
    mut active: ResMut<GameOverActive>,
    mut phase: ResMut<Phase>,
    mut title: ResMut<TitleActive>,
    mut audio: MessageWriter<AudioRequest>,
    music: Option<Res<SystemMusic>>,
    overrides: Option<Res<crate::system_bgm::SystemBgm>>,
) {
    match *phase {
        Phase::Inactive => {
            if active.0 {
                *phase = Phase::Showing;
                // The game-over dirge takes over from the map/battle BGM as the
                // screen appears; the return to the title later swaps in the theme.
                if let Some(music) = music {
                    audio.write(AudioRequest::from_music(crate::system_bgm::resolve(
                        overrides.as_deref(),
                        6,
                        &music.gameover,
                    )));
                }
            }
        }
        Phase::Showing => {
            if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
                *phase = Phase::Returning;
            }
        }
        Phase::Returning => {
            title.0 = true;
            active.0 = false;
            *phase = Phase::Inactive;
        }
    }
}

/// Show the overlay whenever the flow is live, hide it once it hands back.
fn update_ui(phase: Res<Phase>, mut roots: Query<&mut Visibility, With<GameOverRoot>>) {
    if !phase.is_changed() {
        return;
    }
    if let Ok(mut visibility) = roots.single_mut() {
        *visibility = if *phase == Phase::Inactive {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::ButtonInput;

    /// A headless app with just the Game Over flow and the resources it touches.
    fn flow_app() -> App {
        let mut app = App::new();
        app.init_resource::<GameOverActive>();
        app.init_resource::<Phase>();
        app.insert_resource(TitleActive(false));
        app.init_resource::<ButtonInput<KeyCode>>();
        // `drive` writes the game-over BGM; register the channel (the SystemMusic
        // resource is optional, so the flow runs without it).
        app.add_message::<AudioRequest>();
        app.add_systems(Update, drive);
        app
    }

    #[test]
    fn raise_shows_then_confirm_returns_to_title_next_frame() {
        let mut app = flow_app();
        app.world_mut().resource_mut::<GameOverActive>().0 = true;
        app.update();
        assert_eq!(*app.world().resource::<Phase>(), Phase::Showing);
        assert!(!app.world().resource::<TitleActive>().0);

        // Confirm: transitions to Returning; title not raised, still active.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert_eq!(*app.world().resource::<Phase>(), Phase::Returning);
        assert!(!app.world().resource::<TitleActive>().0);
        assert!(app.world().resource::<GameOverActive>().0);

        // The key is released; the next frame hands back to the title.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert_eq!(*app.world().resource::<Phase>(), Phase::Inactive);
        assert!(app.world().resource::<TitleActive>().0);
        assert!(!app.world().resource::<GameOverActive>().0);
    }

    #[test]
    fn stays_showing_while_the_trigger_persists() {
        // An autorun GameOver holds the flag raised; the screen must not loop
        // back through Showing, it simply stays shown until a key arrives.
        let mut app = flow_app();
        app.world_mut().resource_mut::<GameOverActive>().0 = true;
        app.update();
        app.update();
        app.update();
        assert_eq!(*app.world().resource::<Phase>(), Phase::Showing);
    }
}
