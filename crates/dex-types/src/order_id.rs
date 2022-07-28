/// Implements Order IDs
use std::convert::TryInto;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use tonic_sdk_json::{impl_base58_serde, Base58VecU8};

use crate::*;

/// An order ID that includes the order direction, price, and a sequence number.
///
/// [ Side | Sequence number | Price in lots ]
///    |     63 bits           64 bits
///    1 bit
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Clone, Copy, BorshDeserialize, BorshSerialize)]
pub struct OrderId(u128);

impl OrderId {
    /// Order ID into side, sequence number, and price
    pub fn into_parts(self) -> (Side, u64, u64) {
        get_order_id_parts(self)
    }
}

impl_base58_serde!(OrderId);

impl From<OrderId> for Base58VecU8 {
    fn from(oid: OrderId) -> Self {
        oid.0.to_be_bytes().to_vec().into()
    }
}

impl From<&OrderId> for Base58VecU8 {
    fn from(oid: &OrderId) -> Self {
        oid.0.to_be_bytes().to_vec().into()
    }
}

impl From<Base58VecU8> for OrderId {
    fn from(bytes: Base58VecU8) -> Self {
        OrderId(u128::from_be_bytes(bytes.0.try_into().unwrap()))
    }
}

const SEQUENCE_MASK: u128 = !(1_u128 << 127);

pub fn new_order_id(side: Side, price: u64, sequence_number: u64) -> OrderId {
    let side_part = match side {
        Side::Buy => (1u128) << 127,
        Side::Sell => 0,
    };
    let sequence_part = SEQUENCE_MASK & (sequence_number as u128) << 64; // clear the top bit
    let price_part = price as u128;

    OrderId(side_part | sequence_part | price_part)
}

pub fn get_order_id_parts(oid: OrderId) -> (Side, u64, u64) {
    let side_part = oid.0 >> 127;
    let price_part = oid.0 as u64;
    let sequence_part = (SEQUENCE_MASK & (oid.0)) >> 64; // clear the top bit

    let side = if side_part == 1 {
        Side::Buy
    } else {
        Side::Sell
    };

    (side, price_part, sequence_part as u64)
}

#[cfg(test)]
mod test {
    use super::*;

    use proptest::prelude::*;

    const SEQUENCE_NUMBER_MAX: u64 = std::u64::MAX / 2;

    proptest! {
        #[test]
        fn test_order_id(side: Side, price in 1..std::u64::MAX, sequence_number in 1..SEQUENCE_NUMBER_MAX) {
            let order_id = new_order_id(side, price, sequence_number);
            let (s, p, sn) = get_order_id_parts(order_id);

            assert_eq!(side, s, "Wrong side");
            assert_eq!(price, p, "Wrong price");
            assert_eq!(sequence_number, sn, "Wrong sequence number");
        }
    }

    #[test]
    fn test_order_id_round_trip_buy() {
        let side = Side::Buy;
        let price = 456u64;
        let sequence_number = 123;

        let order_id = new_order_id(side, price, sequence_number);
        let (s, p, sn) = get_order_id_parts(order_id);

        assert_eq!(side, s, "Wrong side");
        assert_eq!(price, p, "Wrong price");
        assert_eq!(sequence_number, sn, "Wrong sequence number");
    }

    #[test]
    fn test_order_id_round_trip_sell() {
        let side = Side::Sell;
        let price = 456u64;
        let sequence_number = 123;

        let order_id = new_order_id(side, price, sequence_number);
        let (s, p, sn) = get_order_id_parts(order_id);

        assert_eq!(side, s, "Wrong side");
        assert_eq!(price, p, "Wrong price");
        assert_eq!(sequence_number, sn, "Wrong sequence number");
    }
}
