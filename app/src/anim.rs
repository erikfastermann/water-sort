use std::f32::consts::TAU;

use bevy::ecs::system::SystemParam;
use bevy::math::curve::{Curve, EaseFunction};
use bevy::prelude::*;

use crate::art::Art;
use crate::board::{BoardRoot, Fluids};
use crate::fx::Particle;
use crate::geometry::{BoardGeometry, MAX_INTRO_DELAY};
use crate::theme;
use crate::view::{BoardView, MovePlan};

/// Everything that reacts to a click has to settle in this order within a
/// frame, otherwise a cancelled animation renders one stale frame.
#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Play {
    Input,
    Advance,
    Apply,
}

pub struct AnimPlugin;

impl Plugin for AnimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Flow>()
            .configure_sets(Update, (Play::Input, Play::Advance, Play::Apply).chain())
            .add_systems(Update, advance_flow.in_set(Play::Advance))
            .add_systems(Update, (drive_stream, drive_swirl).in_set(Play::Apply));
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PourPhase {
    Travel,
    Stream,
    Return,
}

enum FlowKind {
    Intro,
    Idle,
    Pour { plan: MovePlan, phase: PourPhase },
    Effects { plan: MovePlan },
}

/// Drives every animation that a cancel click must be able to skip. Core state
/// is already up to date when a `Pour` starts, so finishing early can never
/// desync the board — it only stops offsetting from the resting state.
#[derive(Resource)]
pub struct Flow {
    kind: FlowKind,
    clock: f32,
}

impl Default for Flow {
    fn default() -> Self {
        Self {
            kind: FlowKind::Intro,
            clock: 0.0,
        }
    }
}

pub struct PourState<'a> {
    pub plan: &'a MovePlan,
    pub phase: PourPhase,
    pub t: f32,
    /// Fractional number of items that have left the source bottle.
    pub poured: f32,
}

impl Flow {
    pub fn is_busy(&self) -> bool {
        !matches!(self.kind, FlowKind::Idle)
    }

    pub fn restart_intro(&mut self) {
        self.kind = FlowKind::Intro;
        self.clock = 0.0;
    }

    pub fn start_pour(&mut self, plan: MovePlan) {
        self.kind = FlowKind::Pour {
            plan,
            phase: PourPhase::Travel,
        };
        self.clock = 0.0;
    }

    pub fn advance(&mut self, dt: f32) {
        let mut left = dt;
        while let Some(duration) = self.duration() {
            if self.clock + left < duration {
                self.clock += left;
                return;
            }
            left -= duration - self.clock;
            self.clock = 0.0;
            self.step();
        }
    }

    pub fn finish_now(&mut self) {
        self.advance(f32::INFINITY);
    }

    pub fn intro(&self) -> Option<f32> {
        matches!(self.kind, FlowKind::Intro).then_some(self.clock)
    }

    pub fn pour(&self) -> Option<PourState<'_>> {
        let FlowKind::Pour { plan, phase } = &self.kind else {
            return None;
        };
        let duration = self.duration().unwrap_or(0.0);
        let t = if duration > 0.0 {
            (self.clock / duration).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let count = f32::from(plan.count);
        let poured = match phase {
            PourPhase::Travel => 0.0,
            PourPhase::Stream => t * count,
            PourPhase::Return => count,
        };
        Some(PourState {
            plan,
            phase: *phase,
            t,
            poured,
        })
    }

    /// The plan of the move that is still playing, plus how many seconds its
    /// effects have been running. The clock is negative while the pour itself
    /// is still on screen, so every effect reads as "not started yet".
    pub fn effects(&self) -> Option<(&MovePlan, f32)> {
        match &self.kind {
            FlowKind::Pour { plan, .. } => Some((plan, -1.0)),
            FlowKind::Effects { plan } => Some((plan, self.clock)),
            _ => None,
        }
    }

    /// `Some` while the move that corks `bottle` is still playing. The value is
    /// seconds into the finalize animation, negative while the pour itself runs.
    pub fn cork_progress(&self, bottle: u8) -> Option<f32> {
        match &self.kind {
            FlowKind::Pour { plan, .. } if plan.finalize == Some(bottle) => Some(-1.0),
            FlowKind::Effects { plan } if plan.finalize == Some(bottle) => Some(self.clock),
            _ => None,
        }
    }

    fn swirl(&self) -> Option<(u8, f32)> {
        let FlowKind::Effects { plan } = &self.kind else {
            return None;
        };
        plan.finalize
            .filter(|_| self.clock < theme::FINALIZE_SWIRL)
            .map(|bottle| (bottle, self.clock))
    }

