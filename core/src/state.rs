use std::range::RangeInclusive;
use std::{cmp::min, error::Error};

use serde::{Deserialize, Serialize};

use crate::bits::Bits;

pub(crate) const ITEM_BITS: usize = bits_from_build_env(option_env!("WATER_SORT_ITEM_BITS"), 2, 4);
pub(crate) const BOTTLE_BITS: usize =
    bits_from_build_env(option_env!("WATER_SORT_BOTTLE_BITS"), 2, 5);
pub(crate) const COLOR_BITS: usize =
    bits_from_build_env(option_env!("WATER_SORT_COLOR_BITS"), 2, 4);
pub(crate) const SAFE_COUNTER_BITS: usize =
    bits_from_build_env(option_env!("WATER_SORT_SAFE_COUNTER_BITS"), 2, 3);

/// Need an extra bit for the height and capacity.
pub(crate) const BOTTLE_SIZE_BITS: usize = ITEM_BITS + 1;

pub const ITEM_COUNT: usize = 1 << ITEM_BITS;

/// The zero value is reserved.
pub const BOTTLE_COUNT: usize = 1 << BOTTLE_BITS;

/// The zero value is reserved.
pub const COLOR_COUNT: usize = 1 << COLOR_BITS;

pub const MAX_SAFE_COUNTER: usize = (1 << SAFE_COUNTER_BITS) - 1;

const fn bits_from_build_env(value: Option<&str>, min: usize, max: usize) -> usize {
    let Some(value) = value else {
        return max;
    };

    let bytes = value.as_bytes();
    assert!(!bytes.is_empty());
    assert!(bytes[0] != b'0');

    let mut out = 0usize;
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];
        assert!(b.is_ascii_digit());

        out = out
            .checked_mul(10)
            .unwrap()
            .checked_add((b - b'0') as usize)
            .unwrap();

        i += 1;
    }

    assert!(out >= min);
    assert!(out <= max);
    out
}

