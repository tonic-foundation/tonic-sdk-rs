use std::{iter::Sum, ops::Add};

use near_sdk::Balance;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
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

impl Sum for Tvl {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(
            Tvl {
                base_locked: 0,
                quote_locked: 0,
            },
            |acc, curr| acc + curr,
        )
    }
}
