use std::{error::Error, fs, path::PathBuf};

use clap::{Parser, Subcommand, builder::RangedU64ValueParser};

use water_sort_core::{
    level::{Level, LevelData},
    search::{DFS, MAX_SEARCH_DEPTH, bfs},
    state::{StartingState, State},
};

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let puzzle_path = match &args.command {
        Command::Dfs { puzzle_path, .. } | Command::Bfs { puzzle_path, .. } => puzzle_path,
    };

    let puzzle_raw = fs::read_to_string(puzzle_path)?;
    let state = match serde_json::from_str::<LevelData>(&puzzle_raw) {
        Ok(level_data) => Level::try_from(level_data)?.starting_state,
        Err(_) => {
            let starting_state: StartingState = serde_json::from_str(&puzzle_raw)?;
            State::try_from(&starting_state)?
        }
    };

    let search_result = match args.command {
        Command::Dfs {
            best,
            depth,
            visited_cache_mib,
            ..
        } => {
            let Some(visited_cache_bytes) = visited_cache_mib.checked_mul(1024 * 1024) else {
                return Err("visited cache size too large".into());
            };
            let dfs = DFS::new(state, depth, visited_cache_bytes, !best)?;
            dfs.search()
        }
        Command::Bfs { depth, .. } => bfs(&state, depth)?,
    };

    println!("{search_result}");
    Ok(())
}

/// Utilities for working with water sort puzzles
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Depth first search
    Dfs {
        /// Path to the puzzle starting state
        puzzle_path: PathBuf,

        /// Don't end the search on the first solution found
        #[arg(long)]
        best: bool,

        /// Maximum search depth
        #[arg(
            long,
            value_parser = clap::value_parser!(i8).range(0..=i64::from(MAX_SEARCH_DEPTH)),
            default_value_t = 125,
        )]
        depth: i8,

        /// Visited state cache in MiB
        #[arg(
            long = "visited",
            value_parser = RangedU64ValueParser::<usize>::new().range(1..=(1 << 20)),
            default_value_t = 2048,
        )]
        visited_cache_mib: usize,
    },

    /// Breadth first search
    Bfs {
        /// Path to the puzzle starting state
        puzzle_path: PathBuf,

        /// Maximum search depth
        #[arg(
            long,
            value_parser = clap::value_parser!(i8).range(0..=i64::from(MAX_SEARCH_DEPTH)),
            default_value_t = 30,
        )]
        depth: i8,
    },
}
