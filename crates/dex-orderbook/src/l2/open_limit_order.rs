use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    AccountId,
};
use once_cell::unsync::OnceCell;
use tonic_sdk_dex_types::{new_order_id, LotBalance, OrderId, SequenceNumber, Side};
use tonic_sdk_macros::*;

use crate::*;

#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct OpenLimitOrder {
    pub sequence_number: SequenceNumber,
    pub owner_id: AccountId,
    pub open_qty_lots: LotBalance,
    pub client_id: Option<ClientId>,

    /// Limit price. Access with [unwrap_price](OpenLimitOrder::unwrap_price).
    ///
    /// This value is not stored directly on the trie in this struct. It's the
    /// responsibility of the containing [L2] or other accessor to initialize
    /// the value at runtime.
    #[borsh_skip]
    pub limit_price_lots: OnceCell<LotBalance>,

    /// Bid or ask. Access with [unwrap_side](OpenLimitOrder::unwrap_side).
    ///
    /// This value is not stored directly on the trie in this struct. It's the
    /// responsibility of the containing [L2] or other accessor to initialize
    /// the value at runtime.
    #[borsh_skip]
    pub side: OnceCell<Side>,

    /// Index of the price level. Access with
    /// [unwrap_price_rank](OpenLimitOrder::unwrap_price_rank).
    ///
    /// This value is not stored directly on the trie in this struct. It's the
    /// responsibility of the containing [L2] or other accessor to initialize
    /// the value at runtime.
    #[borsh_skip]
    pub price_rank: OnceCell<u32>,
}

impl OpenLimitOrder {
    impl_lazy_accessors!(limit_price_lots, unwrap_price, initialize_price, LotBalance);
    impl_lazy_accessors!(side, unwrap_side, initialize_side, Side);
    impl_lazy_accessors!(price_rank, unwrap_price_rank, initialize_price_rank, u32);
}

impl OpenLimitOrder {
    pub fn id(&self) -> OrderId {
        new_order_id(
            *self.unwrap_side(),
            *self.unwrap_price(),
            self.sequence_number,
        )
    }
}
