pub mod l2;
pub mod orderbook;

pub use l2::*;
pub use orderbook::*;

use l2::vec::VecL2;

pub type ClientId = u32;

pub type VecOrderbook = Orderbook<VecL2>;

impl Default for VecOrderbook {
    fn default() -> Self {
        let bids = VecL2::new(true);
        let asks = VecL2::new(false);
        Self::new(bids, asks)
    }
}
