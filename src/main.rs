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

use std::{
    cmp::min,
    env,
    error::Error,
    fs,
    hash::{DefaultHasher, Hash, Hasher},
};

use serde::{Deserialize, Serialize};

use crate::bits::Bits;

mod bits;

const ITEM_BITS: usize = 4;
const BOTTLE_BITS: usize = 5;
const COLOR_BITS: usize = 4;
#[cfg(feature = "safes")]
const SAFE_COUNTER_BITS: usize = 3;

/// Need an extra bit for the height and capacity.
const BOTTLE_SIZE_BITS: usize = ITEM_BITS + 1;

const ITEM_COUNT: usize = 1 << ITEM_BITS;

/// The zero value is reserved.
const BOTTLE_COUNT: usize = 1 << BOTTLE_BITS;

/// The zero value is reserved.
const COLOR_COUNT: usize = 1 << COLOR_BITS;

#[cfg(feature = "safes")]
const MAX_SAFE_COUNTER: usize = (1 << SAFE_COUNTER_BITS) - 1;

const MAX_SEARCH_DEPTH: i8 = i8::MAX - 1;

const fn storage_bits(bits: usize) -> usize {
    assert!(bits <= 4096);
    let bits = bits.next_power_of_two();
    if bits < 32 { 32 } else { bits }
}

/// Represents the current game state without history.
///
/// Each color has exactly one bottle, so the count per color must be less than
/// the maximum bottle size. At most one of the following can be chosen per
/// bottle: curtain, safe, or lock.
// TODO: Bottle with counter, refilled when empty with counter not zero.
#[derive(Clone, Copy, Default, Hash)]
struct State {
    /// Must be less than the maximum amount of bottles.
    bottle_count: u8,

