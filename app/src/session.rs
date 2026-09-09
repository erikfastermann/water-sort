#![allow(dead_code)]

use bevy::prelude::*;
use water_sort_core::history::History;
use water_sort_core::layout::Layout;
use water_sort_core::level::Level;
use water_sort_core::state::State;

use crate::view::BoardView;

const FIRST_LEVEL: usize = 0;

pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        let session = Session::new(FIRST_LEVEL).expect("the first level exists");
        app.insert_resource(session.view()).insert_resource(session);
    }
}

#[derive(Resource)]
pub struct Session {
    index: usize,
    level: Level,
    history: History,
}

impl Session {
    pub fn new(index: usize) -> Option<Self> {
        let level = Level::get(index)?;
        let history = History::from(level.starting_state);
        Some(Self {
            index,
            level,
            history,
        })
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
}
