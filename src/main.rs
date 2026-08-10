/*
Search:

- Check current state is final, if yes return depth 0
- Store current state and check if known already, 3 possibilities:
  - First time seen -> Store as invalid
  - Not first time and stored invalid -> Return invalid
    (catches loops and dead states at once)
  - Not first time and stored with dist -> Return stored dist
    (when a move was found which reaches the end, we always store the returned
    dist plus one, because they might be invalid currently or the score has improved,
    don't store if the current is worse)
  - Question: How to store this efficiently? Maybe it would be better to just
    check for loops and ignore transpositions for now, this would be simple by
    hashing the state in the current layer, and then comparing equals with that
    state (what is enough to store here?), but this also means we need to
    decouple state from history
    -> just do loops now
- Find possible moves in layer, Bitmap N x N-1,
  store in Vec at index for current search depth
  (allocate once, also gives max depth for search)
  -> because fixed width, we can allocate on the stack
- Apply each move
- Also track current best score (which is equal to max depth)
  -> if we are at this depth, we can return immediately,
     because we found a solution which is equally good or better already,
     unless we want to enumerate all solutions, but this might be harder

- if we store the state for each depth, we have to mem copy a lot,
  we only need to store this to compare if we have seen this state before,
  storing just the hash should be fine usually, but hash collisions can happen,
  but are really rare
- what are alternatives to not store the full state?
- the case where a move is detected as duplicate, but is actually not is
  annoying, but we probably accept this small risk
- Zobrist hashing can be done later

- we have one state, ability to compute the moves and what changes would need
  to be applied and how to undo a move again

Three ideas:

(1) Standard Rust program
(2) Rust program where the input is compiled in
(3) Minizinc Program driven by something else

We want:

- Load/Store state in memory and on disk
- Get legal moves
- (Un-)apply move
- Move history with navigation
- Solver to check if config/state is solvable
  (satisfiable and shortest solution)
- More tooling which could be used to create levels,
  possibly driven by an LLM
- Debugging infrastructure

(3) might be hard to achieve, with slow startup times
just to compute the next moves (100ms minimum), although
the logic for (un-)applying moves I imagine would be quite
nice, maybe there is some nicer option than Minizinc,
directly driven from Rust, which can be meta-programmed easily.
We always have to ship a complete Minizinc, also annoying,
not too embeddable probably.

Performance with (3) is also questionable, specially with so
many requirements, although partially disabling them sounds
easy. In some cases the performance might also be better,
too hard to tell currently without trying it. Debugging is
often annoying though.

(1) + (2) would be interesting and probably the most versatile.
Corner case bugs could be tricky and everything has to be done
manually. But we get maximum control and would also be an
interesting experiment. Still questionable how good the logic
sharing would work, needs to be seen.

Could inject into state multiple const vars: bottle count (used
instead of associated bottle count if less than MAX), max bottle
height (probably as a total for all bottles, ut for individual
bottles this could be even more powerful), boolean feature flags.
*/

use std::{cmp::min, error::Error};

use crate::bits::Bits;

mod bits;

const ITEM_BITS: usize = 4;
const BOTTLE_BITS: usize = 5;
const COLOR_BITS: usize = 4;
const SAFE_COUNTER_BITS: usize = 3;

/// Need an extra bit for the height and capacity.
const BOTTLE_SIZE_BITS: usize = ITEM_BITS + 1;

const ITEM_COUNT: usize = 1 << ITEM_BITS;

/// The zero value is reserved.
const BOTTLE_COUNT: usize = 1 << BOTTLE_BITS;

/// The zero value is reserved.
const COLOR_COUNT: usize = 1 << COLOR_BITS;

const fn storage_bits(bits: usize) -> usize {
    assert!(bits <= 4096);
    let bits = bits.next_power_of_two();
    if bits < 32 { 32 } else { bits }
}

/// Represents the current game state without history.
///
/// At most one of the following can be chosen per bottle: curtain, safe, or
/// lock.
#[derive(Clone, Copy, Default)]
struct State {
    /// Must be less than the maximum amount of bottles.
    bottle_count: u8,

