pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    const MULT: u64 = 6364136223846793005;

    pub fn new(seed: u64, seq: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: (seq << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(Self::MULT).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1 << 24) as f32
    }

    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + self.next_f32() * (high - low)
    }

    pub fn below(&mut self, limit: u32) -> u32 {
        let threshold = limit.wrapping_neg() % limit;
        loop {
            let value = self.next_u32();
            if value >= threshold {
                return value % limit;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Pcg32;

    #[test]
    fn deterministic() {
        let a: Vec<u32> = (0..8).map(|_| Pcg32::new(7, 1).next_u32()).collect();
        assert!(a.windows(2).all(|w| w[0] == w[1]));
    }

    #[test]
    fn floats_in_unit_range() {
        let mut rng = Pcg32::new(42, 3);
        for _ in 0..10_000 {
            let value = rng.next_f32();
            assert!((0.0..1.0).contains(&value));
        }
    }

    #[test]
    fn below_is_bounded() {
        let mut rng = Pcg32::new(1, 1);
        for _ in 0..10_000 {
            assert!(rng.below(7) < 7);
        }
    }
}
