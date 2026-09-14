#![allow(dead_code)]

use std::{
    fmt::Debug,
    hash::Hash,
    ops::{BitAnd, BitAndAssign, BitOrAssign, Not},
};

// TODO: Proper Debug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    const BITS: usize = <StorageMapping<BITS> as StorageMapper>::Storage::BITS;

    pub const ZERO: Self = Self {
        v: <StorageMapping<BITS> as StorageMapper>::Storage::ZERO,
    };

    pub fn has(&self, index: usize) -> bool {
        self.v.has(index)
    }

    pub fn set(&mut self, index: usize, value: bool) {
        self.v.set(index, value);
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

    pub fn copy_from<const SRC_BITS: usize>(
        &mut self,
        src: &Bits<SRC_BITS>,
        offset: usize,
        bits: usize,
    ) where
        StorageMapping<SRC_BITS>: StorageMapper,
    {
        // TODO: More efficient implementation.

        debug_assert!(
            offset
                .checked_add(bits)
                .is_some_and(|n| n <= Bits::<SRC_BITS>::BITS)
        );
        debug_assert!(bits <= Self::BITS);

        for index in 0..bits {
            self.v.set(index, src.v.has(offset + index));
        }
    }

    pub fn get_u4(&self, index: usize) -> u8 {
        self.v.get_u4(index)
    }

    pub fn set_u4(&mut self, index: usize, value: u8) {
        self.v.set_u4(index, value)
    }

    pub fn get_u8(&self, index: usize) -> u8 {
        self.v.get_u8(index)
    }

    pub fn set_u8(&mut self, index: usize, value: u8) {
        self.v.set_u8(index, value)
    }

    pub fn get_u16(&self, index: usize) -> u16 {
        self.v.get_u16(index)
    }

    pub fn set_u16(&mut self, index: usize, value: u16) {
        self.v.set_u16(index, value)
    }

    pub fn get_u32(&self, index: usize) -> u32 {
        self.v.get_u32(index)
    }

    pub fn set_u32(&mut self, index: usize, value: u32) {
        self.v.set_u32(index, value)
    }

    pub fn get_u64(&self, index: usize) -> u64 {
        self.v.get_u64(index)
    }

    pub fn set_u64(&mut self, index: usize, value: u64) {
        self.v.set_u64(index, value)
    }
}

pub trait StorageMapper {
    type Storage: Storage;
}

pub struct StorageMapping<const BITS: usize>;

impl StorageMapper for StorageMapping<16> {
    type Storage = [u16; 1];
}

impl StorageMapper for StorageMapping<32> {
    type Storage = [u16; 2];
}

impl StorageMapper for StorageMapping<64> {
    type Storage = [u16; 4];
}

impl StorageMapper for StorageMapping<128> {
    type Storage = [u16; 8];
}

impl StorageMapper for StorageMapping<256> {
    type Storage = [u16; 16];
}

impl StorageMapper for StorageMapping<512> {
    type Storage = [u16; 32];
}

impl StorageMapper for StorageMapping<1024> {
    type Storage = [u16; 64];
}

impl StorageMapper for StorageMapping<2048> {
    type Storage = [u16; 128];
}

impl StorageMapper for StorageMapping<4096> {
    type Storage = [u16; 256];
}

pub trait Storage: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord {
    const BITS: usize;
    const ZERO: Self;

    fn has(&self, index: usize) -> bool;
    fn set(&mut self, index: usize, value: bool);
    fn count(&self) -> u32;
    fn first(&self) -> Option<usize>;
    fn last(&self) -> Option<usize>;

    fn get_u4(&self, index: usize) -> u8;
    fn set_u4(&mut self, index: usize, value: u8);
    fn get_u8(&self, index: usize) -> u8;
    fn set_u8(&mut self, index: usize, value: u8);
    fn get_u16(&self, index: usize) -> u16;
    fn set_u16(&mut self, index: usize, value: u16);
    fn get_u32(&self, index: usize) -> u32;
    fn set_u32(&mut self, index: usize, value: u32);
    fn get_u64(&self, index: usize) -> u64;
    fn set_u64(&mut self, index: usize, value: u64);

    fn bitand(self, rhs: Self) -> Self;
    fn bitand_assign(&mut self, rhs: Self);
    fn bitor_assign(&mut self, rhs: Self);
    fn not(self) -> Self;
}

/// Using this in release builds with out-of-bounds indices will silently write
/// to other fields, not unsafe, just wrong. This is done as a performance
/// optimization. It is fine to construct this with N == 0, but not to use it.
impl<const N: usize> Storage for [u16; N] {
    const BITS: usize = N * 16;
    const ZERO: Self = [0; N];

    fn has(&self, index: usize) -> bool {
        assert_ne!(N, 0);
        assert!(N.is_power_of_two());
        debug_assert!(index < N * 16);
        self[(index / 16) & (N - 1)] & (1 << (index % 16)) != 0
    }

