use bevy::prelude::*;
use water_sort_core::history::History;
use water_sort_core::layout::Layout;
use water_sort_core::level::Level;
use water_sort_core::state::State;

use crate::anim::Play;
use crate::storage::{load_last_completed, save_last_completed};
use crate::view::BoardView;

const FIRST_LEVEL: usize = 0;

pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        let session = Session::start();
        app.insert_resource(session.view())
            .insert_resource(session)
            .add_systems(Update, record_completion.in_set(Play::Apply));
    }
}

#[derive(Resource)]
pub struct Session {
    index: usize,
    level: Level,
    history: History,
    exhausted: bool,
    recorded: bool,
}

impl Session {
    pub fn new(index: usize) -> Option<Self> {
        let level = Level::get(index)?;
        let history = History::from(level.starting_state);
        Some(Self {
            index,
            level,
            history,
            exhausted: false,
            recorded: false,
        })
    }

    /// Resumes on the level after the last completed one, stopping at the last
    /// level that still exists so a finished player is never left on an empty
    /// screen.
    pub fn start() -> Self {
        let wanted = load_last_completed().map_or(FIRST_LEVEL, |last| last.saturating_add(1));
        let mut session = Self::new(FIRST_LEVEL).expect("the first level exists");
        while session.index < wanted {
            let Some(next) = Self::new(session.index + 1) else {
                break;
            };
            session = next;
        }
        session
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn layout(&self) -> &Layout {
        &self.level.layout
    }

    pub fn state(&self) -> &State {
        self.history.state()
    }

    pub fn history_mut(&mut self) -> &mut History {
        &mut self.history
    }

    pub fn view(&self) -> BoardView {
        BoardView::build(self.state(), self.layout())
    }

    pub fn can_rewind(&self) -> bool {
        self.history.can_rewind()
    }

    pub fn can_forward(&self) -> bool {
        self.history.can_forward()
    }

    pub fn rewind(&mut self) -> bool {
        let stepped = self.history.rewind();
        self.stepped(stepped)
    }

    pub fn forward(&mut self) -> bool {
        let stepped = self.history.forward();
        self.stepped(stepped)
    }

    /// Walking the history puts a playable board back on screen, so the
    /// "no more levels" message has to go with it.
    fn stepped(&mut self, stepped: bool) -> bool {
        self.exhausted &= !stepped;
        stepped
    }

    /// Loads the next level, or records that there is none left.
    pub fn advance(&mut self) -> bool {
        match Self::new(self.index + 1) {
            Some(next) => {
                *self = next;
                true
            }
            None => {
                self.exhausted = true;
                false
            }
        }
    }

    pub fn exhausted(&self) -> bool {
        self.exhausted
    }
}

fn record_completion(mut session: ResMut<Session>, view: Res<BoardView>) {
    if !view.solved || session.recorded {
        return;
    }
    session.recorded = true;
    save_last_completed(session.index);
}

#[cfg(test)]
mod tests {
    use super::Session;

    fn first_legal_pour(session: &Session) -> (u8, u8) {
        let state = session.state();
        let pours = state.pours().expect("the level is not solved");
        for from in state.bottles() {
            for to in state.bottles() {
                if from != to && pours.has(from, to) {
                    return (from, to);
                }
            }
        }
        panic!("a fresh level has a legal pour");
    }

    #[test]
    fn advancing_runs_out_of_levels() {
        let mut session = Session::new(0).expect("the first level exists");
        let mut levels = 1;
        while session.advance() {
            assert_eq!(session.index(), levels);
            levels += 1;
        }
        assert!(levels > 1);
        assert!(session.exhausted());
        assert!(!session.can_rewind());
    }

    #[test]
    fn history_navigation_returns_to_the_same_view() {
        let mut session = Session::new(0).expect("the first level exists");
        let start = session.view();
        let (from, to) = first_legal_pour(&session);
        session.history_mut().pour(from, to).expect("legal pour");

        let poured = session.view();
        assert!(session.can_rewind());
        assert!(!session.can_forward());

        assert!(session.rewind());
        assert_eq!(session.view().bottles, start.bottles);
        assert!(session.can_forward());

        assert!(session.forward());
        assert_eq!(session.view().bottles, poured.bottles);
        assert!(!session.forward());
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod progress_tests {
    use super::Session;
    use crate::storage::{load_last_completed, save_last_completed};

    /// Native storage is a process global, so the round trip and both resume
    /// cases have to share one test to stay deterministic.
    #[test]
    fn progress_resumes_on_the_next_level() {
        save_last_completed(1);
        assert_eq!(load_last_completed(), Some(1));
        assert_eq!(Session::start().index(), 2);

        save_last_completed(usize::MAX);
        let last = Session::start().index();
        assert!(Session::new(last).is_some());
        assert!(Session::new(last + 1).is_none());
    }
}
