use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

#[cfg(test)]
use proptest_derive::Arbitrary;

#[derive(
    Clone, Copy, Debug, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(test, derive(Arbitrary))]
#[repr(u8)]
pub enum Side {
    Buy,
    Sell,
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buy => write!(f, "buy"),
            Self::Sell => write!(f, "sell"),
        }
    }
}