    content: Bits<{ storage_bits(ITEM_COUNT * BOTTLE_COUNT * COLOR_BITS) }>,
    height: Bits<{ storage_bits(BOTTLE_COUNT * BOTTLE_SIZE_BITS) }>,
    // The capacity must be greater than zero.
    capacity: Bits<{ storage_bits(BOTTLE_COUNT * BOTTLE_SIZE_BITS) }>,

    // The user cannot see this color item. Only relevant for the solver when
    // pouring out of this bottle, items connected by color are only poured
    // until the first invisible item. The top item should always be visible.
    // Unused item slots must never be marked as hidden.
    #[cfg(feature = "hidable_items")]
    item_hidden: Bits<{ storage_bits(ITEM_COUNT * BOTTLE_COUNT) }>,

    // The current item can only be moved out of this bottle and only one at
    // a time. Pouring into this bottle is not allowed if the top item is
    // locked. The lock disappears once the item has been removed.
    #[cfg(feature = "lockable_items")]
    item_locked: Bits<{ storage_bits(ITEM_COUNT * BOTTLE_COUNT) }>,

    // Can only fill into this bottle, never pour out of it. This does not
    // change after initialization.
    #[cfg(feature = "immovable_bottles")]
    bottle_movable: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    // Each move of any bottle, all plugged bottles are unplugged and
    // vice-versa. If a pluggable bottle is emptied, the plug is removed for
    // all future moves.
    #[cfg(feature = "pluggable_bottles")]
    pluggable_bottle: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    #[cfg(feature = "pluggable_bottles")]
    bottle_plugged: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    // A frozen bottle can only be filled into, until it is unfrozen.
    // Unfreezing happens once a bottle inside a frozen group is solved.
    //
    // Each bottle has an index of the group which this bottle is frozen with,
    // or zero if the bottle was never frozen. If a bottle is marked as frozen
    // once, this never changes, just update the frozen bit. As freezing a
    // bottle is only sensible if it has at least one partner, the maximum
    // number of frozen groups should be 16 to be solvable.
    #[cfg(feature = "freezable_bottles")]
    frozen_group: Bits<{ storage_bits(BOTTLE_COUNT * BOTTLE_BITS) }>,
    #[cfg(feature = "freezable_bottles")]
    frozen: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    // A bottle behind a curtain is not visible and cannot be interacted with.
    // When any other bottle is solved, each curtain group unlocks a bottle
    // in a defined order. Each bottle, except for two, can be behind a
    // curtain at the same time to be solvable and a curtain group can consist
    // of only a single bottle.
    #[cfg(feature = "curtains")]
    curtain_order: Bits<{ storage_bits(BOTTLE_COUNT * BOTTLE_BITS) }>,
    // The zero offset is reserved to mark the end of valid groups.
    #[cfg(feature = "curtains")]
    group_offset: Bits<{ storage_bits(BOTTLE_COUNT * BOTTLE_BITS) }>,
    #[cfg(feature = "curtains")]
    behind_curtain: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    // A bottle in a safe, meaning a counter greater than zero, is not visible
    // and cannot be interacted with. Each time any bottle is solved, all
    // non-zero safe counters are decremented by one. Safes could be grouped
    // in the UI, but this is not relevant for the state representation. Each
    // bottle, except for two, could be stored in a safe at the same time to
    // be solvable.
    #[cfg(feature = "safes")]
    safe_counter: Bits<{ storage_bits(BOTTLE_COUNT * SAFE_COUNTER_BITS) }>,

    // All bottles in a locked group cannot be interacted with. One color item
    // has the associated key for each lock group. When this item is at the
    // top of a bottle, the group is unlocked. When multiple items can be
    // poured at the same time, the key must always be the top item.
    #[cfg(feature = "lock_groups")]
    lock_group: Bits<{ storage_bits(BOTTLE_COUNT * BOTTLE_BITS) }>,
    #[cfg(feature = "lock_groups")]
    item_has_lock: Bits<{ storage_bits(ITEM_COUNT * BOTTLE_COUNT) }>,
    #[cfg(feature = "lock_groups")]
    group_key_location: Bits<{ storage_bits(BOTTLE_COUNT * (ITEM_BITS + BOTTLE_BITS)) }>,
    #[cfg(feature = "lock_groups")]
    bottle_locked: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    // Only items of a specific color can be poured into this bottle, or zero,
    // if all colors are allowed. Storing other colors in such a bottle is
    // allowed. This does not change after initialization.
    #[cfg(feature = "colored_bottles")]
    bottle_color: Bits<{ storage_bits(BOTTLE_COUNT * COLOR_BITS) }>,