    content: Bits<{ storage_bits(ITEM_COUNT * BOTTLE_COUNT * COLOR_BITS) }>,
    height: Bits<{ storage_bits(BOTTLE_COUNT * BOTTLE_SIZE_BITS) }>,
    // The capacity must be greater than zero.
    capacity: Bits<{ storage_bits(BOTTLE_COUNT * BOTTLE_SIZE_BITS) }>,
    bottle_finalized: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    color_count: Bits<{ storage_bits(COLOR_COUNT * BOTTLE_SIZE_BITS) }>,

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
    bottle_immovable: Bits<{ storage_bits(BOTTLE_COUNT) }>,

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
    // A frozen group is marked by a run of bits, switching to zero if the
    // current group is marked with one-bits and vice-versa. This means only
    // bottles which are located next to each other in the bitset can be part
    // of a group. Groups which are never frozen are also included, but simply
    // never use the frozen bit. The group assignments never change after
    // initialization. As freezing a bottle is only sensible if it has at least
    // one partner, the maximum number of frozen groups should be no bigger than
    // half of the number of bottles to be solvable.
    #[cfg(feature = "freezable_bottles")]
    frozen_run: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    #[cfg(feature = "freezable_bottles")]
    frozen: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    // A bottle behind a curtain is not visible and cannot be interacted with.
    // When any other bottle is solved, each curtain group unlocks a bottle,
    // ordered from the largest to the smallest index. Each bottle, except for
    // two, can be behind a curtain at the same time to be solvable and a
    // curtain group can consist of only a single bottle.
    #[cfg(feature = "curtains")]
    curtain_range: Bits<{ storage_bits(BOTTLE_COUNT * (BOTTLE_BITS + 1) * 2) }>,
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
    item_has_key: Bits<{ storage_bits(BOTTLE_COUNT * ITEM_COUNT) }>,
    #[cfg(feature = "lock_groups")]
    bottle_key: Bits<{ storage_bits(BOTTLE_COUNT * (BOTTLE_BITS + ITEM_BITS)) }>,
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

impl State {
    fn solved(&self) -> bool {
        for bottle in 1..self.bottle_count + 1 {
            if self.get_height(bottle) != 0 && !self.get_bottle_finalized(bottle) {
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

            if self.get_bottle_immovable(from)
                | self.get_bottle_plugged(from)
                | self.get_frozen(from)
                | self.get_bottle_finalized(from)
                | self.get_behind_curtain(from)
                | (self.get_safe_counter(from) != 0)
                | self.get_bottle_locked(from)
                | self.get_color_curtain_active(from)
            {
                continue;
            }

            for to in 1..self.bottle_count + 1 {
                let to_height = self.get_height(to);
                let to_capacity = self.get_capacity(to);
                let space = to_capacity - to_height;

                let from_top_color = self.get_color(from, from_height.saturating_sub(1));
                let to_top_color = self.get_color(to, to_height.saturating_sub(1));
                let to_bottle_color = self.get_bottle_color(to);

                let to_top_item_locked = self.get_item_locked(to, to_height.saturating_sub(1));

                if (from == to)
                    | (from_height == 0)
                    | (space == 0)
                    | (to_height > 0 && from_top_color != to_top_color)
                    | (to_height > 0 && to_top_item_locked)
                    | self.get_bottle_plugged(to)
                    | self.get_behind_curtain(to)
                    | (self.get_safe_counter(to) != 0)
                    | self.get_bottle_locked(to)
                    | (to_bottle_color != 0 && to_bottle_color != from_top_color)
                    | self.get_color_curtain_active(to)
                {
                    continue;
                }

                let from_top_item_locked = self.get_item_locked(from, from_height - 1);

                let mut count = 1;
                let mut include_next_item = !from_top_item_locked;
                for i in (0..from_height - 1).rev() {
                    include_next_item &= self.pour_include(from, i, from_top_color);
                    count += u8::from(include_next_item);
                }

                let count = min(count, space);
                #[cfg(any(
                    feature = "hidable_items",
                    feature = "pluggable_bottles",
                    feature = "lock_groups"
                ))]
                let next_from_height = from_height.saturating_sub(count);
                #[cfg(any(feature = "hidable_items", feature = "lock_groups"))]
                let next_from_top_index = next_from_height.saturating_sub(1);

                let to_finalized = self.compute_bottle_finalized(to, count, from_top_color);
                let finalize_bottle = if to_finalized { to } else { 0 };

                #[cfg(feature = "lock_groups")]
                let (remove_key, unlock_bottle) =
                    if next_from_height > 0 && self.get_item_has_key(from, next_from_top_index) {
                        let key = to_index(from, next_from_top_index);
                        let unlock_bottle =
                            (1..self.bottle_count + 1).fold(Bits::ZERO, |mut acc, i| {
                                acc.set(usize::from(i), self.get_bottle_key(i) == key);
                                acc
                            });
                        (key, unlock_bottle)
                    } else {
                        (0, Bits::ZERO)
                    };

                debug_assert!(index < buffer.len());
                buffer[index % buffer.len()] = Move {
                    from_bottle: from,
                    to_bottle: to,
                    count,
                    color: from_top_color,
                    finalize_bottle,

                    #[cfg(feature = "hidable_items")]
                    show_item: if next_from_height > 0
                        && self.get_item_hidden(from, next_from_top_index)
                    {
                        to_index(from, next_from_top_index)
                    } else {
                        0
                    },

                    #[cfg(feature = "lockable_items")]
                    unlock_item: if from_top_item_locked {
                        to_index(from, from_height - 1)
                    } else {
                        0
                    },

                    #[cfg(feature = "pluggable_bottles")]
                    unplug_bottle: if next_from_height == 0 && self.get_pluggable_bottle(from) {
                        from
                    } else {
                        0
                    },

                    #[cfg(feature = "freezable_bottles")]
                    unfreeze_run: if to_finalized && self.get_frozen(to) {
                        let run = self.bottle_run(self.frozen_run, to);
                        debug_assert_eq!(run & self.frozen, run);
                        run
                    } else {
                        Bits::ZERO
                    },

                    #[cfg(feature = "curtains")]
                    lift_curtain_range: if to_finalized {
                        (0..self.bottle_count).fold(Bits::ZERO, |mut acc, i| {
                            let value = self
                                .get_curtain_range(i)
                                .is_some_and(|(from, to)| to > from);
                            acc.set(usize::from(i), value);
                            acc
                        })
                    } else {
                        Bits::ZERO
                    },

                    #[cfg(feature = "safes")]
                    decrement_safe_counter: if to_finalized {
                        (1..self.bottle_count + 1).fold(Bits::ZERO, |mut acc, i| {
                            let counter = self.get_safe_counter(i);
                            acc.set(usize::from(i), counter != 0);
                            acc
                        })
                    } else {
                        Bits::ZERO
                    },

                    #[cfg(feature = "lock_groups")]
                    remove_key,
                    #[cfg(feature = "lock_groups")]
                    unlock_bottle,

                    #[cfg(feature = "color_curtains")]
                    lift_color_curtain: if to_finalized {
                        (1..self.bottle_count + 1).fold(Bits::ZERO, |mut acc, i| {
                            acc.set(usize::from(i), self.get_color_curtain(i) == from_top_color);
                            acc
                        })
                    } else {
                        Bits::ZERO
                    },
                };

                index += 1;
            }
        }