    fn duration(&self) -> Option<f32> {
        match &self.kind {
            FlowKind::Idle => None,
            FlowKind::Intro => Some(theme::INTRO_SLIDE + MAX_INTRO_DELAY),
            FlowKind::Pour { plan, phase } => Some(match phase {
                PourPhase::Travel => theme::POUR_TRAVEL,
                PourPhase::Stream => stream_time(plan.count),
                PourPhase::Return => theme::POUR_RETURN,
            }),
            FlowKind::Effects { plan } => Some(effects_time(plan)),
        }
    }

    fn step(&mut self) {
        self.kind = match std::mem::replace(&mut self.kind, FlowKind::Idle) {
            FlowKind::Intro | FlowKind::Idle | FlowKind::Effects { .. } => FlowKind::Idle,
            FlowKind::Pour {
                plan,
                phase: PourPhase::Travel,
            } => FlowKind::Pour {
                plan,
                phase: PourPhase::Stream,
            },
            FlowKind::Pour {
                plan,
                phase: PourPhase::Stream,
            } => FlowKind::Pour {
                plan,
                phase: PourPhase::Return,
            },
            FlowKind::Pour {
                plan,
                phase: PourPhase::Return,
            } => FlowKind::Effects { plan },
        };
    }
}

/// Independent effects overlap, so the phase lasts as long as the slowest one.
fn effects_time(plan: &MovePlan) -> f32 {
    let mut longest = 0.0f32;
    if plan.finalize.is_some() {
        longest = longest.max(theme::FINALIZE_SWIRL + theme::CORK_DROP);
    }
    if plan.reveal_item.is_some() {
        longest = longest.max(theme::REVEAL_ITEM);
    }
    if !plan.plug_toggled.is_empty() || plan.unplug.is_some() {
        longest = longest.max(theme::PLUG_TOGGLE);
    }
    longest
}

pub fn stream_time(count: u8) -> f32 {
    theme::POUR_STREAM_MIN.max(f32::from(count) * theme::POUR_PER_ITEM)
}

pub struct PourMotion {
    pub offset: Vec2,
    pub angle: f32,
}

/// Where the pouring bottle sits relative to its home slot, and how far it is
/// tipped. The tilt pivot is the rim, so the mouth is always at
/// `home mouth + offset` regardless of the angle.
pub fn pour_motion(state: &PourState, geometry: &BoardGeometry, view: &BoardView) -> PourMotion {
    let from = view.get(state.plan.from);
    let to = view.get(state.plan.to);
    let from_rect = geometry.bottle_rect(from.lines, from.repr_column);
    let to_rect = geometry.bottle_rect(to.lines, to.repr_column);

    let sign = if to_rect.center().x <= from_rect.center().x {
        1.0
    } else {
        -1.0
    };
    let target = geometry.mouth(to_rect)
        + Vec2::new(sign * theme::POUR_HOVER_X, theme::POUR_HOVER_Y)
        - geometry.mouth(from_rect);

    let fill = f32::from(state.plan.from_height_before) / f32::from(from.capacity.max(1));
    let angle =
        sign * theme::POUR_ANGLE_FULL.lerp(theme::POUR_ANGLE_EMPTY, (1.0 - fill).clamp(0.0, 1.0));

    match state.phase {
        PourPhase::Travel => {
            let travelled = state.t * theme::POUR_TRAVEL;
            let tipped = ((travelled - (theme::POUR_TRAVEL - theme::POUR_TILT)) / theme::POUR_TILT)
                .clamp(0.0, 1.0);
            PourMotion {
                offset: arc(Vec2::ZERO, target, ease(state.t)),
                angle: angle * ease(tipped),
            }
        }
        PourPhase::Stream => PourMotion {
            offset: target,
            angle,
        },
        PourPhase::Return => {
            let eased = ease(state.t);
            PourMotion {
                offset: arc(target, Vec2::ZERO, eased),
                angle: angle * (1.0 - eased),
            }
        }
    }
}

fn ease(t: f32) -> f32 {
    EaseFunction::CubicOut.sample_clamped(t)
}

fn arc(from: Vec2, to: Vec2, u: f32) -> Vec2 {
    from.lerp(to, u) + Vec2::Y * theme::POUR_ARC * 4.0 * u * (1.0 - u)
}

fn advance_flow(time: Res<Time>, mut flow: ResMut<Flow>) {
    flow.advance(time.delta_secs());
}

/// The board-space context both transient effects need; bundled to keep the
/// system signatures short.
#[derive(SystemParam)]
struct Stage<'w> {
    view: Res<'w, BoardView>,
    geometry: Res<'w, BoardGeometry>,
    art: Res<'w, Art>,
}

#[derive(Component)]
struct StreamDroplet(usize);

#[derive(Component)]
struct SwirlStar(usize);

