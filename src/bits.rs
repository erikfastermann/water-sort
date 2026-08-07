#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Bits<const BITS: usize>
where
    StorageMapping<BITS>: StorageMapper,
{
    v: <StorageMapping<BITS> as StorageMapper>::Storage,
}

impl<const BITS: usize> Bits<BITS>
where
    StorageMapping<BITS>: StorageMapper,
{
    pub const ZERO: Self = Self {
        v: <StorageMapping<BITS> as StorageMapper>::Storage::ZERO,
    };

    pub fn has(self, index: usize) -> bool {
        self.v.has(index)
    }
}

pub trait StorageMapper {
    type Storage: Storage;
}

pub struct StorageMapping<const BITS: usize>;

impl StorageMapper for StorageMapping<32> {
    type Storage = [u32; 1];
}

impl StorageMapper for StorageMapping<128> {
    type Storage = [u32; 4];
}

impl StorageMapper for StorageMapping<512> {
    type Storage = [u32; 16];
}

impl StorageMapper for StorageMapping<2048> {
    type Storage = [u32; 64];
}

trait Storage: Clone + Copy + PartialEq + Eq {
    const ZERO: Self;

    fn has(self, index: usize) -> bool;
    fn set(&mut self, index: usize, value: bool);
}

impl<const N: usize> Storage for [u32; N] {
    const ZERO: Self = [0; N];

    fn has(self, index: usize) -> bool {
        debug_assert!(N.is_power_of_two() && index < N * 32);
        self[(index / 32) & (N - 1)] & (1 << (index % 32)) != 0
    }

    fn set(&mut self, index: usize, value: bool) {
        debug_assert!(N.is_power_of_two() && index < N * 32);
        self[(index / 32) & (N - 1)] |= u32::from(value) << (index % 32);
    }
}