        index
    }

    fn pour_include(&self, bottle: u8, item: u8, color: u8) -> bool {
        (self.get_color(bottle, item) == color)
            & !self.get_item_hidden(bottle, item)
            & !self.get_item_locked(bottle, item)
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

        if mov.finalize_bottle != 0 {
            self.set_bottle_finalized(mov.finalize_bottle, true);
        }

        #[cfg(feature = "hidable_items")]
        if mov.show_item != 0 {
            let (bottle, item) = from_index(mov.show_item);
            self.set_item_hidden(bottle, item, false);
        }

        #[cfg(feature = "lockable_items")]
        if mov.unlock_item != 0 {
            let (bottle, item) = from_index(mov.unlock_item);
            self.set_item_locked(bottle, item, false);
        }

        #[cfg(feature = "pluggable_bottles")]
        {
            if mov.unplug_bottle != 0 {
                debug_assert!(!self.get_bottle_plugged(mov.unplug_bottle));
                self.set_pluggable_bottle(mov.unplug_bottle, false);
            }
            self.bottle_plugged = !self.bottle_plugged & self.pluggable_bottle;
        }

        #[cfg(feature = "freezable_bottles")]
        {
            self.frozen &= !mov.unfreeze_run;
        }

        #[cfg(feature = "curtains")]
        if mov.lift_curtain_range != Bits::ZERO {
            for i in 0..self.bottle_count {
                if !mov.lift_curtain_range.has(usize::from(i)) {
                    continue;
                }
                let Some((from, to)) = self.get_curtain_range(i) else {
                    break;
                };
                debug_assert_ne!(from, 0);
                debug_assert_ne!(to, 0);
                self.set_curtain_range(i, from, to - 1);
                debug_assert!(self.get_behind_curtain(to - 1));
                self.set_behind_curtain(to - 1, false);
            }
        }

        #[cfg(feature = "safes")]
        if mov.decrement_safe_counter != Bits::ZERO {
            for i in 1..self.bottle_count + 1 {
                let new_counter = self.get_safe_counter(i)
                    - u8::from(mov.decrement_safe_counter.has(usize::from(i)));
                self.set_safe_counter(i, new_counter);
            }
        }

        #[cfg(feature = "lock_groups")]
        if mov.remove_key != 0 {
            let (bottle, item) = from_index(mov.remove_key);
            self.set_item_has_key(bottle, item, false);
            self.bottle_locked &= !mov.unlock_bottle;
        }

        #[cfg(feature = "color_curtains")]
        {
            self.color_curtain_active &= !mov.lift_color_curtain;
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

        if mov.finalize_bottle != 0 {
            self.set_bottle_finalized(mov.finalize_bottle, false);
        }

        #[cfg(feature = "hidable_items")]
        if mov.show_item != 0 {
            let (bottle, item) = from_index(mov.show_item);
            self.set_item_hidden(bottle, item, true);
        }

        #[cfg(feature = "lockable_items")]
        if mov.unlock_item != 0 {
            let (bottle, item) = from_index(mov.unlock_item);
            self.set_item_locked(bottle, item, true);
        }

        #[cfg(feature = "pluggable_bottles")]
        {
            self.bottle_plugged = !self.bottle_plugged & self.pluggable_bottle;
            if mov.unplug_bottle != 0 {
                self.set_pluggable_bottle(mov.unplug_bottle, true);
            }
        }

        #[cfg(feature = "freezable_bottles")]
        {
            self.frozen |= mov.unfreeze_run;
        }

        #[cfg(feature = "curtains")]
        if mov.lift_curtain_range != Bits::ZERO {
            for i in 0..self.bottle_count {
                if !mov.lift_curtain_range.has(usize::from(i)) {
                    continue;
                }
                let Some((from, to)) = self.get_curtain_range(i) else {
                    break;
                };
                self.set_curtain_range(i, from, to + 1);
                self.set_behind_curtain(to, true);
            }
        }

        #[cfg(feature = "safes")]
        if mov.decrement_safe_counter != Bits::ZERO {
            for i in 1..self.bottle_count + 1 {
                let new_counter = self.get_safe_counter(i)
                    + u8::from(mov.decrement_safe_counter.has(usize::from(i)));
                self.set_safe_counter(i, new_counter);
            }
        }

        #[cfg(feature = "lock_groups")]
        if mov.remove_key != 0 {
            let (bottle, item) = from_index(mov.remove_key);
            self.set_item_has_key(bottle, item, true);
            self.bottle_locked |= mov.unlock_bottle;
        }

        #[cfg(feature = "color_curtains")]
        {
            self.color_curtain_active |= mov.lift_color_curtain;
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

    fn compute_bottle_finalized(&self, bottle: u8, add_count: u8, add_color: u8) -> bool {
        debug_assert_eq!(add_count == 0, add_color == 0);
        let height = self.get_height(bottle);
        let capacity = self.get_capacity(bottle);
        let new_bottom_color = {
            let bottom_color = self.get_color(bottle, 0);
            if bottom_color == 0 {
                add_color
            } else {
                bottom_color
            }
        };
        let finalized = (0..height).all(|i| {
            (self.get_color(bottle, i) == new_bottom_color)
                & !self.get_item_hidden(bottle, i)
                & !self.get_item_locked(bottle, i)
        });
        (height + add_count == capacity)
            && finalized
            && (add_color == 0 || add_color == new_bottom_color)
            && self.get_color_count(new_bottom_color) == capacity
    }

    fn get_bottle_finalized(&self, bottle: u8) -> bool {
        self.bottle_finalized.has(usize::from(bottle))
    }

    fn set_bottle_finalized(&mut self, bottle: u8, finalized: bool) {
        debug_assert_ne!(bottle, 0);
        self.bottle_finalized.set(usize::from(bottle), finalized);
    }

    fn get_color_count(&self, color: u8) -> u8 {
        self.color_count
            .get_n(usize::from(color) * BOTTLE_SIZE_BITS, BOTTLE_SIZE_BITS) as u8
    }

    fn set_color_count(&mut self, color: u8, count: u8) {
        debug_assert_ne!(color, 0);
        debug_assert!(usize::from(count) <= ITEM_COUNT);
        self.color_count.set_n(
            usize::from(color) * BOTTLE_SIZE_BITS,
            BOTTLE_SIZE_BITS,
            u16::from(count),
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
            let _ = (bottle, item);
            false
        }
    }

    #[cfg(feature = "hidable_items")]
    fn set_item_hidden(&mut self, bottle: u8, item: u8, hidden: bool) {
        debug_assert_ne!(bottle, 0);
        self.item_hidden
            .set(usize::from(bottle) * ITEM_COUNT + usize::from(item), hidden);
    }

    fn get_item_locked(&self, bottle: u8, item: u8) -> bool {
        #[cfg(feature = "lockable_items")]
        {
            self.item_locked
                .has(usize::from(bottle) * ITEM_COUNT + usize::from(item))
        }
        #[cfg(not(feature = "lockable_items"))]
        {
            let _ = (bottle, item);
            false
        }
    }

    #[cfg(feature = "lockable_items")]
    fn set_item_locked(&mut self, bottle: u8, item: u8, locked: bool) {
        debug_assert_ne!(bottle, 0);
        self.item_locked
            .set(usize::from(bottle) * ITEM_COUNT + usize::from(item), locked);
    }

    fn get_bottle_immovable(&self, bottle: u8) -> bool {
        #[cfg(feature = "immovable_bottles")]
        {
            self.bottle_immovable.has(usize::from(bottle))
        }
        #[cfg(not(feature = "immovable_bottles"))]
        {
            let _ = bottle;
            false
        }
    }

    #[cfg(feature = "immovable_bottles")]
    fn set_bottle_immovable(&mut self, bottle: u8, immovable: bool) {
        debug_assert_ne!(bottle, 0);
        self.bottle_immovable.set(usize::from(bottle), immovable);
    }

    #[cfg(feature = "pluggable_bottles")]
    fn get_pluggable_bottle(&self, bottle: u8) -> bool {
        self.pluggable_bottle.has(usize::from(bottle))
    }

    #[cfg(feature = "pluggable_bottles")]
    fn set_pluggable_bottle(&mut self, bottle: u8, pluggable: bool) {
        debug_assert_ne!(bottle, 0);
        self.pluggable_bottle.set(usize::from(bottle), pluggable);
    }

    fn get_bottle_plugged(&self, bottle: u8) -> bool {
        #[cfg(feature = "pluggable_bottles")]
        {
            self.bottle_plugged.has(usize::from(bottle))
        }
        #[cfg(not(feature = "pluggable_bottles"))]
        {
            let _ = bottle;
            false
        }
    }

    #[cfg(feature = "pluggable_bottles")]
    fn set_bottle_plugged(&mut self, bottle: u8, plugged: bool) {
        debug_assert_ne!(bottle, 0);
        self.bottle_plugged.set(usize::from(bottle), plugged);
    }

    fn get_frozen(&self, bottle: u8) -> bool {
        #[cfg(feature = "freezable_bottles")]
        {
            self.frozen.has(usize::from(bottle))
        }
        #[cfg(not(feature = "freezable_bottles"))]
        {
            let _ = bottle;
            false
        }
    }

    #[cfg(feature = "curtains")]
    fn get_curtain_range(&self, index: u8) -> Option<(u8, u8)> {
        let from = self
            .curtain_range
            .get_n(usize::from(index) * (BOTTLE_BITS + 1) * 2, BOTTLE_BITS + 1)
            as u8;
        let to = self.curtain_range.get_n(
            usize::from(index) * (BOTTLE_BITS + 1) * 2 + BOTTLE_BITS + 1,
            BOTTLE_BITS + 1,
        ) as u8;
        match (from, to) {
            (0, 0) => None,
            _ => Some((from, to)),
        }
    }

    #[cfg(feature = "curtains")]
    fn set_curtain_range(&mut self, index: u8, from: u8, to: u8) {
        self.curtain_range.set_n(
            usize::from(index) * (BOTTLE_BITS + 1) * 2,
            BOTTLE_BITS + 1,
            u16::from(from),
        );
        self.curtain_range.set_n(
            usize::from(index) * (BOTTLE_BITS + 1) * 2 + BOTTLE_BITS + 1,
            BOTTLE_BITS + 1,
            u16::from(to),
        );
    }

    fn get_behind_curtain(&self, bottle: u8) -> bool {
        #[cfg(feature = "curtains")]
        {
            self.behind_curtain.has(usize::from(bottle))
        }
        #[cfg(not(feature = "curtains"))]
        {
            let _ = bottle;
            false
        }
    }

    #[cfg(feature = "curtains")]
    fn set_behind_curtain(&mut self, bottle: u8, behind_curtain: bool) {
        debug_assert_ne!(bottle, 0);
        self.behind_curtain.set(usize::from(bottle), behind_curtain);
    }

    fn get_safe_counter(&self, bottle: u8) -> u8 {
        #[cfg(feature = "safes")]
        {
            self.safe_counter
                .get_n(usize::from(bottle) * SAFE_COUNTER_BITS, SAFE_COUNTER_BITS) as u8
        }
        #[cfg(not(feature = "safes"))]
        {
            let _ = bottle;
            0
        }
    }

    #[cfg(feature = "safes")]
    fn set_safe_counter(&mut self, bottle: u8, safe_counter: u8) {
        debug_assert_ne!(bottle, 0);
        self.safe_counter.set_n(
            usize::from(bottle) * SAFE_COUNTER_BITS,
            SAFE_COUNTER_BITS,
            u16::from(safe_counter),
        );
    }

    #[cfg(feature = "lock_groups")]
    fn get_item_has_key(&self, bottle: u8, item: u8) -> bool {
        self.item_has_key
            .has(usize::from(bottle) * ITEM_COUNT + usize::from(item))
    }

    #[cfg(feature = "lock_groups")]
    fn set_item_has_key(&mut self, bottle: u8, item: u8, has_key: bool) {
        debug_assert_ne!(bottle, 0);
        self.item_has_key.set(
            usize::from(bottle) * ITEM_COUNT + usize::from(item),
            has_key,
        );
    }

    #[cfg(feature = "lock_groups")]
    fn get_bottle_key(&self, bottle: u8) -> u16 {
        self.bottle_key.get_n(
            usize::from(bottle) * (BOTTLE_BITS + ITEM_BITS),
            BOTTLE_BITS + ITEM_BITS,
        )
    }

    #[cfg(feature = "lock_groups")]
    fn set_bottle_key(&mut self, bottle: u8, key: u16) {
        debug_assert_ne!(bottle, 0);
        self.bottle_key.set_n(
            usize::from(bottle) * (BOTTLE_BITS + ITEM_BITS),
            BOTTLE_BITS + ITEM_BITS,
            key,
        );
    }

    fn get_bottle_locked(&self, bottle: u8) -> bool {
        #[cfg(feature = "lock_groups")]
        {
            self.bottle_locked.has(usize::from(bottle))
        }
        #[cfg(not(feature = "lock_groups"))]
        {
            let _ = bottle;
            false
        }
    }

    fn get_bottle_color(&self, bottle: u8) -> u8 {
        #[cfg(feature = "colored_bottles")]
        {
            self.bottle_color
                .get_n(usize::from(bottle) * COLOR_BITS, COLOR_BITS) as u8
        }
        #[cfg(not(feature = "colored_bottles"))]
        {
            let _ = bottle;
            0
        }
    }

    #[cfg(feature = "colored_bottles")]
    fn set_bottle_color(&mut self, bottle: u8, color: u8) {
        debug_assert_ne!(bottle, 0);
        self.bottle_color.set_n(
            usize::from(bottle) * COLOR_BITS,
            COLOR_BITS,
            u16::from(color),
        );
    }

    #[cfg(feature = "color_curtains")]
    fn get_color_curtain(&self, bottle: u8) -> u8 {
        self.color_curtain
            .get_n(usize::from(bottle) * COLOR_BITS, COLOR_BITS) as u8
    }

    #[cfg(feature = "color_curtains")]
    fn set_color_curtain(&mut self, bottle: u8, color: u8) {
        debug_assert_ne!(bottle, 0);
        self.color_curtain.set_n(
            usize::from(bottle) * COLOR_BITS,
            COLOR_BITS,
            u16::from(color),
        );
    }

    fn get_color_curtain_active(&self, bottle: u8) -> bool {
        #[cfg(feature = "color_curtains")]
        {
            self.color_curtain_active.has(usize::from(bottle))
        }
        #[cfg(not(feature = "color_curtains"))]
        {
            let _ = bottle;
            false
        }
    }

    #[cfg(feature = "color_curtains")]
    fn set_color_curtain_active(&mut self, bottle: u8, active: bool) {
        debug_assert_ne!(bottle, 0);
        self.color_curtain_active.set(usize::from(bottle), active);
    }

    #[cfg(any(
        feature = "freezable_bottles",
        feature = "curtains",
        feature = "lock_groups"
    ))]
    fn compute_run(
        &self,
        ranges: &[(u8, u8)],
    ) -> Result<
        (
            Bits<{ storage_bits(BOTTLE_COUNT) }>,
            Bits<{ storage_bits(BOTTLE_COUNT) }>,
        ),
        Box<dyn Error>,
    > {
        if !ranges.is_sorted() {
            return Err("bottle ranges not sorted".into());
        }

        for ((_, to_a), (from_b, _)) in ranges.iter().copied().zip(ranges.iter().copied().skip(1)) {
            if to_a > from_b {
                return Err("bottle ranges overlapping".into());
            }
        }

        for (from, to) in ranges.iter().copied() {
            if from >= to {
                return Err("invalid bottle range".into());
            }

            if from > self.bottle_count || to > self.bottle_count {
                return Err("bottle range too large".into());
            }
        }

        let mut run = Bits::ZERO;
        let mut set = Bits::ZERO;
        let mut current_bit = false;
        let mut index = 1;

        for (from, to) in ranges.iter().copied() {
            let swap_bit = index < from + 1;
            while index < from + 1 {
                run.set(usize::from(index), current_bit);
                index += 1;
            }
            if swap_bit {
                current_bit = !current_bit;
            }

            for _ in from + 1..to + 1 {
                run.set(usize::from(index), current_bit);
                set.set(usize::from(index), true);
                index += 1;
            }
            current_bit = !current_bit;
        }

        while index < self.bottle_count + 1 {
            run.set(usize::from(index), current_bit);
            index += 1;
        }

        Ok((run, set))
    }

    #[cfg(feature = "freezable_bottles")]
    fn bottle_run(
        &self,
        b: Bits<{ storage_bits(BOTTLE_COUNT) }>,
        bottle: u8,
    ) -> Bits<{ storage_bits(BOTTLE_COUNT) }> {
        // TODO: Could use some faster bit magic.

        debug_assert_ne!(bottle, 0);
        let mut out = Bits::ZERO;
        out.set(usize::from(bottle), true);
        let offset_set = b.has(usize::from(bottle));

        for i in (1..bottle).rev() {
            if b.has(usize::from(i)) != offset_set {
                break;
            }
            out.set(usize::from(i), true);
        }

        for i in bottle + 1..self.bottle_count + 1 {
            if b.has(usize::from(i)) != offset_set {
                break;
            }
            out.set(usize::from(i), true);
        }

        out
    }
}