fn quadratic(a: Vec2, control: Vec2, b: Vec2, u: f32) -> Vec2 {
    let inv = 1.0 - u;
    a * inv * inv + control * 2.0 * inv * u + b * u * u
}

fn drive_stream(
    mut commands: Commands,
    flow: Res<Flow>,
    stage: Stage,
    board: Single<Entity, With<BoardRoot>>,
    mut fluids: ResMut<Fluids>,
    mut droplets: Query<(Entity, &StreamDroplet, &mut Transform, &mut Sprite)>,
    mut landed: Local<u8>,
) {
    let Stage {
        view,
        geometry,
        art,
    } = &stage;
    let state = flow.pour();
    let Some(state) = state.filter(|state| state.phase == PourPhase::Stream) else {
        for (entity, ..) in &droplets {
            commands.entity(entity).despawn();
        }
        *landed = 0;
        return;
    };

    let motion = pour_motion(&state, geometry, view);
    let from = view.get(state.plan.from);
    let to = view.get(state.plan.to);
    let head = geometry.mouth(geometry.bottle_rect(from.lines, from.repr_column)) + motion.offset;
    let foot = geometry.mouth(geometry.bottle_rect(to.lines, to.repr_column));
    let control = head.midpoint(foot) + Vec2::Y * theme::STREAM_ARC;
    let color = theme::item_color(state.plan.color);

    let boundary = state.poured.floor() as u8;
    if boundary > *landed {
        *landed = boundary;
        fluids.kick(state.plan.to, theme::FLUID_POUR_KICK);
        splash(&mut commands, art, foot, color, boundary);
    }

    if droplets.is_empty() {
        for index in 0..theme::STREAM_DROPLETS {
            commands.spawn((
                StreamDroplet(index),
                Sprite {
                    image: art.droplet.clone(),
                    color,
                    custom_size: Some(Vec2::new(theme::STREAM_DROPLET_W, theme::STREAM_DROPLET_H)),
                    ..default()
                },
                Transform::from_xyz(head.x, head.y, theme::Z_STREAM),
                ChildOf(*board),
            ));
        }
        return;
    }

    let phase = state.t * stream_time(state.plan.count) / theme::STREAM_CYCLE;
    let fade = (state.t / 0.15).min(1.0) * ((1.0 - state.t) / 0.15).min(1.0);
    for (_, droplet, mut transform, mut sprite) in &mut droplets {
        let u = (phase + droplet.0 as f32 / theme::STREAM_DROPLETS as f32).fract();
        let point = quadratic(head, control, foot, u);
        transform.translation = point.extend(theme::Z_STREAM);
        let ends = (u / 0.12).min(1.0) * ((1.0 - u) / 0.12).min(1.0);
        sprite.color = color.with_alpha(fade * ends);
    }
}

fn splash(commands: &mut Commands, art: &Art, at: Vec2, color: Color, seed: u8) {
    for index in 0..theme::SPLASH_COUNT {
        let spread = (index as f32 + 0.5) / theme::SPLASH_COUNT as f32 - 0.5;
        let angle = spread * 1.6 + f32::from(seed % 3) * 0.08;
        commands.spawn((
            Particle {
                velocity: Vec2::new(angle.sin(), angle.cos()) * theme::SPLASH_SPEED,
                gravity: theme::SPLASH_GRAVITY,
                spin: 0.0,
                life: theme::SPLASH_LIFE,
                max_life: theme::SPLASH_LIFE,
            },
            Sprite {
                image: art.droplet.clone(),
                color,
                custom_size: Some(Vec2::new(theme::DROPLET_W, theme::DROPLET_H) * 0.6),
                ..default()
            },
            Transform::from_xyz(at.x, at.y, theme::Z_STREAM),
        ));
    }
}

