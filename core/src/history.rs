use std::error::Error;

use crate::state::{Move, State};

pub struct History {
    state: State,
    moves: Vec<Move>,
    length: usize,
}

impl From<State> for History {
    fn from(state: State) -> Self {
        Self {
            state,
            moves: Vec::new(),
            length: 0,
        }
    }
}

impl History {
    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn pour(&mut self, from: u8, to: u8) -> Result<Move, Box<dyn Error>> {
        let Some(mut moves) = self.state.moves() else {
            return Err("already solved".into());
        };

        let mov = moves.find(|mov| mov.from_bottle() == from && mov.to_bottle() == to);
        drop(moves);

        let Some(mov) = mov else {
            return Err("move not allowed".into());
        };

        self.state.apply_move_unchecked(mov);
        self.moves.truncate(self.length);
        self.moves.push(mov);
        self.length = self.moves.len();

        Ok(mov)
    }

    pub fn can_rewind(&self) -> bool {
        self.length > 0
    }

    pub fn rewind(&mut self) -> bool {
        if !self.can_rewind() {
            return false;
        }

        let mov = self.moves[self.length - 1];
        self.state.undo_move_unchecked(mov);
        self.length -= 1;
        true
    }

    pub fn can_forward(&self) -> bool {
        self.length < self.moves.len()
    }

    pub fn forward(&mut self) -> bool {
        if !self.can_forward() {
            return false;
        }

        let mov = self.moves[self.length];
        self.state.apply_move_unchecked(mov);
        self.length += 1;
        true
    }
}
