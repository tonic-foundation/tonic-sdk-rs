use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    Balance,
};
use tonic_sdk_dex_types::{LotBalance, SequenceNumber};

use crate::*;

pub trait L2: BorshDeserialize + BorshSerialize + OrderIter + TakeL2Depth {
    /// The order with the greatest price.
    fn max_order(&self) -> Option<OpenLimitOrder>;

    /// The order with the least price.
    fn min_order(&self) -> Option<OpenLimitOrder>;

    /// Save an order.
    fn save_order(&mut self, order: OpenLimitOrder);

    fn get_order(
        &self,
        price_lots: LotBalance,
        sequence_number: SequenceNumber,
    ) -> Option<OpenLimitOrder>;

    fn get_price_rank(&self, price_lots: LotBalance) -> u32;

    fn delete_order(
        &mut self,
        price_lots: LotBalance,
        seq: SequenceNumber,
    ) -> Option<OpenLimitOrder>;

    fn is_empty(&self) -> bool;
}

/// Trait for structs that can iterate over orders.
pub trait OrderIter {
    // hack for lifetimes: shouldn't need to return a Box but doesn't really
    // cost anything
    fn iter(&self) -> Box<dyn Iterator<Item = OpenLimitOrder> + '_>;
}

/// Trait for structs that can produce a vector of (price, [orders at that price]).
///
/// Used to make [crate::OrderbookView].
pub trait TakeL2Depth {
    fn take_depth(&self, depth: usize) -> Vec<(LotBalance, Vec<OpenLimitOrder>)>;
}

impl<T> TakeL2Depth for T
where
    T: OrderIter,
{
    fn take_depth(&self, depth: usize) -> Vec<(LotBalance, Vec<OpenLimitOrder>)> {
        let mut ret: Vec<(LotBalance, Vec<OpenLimitOrder>)> = vec![];

        let mut curr_acc: Vec<OpenLimitOrder> = vec![];
        let mut curr_price: Option<LotBalance> = None;

        for order in self.iter() {
            if ret.len() >= depth {
                break;
            }
            if curr_price.is_none() {
                curr_price = Some(order.unwrap_price());
            }
            if curr_price.unwrap() != order.unwrap_price() {
                ret.push((curr_price.unwrap(), curr_acc.clone()));
                curr_price = Some(order.unwrap_price());
                curr_acc = vec![];
            }
            curr_acc.push(order);
        }

        // base case: orderbook finished iterating but all orders had same price
        if !curr_acc.is_empty() {
            ret.push((curr_price.unwrap(), curr_acc.clone()));
        }

        ret
    }
}

/// Trait for structs that represent ownership of base and/or quote tokens.
pub trait ValueLocked {
    fn value_locked(
        &self,
        base_lot_size: Balance,
        quote_lot_size: Balance,
        base_denomination: Balance,
    ) -> Tvl;
}

impl<T> ValueLocked for T
where
    T: OrderIter,
{
    fn value_locked(
        &self,
        base_lot_size: Balance,
        quote_lot_size: Balance,
        base_denomination: Balance,
    ) -> Tvl {
        self.iter()
            .map(|o| o.value_locked(base_lot_size, quote_lot_size, base_denomination))
            .sum()
    }
}