    // The bottle cannot be interacted with, until that specific color is
    // solved. The zero color implies that the bottle is not hidden by a
    // color curtain.
    #[cfg(feature = "color_curtains")]
    color_curtain: Bits<{ storage_bits(BOTTLE_COUNT * COLOR_BITS) }>,
    #[cfg(feature = "color_curtains")]
    color_curtain_active: Bits<{ storage_bits(BOTTLE_COUNT) }>,
}

// TODO:
// Currently only the basic moves are implemented.
// All other features are still missing.

impl State {
    fn search(&mut self, depth: usize) -> isize {
        let mut buffers = vec![[Move::default(); BOTTLE_COUNT * BOTTLE_COUNT]; depth];
        self.search_recursive(&mut buffers)
    }

    fn search_recursive(&mut self, buffers: &mut [[Move; BOTTLE_COUNT * BOTTLE_COUNT]]) -> isize {
        if self.solved() {
            return 0;
        }
        if buffers.is_empty() {
            return -1;
        }

        let (buffers, next_buffers) = buffers.split_at_mut(1);
        let buffer = &mut buffers[0];

        let move_count = self.fill_moves(buffer);
        let moves = &buffer[..move_count];

        if moves.is_empty() {
            return -1;
        }

        for mov in moves.iter().copied() {
            self.apply_move(mov);
            let result = self.search_recursive(next_buffers);
            self.undo_move(mov);

            if result >= 0 {
                return result.checked_add(1).unwrap();
            }
        }

        -1
    }

    fn solved(&self) -> bool {
        // TODO: Visible check?

        for bottle in 1..self.bottle_count + 1 {
            let height = self.get_height(bottle);
            let capacity = self.get_capacity(bottle);
            let bottom_color = self.get_color(bottle, 0);
            let colors_match = (1..height).all(|i| self.get_color(bottle, i) == bottom_color);

            if height != 0 && (height != capacity || !colors_match) {
                return false;
            }
        }

        true
    }

    fn fill_moves(&self, buffer: &mut [Move; BOTTLE_COUNT * BOTTLE_COUNT]) -> usize {
        debug_assert!(usize::try_from(u16::MAX).is_ok());

        let mut index = 0;

        for from in 1..self.bottle_count + 1 {
            let from_height = self.get_height(from);
            debug_assert!(!self.get_item_hidden(from, from_height.saturating_sub(1)));

            for to in 1..self.bottle_count + 1 {
                let to_height = self.get_height(to);
                let to_capacity = self.get_capacity(to);
                let space = to_capacity - to_height;

                let from_top_color = self.get_color(from, from_height.saturating_sub(1));
                let to_top_color = self.get_color(to, to_height.saturating_sub(1));

                let mut count = 1;
                let mut same_color = true;
                for i in (0..from_height.saturating_sub(1)).rev() {
                    same_color &= (self.get_color(from, i) == from_top_color)
                        & !self.get_item_hidden(from, i);
                    count += u8::from(same_color);
                }

                if from == to
                    || from_height == 0
                    || space == 0
                    || (to_height > 0 && from_top_color != to_top_color)
                {
                    continue;
                }

                let count = min(count, space);
                let next_from_height = from_height.saturating_sub(count);
                let next_from_top_index = next_from_height.saturating_sub(1);

                let show_item =
                    if self.get_item_hidden(from, next_from_top_index) && next_from_height > 0 {
                        to_index(from, next_from_top_index)
                    } else {
                        0
                    };

                debug_assert!(index < buffer.len());
                buffer[index % buffer.len()] = Move {
                    from_bottle: from,
                    to_bottle: to,
                    count,
                    color: from_top_color,
                    show_item,
                    unlock_item: 0,
                    unplug_bottle: 0,
                    unfreeze_group: 0,
                    lift_curtain: Bits::ZERO,
                    decrement_safe_counters: Bits::ZERO,
                    remove_lock_group_key: 0,
                    unlock_bottles: Bits::ZERO,
                    lift_color_curtain: Bits::ZERO,
                };

                index += 1;
            }
        }

        index
    }

