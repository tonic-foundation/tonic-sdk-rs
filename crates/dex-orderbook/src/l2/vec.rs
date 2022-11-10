/// Orderbook backend implemented as a flat list of orders sorted by price.
/// Initially it might seem that keeping a 2d array (vec of prices, queue of
/// orders per price) would be more efficient. However, in the typical case,
/// only a small percentage of orders will share a price level while the rest
/// have unique prices. Storing as a flat vec eliminates the storage overhead of
/// vec serialization.
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use tonic_sdk_dex_types::{LotBalance, SequenceNumber, Side};

use crate::*;

/// One side of an orderbook. This is represented as a list of prices, with a
/// list of orders at each price level.
#[derive(Debug, Default, Clone, BorshDeserialize, BorshSerialize)]
pub struct VecL2 {
    /// list of (price, order)
    pub orders: Vec<(LotBalance, OpenLimitOrder)>,

    /// Whether prices should be sorted in reverse (ie descending order). When
    /// true (eg for the bid side), price levels are automatically inserted and
    /// searched in reverse.
    pub reverse_prices: bool,
}

impl VecL2 {
    /// Iterator of [OpenLimitOrder] that initializes the price and side of its
    /// contents.
    pub fn initializing_iter(&self) -> impl Iterator<Item = OpenLimitOrder> + '_ {
        self.orders.iter().map(move |(price, _order)| {
            let mut order = _order.clone();
            order.initialize_price(*price);
            order.initialize_side(self.side());
            order.initialize_price_rank(self.get_price_rank(*price));
            order
        })
    }
}

impl OrderIter for VecL2 {
    /// Iterate through all orders (flattens price levels)
    fn iter(&self) -> Box<dyn Iterator<Item = OpenLimitOrder> + '_> {
        Box::new(self.initializing_iter())
    }
}

impl L2 for VecL2 {
    fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    fn max_order(&self) -> Option<OpenLimitOrder> {
        self.orders
            .iter()
            .max_by_key(|(p, o)| (*p, o.sequence_number))
            .map(|(p, o)| {
                let mut out = o.clone();
                out.initialize_price(*p);
                out.initialize_side(self.side());
                out.initialize_price_rank(self.get_price_rank(*p));
                out
            })
    }

    fn min_order(&self) -> Option<OpenLimitOrder> {
        self.orders
            .iter()
            .min_by_key(|(p, o)| (*p, o.sequence_number))
            .map(|(p, o)| {
                let mut out = o.clone();
                out.initialize_price(*p);
                out.initialize_side(self.side());
                out.initialize_price_rank(self.get_price_rank(*p));
                out
            })
    }

    fn save_order(&mut self, order: OpenLimitOrder) {
        let price = order.unwrap_price();
        match self.find_order_loc(price, order.sequence_number) {
            Ok(loc) => self.orders[loc] = (price, order),
            Err(loc) => self.orders.insert(loc, (price, order)),
        }
    }

    fn get_order(&self, price_lots: LotBalance, seq: SequenceNumber) -> Option<OpenLimitOrder> {
        self.orders
            .iter()
            .find(|(p, o)| *p == price_lots && o.sequence_number == seq)
            .map(|(p, o)| {
                let mut ret = o.clone();
                ret.initialize_price(*p);
                ret.initialize_side(self.side());
                ret.initialize_price_rank(self.get_price_rank(price_lots));
                ret
            })
    }

    fn delete_order(
        &mut self,
        price_lots: LotBalance,
        seq: SequenceNumber,
    ) -> Option<OpenLimitOrder> {
        if let Ok(loc) = self.find_order_loc(price_lots, seq) {
            let price_rank = self.get_price_rank(price_lots);
            let (_, mut order) = self.orders.remove(loc);
            order.initialize_price(price_lots);
            order.initialize_side(self.side());
            order.initialize_price_rank(price_rank);
            Some(order)
        } else {
            None
        }
    }

    fn get_price_rank(&self, price_lots: LotBalance) -> u32 {
        match self.get_price_rank_result(price_lots) {
            Ok(rank) => rank as u32,
            Err(rank) => rank as u32,
        }
    }
}

impl VecL2 {
    pub fn new(reverse_prices: bool) -> Self {
        Self {
            reverse_prices,
            ..Default::default()
        }
    }

    /// Return number of unique price levels.
    pub fn unique_prices_count(&self) -> u32 {
        if self.orders.is_empty() {
            return 0;
        }
        let mut count = 1;
        let mut prev_price = self.orders[0].0;
        for (p, _) in &self.orders {
            if prev_price != *p {
                count += 1;
                prev_price = *p;
            }
        }
        count
    }

    fn side(&self) -> Side {
        if self.reverse_prices {
            Side::Buy
        } else {
            Side::Sell
        }
    }