impl TryFrom<&StartingState> for State {
    type Error = Box<dyn Error>;

    fn try_from(value: &StartingState) -> Result<Self, Self::Error> {
        if value.content.len() >= BOTTLE_COUNT {
            return Err("too many bottles".into());
        }
        if value.content.len() != value.capacity.len() {
            return Err("content and capacity lengths do not match".into());
        }
        if value.content.iter().any(|v| v.len() > ITEM_COUNT) {
            return Err("bottle item count too large".into());
        }
        if value.capacity.iter().any(|v| *v == 0) {
            return Err("capacity zero".into());
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
        let mut color_counts = [0u8; COLOR_COUNT];

        for (index, bottle) in value.content.iter().enumerate() {
            let out_index = u8::try_from(index.checked_add(1).unwrap()).unwrap();
            state.set_capacity(out_index, value.capacity[index]);
            state.set_height(out_index, u8::try_from(bottle.len()).unwrap());

            for (item, color) in bottle.iter().copied().enumerate() {
                if color == 0 || usize::from(color) >= COLOR_COUNT {
                    return Err("invalid color value".into());
                }
                state.set_color(out_index, u8::try_from(item).unwrap(), color);

                if usize::from(color_counts[usize::from(color)]) >= ITEM_COUNT {
                    return Err("too many items of a single color".into());
                }
                color_counts[usize::from(color)] += 1;
            }
        }

        for (color, count) in color_counts.into_iter().enumerate().skip(1) {
            state.set_color_count(u8::try_from(color).unwrap(), count);
        }

        // TODO: We could check for duplicates for the following items.

        #[cfg(feature = "hidable_items")]
        for (bottle, item) in value.hidden_items.iter().copied() {
            if usize::from(bottle) >= value.content.len() {
                return Err("hidden item bottle not in range".into());
            }

            if usize::from(item) >= value.content[usize::from(bottle)].len().saturating_sub(1) {
                return Err("hidden item not in range".into());
            }

            state.set_item_hidden(bottle.checked_add(1).unwrap(), item, true);
        }

        #[cfg(feature = "lockable_items")]
        for (bottle, item) in value.locked_items.iter().copied() {
            if usize::from(bottle) >= value.content.len() {
                return Err("locked item bottle not in range".into());
            }

            if usize::from(item) >= value.content[usize::from(bottle)].len() {
                return Err("locked item not in range".into());
            }

            state.set_item_locked(bottle.checked_add(1).unwrap(), item, true);
        }

        #[cfg(feature = "immovable_bottles")]
        for bottle in value.immovable_bottles.iter().copied() {
            if usize::from(bottle) >= value.content.len() {
                return Err("immovable bottle not in range".into());
            }

            state.set_bottle_immovable(bottle.checked_add(1).unwrap(), true);
        }

        #[cfg(feature = "pluggable_bottles")]
        for (bottle, start_plugged) in value.pluggable_bottles.iter().copied() {
            if usize::from(bottle) >= value.content.len() {
                return Err("pluggable bottle not in range".into());
            }

            state.set_pluggable_bottle(bottle.checked_add(1).unwrap(), true);
            state.set_bottle_plugged(bottle.checked_add(1).unwrap(), start_plugged);
        }

        #[cfg(feature = "freezable_bottles")]
        {
            (state.frozen_run, state.frozen) = state.compute_run(&value.frozen_bottle_ranges)?;
        }

        #[cfg(feature = "curtains")]
        {
            (_, state.behind_curtain) = state.compute_run(&value.curtain_ranges)?;
            for (index, (from, to)) in value.curtain_ranges.iter().copied().enumerate() {
                state.set_curtain_range(u8::try_from(index).unwrap(), from + 1, to + 1);
            }
        }

        #[cfg(feature = "safes")]
        for (bottle, counter) in value.safes.iter().copied() {
            if usize::from(bottle) >= value.content.len() {
                return Err("safe counter not in range".into());
            }
            if usize::from(counter) > MAX_SAFE_COUNTER {
                return Err("safe counter value not in range".into());
            }
            state.set_safe_counter(bottle + 1, counter);
        }

        #[cfg(feature = "lock_groups")]
        {
            if value.lock_group_ranges.len() != value.lock_group_keys.len() {
                return Err("number of lock group ranges and keys don't match".into());
            }
            (_, state.bottle_locked) = state.compute_run(&value.lock_group_ranges)?;

            for ((bottle, item), (from, to)) in value
                .lock_group_keys
                .iter()
                .copied()
                .zip(value.lock_group_ranges.iter().copied())
            {
                if usize::from(bottle) >= value.content.len() {
                    return Err("lock group key bottle not in range".into());
                }

                if usize::from(item) >= value.content[usize::from(bottle)].len() {
                    return Err("lock group key item not in range".into());
                }

                if usize::from(item) == value.content[usize::from(bottle)].len() - 1 {
                    return Err("lock group key should not start on top of a bottle".into());
                }

                let color = value.content[usize::from(bottle)][usize::from(item) + 1];
                if state.pour_include(bottle + 1, item, color)
                    && state.pour_include(bottle + 1, item + 1, color)
                {
                    return Err("lock group key must be the first item of a pour".into());
                }

                // TODO: Could check if lock groups contain loops.
                if (from..to).contains(&bottle) {
                    return Err("lock group key stored inside itself".into());
                }

                if state.get_item_has_key(bottle + 1, item) {
                    return Err("lock group key duplicate".into());
                }

                state.set_item_has_key(bottle + 1, item, true);
                let key = to_index(bottle + 1, item);
                for i in from..to {
                    state.set_bottle_key(i + 1, key);
                }
            }
        }

        for i in 1..state.bottle_count + 1 {
            let feature_count = usize::from(state.get_behind_curtain(i))
                + usize::from(state.get_safe_counter(i) != 0)
                + usize::from(state.get_bottle_locked(i));
            if feature_count > 1 {
                return Err(
                    "at most one of curtain, safe, or lock can be chosen per bottle".into(),
                );
            }
        }

        #[cfg(feature = "colored_bottles")]
        for (bottle, color) in value.colored_bottles.iter().copied() {
            if usize::from(bottle) >= value.content.len() {
                return Err("colored bottle not in range".into());
            }
            if color == 0 || usize::from(color) >= COLOR_COUNT {
                return Err("colored bottle color value not in range".into());
            }
            // TODO: Could check if the color appears at all.
            state.set_bottle_color(bottle + 1, color);
        }

        #[cfg(feature = "color_curtains")]
        for (bottle, color) in value.color_curtains.iter().copied() {
            if usize::from(bottle) >= value.content.len() {
                return Err("color curtain not in range".into());
            }
            if color == 0 || usize::from(color) >= COLOR_COUNT {
                return Err("color curtain value not in range".into());
            }
            // TODO:
            // Could check if the color appears at all or if a curtain hides
            // a color from itself.
            state.set_color_curtain(bottle + 1, color);
            state.set_color_curtain_active(bottle + 1, true);
        }

        for bottle in 1..state.bottle_count + 1 {
            if state.compute_bottle_finalized(bottle, 0, 0) {
                return Err("finalized bottles not allowed".into());
            }
        }

        Ok(state)
    }
}

#[cfg(any(
    feature = "hidable_items",
    feature = "lockable_items",
    feature = "lock_groups"
))]
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

