use std::convert::TryFrom;
use std::fmt::Display;
use std::ops::Deref;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};

use tonic_sdk_json::{impl_base58_serde, Base58VecU8};

/// Market IDs are sha256 hashes (ie 32 byte arrays)
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Clone, Copy, BorshDeserialize, BorshSerialize)]
pub struct MarketId(pub [u8; 32]);

impl MarketId {
    pub fn new_unchecked(data: &[u8]) -> Self {
        let mut buf: [u8; 32] = Default::default();
        buf.copy_from_slice(&data[..32]);
        Self(buf)
    }
}

impl_base58_serde!(MarketId);

impl TryFrom<&Vec<u8>> for MarketId {
    type Error = ();

    fn try_from(d: &Vec<u8>) -> Result<Self, Self::Error> {
        if d.len() != 32 {
            Err(())
        } else {
            Ok(MarketId::new_unchecked(d))
        }
    }
}

impl Display for MarketId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MarketId<{}>",
            near_sdk::bs58::encode(&self.0).into_string()
        )
    }
}

impl From<Base58VecU8> for MarketId {
    fn from(b: Base58VecU8) -> Self {
        MarketId::try_from(&b.0).expect("malformed market ID")
    }
}

impl From<&Base58VecU8> for MarketId {
    fn from(b: &Base58VecU8) -> Self {
        MarketId::try_from(&b.0).expect("malformed market ID")
    }
}

impl From<MarketId> for Base58VecU8 {
    fn from(m: MarketId) -> Self {
        m.0.to_vec().into()
    }
}

impl From<&MarketId> for Base58VecU8 {
    fn from(m: &MarketId) -> Self {
        m.0.to_vec().into()
    }
}

impl Deref for MarketId {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