pub(crate) const fn storage_bits(bits: usize) -> usize {
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
#[derive(Clone, Copy)]
pub struct State {
    /// Must be less than the maximum amount of bottles.
    pub(crate) bottle_count: u8,

    pub(crate) content: Bits<{ storage_bits(ITEM_COUNT * BOTTLE_COUNT * COLOR_BITS) }>,
    pub(crate) height: Bits<{ storage_bits(BOTTLE_COUNT * BOTTLE_SIZE_BITS) }>,
    // The capacity must be greater than zero.
    pub(crate) capacity: Bits<{ storage_bits(BOTTLE_COUNT * BOTTLE_SIZE_BITS) }>,
    pub(crate) bottle_finalized: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    pub(crate) color_count: Bits<{ storage_bits(COLOR_COUNT * BOTTLE_SIZE_BITS) }>,

    // The user cannot see this color item. Only relevant for the solver when
    // pouring out of this bottle, items connected by color are only poured
    // until the first invisible item. The top item should always be visible.
    // Unused item slots must never be marked as hidden.
    #[cfg(feature = "hidable_items")]
    pub(crate) item_hidden: Bits<{ storage_bits(ITEM_COUNT * BOTTLE_COUNT) }>,

    // The current item can only be moved out of this bottle and only one at
    // a time. Pouring into this bottle is not allowed if the top item is
    // locked. The lock disappears once the item has been removed.
    #[cfg(feature = "lockable_items")]
    pub(crate) item_locked: Bits<{ storage_bits(ITEM_COUNT * BOTTLE_COUNT) }>,

    // Can only fill into this bottle, never pour out of it. This does not
    // change after initialization.
    #[cfg(feature = "immovable_bottles")]
    pub(crate) bottle_immovable: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    // Each move of any bottle, all plugged bottles are unplugged and
    // vice-versa. If a pluggable bottle is emptied, the plug is removed for
    // all future moves.
    #[cfg(feature = "pluggable_bottles")]
    pub(crate) pluggable_bottle: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    #[cfg(feature = "pluggable_bottles")]
    pub(crate) bottle_plugged: Bits<{ storage_bits(BOTTLE_COUNT) }>,

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
    pub(crate) frozen_run: Bits<{ storage_bits(BOTTLE_COUNT) }>,
    #[cfg(feature = "freezable_bottles")]
    pub(crate) frozen: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    // A bottle behind a curtain is not visible and cannot be interacted with.
    // When any other bottle is solved, each curtain group unlocks a bottle,
    // ordered from the largest to the smallest index. Each bottle, except for
    // two, can be behind a curtain at the same time to be solvable and a
    // curtain group can consist of only a single bottle.
    #[cfg(feature = "curtains")]
    pub(crate) curtain_range: Bits<{ storage_bits(BOTTLE_COUNT * (BOTTLE_BITS + 1) * 2) }>,
    #[cfg(feature = "curtains")]
    pub(crate) behind_curtain: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    // A bottle in a safe, meaning a counter greater than zero, is not visible
    // and cannot be interacted with. Each time any bottle is solved, all
    // non-zero safe counters are decremented by one. Safes could be grouped
    // in the UI, but this is not relevant for the state representation. Each
    // bottle, except for two, could be stored in a safe at the same time to
    // be solvable.
    #[cfg(feature = "safes")]
    pub(crate) safe_counter: Bits<{ storage_bits(BOTTLE_COUNT * SAFE_COUNTER_BITS) }>,

    // All bottles in a locked group cannot be interacted with. One color item
    // has the associated key for each lock group. When this item is at the
    // top of a bottle, the group is unlocked. When multiple items can be
    // poured at the same time, the key must always be the top item.
    #[cfg(feature = "lock_groups")]
    pub(crate) item_has_key: Bits<{ storage_bits(BOTTLE_COUNT * ITEM_COUNT) }>,
    #[cfg(feature = "lock_groups")]
    pub(crate) bottle_key: Bits<{ storage_bits(BOTTLE_COUNT * (BOTTLE_BITS + ITEM_BITS)) }>,
    #[cfg(feature = "lock_groups")]
    pub(crate) bottle_locked: Bits<{ storage_bits(BOTTLE_COUNT) }>,

    // Only items of a specific color can be poured into this bottle, or zero,
    // if all colors are allowed. Storing other colors in such a bottle is
    // allowed. This does not change after initialization.
    #[cfg(feature = "colored_bottles")]
    pub(crate) bottle_color: Bits<{ storage_bits(BOTTLE_COUNT * COLOR_BITS) }>,

    // The bottle cannot be interacted with, until that specific color is
    // solved. The zero color implies that the bottle is not hidden by a
    // color curtain.
    #[cfg(feature = "color_curtains")]
    pub(crate) color_curtain: Bits<{ storage_bits(BOTTLE_COUNT * COLOR_BITS) }>,
    #[cfg(feature = "color_curtains")]
    pub(crate) color_curtain_active: Bits<{ storage_bits(BOTTLE_COUNT) }>,
}

impl State {
    const ZERO: Self = Self {
        bottle_count: 0,
        content: Bits::ZERO,
        height: Bits::ZERO,
        capacity: Bits::ZERO,
        bottle_finalized: Bits::ZERO,
        color_count: Bits::ZERO,
        #[cfg(feature = "hidable_items")]
        item_hidden: Bits::ZERO,
        #[cfg(feature = "lockable_items")]
        item_locked: Bits::ZERO,
        #[cfg(feature = "immovable_bottles")]
        bottle_immovable: Bits::ZERO,
        #[cfg(feature = "pluggable_bottles")]
        pluggable_bottle: Bits::ZERO,
        #[cfg(feature = "pluggable_bottles")]
        bottle_plugged: Bits::ZERO,
        #[cfg(feature = "freezable_bottles")]
        frozen_run: Bits::ZERO,
        #[cfg(feature = "freezable_bottles")]
        frozen: Bits::ZERO,
        #[cfg(feature = "curtains")]
        curtain_range: Bits::ZERO,
        #[cfg(feature = "curtains")]
        behind_curtain: Bits::ZERO,
        #[cfg(feature = "safes")]
        safe_counter: Bits::ZERO,
        #[cfg(feature = "lock_groups")]
        item_has_key: Bits::ZERO,
        #[cfg(feature = "lock_groups")]
        bottle_key: Bits::ZERO,
        #[cfg(feature = "lock_groups")]
        bottle_locked: Bits::ZERO,
        #[cfg(feature = "colored_bottles")]
        bottle_color: Bits::ZERO,
        #[cfg(feature = "color_curtains")]
        color_curtain: Bits::ZERO,
        #[cfg(feature = "color_curtains")]
        color_curtain_active: Bits::ZERO,
    };

    pub fn bottles(&self) -> impl Iterator<Item = u8> {
        1..self.bottle_count + 1
    }

    pub fn solved(&self) -> bool {
        for bottle in 1..self.bottle_count + 1 {
            if self.get_height(bottle) != 0 && !self.get_bottle_finalized(bottle) {
                return false;
            }
        }

        true
    }

    pub fn pours(&self) -> Option<Pours> {
        let pours = self.moves()?.fold(Pours(Bits::ZERO), |acc, mov| {
            acc.with(mov.from_bottle, mov.to_bottle)
        });
        Some(pours)
    }

    pub(crate) fn moves(&self) -> Option<impl Iterator<Item = Move>> {
        if self.solved() {
            return None;
        }

        let iter = self
            .bottles()
            .filter(|from| self.can_move_from(*from))
            .flat_map(move |from| self.bottles().filter_map(move |to| self.move_to(from, to)));
        Some(iter)
    }

    pub(crate) fn fill_moves(&self, buffer: &mut [Move; BOTTLE_COUNT * BOTTLE_COUNT]) -> usize {
        let mut index = 0;

        for from in 1..self.bottle_count + 1 {
            if !self.can_move_from(from) {
                continue;
            }

            for to in 1..self.bottle_count + 1 {
                let Some(mov) = self.move_to(from, to) else {
                    continue;
                };

                debug_assert!(index < buffer.len());
                buffer[index % buffer.len()] = mov;
                index += 1;
            }
        }

        index
    }

    pub fn can_move_from(&self, bottle: u8) -> bool {
        let from_height = self.get_height(bottle);
        debug_assert!(!self.get_item_hidden(bottle, from_height.saturating_sub(1)));

        let invalid = (from_height == 0)
            | self.get_bottle_immovable(bottle)
            | self.get_bottle_plugged(bottle)
            | self.get_frozen(bottle)
            | self.get_bottle_finalized(bottle)
            | self.get_behind_curtain(bottle)
            | (self.get_safe_counter(bottle) != 0)
            | self.get_bottle_locked(bottle)
            | self.get_color_curtain_active(bottle);

        !invalid
    }

    fn move_to(&self, from: u8, to: u8) -> Option<Move> {
        let from_height = self.get_height(from);

        let to_height = self.get_height(to);
        let to_capacity = self.get_capacity(to);
        let space = to_capacity - to_height;

        let from_top_color = self.get_color(from, from_height.saturating_sub(1));
        let to_top_color = self.get_color(to, to_height.saturating_sub(1));
        let to_bottle_color = self.get_bottle_color(to);

        let to_top_item_locked = self.get_item_locked(to, to_height.saturating_sub(1));

        if (from == to)
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
            return None;
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
                let unlock_bottle = (1..self.bottle_count + 1).fold(Bits::ZERO, |mut acc, i| {
                    acc.set(usize::from(i), self.get_bottle_key(i) == key);
                    acc
                });
                (key, unlock_bottle)
            } else {
                (0, Bits::ZERO)
            };

        Some(Move {
            from_bottle: from,
            to_bottle: to,
            count,
            color: from_top_color,
            finalize_bottle,

            #[cfg(feature = "hidable_items")]
            show_item: if next_from_height > 0 && self.get_item_hidden(from, next_from_top_index) {
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
                let (run, _) = self.bottle_run(self.frozen_run, to);
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
        })
    }

    fn pour_include(&self, bottle: u8, item: u8, color: u8) -> bool {
        (self.get_color(bottle, item) == color)
            & !self.get_item_hidden(bottle, item)
            & !self.get_item_locked(bottle, item)
    }

    pub fn pour(&mut self, from: u8, to: u8) -> Result<Move, Box<dyn Error>> {
        let Some(mut moves) = self.moves() else {
            return Err("already solved".into());
        };

        let mov = moves.find(|mov| mov.from_bottle == from && mov.to_bottle == to);
        drop(moves);

        match mov {
            Some(mov) => {
                self.apply_move_unchecked(mov);
                Ok(mov)
            }
            None => Err("move not allowed".into()),
        }
    }

    pub(crate) fn apply_move_unchecked(&mut self, mov: Move) {
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

    pub(crate) fn undo_move_unchecked(&mut self, mov: Move) {
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

    pub fn get_height(&self, bottle: u8) -> u8 {
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

    pub fn get_capacity(&self, bottle: u8) -> u8 {
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

    pub fn get_color(&self, bottle: u8, item: u8) -> u8 {
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

    pub fn get_bottle_finalized(&self, bottle: u8) -> bool {
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

    pub fn get_item_hidden(&self, bottle: u8, item: u8) -> bool {
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

    pub fn get_item_locked(&self, bottle: u8, item: u8) -> bool {
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

    pub fn get_bottle_immovable(&self, bottle: u8) -> bool {
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

    pub fn get_pluggable_bottle(&self, bottle: u8) -> bool {
        #[cfg(feature = "pluggable_bottles")]
        {
            self.pluggable_bottle.has(usize::from(bottle))
        }
        #[cfg(not(feature = "pluggable_bottles"))]
        {
            let _ = bottle;
            false
        }
    }

    #[cfg(feature = "pluggable_bottles")]
    fn set_pluggable_bottle(&mut self, bottle: u8, pluggable: bool) {
        debug_assert_ne!(bottle, 0);
        self.pluggable_bottle.set(usize::from(bottle), pluggable);
    }

    pub fn get_bottle_plugged(&self, bottle: u8) -> bool {
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

    pub fn get_frozen_ranges(&self) -> impl Iterator<Item = RangeInclusive<u8>> {
        #[cfg(feature = "freezable_bottles")]
        {
            use itertools::Itertools;

            (1..self.bottle_count + 1)
                .filter(|bottle| self.get_frozen(*bottle))
                .map(|bottle| self.bottle_run(self.frozen_run, bottle).1)
                .dedup()
        }

        #[cfg(not(feature = "freezable_bottles"))]
        {
            std::iter::empty()
        }
    }

    pub fn get_frozen(&self, bottle: u8) -> bool {
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

    pub fn get_curtain_ranges(&self) -> impl Iterator<Item = RangeInclusive<u8>> {
        #[cfg(feature = "curtains")]
        {
            (0..self.bottle_count)
                .filter_map(|i| self.get_curtain_range(i))
                .filter(|(from, to)| to > from)
                .map(|(from, to)| (from..=to - 1).into())
        }
        #[cfg(not(feature = "curtains"))]
        {
            std::iter::empty()
        }
    }

    #[cfg(feature = "curtains")]
    pub(crate) fn get_curtain_range(&self, index: u8) -> Option<(u8, u8)> {
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

    pub fn get_behind_curtain(&self, bottle: u8) -> bool {
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

    pub fn get_safe_counter(&self, bottle: u8) -> u8 {
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

    pub fn get_item_has_key(&self, bottle: u8, item: u8) -> bool {
        #[cfg(feature = "lock_groups")]
        {
            self.item_has_key
                .has(usize::from(bottle) * ITEM_COUNT + usize::from(item))
        }
        #[cfg(not(feature = "lock_groups"))]
        {
            let _ = (bottle, item);
            false
        }
    }

    #[cfg(feature = "lock_groups")]
    fn set_item_has_key(&mut self, bottle: u8, item: u8, has_key: bool) {
        debug_assert_ne!(bottle, 0);
        self.item_has_key.set(
            usize::from(bottle) * ITEM_COUNT + usize::from(item),
            has_key,
        );
    }

    pub fn get_lock_group_ranges(&self) -> impl Iterator<Item = (u16, RangeInclusive<u8>)> {
        #[cfg(feature = "lock_groups")]
        {
            let mut bottle = 1u8;
            std::iter::from_fn(move || {
                while bottle < self.bottle_count + 1 {
                    let start = bottle;
                    let key = self.get_bottle_key(start);
                    bottle += 1;
                    while bottle < self.bottle_count + 1 && self.get_bottle_key(bottle) == key {
                        bottle += 1;
                    }
                    if self.get_bottle_locked(start) {
                        return Some((key, (start..=bottle - 1).into()));
                    }
                }

                None
            })
        }
        #[cfg(not(feature = "lock_groups"))]
        {
            std::iter::empty()
        }
    }

    pub fn get_bottle_key(&self, bottle: u8) -> u16 {
        #[cfg(feature = "lock_groups")]
        {
            self.bottle_key.get_n(
                usize::from(bottle) * (BOTTLE_BITS + ITEM_BITS),
                BOTTLE_BITS + ITEM_BITS,
            )
        }
        #[cfg(not(feature = "lock_groups"))]
        {
            let _ = bottle;
            0
        }
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

    pub fn get_bottle_locked(&self, bottle: u8) -> bool {
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

    pub fn get_bottle_color(&self, bottle: u8) -> u8 {
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

    pub fn get_color_curtain(&self, bottle: u8) -> u8 {
        #[cfg(feature = "color_curtains")]
        {
            self.color_curtain
                .get_n(usize::from(bottle) * COLOR_BITS, COLOR_BITS) as u8
        }
        #[cfg(not(feature = "color_curtains"))]
        {
            let _ = bottle;
            0
        }
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

    pub fn get_color_curtain_active(&self, bottle: u8) -> bool {
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
            if from == 0 || to == 0 {
                return Err("bottle range contains zero".into());
            }

            if from >= to {
                return Err("invalid bottle range".into());
            }

            if from > self.bottle_count || to > self.bottle_count + 1 {
                return Err("bottle range too large".into());
            }
        }

        let mut run = Bits::ZERO;
        let mut set = Bits::ZERO;
        let mut current_bit = false;
        let mut index = 1;

        for (from, to) in ranges.iter().copied() {
            let swap_bit = index < from;
            while index < from {
                run.set(usize::from(index), current_bit);
                index += 1;
            }
            if swap_bit {
                current_bit = !current_bit;
            }

            for _ in from..to {
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
    pub(crate) fn bottle_run(
        &self,
        b: Bits<{ storage_bits(BOTTLE_COUNT) }>,
        bottle: u8,
    ) -> (Bits<{ storage_bits(BOTTLE_COUNT) }>, RangeInclusive<u8>) {
        // TODO: Could use some faster bit magic.

        debug_assert_ne!(bottle, 0);
        let mut out = Bits::ZERO;
        let mut start = bottle;
        let mut end = bottle;
        out.set(usize::from(bottle), true);
        let offset_set = b.has(usize::from(bottle));

        for i in (1..bottle).rev() {
            if b.has(usize::from(i)) != offset_set {
                break;
            }
            out.set(usize::from(i), true);
            start = i;
        }

        for i in bottle + 1..self.bottle_count + 1 {
            if b.has(usize::from(i)) != offset_set {
                break;
            }
            out.set(usize::from(i), true);
            end = i;
        }

        (out, (start..=end).into())
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
        if value.capacity.contains(&0) {
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

        let mut state = State::ZERO;
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
            if bottle == 0 {
                return Err("hidden item bottle is zero".into());
            }

            if usize::from(bottle) > value.content.len() {
                return Err("hidden item bottle not in range".into());
            }

            if usize::from(item)
                >= value.content[usize::from(bottle) - 1]
                    .len()
                    .saturating_sub(1)
            {
                return Err("hidden item not in range".into());
            }

            state.set_item_hidden(bottle, item, true);
        }

        #[cfg(feature = "lockable_items")]
        for (bottle, item) in value.locked_items.iter().copied() {
            if bottle == 0 {
                return Err("locked item bottle is zero".into());
            }

            if usize::from(bottle) > value.content.len() {
                return Err("locked item bottle not in range".into());
            }

            if usize::from(item) >= value.content[usize::from(bottle) - 1].len() {
                return Err("locked item not in range".into());
            }

            state.set_item_locked(bottle, item, true);
        }

        #[cfg(feature = "immovable_bottles")]
        for bottle in value.immovable_bottles.iter().copied() {
            if bottle == 0 {
                return Err("immovable bottle is zero".into());
            }

            if usize::from(bottle) > value.content.len() {
                return Err("immovable bottle not in range".into());
            }

            state.set_bottle_immovable(bottle, true);
        }

        #[cfg(feature = "pluggable_bottles")]
        for (bottle, start_plugged) in value.pluggable_bottles.iter().copied() {
            if bottle == 0 {
                return Err("pluggable bottle is zero".into());
            }

            if usize::from(bottle) > value.content.len() {
                return Err("pluggable bottle not in range".into());
            }

            state.set_pluggable_bottle(bottle, true);
            state.set_bottle_plugged(bottle, start_plugged);
        }

        #[cfg(feature = "freezable_bottles")]
        {
            (state.frozen_run, state.frozen) = state.compute_run(&value.frozen_bottle_ranges)?;
        }

        #[cfg(feature = "curtains")]
        {
            (_, state.behind_curtain) = state.compute_run(&value.curtain_ranges)?;
            for (index, (from, to)) in value.curtain_ranges.iter().copied().enumerate() {
                state.set_curtain_range(u8::try_from(index).unwrap(), from, to);
            }
        }

        #[cfg(feature = "safes")]
        for (bottle, counter) in value.safes.iter().copied() {
            if bottle == 0 {
                return Err("safe counter bottle is zero".into());
            }

            if usize::from(bottle) > value.content.len() {
                return Err("safe counter bottle not in range".into());
            }

            if usize::from(counter) > MAX_SAFE_COUNTER {
                return Err("safe counter value not in range".into());
            }

            state.set_safe_counter(bottle, counter);
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
                if bottle == 0 {
                    return Err("lock group key bottle is zero".into());
                }

                if usize::from(bottle) > value.content.len() {
                    return Err("lock group key bottle not in range".into());
                }

                if usize::from(item) >= value.content[usize::from(bottle) - 1].len() {
                    return Err("lock group key item not in range".into());
                }

                if usize::from(item) == value.content[usize::from(bottle) - 1].len() - 1 {
                    return Err("lock group key should not start on top of a bottle".into());
                }

                let color = value.content[usize::from(bottle) - 1][usize::from(item) + 1];
                if state.pour_include(bottle, item, color)
                    && state.pour_include(bottle, item + 1, color)
                {
                    return Err("lock group key must be the first item of a pour".into());
                }

                // TODO: Could check if lock groups contain loops.
                if (from..to).contains(&bottle) {
                    return Err("lock group key stored inside itself".into());
                }

                if state.get_item_has_key(bottle, item) {
                    return Err("lock group key duplicate".into());
                }

                state.set_item_has_key(bottle, item, true);
                let key = to_index(bottle, item);
                for i in from..to {
                    state.set_bottle_key(i, key);
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
            if bottle == 0 {
                return Err("colored bottle is zero".into());
            }

            if usize::from(bottle) > value.content.len() {
                return Err("colored bottle not in range".into());
            }

            if color == 0 || usize::from(color) >= COLOR_COUNT {
                return Err("colored bottle color value not in range".into());
            }

            // TODO: Could check if the color appears at all.
            state.set_bottle_color(bottle, color);
        }

        #[cfg(feature = "color_curtains")]
        for (bottle, color) in value.color_curtains.iter().copied() {
            if bottle == 0 {
                return Err("color curtain bottle is zero".into());
            }

            if usize::from(bottle) > value.content.len() {
                return Err("color curtain bottle not in range".into());
            }

            if color == 0 || usize::from(color) >= COLOR_COUNT {
                return Err("color curtain value not in range".into());
            }

            // TODO:
            // Could check if the color appears at all or if a curtain hides
            // a color from itself.
            state.set_color_curtain(bottle, color);
            state.set_color_curtain_active(bottle, true);
        }

        for bottle in 1..state.bottle_count + 1 {
            if state.compute_bottle_finalized(bottle, 0, 0) {
                return Err("finalized bottles not allowed".into());
            }
        }

        Ok(state)
    }
}

pub fn from_index(index: u16) -> (u8, u8) {
    debug_assert!(usize::from(index) < BOTTLE_COUNT * ITEM_COUNT);
    let bottle = index as usize / ITEM_COUNT;
    let item = index as usize % ITEM_COUNT;
    debug_assert!(u8::try_from(bottle).is_ok());
    debug_assert!(u8::try_from(item).is_ok());
    (bottle as u8, item as u8)
}

pub fn to_index(bottle: u8, item: u8) -> u16 {
    debug_assert!(u16::try_from(ITEM_COUNT).is_ok());
    debug_assert!(usize::from(bottle) < BOTTLE_COUNT);
    debug_assert!(usize::from(item) < ITEM_COUNT);
    u16::from(bottle) * ITEM_COUNT as u16 + u16::from(item)
}

/// Collect all changes to apply a move, which can be undone.
// All optional item indices use the zero value, which is possible because the
// zero bottle index is reserved.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move {
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

impl Move {
    pub(crate) const ZERO: Self = Self {
        from_bottle: 0,
        to_bottle: 0,
        count: 0,
        color: 0,
        finalize_bottle: 0,

        #[cfg(feature = "hidable_items")]
        show_item: 0,
        #[cfg(feature = "lockable_items")]
        unlock_item: 0,
        #[cfg(feature = "pluggable_bottles")]
        unplug_bottle: 0,
        #[cfg(feature = "freezable_bottles")]
        unfreeze_run: Bits::ZERO,
        #[cfg(feature = "curtains")]
        lift_curtain_range: Bits::ZERO,
        #[cfg(feature = "safes")]
        decrement_safe_counter: Bits::ZERO,
        #[cfg(feature = "lock_groups")]
        remove_key: 0,
        #[cfg(feature = "lock_groups")]
        unlock_bottle: Bits::ZERO,
        #[cfg(feature = "color_curtains")]
        lift_color_curtain: Bits::ZERO,
    };

    pub fn from_bottle(&self) -> u8 {
        self.from_bottle
    }

    pub fn to_bottle(&self) -> u8 {
        self.to_bottle
    }

    pub fn count(&self) -> u8 {
        self.count
    }

    pub fn color(&self) -> u8 {
        self.color
    }

    pub fn finalize_bottle(&self) -> Option<u8> {
        if self.finalize_bottle != 0 {
            Some(self.finalize_bottle)
        } else {
            None
        }
    }

    pub fn show_item(&self) -> Option<(u8, u8)> {
        #[cfg(feature = "hidable_items")]
        if self.show_item != 0 {
            Some(from_index(self.show_item))
        } else {
            None
        }
        #[cfg(not(feature = "hidable_items"))]
        {
            None
        }
    }

    pub fn unlock_item(&self) -> Option<(u8, u8)> {
        #[cfg(feature = "lockable_items")]
        if self.unlock_item != 0 {
            Some(from_index(self.unlock_item))
        } else {
            None
        }
        #[cfg(not(feature = "lockable_items"))]
        {
            None
        }
    }

    pub fn unplug_bottle(&self) -> Option<u8> {
        #[cfg(feature = "pluggable_bottles")]
        if self.unplug_bottle != 0 {
            Some(self.unplug_bottle)
        } else {
            None
        }
        #[cfg(not(feature = "pluggable_bottles"))]
        {
            None
        }
    }

    pub fn unfreeze_run(&self) -> Option<RangeInclusive<u8>> {
        #[cfg(feature = "freezable_bottles")]
        if self.unfreeze_run != Bits::ZERO {
            Some(
                (self.unfreeze_run.first().unwrap() as u8
                    ..=self.unfreeze_run.last().unwrap() as u8)
                    .into(),
            )
        } else {
            None
        }
        #[cfg(not(feature = "freezable_bottles"))]
        {
            None
        }
    }

    pub fn lift_curtains(&self) -> bool {
        #[cfg(feature = "curtains")]
        {
            self.lift_curtain_range != Bits::ZERO
        }
        #[cfg(not(feature = "curtains"))]
        {
            false
        }
    }

    pub fn decrement_safe_counters(&self) -> bool {
        #[cfg(feature = "safes")]
        {
            self.decrement_safe_counter != Bits::ZERO
        }
        #[cfg(not(feature = "safes"))]
        {
            false
        }
    }

    pub fn decrement_safe_counter(&self, bottle: u8) -> bool {
        #[cfg(feature = "safes")]
        {
            self.decrement_safe_counter.has(usize::from(bottle))
        }
        #[cfg(not(feature = "safes"))]
        {
            let _ = bottle;
            false
        }
    }

    pub fn remove_key(&self) -> Option<(u8, u8)> {
        #[cfg(feature = "lock_groups")]
        if self.remove_key != 0 {
            Some(from_index(self.remove_key))
        } else {
            None
        }
        #[cfg(not(feature = "lock_groups"))]
        {
            None
        }
    }

    pub fn unlock_run(&self) -> Option<RangeInclusive<u8>> {
        #[cfg(feature = "lock_groups")]
        if self.unlock_bottle != Bits::ZERO {
            Some(
                (self.unlock_bottle.first().unwrap() as u8
                    ..=self.unlock_bottle.last().unwrap() as u8)
                    .into(),
            )
        } else {
            None
        }
        #[cfg(not(feature = "lock_groups"))]
        {
            None
        }
    }

    pub fn lift_color_curtains(&self) -> bool {
        #[cfg(feature = "color_curtains")]
        {
            self.lift_color_curtain != Bits::ZERO
        }
        #[cfg(not(feature = "color_curtains"))]
        {
            false
        }
    }

    pub fn lift_color_curtain(&self, bottle: u8) -> bool {
        #[cfg(feature = "color_curtains")]
        {
            self.lift_color_curtain.has(usize::from(bottle))
        }
        #[cfg(not(feature = "color_curtains"))]
        {
            let _ = bottle;
            false
        }
    }
}

pub struct Pours(Bits<{ storage_bits(BOTTLE_COUNT * BOTTLE_COUNT) }>);

impl Pours {
    fn with(mut self, from: u8, to: u8) -> Self {
        assert_ne!(from, 0);
        assert!(usize::from(from) < BOTTLE_COUNT);
        assert_ne!(to, 0);
        assert!(usize::from(to) < BOTTLE_COUNT);

        self.0
            .set(usize::from(from) * BOTTLE_COUNT + usize::from(to), true);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.0 == Bits::ZERO
    }

    pub fn has(&self, from: u8, to: u8) -> bool {
        assert_ne!(from, 0);
        assert!(usize::from(from) < BOTTLE_COUNT);
        assert_ne!(to, 0);
        assert!(usize::from(to) < BOTTLE_COUNT);

        self.0
            .has(usize::from(from) * BOTTLE_COUNT + usize::from(to))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartingState {
    pub content: Vec<Vec<u8>>,
    pub capacity: Vec<u8>,

    #[cfg(feature = "hidable_items")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_items: Vec<(u8, u8)>,

    #[cfg(feature = "lockable_items")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locked_items: Vec<(u8, u8)>,

    #[cfg(feature = "immovable_bottles")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub immovable_bottles: Vec<u8>,

    #[cfg(feature = "pluggable_bottles")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pluggable_bottles: Vec<(u8, bool)>,

    #[cfg(feature = "freezable_bottles")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frozen_bottle_ranges: Vec<(u8, u8)>,

    #[cfg(feature = "curtains")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub curtain_ranges: Vec<(u8, u8)>,

    #[cfg(feature = "safes")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub safes: Vec<(u8, u8)>,

    #[cfg(feature = "lock_groups")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lock_group_ranges: Vec<(u8, u8)>,

    #[cfg(feature = "lock_groups")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lock_group_keys: Vec<(u8, u8)>,

    #[cfg(feature = "colored_bottles")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colored_bottles: Vec<(u8, u8)>,

    #[cfg(feature = "color_curtains")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub color_curtains: Vec<(u8, u8)>,
}