#[cfg(any(
    feature = "hidable_items",
    feature = "lockable_items",
    feature = "lock_groups"
))]
fn to_index(bottle: u8, item: u8) -> u16 {
    debug_assert!(u16::try_from(ITEM_COUNT).is_ok());
    debug_assert!(usize::from(bottle) < BOTTLE_COUNT);
    debug_assert!(usize::from(item) < ITEM_COUNT);
    u16::from(bottle) * ITEM_COUNT as u16 + u16::from(item)
}

/// Collect all changes to apply a move, which can be undone.
// All optional item indices use the zero value, which is possible because the
// zero bottle index is reserved.
// TODO: Having Default on Move is not great, as that is always invalid.
#[derive(Clone, Copy, Default)]
struct Move {
    from_bottle: u8,
    to_bottle: u8,
    count: u8,
    color: u8,
    finalize_bottle: u8,

    #[cfg(feature = "hidable_items")]
    show_item: u16,
    #[cfg(feature = "lockable_items")]
    unlock_item: u16,
    #[cfg(feature = "pluggable_bottles")]
    unplug_bottle: u8,
    #[cfg(feature = "freezable_bottles")]
    unfreeze_run: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    #[cfg(feature = "curtains")]
    lift_curtain_range: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    #[cfg(feature = "safes")]
    decrement_safe_counter: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    #[cfg(feature = "lock_groups")]
    remove_key: u16,
    #[cfg(feature = "lock_groups")]
    unlock_bottle: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    #[cfg(feature = "color_curtains")]
    lift_color_curtain: Bits<{ storage_bits(BOTTLE_COUNT) }>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StartingState {
    content: Vec<Vec<u8>>,
    capacity: Vec<u8>,

    #[cfg(feature = "hidable_items")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hidden_items: Vec<(u8, u8)>,

    #[cfg(feature = "lockable_items")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    locked_items: Vec<(u8, u8)>,

    #[cfg(feature = "immovable_bottles")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    immovable_bottles: Vec<u8>,

    #[cfg(feature = "pluggable_bottles")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pluggable_bottles: Vec<(u8, bool)>,

    #[cfg(feature = "freezable_bottles")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    frozen_bottle_ranges: Vec<(u8, u8)>,

    #[cfg(feature = "curtains")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    curtain_ranges: Vec<(u8, u8)>,

    #[cfg(feature = "safes")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    safes: Vec<(u8, u8)>,

    #[cfg(feature = "lock_groups")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    lock_group_ranges: Vec<(u8, u8)>,

    #[cfg(feature = "lock_groups")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    lock_group_keys: Vec<(u8, u8)>,

    #[cfg(feature = "colored_bottles")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    colored_bottles: Vec<(u8, u8)>,

    #[cfg(feature = "color_curtains")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    color_curtains: Vec<(u8, u8)>,
}

struct Searcher {
    state: State,
    depth: i8,
    visited: Vec<i8>,
}

impl Searcher {
    fn new(state: State, depth: i8, visited_size: usize) -> Result<Self, Box<dyn Error>> {
        if depth < 0 || depth > MAX_SEARCH_DEPTH {
            return Err("invalid depth".into());
        }

        if visited_size < 1_000_000
            || visited_size > usize::try_from(u32::MAX).unwrap()
            || !visited_size.is_power_of_two()
        {
            return Err("invalid visited size".into());
        }

        Ok(Self {
            state,
            depth,
            visited: vec![0; visited_size],
        })
    }

