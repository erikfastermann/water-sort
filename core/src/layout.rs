#[cfg(any(
    feature = "freezable_bottles",
    feature = "curtains",
    feature = "lock_groups"
))]
use std::range::RangeInclusive;
use std::{error::Error, range::Range};

use crate::{bits::Bits, state::State};

#[derive(Clone, Copy)]
pub struct Layout([[u8; Self::REPR_LINE_LEN as usize]; Self::LINES as usize]);

impl Layout {
    pub const LINES: u8 = 3;

    pub const COLUMNS: u8 = 6;

    const TOTAL: u8 = Self::LINES * Self::COLUMNS;

    const SEPARATOR: u8 = b'|';

    const CHARS: [u8; Self::TOTAL as usize] = *b"123456789ABCDEFGHI";

    pub const MAX_CAPACITY: [u8; Self::LINES as usize] = [4, 10, 16];

    pub const REPR_LINE_LEN: u8 = 2 * Self::COLUMNS - 1;

    const REPR_UNUSED_MARKER: u8 = 0;

    pub(crate) fn new(data: &[impl AsRef<str>], state: &State) -> Result<Layout, Box<dyn Error>> {
        if data.is_empty() || data.len() > usize::from(Self::LINES) {
            return Err("no or too many lines".into());
        }

        for line in data {
            let line = line.as_ref();

            if !line.is_ascii() {
                return Err("line data is not only ascii".into());
            }

            if line.len() != usize::from(Self::REPR_LINE_LEN) {
                return Err("bad line data length".into());
            }

            if line.as_bytes().contains(&Self::REPR_UNUSED_MARKER) {
                return Err("input line contains unused marker".into());
            }
        }

        let mut out = Layout(
            [[Self::REPR_UNUSED_MARKER; Self::REPR_LINE_LEN as usize]; Self::LINES as usize],
        );
        for (index, line) in data.iter().enumerate() {
            out.0[index].copy_from_slice(line.as_ref().as_bytes());
        }

        out.validate(state)?;
        Ok(out)
    }

    fn validate(&self, state: &State) -> Result<(), Box<dyn Error>> {
        for line in self.lines() {
            Self::validate_line(line)?;
        }

        let used_chars = self
            .lines()
            .flatten()
            .filter(|ch| *ch != Self::SEPARATOR)
            .fold(Bits::<256>::ZERO, |mut acc, ch| {
                acc.set(usize::from(ch), true);
                acc
            });
        if used_chars.count() != u32::from(state.bottle_count) {
            return Err("used chars does not match bottle count".into());
        }

        for bottle in 1..state.bottle_count + 1 {
            self.validate_bottle(state, bottle)?;
        }

        #[cfg(feature = "freezable_bottles")]
        self.validate_ranges(state, state.get_frozen_ranges())?;

        #[cfg(feature = "curtains")]
        self.validate_ranges(state, state.get_curtain_ranges())?;

        #[cfg(feature = "lock_groups")]
        self.validate_ranges(state, state.get_lock_group_ranges().map(|(_, range)| range))?;

        Ok(())
    }

    fn validate_line(line: [u8; Self::REPR_LINE_LEN as usize]) -> Result<(), Box<dyn Error>> {
        if !line
            .into_iter()
            .all(|ch| ch == Self::SEPARATOR || Self::CHARS.contains(&ch))
        {
            return Err("bad line chars".into());
        }

        if line.into_iter().all(|ch| ch == Self::SEPARATOR) {
            return Err("line only consists of separators".into());
        }

        let prefix_separators = line
            .into_iter()
            .take_while(|ch| *ch == Self::SEPARATOR)
            .count();
        let suffix_separators = line
            .into_iter()
            .rev()
            .take_while(|ch| *ch == Self::SEPARATOR)
            .count();
        if prefix_separators != suffix_separators {
            return Err("number of prefix and suffix separators should match".into());
        }

        if !line[prefix_separators..line.len() - suffix_separators]
            .iter()
            .copied()
            .enumerate()
            .all(|(i, ch)| {
                if i % 2 == 0 {
                    Self::CHARS.contains(&ch)
                } else {
                    ch == Self::SEPARATOR
                }
            })
        {
            return Err("not alternating separator and index".into());
        }

        Ok(())
    }