    fn set(&mut self, index: usize, value: bool) {
        assert_ne!(N, 0);
        assert!(N.is_power_of_two());
        debug_assert!(index < N * 16);
        let slot = &mut self[(index / 16) & (N - 1)];
        *slot &= !(1 << (index % 16));
        *slot |= u16::from(value) << (index % 16);
    }

    fn count(&self) -> u32 {
        self.iter().map(|n| n.count_ones()).sum()
    }

    fn first(&self) -> Option<usize> {
        // TODO: More efficient implementation.
        (0..N * 16).find(|x| self.has(*x))
    }

    fn last(&self) -> Option<usize> {
        // TODO: More efficient implementation.
        (0..N * 16).rev().find(|x| self.has(*x))
    }

    fn get_u4(&self, index: usize) -> u8 {
        assert!(N.is_power_of_two());
        debug_assert!(
            index
                .checked_mul(4)
                .and_then(|i| i.checked_add(4))
                .is_some_and(|n| n <= N * 16)
        );

        let slot = self[(index / 4) & (N - 1)];
        (slot >> ((index % 4) * 4) & 0xf) as u8
    }

    fn set_u4(&mut self, index: usize, value: u8) {
        assert!(N.is_power_of_two());
        debug_assert!(
            index
                .checked_mul(4)
                .and_then(|i| i.checked_add(4))
                .is_some_and(|n| n <= N * 16)
        );
        debug_assert_eq!(value & 0xf, value);

        let slot = &mut self[(index / 4) & (N - 1)];
        *slot &= !(0xf << ((index % 4) * 4));
        *slot |= u16::from(value) << ((index % 4) * 4);
    }

    fn get_u8(&self, index: usize) -> u8 {
        assert!(N.is_power_of_two());
        debug_assert!(
            index
                .checked_mul(8)
                .and_then(|i| i.checked_add(8))
                .is_some_and(|n| n <= N * 16)
        );

        let slot = self[(index / 2) & (N - 1)];
        (slot >> ((index % 2) * 8)) as u8
    }

    fn set_u8(&mut self, index: usize, value: u8) {
        assert!(N.is_power_of_two());
        debug_assert!(
            index
                .checked_mul(8)
                .and_then(|i| i.checked_add(8))
                .is_some_and(|n| n <= N * 16)
        );

        let slot = &mut self[(index / 2) & (N - 1)];
        *slot &= !(0xff << ((index % 2) * 8));
        *slot |= u16::from(value) << ((index % 2) * 8);
    }

    fn get_u16(&self, index: usize) -> u16 {
        assert!(N.is_power_of_two());
        debug_assert!(
            index
                .checked_mul(16)
                .and_then(|i| i.checked_add(16))
                .is_some_and(|n| n <= N * 16)
        );

        self[index & (N - 1)]
    }

    fn set_u16(&mut self, index: usize, value: u16) {
        assert!(N.is_power_of_two());
        debug_assert!(
            index
                .checked_mul(16)
                .and_then(|i| i.checked_add(16))
                .is_some_and(|n| n <= N * 16)
        );

        self[index & (N - 1)] = value;
    }

    fn get_u32(&self, index: usize) -> u32 {
        assert!(N.is_power_of_two());
        debug_assert!(
            index
                .checked_mul(32)
                .and_then(|i| i.checked_add(32))
                .is_some_and(|n| n <= N * 16)
        );

        u32::from(self[(index * 2 + 1) & (N - 1)]) << 16 | u32::from(self[(index * 2) & (N - 1)])
    }

    fn set_u32(&mut self, index: usize, value: u32) {
        assert!(N.is_power_of_two());
        debug_assert!(
            index
                .checked_mul(32)
                .and_then(|i| i.checked_add(32))
                .is_some_and(|n| n <= N * 16)
        );

        self[(index * 2 + 1) & (N - 1)] = (value >> 16) as u16;
        self[(index * 2) & (N - 1)] = value as u16;
    }

    fn get_u64(&self, index: usize) -> u64 {
        assert!(N.is_power_of_two());
        debug_assert!(
            index
                .checked_mul(64)
                .and_then(|i| i.checked_add(64))
                .is_some_and(|n| n <= N * 16)
        );

        u64::from(self[(index * 4 + 3) & (N - 1)]) << 48
            | u64::from(self[(index * 4 + 2) & (N - 1)]) << 32
            | u64::from(self[(index * 4 + 1) & (N - 1)]) << 16
            | u64::from(self[(index * 4) & (N - 1)])
    }

    fn set_u64(&mut self, index: usize, value: u64) {
        assert!(N.is_power_of_two());
        debug_assert!(
            index
                .checked_mul(64)
                .and_then(|i| i.checked_add(64))
                .is_some_and(|n| n <= N * 16)
        );

        self[(index * 4 + 3) & (N - 1)] = (value >> 48) as u16;
        self[(index * 4 + 2) & (N - 1)] = (value >> 32) as u16;
        self[(index * 4 + 1) & (N - 1)] = (value >> 16) as u16;
        self[(index * 4) & (N - 1)] = value as u16;
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