fn drive_swirl(
    mut commands: Commands,
    flow: Res<Flow>,
    stage: Stage,
    board: Single<Entity, With<BoardRoot>>,
    mut stars: Query<(Entity, &SwirlStar, &mut Transform, &mut Sprite)>,
) {
    let Stage {
        view,
        geometry,
        art,
    } = &stage;
    let Some((bottle, elapsed)) = flow.swirl() else {
        for (entity, ..) in &stars {
            commands.entity(entity).despawn();
        }
        return;
    };

    if stars.is_empty() {
        for index in 0..theme::SWIRL_STARS {
            commands.spawn((
                SwirlStar(index),
                Sprite {
                    image: art.star.clone(),
                    color: theme::STAR.with_alpha(0.0),
                    custom_size: Some(Vec2::splat(theme::SWIRL_SIZE)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, theme::Z_SWIRL),
                ChildOf(*board),
            ));
        }
        return;
    }

    let bounds = {
        let view = view.get(bottle);
        geometry.bottle_rect(view.lines, view.repr_column)
    };
    let interior = geometry.interior_rect(bounds);
    let mouth = geometry.mouth(bounds);
    let rise = (theme::FINALIZE_SWIRL - theme::SWIRL_STARS as f32 * theme::SWIRL_STAGGER).max(0.1);
    let color = theme::item_color(view.get(bottle).items.last().map_or(1, |item| item.color));

    for (_, star, mut transform, mut sprite) in &mut stars {
        let index = star.0 as f32;
        let progress = ((elapsed - index * theme::SWIRL_STAGGER) / rise).clamp(0.0, 1.0);
        let angle = progress * TAU * theme::SWIRL_TURNS + index * TAU / theme::SWIRL_STARS as f32;
        let x = mouth.x + angle.sin() * theme::SWIRL_RADIUS;
        let y = interior.min.y.lerp(mouth.y, progress);
        let scale = (progress * std::f32::consts::PI).sin();
        let alpha = if progress > 0.7 {
            ((1.0 - progress) / 0.3).clamp(0.0, 1.0)
        } else {
            1.0
        };

        transform.translation = Vec3::new(x, y, theme::Z_SWIRL);
        transform.scale = Vec3::splat(scale.max(0.001));
        sprite.color = color
            .mix(&theme::STAR, theme::SWIRL_TINT)
            .with_alpha(alpha * scale);
    }
}

#[cfg(test)]
mod tests {
    use super::{Flow, FlowKind, PourPhase, stream_time};
    use crate::theme;
    use crate::view::MovePlan;

    fn plan(count: u8, finalize: Option<u8>) -> MovePlan {
        MovePlan {
            from: 1,
            to: 2,
            count,
            color: 1,
            from_height_before: count,
            to_height_before: 0,
            finalize,
            reveal_item: None,
            unlock_item: None,
            unplug: None,
            plug_toggled: Vec::new(),
            unfreeze: None,
            curtains_lifted: Vec::new(),
            safe_ticks: Vec::new(),
            safes_opened: Vec::new(),
            key_used: None,
            unlock_run: None,
            color_curtains_lifted: Vec::new(),
        }
    }

    #[test]
    fn intro_ends_by_itself() {
        let mut flow = Flow::default();
        assert!(flow.is_busy());
        flow.advance(0.01);
        assert!(flow.intro().is_some());
        flow.advance(10.0);
        assert!(!flow.is_busy());
    }

    #[test]
    fn a_pour_walks_every_phase() {
        let mut flow = Flow::default();
        flow.finish_now();
        flow.start_pour(plan(2, Some(2)));

        let mut seen = Vec::new();
        for _ in 0..400 {
            if let Some(state) = flow.pour()
                && seen.last() != Some(&state.phase)
            {
                seen.push(state.phase);
            }
            flow.advance(1.0 / 60.0);
        }

        assert_eq!(
            seen,
            [PourPhase::Travel, PourPhase::Stream, PourPhase::Return]
        );
        assert!(!flow.is_busy());
    }

    #[test]
    fn finishing_early_lands_on_idle_from_any_phase() {
        for elapsed in 0..40 {
            let mut flow = Flow::default();
            flow.finish_now();
            flow.start_pour(plan(3, Some(2)));
            flow.advance(elapsed as f32 * 0.03);
            flow.finish_now();
            assert!(!flow.is_busy());
            assert!(flow.pour().is_none());
            assert!(flow.cork_progress(2).is_none());
        }
    }

    #[test]
    fn a_pour_without_effects_still_reaches_idle() {
        let mut flow = Flow::default();
        flow.finish_now();
        flow.start_pour(plan(1, None));
        flow.advance(theme::POUR_TRAVEL + stream_time(1) + theme::POUR_RETURN);
        assert!(!flow.is_busy());
    }

    #[test]
    fn an_effect_without_a_finalize_still_gets_its_own_time() {
        let mut flow = Flow::default();
        flow.finish_now();
        let mut script = plan(1, None);
        script.plug_toggled = vec![3];
        flow.start_pour(script);

        flow.advance(theme::POUR_TRAVEL + stream_time(1) + theme::POUR_RETURN + 0.01);
        let (_, clock) = flow.effects().expect("the plug is still swinging");
        assert!(clock >= 0.0);

        flow.advance(theme::PLUG_TOGGLE);
        assert!(!flow.is_busy());
    }

    #[test]
    fn poured_reaches_the_full_count() {
        let mut flow = Flow::default();
        flow.finish_now();
        flow.start_pour(plan(3, None));
        flow.advance(theme::POUR_TRAVEL + stream_time(3));
        let state = flow.pour().expect("still returning");
        assert_eq!(state.phase, PourPhase::Return);
        assert_eq!(state.poured, 3.0);
        assert!(matches!(flow.kind, FlowKind::Pour { .. }));
    }
}