    fn apply_move(&mut self, mov: Move) {
        let from_height = self.get_height(mov.from_bottle);
        let to_height = self.get_height(mov.to_bottle);

        for i in from_height - mov.count..from_height {
            self.set_color(mov.from_bottle, i, 0);
        }

        for i in to_height..to_height + mov.count {
            self.set_color(mov.to_bottle, i, mov.color);
        }

        self.set_height(mov.from_bottle, from_height - mov.count);
        self.set_height(mov.to_bottle, to_height + mov.count);

        #[cfg(feature = "hidable_items")]
        if mov.show_item != 0 {
            self.set_item_hidden(
                from_index(mov.show_item).0,
                from_index(mov.show_item).1,
                false,
            );
        }
    }

    fn undo_move(&mut self, mov: Move) {
        let from_height = self.get_height(mov.from_bottle);
        let to_height = self.get_height(mov.to_bottle);

        for i in from_height..from_height + mov.count {
            self.set_color(mov.from_bottle, i, mov.color);
        }

        for i in to_height - mov.count..to_height {
            self.set_color(mov.to_bottle, i, 0);
        }

        self.set_height(mov.from_bottle, from_height + mov.count);
        self.set_height(mov.to_bottle, to_height - mov.count);

        #[cfg(feature = "hidable_items")]
        if mov.show_item != 0 {
            self.set_item_hidden(
                from_index(mov.show_item).0,
                from_index(mov.show_item).1,
                true,
            );
        }
    }

    fn get_height(&self, bottle: u8) -> u8 {
        self.height
            .get_n(usize::from(bottle) * BOTTLE_SIZE_BITS, BOTTLE_SIZE_BITS) as u8
    }

    fn set_height(&mut self, bottle: u8, height: u8) {
        debug_assert_ne!(bottle, 0);
        self.height.set_n(
            usize::from(bottle) * BOTTLE_SIZE_BITS,
            BOTTLE_SIZE_BITS,
            u16::from(height),
        );
    }

    fn get_capacity(&self, bottle: u8) -> u8 {
        self.capacity
            .get_n(usize::from(bottle) * BOTTLE_SIZE_BITS, BOTTLE_SIZE_BITS) as u8
    }

    fn set_capacity(&mut self, bottle: u8, capacity: u8) {
        debug_assert_ne!(bottle, 0);
        self.capacity.set_n(
            usize::from(bottle) * BOTTLE_SIZE_BITS,
            BOTTLE_SIZE_BITS,
            u16::from(capacity),
        );
    }

    fn get_color(&self, bottle: u8, item: u8) -> u8 {
        self.content.get_n(
            usize::from(bottle) * ITEM_COUNT * COLOR_BITS + usize::from(item) * COLOR_BITS,
            COLOR_BITS,
        ) as u8
    }

    fn set_color(&mut self, bottle: u8, item: u8, color: u8) {
        debug_assert_ne!(bottle, 0);
        self.content.set_n(
            usize::from(bottle) * ITEM_COUNT * COLOR_BITS + usize::from(item) * COLOR_BITS,
            COLOR_BITS,
            u16::from(color),
        );
    }

    fn get_item_hidden(&self, bottle: u8, item: u8) -> bool {
        #[cfg(feature = "hidable_items")]
        {
            self.item_hidden
                .has(usize::from(bottle) * ITEM_COUNT + usize::from(item))
        }
        #[cfg(not(feature = "hidable_items"))]
        {
            false
        }
    }

    #[cfg(feature = "hidable_items")]
    fn set_item_hidden(&mut self, bottle: u8, item: u8, hidden: bool) {
        debug_assert_ne!(bottle, 0);
        self.item_hidden
            .set(usize::from(bottle) * ITEM_COUNT + usize::from(item), hidden);
    }
}

impl TryFrom<&StateData> for State {
    type Error = Box<dyn Error>;

