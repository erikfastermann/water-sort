use bevy::ecs::system::SystemParam;
use bevy::math::curve::{Curve, EaseFunction};
use bevy::prelude::*;
use bevy::sprite::{Text2d, Text2dShadow};
use bevy::text::LineHeight;

use crate::anim::{Flow, Play};
use crate::art::{Art, Plate};
use crate::fx::{FitViewport, Particle};
use crate::geometry::{Anchored, Band, HEADER_CENTER, NAV_CENTER, PLAY_CENTER};
use crate::input::PendingClick;
use crate::rng::Pcg32;
use crate::session::Session;
use crate::theme;
use crate::view::BoardView;

const CONFETTI_SEED: u64 = 0xC0FF_E771;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavRequest>()
            .init_resource::<Shell>()
            .insert_resource(ConfettiRng(Pcg32::new(CONFETTI_SEED, 1)))
            .add_systems(Startup, spawn_hud)
            .add_systems(
                Update,
                handle_nav
                    .in_set(Play::Input)
                    .before(crate::input::handle_click),
            )
            .add_systems(
                Update,
                (sync_header, sync_nav, sync_overlay).in_set(Play::Apply),
            );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NavAction {
    Rewind,
    Forward,
    Next,
}

#[derive(Component, Clone, Copy)]
pub struct NavButton {
    pub action: NavAction,
}

#[derive(Resource, Default)]
pub struct NavRequest(Option<NavAction>);

impl NavRequest {
    pub fn take(&mut self) -> Option<NavAction> {
        self.0.take()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Overlay {
    None,
    Complete,
    Stuck,
    Exhausted,
}

impl Overlay {
    fn banner(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Complete => "LEVEL COMPLETE",
            Self::Stuck => "NO MORE MOVES",
            Self::Exhausted => "NO MORE LEVELS\nCOME BACK LATER",
        }
    }

    fn dim(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Stuck => 0.75,
            Self::Complete | Self::Exhausted => 1.0,
        }
    }
}

#[derive(Resource)]
struct Shell {
    overlay: Overlay,
    dim: f32,
    clock: f32,
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            overlay: Overlay::None,
            dim: 0.0,
            clock: 0.0,
        }
    }
}

#[derive(Resource)]
struct ConfettiRng(Pcg32);

/// The resources the overlay derives itself from, bundled to keep the system
/// signature short.
#[derive(SystemParam)]
struct Progress<'w> {
    view: Res<'w, BoardView>,
    flow: Res<'w, Flow>,
    session: Res<'w, Session>,
    shell: ResMut<'w, Shell>,
    rng: ResMut<'w, ConfettiRng>,
}

#[derive(Component)]
struct HeaderLabel;

#[derive(Component)]
struct Banner;

#[derive(Component)]
struct Dim;

#[derive(Clone, Copy)]
enum Role {
    Ring,
    Gold,
    Face,
    Glyph,
}

#[derive(Component)]
struct NavPart {
    action: NavAction,
    role: Role,
}

fn spawn_hud(mut commands: Commands, art: Res<Art>, session: Res<Session>) {
    commands.spawn((
        Dim,
        Sprite {
            color: theme::OVERLAY_DIM.with_alpha(0.0),
            custom_size: Some(Vec2::new(theme::CANVAS_W, theme::CANVAS_H)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, theme::Z_OVERLAY),
        FitViewport,
        Pickable::IGNORE,
    ));

    let header = plate(
        &mut commands,
        &art.header_pill,
        HEADER_CENTER,
        theme::Z_HUD,
        theme::HEADER_BORDER,
        theme::HEADER_FILL,
    );
    commands
        .entity(header)
        .insert(Anchored::new(Band::Header, Vec2::ZERO));
    commands.spawn((
        HeaderLabel,
        label(level_text(&session), theme::HEADER_FONT, theme::HEADER_TEXT),
        Transform::from_xyz(0.0, 0.0, theme::Z_HUD_GLYPH),
        ChildOf(header),
    ));

    let panel = plate(
        &mut commands,
        &art.nav_panel,
        NAV_CENTER,
        theme::Z_HUD,
        theme::NAV_BORDER,
        theme::NAV_FILL,
    );
    commands
        .entity(panel)
        .insert(Anchored::new(Band::Nav, Vec2::ZERO));
    let offset = (theme::NAV_BUTTON + theme::NAV_GAP) * 0.5;
    nav_button(&mut commands, &art, NavAction::Rewind, -Vec2::X * offset);
    nav_button(&mut commands, &art, NavAction::Forward, Vec2::X * offset);

    let next = plate(
        &mut commands,
        &art.next_pill,
        PLAY_CENTER + Vec2::Y * theme::NEXT_OFFSET,
        theme::Z_HUD + theme::Z_HUD_GLYPH,
        theme::NAV_RING,
        theme::NAV_BTN,
    );
    commands.entity(next).insert((
        NavButton {
            action: NavAction::Next,
        },
        Anchored::new(Band::Play, Vec2::Y * theme::NEXT_OFFSET),
        Visibility::Hidden,
        Pickable::default(),
    ));
    commands.spawn((
        label("NEXT LEVEL", theme::NEXT_FONT, theme::NAV_GLYPH),
        Transform::from_xyz(theme::NEXT_LABEL_X, 0.0, theme::Z_HUD_GLYPH),
        ChildOf(next),
    ));
    commands.spawn((
        Sprite {
            image: art.nav_next.clone(),
            color: theme::NAV_GLYPH,
            custom_size: Some(Vec2::splat(theme::NEXT_GLYPH_SIZE)),
            ..default()
        },
        Transform::from_xyz(theme::NEXT_GLYPH_X, 0.0, theme::Z_HUD_GLYPH),
        Pickable::IGNORE,
        ChildOf(next),
    ));

    commands.spawn((
        Banner,
        label("", theme::BANNER_FONT, theme::BANNER_TEXT),
        TextLayout::justify(Justify::Center),
        LineHeight::RelativeToFont(theme::BANNER_LINE),
        Text2dShadow {
            offset: theme::BANNER_SHADOW_OFFSET,
            color: theme::BANNER_SHADOW,
        },
        Visibility::Hidden,
        Anchored::new(Band::Play, Vec2::Y * theme::BANNER_OFFSET),
        Transform::from_translation(
            (PLAY_CENTER + Vec2::Y * theme::BANNER_OFFSET).extend(theme::Z_BANNER),
        ),
    ));
}

fn label(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text2d::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(color),
    )
}

