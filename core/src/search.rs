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
        if !(0..=MAX_SEARCH_DEPTH).contains(&depth) {
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
        let mut buffers =
            vec![[Move::ZERO; BOTTLE_COUNT * BOTTLE_COUNT]; usize::try_from(self.depth).unwrap()];
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
        let hash = search_hash(&self.state);
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
    if !(0..=MAX_SEARCH_DEPTH).contains(&depth) {
        return Err("invalid depth".into());
    }

    let mut moves_buffer = [Move::ZERO; BOTTLE_COUNT * BOTTLE_COUNT];
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(SearchState::from(input_state));
    visited.insert(search_hash(input_state));

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
            let hash = search_hash(&state);

            if visited.insert(hash) {
                queue.push_back(SearchState::from(&state));
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
    fn to_state_unchecked(self, base: &State) -> State {
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
}

#[derive(Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct HashBottle {
    /// Only set for specific features.
    bottle: u8,

    height: u8,
    capacity: u8,
    content: u64,
    bottle_finalized: bool,

    #[cfg(feature = "hidable_items")]
    item_hidden: u16,

    #[cfg(feature = "lockable_items")]
    item_locked: u16,

    #[cfg(feature = "immovable_bottles")]
    bottle_immovable: bool,

    #[cfg(feature = "pluggable_bottles")]
    pluggable_bottle: bool,
    #[cfg(feature = "pluggable_bottles")]
    bottle_plugged: bool,

    #[cfg(feature = "freezable_bottles")]
    frozen: bool,

    #[cfg(feature = "curtains")]
    behind_curtain: bool,

    #[cfg(feature = "safes")]
    safe_counter: u8,

    #[cfg(feature = "lock_groups")]
    item_has_key: u16,
    #[cfg(feature = "lock_groups")]
    bottle_locked: bool,

    #[cfg(feature = "colored_bottles")]
    bottle_color: u8,

    #[cfg(feature = "color_curtains")]
    color_curtain_active: bool,
}

fn search_hash(state: &State) -> u64 {
    // Hashing a state for search can use a canonicalized form. Let s1 and s2
    // be two states which were reached by a (possibly different) sequence of
    // moves from an initial state s0. If the reachable final states from s1
    // and s2 are identical, then hash(s1) and hash(s2) should also be
    // identical. This function tries to approach this goal, but there are many
    // cases still remaining where identical final states do not result in an
    // identical hash.
    //
    // Hash collision attacks should not be a problem in our use case.

    let mut bottles = [HashBottle::default(); BOTTLE_COUNT];
    let mut bottle_count = 0usize;

    for bottle in state.bottles() {
        let frozen = state.get_frozen(bottle);
        let behind_curtain = state.get_behind_curtain(bottle);
        let safe_counter = state.get_safe_counter(bottle);
        let item_has_key = state.get_items_have_key(bottle);
        let bottle_locked = state.get_bottle_locked(bottle);
        let color_curtain_active = state.get_color_curtain_active(bottle);

        let with_bottle_id = frozen
            || behind_curtain
            || safe_counter != 0
            || bottle_locked
            || color_curtain_active
            || item_has_key != 0;

        let hash_bottle = HashBottle {
            bottle: if with_bottle_id { bottle } else { 0 },
            height: state.get_height(bottle),
            capacity: state.get_capacity(bottle),
            content: state.get_colors(bottle),
            bottle_finalized: state.get_bottle_finalized(bottle),

            #[cfg(feature = "hidable_items")]
            item_hidden: state.get_items_hidden(bottle),

            #[cfg(feature = "lockable_items")]
            item_locked: state.get_items_locked(bottle),

            #[cfg(feature = "immovable_bottles")]
            bottle_immovable: state.get_bottle_immovable(bottle),

            #[cfg(feature = "pluggable_bottles")]
            pluggable_bottle: state.get_pluggable_bottle(bottle),
            #[cfg(feature = "pluggable_bottles")]
            bottle_plugged: state.get_bottle_plugged(bottle),

            #[cfg(feature = "freezable_bottles")]
            frozen,

            #[cfg(feature = "curtains")]
            behind_curtain,

            #[cfg(feature = "safes")]
            safe_counter,

            #[cfg(feature = "lock_groups")]
            item_has_key,
            #[cfg(feature = "lock_groups")]
            bottle_locked,

            #[cfg(feature = "colored_bottles")]
            bottle_color: state.get_bottle_color(bottle),

            #[cfg(feature = "color_curtains")]
            color_curtain_active,
        };

        debug_assert!(bottle_count < bottles.len());
        debug_assert!(bottles.len().is_power_of_two());

        bottles[bottle_count & (bottles.len() - 1)] = hash_bottle;
        bottle_count += 1;
    }

    let bottles = &mut bottles[..bottle_count];
    bottles.sort_unstable();

    let mut s = DefaultHasher::new();
    bottles.hash(&mut s);
    s.finish()
}
