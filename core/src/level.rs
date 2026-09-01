use std::error::Error;

use serde::Deserialize;

use crate::{
    layout::Layout,
    state::{StartingState, State},
};

const LEVEL_DATA: &[&str] = &[include_str!("level_data/0000-example.json")];

const DESCRIPTION_MIN_LEN: usize = 100;
const DESCRIPTION_MAX_LEN: usize = 200;

pub struct Level {
    pub description: String,
    pub layout: Layout,
    pub starting_state: State,
}

impl Level {
    pub fn get(index: usize) -> Option<Level> {
        Self::try_get(index).unwrap()
    }

    fn try_get(index: usize) -> Result<Option<Level>, Box<dyn Error>> {
        let Some(data_raw) = LEVEL_DATA.get(index) else {
            return Ok(None);
        };

        let data: LevelData = serde_json::from_str(*data_raw)?;
        Ok(Some(Level::try_from(data)?))
    }
}

impl TryFrom<LevelData> for Level {
    type Error = Box<dyn Error>;

    fn try_from(value: LevelData) -> Result<Self, Self::Error> {
        if !value.description.is_ascii() {
            return Err("description is not ascii".into());
        }

        if !(DESCRIPTION_MIN_LEN..=DESCRIPTION_MAX_LEN).contains(&value.description.len()) {
            return Err("description too short or too long".into());
        }

        let starting_state = State::try_from(&value.starting_state)?;
        let layout = Layout::new(&value.layout, &starting_state)?;

        Ok(Level {
            description: value.description,
            layout,
            starting_state,
        })
    }
}

#[derive(Deserialize)]
struct LevelData {
    description: String,
    layout: Vec<String>,
    starting_state: StartingState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_all_levels() {
        for index in 0..LEVEL_DATA.len() {
            Level::try_get(index)
                .unwrap_or_else(|e| panic!("level {index}: {e}"))
                .unwrap();
        }
    }
}