    fn search(mut self) -> i8 {
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
        let hash = self.hash_state();
        let visited = self.get_visited(hash);
        if visited != 0 {
            // This can give false negatives, which might prune useful search
            // states. We can make this less likely by adding a u8 of the upper
            // bits of the hash, but the problem can't be avoided completely.

            // TODO:
            // This can give false positives, but this should only be relevant
            // when searching for the best move and not ending the search on
            // the first solution found.

            if visited == i8::MIN {
                return i8::MIN;
            } else if visited < 0 {
                if -(visited + 1) >= remaining_depth {
                    return visited;
                }
                // Otherwise we need to check again.
            } else {
                if visited - 1 <= remaining_depth {
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

        for mov in moves.iter().copied() {
            self.state.apply_move(mov);
            let result = self.search_recursive(next_buffer);
            self.state.undo_move(mov);

            all_failed &= result == i8::MIN;
            if result > 0 {
                self.set_visited(hash, result + 1);
                return result + 1;
            }
        }

        let error_marker = if all_failed { i8::MIN } else { error_marker };
        self.set_visited(hash, error_marker);
        error_marker
    }

    fn hash_state(&self) -> u64 {
        // Hash collision attacks should not be a problem in our use case. Only
        // hash the fields which can actually change.
        let mut s = DefaultHasher::new();
        self.state.content.hash(&mut s);
        self.state.height.hash(&mut s);
        self.state.bottle_finalized.hash(&mut s);
        #[cfg(feature = "hidable_items")]
        self.state.item_hidden.hash(&mut s);
        #[cfg(feature = "lockable_items")]
        self.state.item_locked.hash(&mut s);
        #[cfg(feature = "freezable_bottles")]
        self.state.frozen.hash(&mut s);
        #[cfg(feature = "pluggable_bottles")]
        self.state.pluggable_bottle.hash(&mut s);
        #[cfg(feature = "pluggable_bottles")]
        self.state.bottle_plugged.hash(&mut s);
        #[cfg(feature = "curtains")]
        self.state.curtain_range.hash(&mut s);
        #[cfg(feature = "curtains")]
        self.state.behind_curtain.hash(&mut s);
        #[cfg(feature = "safes")]
        self.state.safe_counter.hash(&mut s);
        #[cfg(feature = "lock_groups")]
        self.state.item_has_key.hash(&mut s);
        #[cfg(feature = "lock_groups")]
        self.state.bottle_locked.hash(&mut s);
        #[cfg(feature = "color_curtains")]
        self.state.color_curtain_active.hash(&mut s);
        s.finish()
    }

    fn get_visited(&self, hash: u64) -> i8 {
        self.visited[hash as usize & (self.visited.len() - 1)]
    }

    fn set_visited(&mut self, hash: u64, value: i8) {
        let visited_len = self.visited.len();
        self.visited[hash as usize & (visited_len - 1)] = value;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        return Err("USAGE: solver PUZZLE_PATH DEPTH".into());
    }

    let depth: i8 = args[2].parse()?;
    let starting_state_raw = fs::read_to_string(&args[1])?;
    let starting_state: StartingState = serde_json::from_str(&starting_state_raw)?;

    let state = State::try_from(&starting_state)?;
    // TODO: Get visited_size from the commandline.
    let searcher = Searcher::new(state, depth, 2 * 1024 * 1024 * 1024)?;
    println!("{:?}", searcher.search());

    Ok(())
}
