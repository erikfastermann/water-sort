use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::sprite::{SpritePickingMode, SpritePickingSettings};

use crate::anim::{Flow, Play};
use crate::board::HitTarget;
use crate::fx::Particle;
use crate::session::Session;
use crate::view::{BoardView, MovePlan};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Selection>()
            .init_resource::<PendingClick>()
            .insert_resource(SpritePickingSettings {
                require_markers: false,
                picking_mode: SpritePickingMode::BoundingBox,
            })
            .add_observer(record_click)
            .add_systems(Update, handle_click.in_set(Play::Input));
    }
}

#[derive(Resource, Default)]
pub struct Selection(Option<u8>);

impl Selection {
    pub fn get(&self) -> Option<u8> {
        self.0
    }

    pub fn clear(&mut self) {
        self.0 = None;
    }
}

/// Clicks are funnelled through a single resource so the cancel-swallow rule
/// has exactly one choke point.
#[derive(Resource, Default)]
pub struct PendingClick(Option<Entity>);

impl PendingClick {
    pub fn set(&mut self, entity: Entity) {
        self.0 = Some(entity);
    }

    pub fn entity(&self) -> Option<Entity> {
        self.0
    }

    pub fn take(&mut self) -> Option<Entity> {
        self.0.take()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Ignore,
    Deselect,
    Select(u8),
    Pour { from: u8, to: u8 },
}

pub fn decide(selection: Option<u8>, clicked: Option<u8>, view: &BoardView) -> Action {
    if view.solved || view.stuck {
        return Action::Deselect;
    }
    let Some(id) = clicked else {
        return Action::Deselect;
    };
    if !view.get(id).interactable {
        return Action::Deselect;
    }

    match selection {
        None if view.get(id).can_move_from => Action::Select(id),
        None => Action::Ignore,
        Some(selected) if selected == id => Action::Deselect,
        Some(selected) => Action::Pour {
            from: selected,
            to: id,
        },
    }
}

/// `Pointer<Click>` auto-propagates up the hierarchy; stopping it keeps this
/// global observer to one call per click.
fn record_click(mut click: On<Pointer<Click>>, mut pending: ResMut<PendingClick>) {
    let entity = click.original_event_target();
    click.propagate(false);
    pending.set(entity);
}

/// The state a click can change, bundled so the handler stays readable.
#[derive(SystemParam)]
pub struct Game<'w> {
    session: ResMut<'w, Session>,
    view: ResMut<'w, BoardView>,
    flow: ResMut<'w, Flow>,
}

pub fn handle_click(
    mut commands: Commands,
    mut pending: ResMut<PendingClick>,
    mut selection: ResMut<Selection>,
    mut game: Game,
    targets: Query<&HitTarget>,
    particles: Query<Entity, With<Particle>>,
) {
    let Some(entity) = pending.take() else {
        return;
    };

    if game.flow.is_busy() {
        game.flow.finish_now();
        selection.clear();
        for particle in &particles {
            commands.entity(particle).try_despawn();
        }
        return;
    }

    let clicked = targets.get(entity).ok().map(|target| target.id);
    match decide(selection.get(), clicked, &game.view) {
        Action::Ignore => {}
        Action::Deselect => selection.clear(),
        Action::Select(id) => selection.0 = Some(id),
        Action::Pour { from, to } => {
            if pour(&mut game.session, &mut game.view, &mut game.flow, from, to) {
                selection.clear();
            }
        }
    }
}

/// Core state advances here, before a single frame of animation has played, so
/// cancelling the animation can never desync the board.
fn pour(session: &mut Session, view: &mut BoardView, flow: &mut Flow, from: u8, to: u8) -> bool {
    let before = view.clone();
    let Ok(mov) = session.history_mut().pour(from, to) else {
        return false;
    };

    let after = session.view();
    let plan = MovePlan::build(&mov, &before, &after);
    *view = after;
    flow.start_pour(plan);
    true
}

#[cfg(test)]
mod tests {
    use super::{Action, decide};
    use crate::anim::Flow;
    use crate::session::Session;
    use crate::solver::solve;
    use crate::view::MovePlan;

    #[test]
    fn selection_follows_the_spec() {
        let session = Session::new(0).expect("level exists");
        let view = session.view();
        let movable = view
            .bottles
            .iter()
            .find(|bottle| bottle.can_move_from)
            .expect("a level starts with a legal source");

        assert_eq!(decide(None, None, &view), Action::Deselect);
        assert_eq!(
            decide(None, Some(movable.id), &view),
            Action::Select(movable.id)
        );
        assert_eq!(
            decide(Some(movable.id), Some(movable.id), &view),
            Action::Deselect
        );

        let other = view
            .bottles
            .iter()
            .find(|bottle| bottle.id != movable.id)
            .expect("more than one bottle");
        assert_eq!(
            decide(Some(movable.id), Some(other.id), &view),
            Action::Pour {
                from: movable.id,
                to: other.id
            }
        );
    }

    #[test]
    fn a_full_solve_drives_the_flow_to_idle() {
        let solution = solve(0);
        assert!(!solution.is_empty());

        let mut session = Session::new(0).expect("level exists");
        let mut view = session.view();
        let mut flow = Flow::default();
        flow.finish_now();

        let mut finalized = 0;
        for (from, to) in solution {
            assert_eq!(decide(None, Some(from), &view), Action::Select(from));
            assert_eq!(
                decide(Some(from), Some(to), &view),
                Action::Pour { from, to }
            );

            let before = view.clone();
            let mov = session.history_mut().pour(from, to).expect("legal pour");
            let after = session.view();
            let plan = MovePlan::build(&mov, &before, &after);
            finalized += usize::from(plan.finalize.is_some());
            view = after;

            flow.start_pour(plan);
            assert!(flow.is_busy());
            // Half the animations play out, half are cancelled part way.
            if from % 2 == 0 {
                flow.advance(0.05);
            }
            flow.finish_now();
            assert!(!flow.is_busy());
            assert!(flow.pour().is_none());
        }

        assert!(view.solved);
        assert!(finalized > 0);
    }
}
