pub mod market_id;
pub mod order_id;
pub mod order_type;
pub mod side;
pub mod token_type;

pub use market_id::*;
pub use order_id::*;
pub use order_type::*;
pub use side::*;
pub use token_type::*;

uint::construct_uint! {
    pub struct U256(4);
}

/// Sequence number is capped at 2^63. At 50k TPS, each placing 100 batch
/// orders, this would be around 58k years of order IDs.
pub type SequenceNumber = u64;
pub type LotBalance = u64;
