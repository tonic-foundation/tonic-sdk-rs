pub use near_sdk::AccountId;
pub use tonic_sdk_dex_types::*;

pub use crate::*;

pub fn add_orders(ob: &mut VecOrderbook, orders: Vec<NewOrder>) {
    for (_, order) in orders.into_iter().enumerate() {
        ob.place_order(&AccountId::new_unchecked("test_user".to_string()), order);
    }
}

pub fn orderbook() -> VecOrderbook {
    VecOrderbook::default()
}

pub fn place_order(ob: &mut VecOrderbook, account_id: &AccountId, order: NewOrder) -> OrderId {
    let res = ob.place_order(account_id, order);
    res.id
}

#[derive(Default)]
pub struct Counter {
    pub prev: u64,
}

impl Counter {
    pub fn next(&mut self) -> u64 {
        self.prev += 1;
        self.prev
    }
}

pub fn new_counter() -> Counter {
    Counter::default()
}
