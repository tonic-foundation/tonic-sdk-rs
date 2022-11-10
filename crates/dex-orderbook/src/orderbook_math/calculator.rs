use near_sdk::Balance;
use tonic_sdk_dex_types::{LotBalance, U256};

use crate::*;

/// Struct for doing math in the orderbook.
pub struct OrderbookCalculator {
    pub base_lot_size: Balance,
    pub quote_lot_size: Balance,
    pub base_denomination: Balance,
}

impl OrderbookCalculator {
    /// Get the value of a bid in terms of native quote token.
    pub fn get_bid_quote_value(&self, quantity: LotBalance, price: LotBalance) -> Balance {
        get_bid_quote_value(
            quantity,
            price,
            self.base_lot_size,
            self.quote_lot_size,
            self.base_denomination,
        )
    }

    /// Get quantity of base that a given amount of quote is worth in terms of base lots
    pub fn get_base_purchasable(&self, quote_amount: LotBalance, price: LotBalance) -> LotBalance {
        get_base_purchasable(
            quote_amount,
            price,
            self.base_lot_size,
            self.base_denomination,
        )
    }
}

/// Get the value of a bid in terms of native quote token.
pub fn get_bid_quote_value(
    quantity: LotBalance,
    price: LotBalance,
    base_lot_size: Balance,
    quote_lot_size: Balance,
    base_denomination: Balance,
) -> Balance {
    BN!(quantity)
        .mul(base_lot_size)
        .mul(price as u128)
        .mul(quote_lot_size)
        .div(base_denomination)
        .as_u128()
}

/// Get quantity of base that a given amount of quote is worth in terms of base lots
pub fn get_base_purchasable(
    quote_amount: LotBalance,
    price: LotBalance,
    base_lot_size: Balance,
    base_denomination: Balance,
) -> LotBalance {
    BN!(quote_amount)
        .mul(base_denomination)
        .div(price as u128)
        .div(base_lot_size)
        .as_u64()
}