    fn validate_bottle(&self, state: &State, bottle: u8) -> Result<(), Box<dyn Error>> {
        let ch = Self::bottle_to_char(bottle).unwrap();
        let capacity = state.get_capacity(bottle);

        let line_count = Self::MAX_CAPACITY
            .iter()
            .copied()
            .position(|max_capacity| capacity <= max_capacity)
            .map(|n| n + 1);
        let Some(line_count) = line_count else {
            return Err("bottle capacity too large".into());
        };

        let line_count_got = self
            .lines()
            .flatten()
            .filter(|current_ch| *current_ch == ch)
            .count();
        if line_count_got != line_count {
            return Err("bottle char count does not match expected line count".into());
        }

        let (first_line, column) = self.first_position(ch).unwrap();

        let line_count_stacked = self.0[usize::from(first_line)..]
            .iter()
            .take_while(|line| line[usize::from(column)] == ch)
            .count();
        if line_count_stacked != line_count {
            return Err("bottle chars are not properly stacked".into());
        }

        Ok(())
    }

    #[cfg(any(
        feature = "freezable_bottles",
        feature = "curtains",
        feature = "lock_groups"
    ))]
    fn validate_ranges(
        &self,
        state: &State,
        iter: impl IntoIterator<Item = RangeInclusive<u8>>,
    ) -> Result<(), Box<dyn Error>> {
        for range in iter {
            let start_ch = Self::bottle_to_char(range.start).unwrap();
            let (line, column) = self.first_position(start_ch).unwrap();

            for (offset, bottle) in range.into_iter().enumerate() {
                if state.get_capacity(bottle) > Self::MAX_CAPACITY[0] {
                    return Err(
                        "freezable, curtain, or lock group bottle cannot span multiple rows".into(),
                    );
                }

                let expected_bottle_ch = Self::bottle_to_char(bottle).unwrap();
                let got_bottle_ch = self.0[usize::from(line)]
                    .get(usize::from(column) + 2 * offset)
                    .copied();
                if got_bottle_ch != Some(expected_bottle_ch) {
                    return Err(
                        "freezable, curtain, or lock group not in single line ascending without gaps".into(),
                    );
                }
            }
        }

        Ok(())
    }

    pub fn line_count(&self) -> u8 {
        self.lines().count() as u8
    }

    pub fn bottle_position(&self, bottle: u8) -> Option<(Range<u8>, u8)> {
        let ch = Self::bottle_to_char(bottle)?;
        let (line, column) = self.first_position(ch)?;
        let count = self.0[usize::from(line)..]
            .iter()
            .take_while(|c| c[usize::from(column)] == ch)
            .count();
        Some(((line..line + count as u8).into(), column))
    }

    fn bottle_to_char(bottle: u8) -> Option<u8> {
        usize::from(bottle)
            .checked_sub(1)
            .and_then(|i| Self::CHARS.get(i).copied())
    }

    fn lines(&self) -> impl Iterator<Item = [u8; Self::REPR_LINE_LEN as usize]> {
        self.0
            .into_iter()
            .take_while(|line| line[0] != Self::REPR_UNUSED_MARKER)
    }

    fn first_position(&self, ch: u8) -> Option<(u8, u8)> {
        self.lines()
            .enumerate()
            .flat_map(|(l, line)| {
                line.into_iter()
                    .enumerate()
                    .map(move |(c, current_ch)| (l, c, current_ch))
            })
            .find(|(_, _, current_ch)| *current_ch == ch)
            .map(|(l, c, _)| (l as u8, c as u8))
    }
}