    fn try_from(value: &StateData) -> Result<Self, Self::Error> {
        if value.content.len() >= BOTTLE_COUNT {
            return Err("too many bottles".into());
        }
        if value.content.len() != value.capacity.len() {
            return Err("content and capacity lengths do not match".into());
        }
        if value.content.iter().any(|v| v.len() > ITEM_COUNT) {
            return Err("bottle item count too large".into());
        }
        if value.capacity.iter().any(|v| usize::from(*v) > ITEM_COUNT) {
            return Err("capacity too large".into());
        }
        if value
            .content
            .iter()
            .enumerate()
            .any(|(i, v)| v.len() > usize::from(value.capacity[i]))
        {
            return Err("height exceeds capacity".into());
        }

        let mut state = State::default();
        state.bottle_count = u8::try_from(value.content.len()).unwrap();

        for (index, bottle) in value.content.iter().enumerate() {
            let out_index = u8::try_from(index.checked_add(1).unwrap()).unwrap();
            state.set_capacity(out_index, value.capacity[index]);
            state.set_height(out_index, u8::try_from(bottle.len()).unwrap());

            for (item, color) in bottle.iter().copied().enumerate() {
                if color == 0 || usize::from(color) >= COLOR_COUNT {
                    return Err("invalid color value".into());
                }
                state.set_color(out_index, u8::try_from(item).unwrap(), color);
            }
        }

        for (bottle, item) in value.hidden_items.iter().copied() {
            if usize::from(bottle) >= value.content.len() {
                return Err("hidden item bottle not in range".into());
            }

            if usize::from(item) >= value.content[usize::from(bottle)].len().saturating_sub(1) {
                return Err("hidden item not in range".into());
            }

            state.set_item_hidden(bottle.checked_add(1).unwrap(), item, true);
        }

        Ok(state)
    }
}

fn from_index(index: u16) -> (u8, u8) {
    debug_assert!(
        usize::try_from(index)
            .ok()
            .is_some_and(|n| n < BOTTLE_COUNT * ITEM_COUNT)
    );
    let bottle = index as usize / ITEM_COUNT;
    let item = index as usize % ITEM_COUNT;
    debug_assert!(u8::try_from(bottle).is_ok());
    debug_assert!(u8::try_from(item).is_ok());
    (bottle as u8, item as u8)
}

fn to_index(bottle: u8, item: u8) -> u16 {
    debug_assert!(u16::try_from(ITEM_COUNT).is_ok());
    debug_assert!(usize::from(bottle) < BOTTLE_COUNT);
    debug_assert!(usize::from(item) < ITEM_COUNT);
    u16::from(bottle) * ITEM_COUNT as u16 + u16::from(item)
}

/// Collect all changes to apply a move, which can be undone.
// All optional item indices use the zero value, which is possible because the
// zero bottle index is reserved.
#[derive(Clone, Copy, Default)]
struct Move {
    from_bottle: u8,
    to_bottle: u8,
    count: u8,
    color: u8,

    #[cfg(feature = "hidable_items")]
    show_item: u16,
    #[cfg(feature = "lockable_items")]
    unlock_item: u16,
    #[cfg(feature = "pluggable_bottles")]
    unplug_bottle: u16,
    #[cfg(feature = "freezable_bottles")]
    unfreeze_group: u8,
    #[cfg(feature = "curtains")]
    lift_curtain: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    #[cfg(feature = "safes")]
    decrement_safe_counters: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    #[cfg(feature = "lock_groups")]
    remove_lock_group_key: u16,
    #[cfg(feature = "lock_groups")]
    unlock_bottles: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    #[cfg(feature = "color_curtains")]
    lift_color_curtain: Bits<{ storage_bits(BOTTLE_COUNT) }>,
}

struct StateData {
    content: Vec<Vec<u8>>,
    capacity: Vec<u8>,
    hidden_items: Vec<(u8, u8)>,
}

// TODO:
// - Bottle With Counter, Refilled When Empty With Counter Greater Than Zero

fn main() -> Result<(), Box<dyn Error>> {
    let mut state = State::try_from(&StateData {
        content: vec![
            vec![1, 5, 4],
            vec![2, 2, 3, 1],
            vec![3, 3, 1, 5],
            vec![4, 4, 4],
            vec![5, 5],
            vec![6, 6, 3, 1],
            vec![2, 6, 6, 2],
        ],
        capacity: vec![4; 7],
        hidden_items: vec![],
    })?;

    println!("{}", state.search(20));
    Ok(())
}
