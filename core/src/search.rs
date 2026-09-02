use std::{
    cmp::min,
    collections::{HashSet, VecDeque},
    error::Error,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::{
    bits::Bits,
    state::{BOTTLE_COUNT, BOTTLE_SIZE_BITS, COLOR_BITS, ITEM_COUNT, Move, State, storage_bits},
};

pub const MAX_SEARCH_DEPTH: i8 = 125;

pub struct DFS {
    state: State,
    depth: i8,
    visited: Vec<u32>,
    find_first: bool,
}

impl DFS {
    pub fn new(
        state: State,
        depth: i8,
        visited_cache_bytes: usize,
        find_first: bool,
    ) -> Result<Self, Box<dyn Error>> {
        if depth < 0 || depth > MAX_SEARCH_DEPTH {
            return Err("invalid depth".into());
        }

        if visited_cache_bytes < 1_000_000
            || visited_cache_bytes as u64 > 0xff_ff_ff_ff_ff_ff
            || !visited_cache_bytes.is_power_of_two()
        {
            return Err("invalid visited size".into());
        }

        Ok(Self {
            state,
            depth,
            visited: vec![0; visited_cache_bytes / 4],
            find_first,
        })
    }

    pub fn search(mut self) -> i8 {
        let mut buffers = vec![
            [Move::default(); BOTTLE_COUNT * BOTTLE_COUNT];
            usize::try_from(self.depth).unwrap()
        ];
        let result = self.search_recursive(&mut buffers);
        if result < 0 {
            -1
        } else {
            assert_ne!(result, 0);
            result - 1
        }
    }

    fn search_recursive(&mut self, buffers: &mut [[Move; BOTTLE_COUNT * BOTTLE_COUNT]]) -> i8 {
        let remaining_depth = buffers.len() as i8;
        let error_marker = -remaining_depth - 1;

        // TODO: Skip calculating the hash on leaf nodes?
        let hash = SearchState::from(&self.state).search_hash();
        let visited = self.get_visited(hash);
        if visited != 0 {
            // This can give false negatives, which might prune useful search
            // states. This can also give false positives, which might return a
            // shorter solution than what is actually possible.

            if visited == i8::MIN {
                return i8::MIN;
            } else if visited < 0 {
                if -(visited + 1) >= remaining_depth {
                    return visited;
                }
                // Otherwise we need to check again.
            } else {
                if visited - 1 <= remaining_depth {
                    // We exhaustively searched this state before and know this
                    // is the true optimum.
                    return visited;
                } else {
                    return error_marker;
                }
            }
        }

        if self.state.solved() {
            self.set_visited(hash, 1);
            return 1;
        }

        if buffers.is_empty() {
            self.set_visited(hash, error_marker);
            return error_marker;
        }

        let (moves, next_buffer) = buffers.split_at_mut(1);
        let move_count = self.state.fill_moves(&mut moves[0]);
        let moves = &mut moves[0][..move_count];

        if moves.is_empty() {
            self.set_visited(hash, i8::MIN);
            return i8::MIN;
        }

        self.set_visited(hash, error_marker);
        let mut all_failed = true;
        let mut best_value = i8::MAX;

        for mov in moves.iter().copied() {
            self.state.apply_move_unchecked(mov);
            let result = self.search_recursive(next_buffer);
            self.state.undo_move_unchecked(mov);

            all_failed &= result == i8::MIN;
            if result > 0 {
                best_value = min(best_value, result + 1);
                if self.find_first {
                    self.set_visited(hash, best_value);
                    return best_value;
                }
            }
        }

        let return_value = if best_value == i8::MAX {
            if all_failed { i8::MIN } else { error_marker }
        } else {
            best_value
        };
        self.set_visited(hash, return_value);
        return_value
    }

    fn get_visited(&self, hash: u64) -> i8 {
        let stored = self.visited[hash as usize & (self.visited.len() - 1)];
        if (hash >> 40) as u32 == (stored >> 8) {
            stored as u8 as i8
        } else {
            0
        }
    }

    fn set_visited(&mut self, hash: u64, value: i8) {
        let visited_len = self.visited.len();
        let stored = ((hash >> 40) << 8) as u32 | value as u8 as u32;
        self.visited[hash as usize & (visited_len - 1)] = stored;
    }
}

pub fn bfs(input_state: &State, depth: i8) -> Result<i8, Box<dyn Error>> {
    if depth < 0 || depth > MAX_SEARCH_DEPTH {
        return Err("invalid depth".into());
    }

    let mut moves_buffer = [Move::default(); BOTTLE_COUNT * BOTTLE_COUNT];
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(SearchState::from(input_state));
    visited.insert(SearchState::from(input_state).search_hash());

    let mut current_states = 1usize;
    let mut next_states = 0usize;
    let mut current_depth = 0i8;

    while let Some(search_state) = queue.pop_front() {
        let mut state = search_state.to_state_unchecked(input_state);

        if state.solved() {
            return Ok(current_depth);
        }

        let move_count = state.fill_moves(&mut moves_buffer);
        let moves = &moves_buffer[..move_count];

        for mov in moves.iter().copied() {
            state.apply_move_unchecked(mov);
            let next_search_state = SearchState::from(&state);

            let hash = next_search_state.search_hash();
            if !visited.contains(&hash) {
                visited.insert(hash);
                queue.push_back(next_search_state);
                next_states += 1;
            }

            state.undo_move_unchecked(mov);
        }

        current_states -= 1;
        if current_states == 0 {
            eprintln!("depth {current_depth} done!");
            if current_depth == depth {
                return Ok(-1);
            }
            eprintln!("-> next states: {next_states}");

            current_states = next_states;
            next_states = 0;
            current_depth += 1;
        }
    }

    Ok(-1)
}

#[derive(Clone, Copy, Hash)]
/// Contains only the fields that can actually change.
struct SearchState {
    content: Bits<{ storage_bits(ITEM_COUNT * BOTTLE_COUNT * COLOR_BITS) }>,
    height: Bits<{ storage_bits(BOTTLE_COUNT * BOTTLE_SIZE_BITS) }>,
    bottle_finalized: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    #[cfg(feature = "hidable_items")]
    item_hidden: Bits<{ storage_bits(ITEM_COUNT * BOTTLE_COUNT) }>,

    #[cfg(feature = "lockable_items")]
    item_locked: Bits<{ storage_bits(ITEM_COUNT * BOTTLE_COUNT) }>,

    #[cfg(feature = "pluggable_bottles")]
    pluggable_bottle: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    #[cfg(feature = "pluggable_bottles")]
    bottle_plugged: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    #[cfg(feature = "freezable_bottles")]
    frozen: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    #[cfg(feature = "curtains")]
    curtain_range: Bits<{ storage_bits(BOTTLE_COUNT * (crate::state::BOTTLE_BITS + 1) * 2) }>,
    #[cfg(feature = "curtains")]
    behind_curtain: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    #[cfg(feature = "safes")]
    safe_counter: Bits<{ storage_bits(BOTTLE_COUNT * crate::state::SAFE_COUNTER_BITS) }>,

    #[cfg(feature = "lock_groups")]
    item_has_key: Bits<{ storage_bits(BOTTLE_COUNT * ITEM_COUNT) }>,
    #[cfg(feature = "lock_groups")]
    bottle_locked: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    #[cfg(feature = "color_curtains")]
    color_curtain_active: Bits<{ storage_bits(BOTTLE_COUNT) }>,
}

impl From<&State> for SearchState {
    fn from(value: &State) -> Self {
        Self {
            content: value.content,
            height: value.height,
            bottle_finalized: value.bottle_finalized,
            #[cfg(feature = "hidable_items")]
            item_hidden: value.item_hidden,
            #[cfg(feature = "lockable_items")]
            item_locked: value.item_locked,
            #[cfg(feature = "pluggable_bottles")]
            pluggable_bottle: value.pluggable_bottle,
            #[cfg(feature = "pluggable_bottles")]
            bottle_plugged: value.bottle_plugged,
            #[cfg(feature = "freezable_bottles")]
            frozen: value.frozen,
            #[cfg(feature = "curtains")]
            curtain_range: value.curtain_range,
            #[cfg(feature = "curtains")]
            behind_curtain: value.behind_curtain,
            #[cfg(feature = "safes")]
            safe_counter: value.safe_counter,
            #[cfg(feature = "lock_groups")]
            item_has_key: value.item_has_key,
            #[cfg(feature = "lock_groups")]
            bottle_locked: value.bottle_locked,
            #[cfg(feature = "color_curtains")]
            color_curtain_active: value.color_curtain_active,
        }
    }
}

impl SearchState {
    fn to_state_unchecked(&self, base: &State) -> State {
        State {
            bottle_count: base.bottle_count,
            content: self.content,
            height: self.height,
            capacity: base.capacity,
            bottle_finalized: self.bottle_finalized,
            color_count: base.color_count,
            #[cfg(feature = "hidable_items")]
            item_hidden: self.item_hidden,
            #[cfg(feature = "lockable_items")]
            item_locked: self.item_locked,
            #[cfg(feature = "immovable_bottles")]
            bottle_immovable: base.bottle_immovable,
            #[cfg(feature = "pluggable_bottles")]
            pluggable_bottle: self.pluggable_bottle,
            #[cfg(feature = "pluggable_bottles")]
            bottle_plugged: self.bottle_plugged,
            #[cfg(feature = "freezable_bottles")]
            frozen_run: base.frozen_run,
            #[cfg(feature = "freezable_bottles")]
            frozen: self.frozen,
            #[cfg(feature = "curtains")]
            curtain_range: self.curtain_range,
            #[cfg(feature = "curtains")]
            behind_curtain: self.behind_curtain,
            #[cfg(feature = "safes")]
            safe_counter: self.safe_counter,
            #[cfg(feature = "lock_groups")]
            item_has_key: self.item_has_key,
            #[cfg(feature = "lock_groups")]
            bottle_key: base.bottle_key,
            #[cfg(feature = "lock_groups")]
            bottle_locked: self.bottle_locked,
            #[cfg(feature = "colored_bottles")]
            bottle_color: base.bottle_color,
            #[cfg(feature = "color_curtains")]
            color_curtain: base.color_curtain,
            #[cfg(feature = "color_curtains")]
            color_curtain_active: self.color_curtain_active,
        }
    }

    fn search_hash(&self) -> u64 {
        // Hash collision attacks should not be a problem in our use case.
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        s.finish()
    }
}
