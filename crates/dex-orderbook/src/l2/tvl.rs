use std::ops::Add;

use near_sdk::Balance;

pub struct Tvl {
    pub base_locked: Balance,
    pub quote_locked: Balance,
}

impl Add for Tvl {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            base_locked: self.base_locked + rhs.base_locked,
            quote_locked: self.quote_locked + rhs.quote_locked,
        }
    }
}
