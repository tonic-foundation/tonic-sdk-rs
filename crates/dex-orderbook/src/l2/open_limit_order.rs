use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    AccountId, Balance,
};
use tonic_sdk_dex_types::{new_order_id, LotBalance, OrderId, SequenceNumber, Side, U256};
use tonic_sdk_macros::*;

#[cfg(feature = "fuzz")]
use near_sdk::serde::{self, Deserialize};

use crate::*;

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(
    feature = "fuzz",
    derive(Serialize, Deserialize),
    serde(crate = "near_sdk::serde")
)]
pub struct OpenLimitOrder {
    pub sequence_number: SequenceNumber,
    pub owner_id: AccountId,
    pub open_qty_lots: LotBalance,
    pub client_id: Option<ClientId>,

    /// Limit price (price per one whole base token) expressed in lots of the
    /// quote token. Access with [unwrap_price](OpenLimitOrder::unwrap_price).
    ///
    /// This value is not stored directly on the trie in this struct. It's the
    /// responsibility of the containing [L2] or other accessor to initialize
    /// the value at runtime.
    #[borsh_skip]
    pub limit_price_lots: Option<LotBalance>,

    /// Bid or ask. Access with [unwrap_side](OpenLimitOrder::unwrap_side).
    ///
    /// This value is not stored directly on the trie in this struct. It's the
    /// responsibility of the containing [L2] or other accessor to initialize
    /// the value at runtime.
    #[borsh_skip]
    pub side: Option<Side>,
}

impl OpenLimitOrder {
    impl_lazy_accessors!(limit_price_lots, unwrap_price, initialize_price, LotBalance);
    impl_lazy_accessors!(side, unwrap_side, initialize_side, Side);
}

impl OpenLimitOrder {
    pub fn id(&self) -> OrderId {
        new_order_id(
            self.unwrap_side(),
            self.unwrap_price(),
            self.sequence_number,
        )
    }
}

impl ValueLocked for OpenLimitOrder {
    fn value_locked(
        &self,
        base_lot_size: Balance,
        quote_lot_size: Balance,
        base_denomination: Balance,
    ) -> Tvl {
        match self.unwrap_side() {
            Side::Buy => Tvl {
                base_locked: 0,
                quote_locked: (U256::from(self.open_qty_lots)
                    * U256::from(base_lot_size)
                    * U256::from(self.unwrap_price())
                    * U256::from(quote_lot_size)
                    / U256::from(base_denomination))
                .as_u128(),
            },
            Side::Sell => Tvl {
                base_locked: self.open_qty_lots as u128 * base_lot_size,
                quote_locked: 0,
            },
        }
    }
}