fn level_text(session: &Session) -> String {
    format!("LEVEL {}", session.index() + 1)
}

fn plate(
    commands: &mut Commands,
    plate: &Plate,
    center: Vec2,
    z: f32,
    border: Color,
    fill: Color,
) -> Entity {
    let outer = commands
        .spawn((
            Sprite {
                image: plate.outer.clone(),
                color: border,
                custom_size: Some(plate.size),
                ..default()
            },
            Transform::from_translation(center.extend(z)),
            Pickable::IGNORE,
        ))
        .id();
    commands.spawn((
        Sprite {
            image: plate.inner.clone(),
            color: fill,
            custom_size: Some(plate.inner_size()),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, theme::Z_HUD_FACE),
        Pickable::IGNORE,
        ChildOf(outer),
    ));
    outer
}

fn nav_button(commands: &mut Commands, art: &Art, action: NavAction, offset: Vec2) {
    let disc = |size: f32, role: Role, color: Color, z: f32| {
        (
            NavPart { action, role },
            Sprite {
                image: art.nav_button.clone(),
                color,
                custom_size: Some(Vec2::splat(size)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, z),
            Pickable::IGNORE,
        )
    };

    let root = commands
        .spawn((
            NavButton { action },
            NavPart {
                action,
                role: Role::Ring,
            },
            Sprite {
                image: art.nav_button.clone(),
                color: theme::NAV_RING,
                custom_size: Some(Vec2::splat(theme::NAV_BUTTON)),
                ..default()
            },
            Transform::from_translation(
                (NAV_CENTER + offset).extend(theme::Z_HUD + theme::Z_HUD_GLYPH),
            ),
            Anchored::new(Band::Nav, offset),
            Pickable::default(),
        ))
        .id();
    commands.spawn((
        disc(
            theme::NAV_BUTTON - 2.0 * theme::NAV_RING_W,
            Role::Gold,
            theme::NAV_RING_IN,
            theme::Z_HUD_FACE,
        ),
        ChildOf(root),
    ));
    commands.spawn((
        disc(
            theme::NAV_BUTTON - 2.0 * (theme::NAV_RING_W + theme::NAV_RING_IN_W),
            Role::Face,
            theme::NAV_BTN,
            theme::Z_HUD_GLYPH,
        ),
        ChildOf(root),
    ));
    commands.spawn((
        NavPart {
            action,
            role: Role::Glyph,
        },
        Sprite {
            image: art.nav_undo.clone(),
            color: theme::NAV_GLYPH,
            custom_size: Some(Vec2::splat(theme::NAV_GLYPH_SIZE)),
            flip_x: matches!(action, NavAction::Forward),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, theme::Z_HUD_GLYPH + theme::Z_HUD_FACE),
        Pickable::IGNORE,
        ChildOf(root),
    ));
}

/// Runs before the game's own click handler so a nav press is not mistaken for
/// a click on the background. A press that arrives while an animation is
/// running is left alone, so it cancels like any other click.
pub fn handle_nav(
    flow: Res<Flow>,
    mut pending: ResMut<PendingClick>,
    mut request: ResMut<NavRequest>,
    buttons: Query<&NavButton>,
) {
    if flow.is_busy() {
        return;
    }
    let Some(entity) = pending.entity() else {
        return;
    };
    let Ok(button) = buttons.get(entity) else {
        return;
    };
    pending.take();
    request.0 = Some(button.action);
}

fn state_of(view: &BoardView, flow: &Flow, session: &Session) -> Overlay {
    if session.exhausted() {
        Overlay::Exhausted
    } else if flow.is_busy() {
        Overlay::None
    } else if view.solved {
        Overlay::Complete
    } else if view.stuck {
        Overlay::Stuck
    } else {
        Overlay::None
    }
}

fn sync_header(session: Res<Session>, mut labels: Query<&mut Text2d, With<HeaderLabel>>) {
    let text = level_text(&session);
    for mut label in &mut labels {
        if label.0 != text {
            label.0.clone_from(&text);
        }
    }
}

fn sync_nav(
    shell: Res<Shell>,
    session: Res<Session>,
    mut buttons: Query<(&NavButton, &mut Visibility)>,
    mut parts: Query<(&NavPart, &mut Sprite)>,
) {
    let enabled = |action: NavAction| match action {
        NavAction::Rewind => session.can_rewind(),
        NavAction::Forward => session.can_forward(),
        NavAction::Next => shell.overlay == Overlay::Complete,
    };

    for (button, mut visibility) in &mut buttons {
        if matches!(button.action, NavAction::Next) {
            *visibility = if enabled(button.action) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }

    for (part, mut sprite) in &mut parts {
        let on = enabled(part.action);
        sprite.color = match (part.role, on) {
            (Role::Ring, true) => theme::NAV_RING,
            (Role::Gold, true) => theme::NAV_RING_IN,
            (Role::Face, true) => theme::NAV_BTN,
            (Role::Glyph, true) => theme::NAV_GLYPH,
            (Role::Ring | Role::Gold, false) => theme::NAV_BTN_OFF,
            (Role::Face, false) => theme::NAV_BTN_OFF,
            (Role::Glyph, false) => theme::NAV_GLYPH_OFF,
        };
    }
}

/// The overlay is derived from the resting board every frame, exactly like the
/// board decorations, so cancelling or rewinding needs no bookkeeping.
fn sync_overlay(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<Art>,
    mut progress: Progress,
    mut dims: Query<&mut Sprite, With<Dim>>,
    banner: Single<(&mut Text2d, &mut Transform, &mut Visibility), With<Banner>>,
) {
    let overlay = state_of(&progress.view, &progress.flow, &progress.session);
    let shell = &mut progress.shell;
    if overlay != shell.overlay {
        shell.overlay = overlay;
        shell.clock = 0.0;
        if overlay == Overlay::Complete {
            confetti(&mut commands, &art, &mut progress.rng.0);
        }
    }
    shell.clock += time.delta_secs();

    let step = time.delta_secs() / theme::COMPLETE_DIM;
    let target = overlay.dim();
    shell.dim = if shell.dim < target {
        (shell.dim + step).min(target)
    } else {
        (shell.dim - step).max(target)
    };
    for mut sprite in &mut dims {
        sprite.color = theme::OVERLAY_DIM.with_alpha(theme::OVERLAY_DIM.alpha() * shell.dim);
    }

    let (mut text, mut transform, mut visibility) = banner.into_inner();
    if overlay == Overlay::None {
        *visibility = Visibility::Hidden;
        return;
    }

    *visibility = Visibility::Inherited;
    let wanted = overlay.banner();
    if text.0 != wanted {
        text.0 = wanted.to_owned();
    }

    let grown = EaseFunction::BackOut.sample_clamped(shell.clock / theme::COMPLETE_TEXT);
    transform.scale = Vec3::splat(theme::BANNER_SCALE_FROM.lerp(1.0, grown));
    let sway = (shell.clock * std::f32::consts::TAU * theme::BANNER_SWAY_HZ).sin();
    transform.rotation = Quat::from_rotation_z(theme::BANNER_SWAY * sway * grown);
}

fn confetti(commands: &mut Commands, art: &Art, rng: &mut Pcg32) {
    let colors = theme::item_colors();
    for _ in 0..theme::CONFETTI_COUNT {
        let velocity = Vec2::new(
            rng.range(-theme::CONFETTI_SPREAD_X, theme::CONFETTI_SPREAD_X),
            -rng.range(theme::CONFETTI_SPEED_MIN, theme::CONFETTI_SPEED_MAX),
        );
        let color = colors[1 + rng.below(colors.len() as u32 - 1) as usize];
        commands.spawn((
            Particle {
                velocity,
                gravity: theme::CONFETTI_GRAVITY,
                spin: rng.range(-theme::CONFETTI_SPIN, theme::CONFETTI_SPIN),
                life: theme::CONFETTI_LIFE,
                max_life: theme::CONFETTI_LIFE,
            },
            Sprite {
                image: art.confetti.clone(),
                color,
                custom_size: Some(Vec2::new(theme::CONFETTI_W, theme::CONFETTI_H)),
                ..default()
            },
            Transform::from_xyz(
                rng.range(-theme::CANVAS_W * 0.5, theme::CANVAS_W * 0.5),
                theme::CANVAS_H * 0.5 + rng.range(0.0, theme::CONFETTI_DROP),
                theme::Z_CONFETTI,
            ),
            Pickable::IGNORE,
        ));
    }
}
