/// Same as Base64VecU8 from the NEAR SDK, but for base 58.
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{self, Deserialize, Serialize};

/// Helper class to serialize/deserialize `Vec<u8>` to base58 string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Base58VecU8(#[serde(with = "base58_bytes")] pub Vec<u8>);

impl From<Vec<u8>> for Base58VecU8 {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl From<Base58VecU8> for Vec<u8> {
    fn from(v: Base58VecU8) -> Vec<u8> {
        v.0
    }
}

/// Convenience module to allow anotating a serde structure as base58 bytes.
///
/// # Example
/// ```ignore
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct NewStruct {
///     #[serde(with = "base58_bytes")]
///     field: Vec<u8>,
/// }
/// ```
mod base58_bytes {
    use super::*;
    use near_sdk::bs58;
    use near_sdk::serde::{Deserializer, Serializer};
    use serde::de;

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&bs58::encode(&bytes).into_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        bs58::decode(s.as_str())
            .into_vec()
            .map_err(de::Error::custom)
    }
}

#[macro_export]
macro_rules! impl_base58_serde {
    ($iden: ident) => {
        impl Serialize for $iden {
            fn serialize<S>(
                &self,
                serializer: S,
            ) -> Result<
                <S as near_sdk::serde::Serializer>::Ok,
                <S as near_sdk::serde::Serializer>::Error,
            >
            where
                S: near_sdk::serde::Serializer,
            {
                let wrapped: Base58VecU8 = self.into();
                near_sdk::serde::Serialize::serialize(&wrapped, serializer)
            }
        }

        impl<'de> Deserialize<'de> for $iden {
            fn deserialize<D>(
                deserializer: D,
            ) -> Result<Self, <D as near_sdk::serde::Deserializer<'de>>::Error>
            where
                D: near_sdk::serde::Deserializer<'de>,
            {
                let wrapped: Base58VecU8 = near_sdk::serde::Deserialize::deserialize(deserializer)?;
                Ok(wrapped.into())
            }
        }
    };
}