    fn find_order_loc(&self, price_lots: LotBalance, seq: SequenceNumber) -> Result<usize, usize> {
        if self.reverse_prices {
            self.orders
                // reverse by price only; sequence numbers still need to be in order
                .binary_search_by_key(&(!price_lots, seq), |(price, order)| {
                    (!*price, order.sequence_number)
                })
        } else {
            self.orders
                .binary_search_by_key(&(price_lots, seq), |(price, order)| {
                    (*price, order.sequence_number)
                })
        }
    }

    /// Get the "index" of a price level (the rank of a price level).
    ///
    /// If found, return `Result::Ok`, value is the index of the price level. If
    /// not found, return `Result::Err`, value is the index where the price
    /// level would be.
    fn get_price_rank_result(&self, price_lots: LotBalance) -> Result<usize, usize> {
        let mut price_levels = self
            .orders
            .iter()
            .map(|(level, _)| *level)
            .collect::<Vec<_>>();
        price_levels.dedup();

        if self.reverse_prices {
            price_levels.binary_search_by_key(&(!price_lots), |price| (!*price))
        } else {
            price_levels.binary_search_by_key(&(price_lots), |price| (*price))
        }
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::AccountId;

    use super::*;

    fn make_order(price: u64, sequence_number: u64) -> OpenLimitOrder {
        OpenLimitOrder {
            sequence_number,
            owner_id: AccountId::new_unchecked("a.near".to_string()),
            open_qty_lots: 1,
            client_id: None,
            limit_price_lots: Some(price),
            side: Some(Side::Buy),
            price_rank: None, // doesn't matter for the test
        }
    }

    // TODO: good candidate for proptest
    #[test]
    fn sort_regular() {
        let mut l2 = VecL2::new(false);
        // insert 3 orders, 2 sharing a price
        // should sort like this:           [ 1 1 2 ]
        // with sequence numbers like this: [ 1 3 2 ]
        l2.save_order(make_order(1, 1));
        l2.save_order(make_order(2, 2));
        l2.save_order(make_order(1, 3));

        // check it's sorted by price ascending
        assert_eq!(
            l2.orders.first().unwrap().0,
            1,
            "wrong price for first order"
        );
        assert_eq!(l2.orders.last().unwrap().0, 2, "wrong price for last order");

        // check that orders with a common price are sorted by sequence number ascending
        assert!(
            l2.orders[0].1.sequence_number == 1 && l2.orders[1].1.sequence_number == 3,
            "orders with same price not sorted by sequence number ascending"
        );
    }

    // TODO: good candidate for proptest
    #[test]
    fn sort_reverse() {
        let mut l2 = VecL2::new(true);
        // insert 3 orders, 2 sharing a price
        // should sort like this:           [ 2 1 1 ]
        // with sequence numbers like this: [ 2 1 3 ]
        l2.save_order(make_order(1, 1));
        l2.save_order(make_order(2, 2));
        l2.save_order(make_order(1, 3));

        // check it's sorted by price descending
        assert_eq!(
            l2.orders.first().unwrap().0,
            2,
            "wrong price for first order"
        );
        assert_eq!(l2.orders.last().unwrap().0, 1, "wrong price for last order");

        // though prices are reversed, sequence number should still be sorted ascending
        // for orders with a common price
        assert!(
            l2.orders[1].1.sequence_number == 1 && l2.orders[2].1.sequence_number == 3,
            "orders with same price not sorted by sequence number ascending"
        );
    }

    #[test]
    fn get_price_rank() {
        // sort ascending (ask side); lower prices should have lower rank
        // price rank for price 1 should be 0
        // price rank for price 2 should be 1
        // price rank for price 3 should be 2
        // price rank for price 5 should be 3
        let mut l2 = VecL2::new(false);
        l2.save_order(make_order(1, 1));
        l2.save_order(make_order(1, 2));
        l2.save_order(make_order(2, 3));
        l2.save_order(make_order(4, 4));

        assert_eq!(l2.get_price_rank(1), 0, "wrong price rank for price 1");
        assert_eq!(l2.get_price_rank(2), 1, "wrong price rank for price 2");
        assert_eq!(l2.get_price_rank(3), 2, "wrong price rank for price 3");
        assert_eq!(l2.get_price_rank(5), 3, "wrong price rank for price 5");

        // sort descending (bid side); higher prices should have lower rank
        // price rank for price 5 should be 0 (comes before 4)
        // price rank for price 3 should be 1 (comes after 4)
        // price rank for price 2 should be 1 (comes after 4)
        // price rank for price 1 should be 2 (comes after 4, 2)
        let mut l2 = VecL2::new(true);
        l2.save_order(make_order(1, 1));
        l2.save_order(make_order(1, 2));
        l2.save_order(make_order(2, 3));
        l2.save_order(make_order(4, 4));

        assert_eq!(l2.get_price_rank(5), 0, "wrong price rank for price 5");
        assert_eq!(l2.get_price_rank(3), 1, "wrong price rank for price 3");
        assert_eq!(l2.get_price_rank(2), 1, "wrong price rank for price 2");
        assert_eq!(l2.get_price_rank(1), 2, "wrong price rank for price 1");
    }
}
