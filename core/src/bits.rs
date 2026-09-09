#![allow(dead_code)]

use std::{
    fmt::Debug,
    hash::Hash,
    ops::{BitAnd, BitAndAssign, BitOrAssign, Not},
};

// TODO: Proper Debug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bits<const BITS: usize>
where
    StorageMapping<BITS>: StorageMapper,
{
    v: <StorageMapping<BITS> as StorageMapper>::Storage,
}

impl<const BITS: usize> Default for Bits<BITS>
where
    StorageMapping<BITS>: StorageMapper,
{
    fn default() -> Self {
        Self { v: Storage::ZERO }
    }
}

impl<const BITS: usize> BitAnd for Bits<BITS>
where
    StorageMapping<BITS>: StorageMapper,
{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            v: self.v.bitand(rhs.v),
        }
    }
}

impl<const BITS: usize> BitAndAssign for Bits<BITS>
where
    StorageMapping<BITS>: StorageMapper,
{
    fn bitand_assign(&mut self, rhs: Self) {
        self.v.bitand_assign(rhs.v);
    }
}

impl<const BITS: usize> BitOrAssign for Bits<BITS>
where
    StorageMapping<BITS>: StorageMapper,
{
    fn bitor_assign(&mut self, rhs: Self) {
        self.v.bitor_assign(rhs.v);
    }
}

impl<const BITS: usize> Not for Bits<BITS>
where
    StorageMapping<BITS>: StorageMapper,
{
    type Output = Self;

    fn not(self) -> Self::Output {
        Self { v: self.v.not() }
    }
}

impl<const BITS: usize> Bits<BITS>
where
    StorageMapping<BITS>: StorageMapper,
{
    pub const ZERO: Self = Self {
        v: <StorageMapping<BITS> as StorageMapper>::Storage::ZERO,
    };

    pub fn has(&self, index: usize) -> bool {
        self.v.has(index)
    }

    pub fn set(&mut self, index: usize, value: bool) {
        self.v.set(index, value);
    }

    pub fn get_n(&self, offset: usize, bits: usize) -> u16 {
        self.v.get_n(offset, bits)
    }

    pub fn set_n(&mut self, offset: usize, bits: usize, value: u16) {
        self.v.set_n(offset, bits, value);
    }

    pub fn count(&self) -> u32 {
        self.v.count()
    }

    pub fn first(&self) -> Option<usize> {
        self.v.first()
    }

    pub fn last(&self) -> Option<usize> {
        self.v.last()
    }
}

pub trait StorageMapper {
    type Storage: Storage;
}

pub struct StorageMapping<const BITS: usize>;

impl StorageMapper for StorageMapping<32> {
    type Storage = [u32; 1];
}

impl StorageMapper for StorageMapping<64> {
    type Storage = [u32; 2];
}

impl StorageMapper for StorageMapping<128> {
    type Storage = [u32; 4];
}

impl StorageMapper for StorageMapping<256> {
    type Storage = [u32; 8];
}

impl StorageMapper for StorageMapping<512> {
    type Storage = [u32; 16];
}

impl StorageMapper for StorageMapping<1024> {
    type Storage = [u32; 32];
}

impl StorageMapper for StorageMapping<2048> {
    type Storage = [u32; 64];
}

impl StorageMapper for StorageMapping<4096> {
    type Storage = [u32; 128];
}

pub trait Storage: Debug + Clone + Copy + PartialEq + Eq + Hash {
    const ZERO: Self;

    fn has(&self, index: usize) -> bool;
    fn set(&mut self, index: usize, value: bool);
    fn get_n(&self, offset: usize, bits: usize) -> u16;
    fn set_n(&mut self, offset: usize, bits: usize, value: u16);
    fn count(&self) -> u32;
    fn first(&self) -> Option<usize>;
    fn last(&self) -> Option<usize>;

    fn bitand(self, rhs: Self) -> Self;
    fn bitand_assign(&mut self, rhs: Self);
    fn bitor_assign(&mut self, rhs: Self);
    fn not(self) -> Self;
}

/// Using this in release builds with out-of-bounds indices will silently write
/// to other fields, not unsafe, just wrong. This is done as a performance
/// optimization. It is fine to construct this with N == 0, but not to use it.
impl<const N: usize> Storage for [u32; N] {
    const ZERO: Self = [0; N];

    fn has(&self, index: usize) -> bool {
        assert_ne!(N, 0);
        assert!(N.is_power_of_two());
        debug_assert!(index < N * 32);
        self[(index / 32) & (N - 1)] & (1 << (index % 32)) != 0
    }

    fn set(&mut self, index: usize, value: bool) {
        assert_ne!(N, 0);
        assert!(N.is_power_of_two());
        debug_assert!(index < N * 32);
        let slot = &mut self[(index / 32) & (N - 1)];
        *slot &= !(1 << (index % 32));
        *slot |= u32::from(value) << (index % 32);
    }

    fn get_n(&self, offset: usize, bits: usize) -> u16 {
        // TODO: More efficient implementation.

        assert!(bits >= 1);
        assert!(bits <= 16);
        debug_assert!(offset.checked_add(bits - 1).is_some_and(|n| n < N * 32));

        let mut v = 0u16;
        for i in 0..bits {
            let x = self.has(offset + i);
            v <<= 1;
            v |= u16::from(x);
        }

        v
    }

    fn set_n(&mut self, offset: usize, bits: usize, value: u16) {
        // TODO: More efficient implementation.

        assert!(bits >= 1);
        assert!(bits <= 16);
        debug_assert!(offset.checked_add(bits - 1).is_some_and(|n| n < N * 32));
        debug_assert!(value <= u16::try_from((1 << bits) - 1).unwrap());

        let mut v = value;
        for i in (0..bits).rev() {
            self.set(offset + i, v & 1 != 0);
            v >>= 1;
        }
    }

    fn count(&self) -> u32 {
        self.iter().map(|n| n.count_ones()).sum()
    }

    fn first(&self) -> Option<usize> {
        // TODO: More efficient implementation.
        (0..N * 32).find(|x| self.has(*x))
    }

    fn last(&self) -> Option<usize> {
        // TODO: More efficient implementation.
        (0..N * 32).rev().find(|x| self.has(*x))
    }

    fn bitand(mut self, rhs: Self) -> Self {
        for (a, b) in self.iter_mut().zip(rhs) {
            *a &= b;
        }
        self
    }

    fn bitand_assign(&mut self, rhs: Self) {
        for (a, b) in self.iter_mut().zip(rhs) {
            *a &= b;
        }
    }

    fn bitor_assign(&mut self, rhs: Self) {
        for (a, b) in self.iter_mut().zip(rhs) {
            *a |= b;
        }
    }

    fn not(mut self) -> Self {
        for a in self.iter_mut() {
            *a = !(*a);
        }
        self
    }
}
