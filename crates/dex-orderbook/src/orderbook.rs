/// Implements the matching engine.
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{AccountId, Balance};
use std::fmt::Debug;

use tonic_sdk_dex_errors as errors;
use tonic_sdk_dex_types::*;
use tonic_sdk_macros::*;

use crate::*;

/// The immediate outcome of creating a new order.
#[derive(
    Clone, Copy, Debug, PartialEq, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
#[serde(crate = "near_sdk::serde")]
pub enum OrderOutcome {
    /// The order was completely filled and not placed on the book.
    Filled,

    /// The order was partially filled. The remainder was placed on the book.
    PartialFill,

    /// The order was cancelled.
    Cancelled,

    /// The order was placed on the book. No part of the order was immediately
    /// filled.
    Posted,

    /// The order was not placed and no changes have been made to the
    /// user's account
    Rejected,
}

/// Internal struct representing an order ready to be processed by the matching
/// engine.
#[derive(Debug)]
pub struct NewOrder {
    pub sequence_number: SequenceNumber,
    pub limit_price_lots: Option<LotBalance>,
    /// Maximum amount to buy. This is one of the ways to control stopping
    /// behavior along with `max_qty_lots`
    pub available_quote_lots: Option<LotBalance>,
    /// Maximum amount to buy. This is one of the ways to control stopping
    /// behavior along with `available_quote_lots`
    pub max_qty_lots: LotBalance,
    pub side: Side,
    pub order_type: OrderType,
    pub base_denomination: u128,
    pub quote_lot_size: u128,
    pub base_lot_size: u128,
    pub client_id: Option<ClientId>,
}

/// Internal struct representing a match ready to be executed.
#[derive(Clone, Debug)]
pub struct Match {
    pub maker_order_id: OrderId,
    pub maker_user_id: AccountId,
    pub fill_qty_lots: LotBalance,
    pub fill_price_lots: LotBalance,
    pub native_quote_paid: Balance,

    /// Was the matched maker order removed. Used to update [Account]'s
    /// [OpenOrdersMap] during balance settlement.
    maker_order_removed: Option<bool>,
}

impl Match {
    pub fn did_remove_maker_order(&self) -> bool {
        self.maker_order_removed.unwrap()
    }
}

/// Result of running the matching engine. Used to settle account balance
/// changes.
#[derive(Debug)]
pub struct PlaceOrderResult {
    pub id: OrderId,
    pub fill_qty_lots: LotBalance,
    pub open_qty_lots: LotBalance,
    /// For a bid: the amount spent taking liquidity off the book (if any).
    /// For an ask: not used.
    pub quote_amount_lots: LotBalance,
    pub outcome: OrderOutcome,
    pub matches: Vec<Match>,
}

impl PlaceOrderResult {
    pub fn is_posted(&self) -> bool {
        self.open_qty_lots > 0
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PlaceOrderResultView {
    pub id: OrderId,
    pub outcome: OrderOutcome,

    /// Amount of base immediately traded.
    pub base_fill_quantity: U128,

    /// Amount of quote immediately traded.
    pub quote_fill_quantity: U128,

    /// Amount of base still open.
    pub open_quantity: U128,
}

impl PlaceOrderResult {
    pub fn into_view(self, base_lot_size: u128, quote_lot_size: u128) -> PlaceOrderResultView {
        PlaceOrderResultView {
            id: self.id,
            outcome: self.outcome,
            base_fill_quantity: U128::from(self.fill_qty_lots as u128 * base_lot_size),
            open_quantity: U128::from(self.open_qty_lots as u128 * base_lot_size),
            quote_fill_quantity: U128::from(self.quote_amount_lots as u128 * quote_lot_size),
        }
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub struct Orderbook<T: L2> {
    pub bids: T,
    pub asks: T,
}

#[derive(Debug)]
pub struct MatchOrderResult {
    unfilled_qty_lots: LotBalance,
    unused_quote_lots: Option<LotBalance>,
    matches: Vec<Match>,
}

impl<T: L2> Orderbook<T> {
    pub fn new(bids: T, asks: T) -> Self {
        Self { bids, asks }
    }
}

impl<T: L2> Orderbook<T> {
    pub fn find_bbo(&self, side: Side) -> Option<OpenLimitOrder> {
        match side {
            Side::Buy => self.bids.max_order(),
            Side::Sell => self.asks.min_order(),
        }
    }

    fn insert_order(&mut self, order: OpenLimitOrder) {
        match order.unwrap_side() {
            Side::Buy => self.bids.save_order(order),
            Side::Sell => self.asks.save_order(order),
        };
    }

    /// Place a new order and run the matching engine. This modifies the
    /// orderbook and returns a struct containing information needed to settle
    /// account balance changes resulting from the order.
    pub fn place_order(&mut self, user_id: &AccountId, order: NewOrder) -> PlaceOrderResult {
        let order_id = new_order_id(
            order.side,
            order.limit_price_lots.unwrap_or_default(),
            order.sequence_number,
        );

        let MatchOrderResult {
            unfilled_qty_lots,
            unused_quote_lots,
            mut matches,
        } = self.match_order(user_id, &order);

        let rejected: bool = {
            match order.order_type {
                OrderType::PostOnly => unfilled_qty_lots < order.max_qty_lots,
                OrderType::FillOrKill => unfilled_qty_lots > 0, // XXX: this should be cancelled, not rejected
                _ => false,
            }
        };

        if rejected {
            // no orderbook state modified at this point, return to cancel
            return PlaceOrderResult {
                id: order_id,
                fill_qty_lots: 0,
                open_qty_lots: 0,
                quote_amount_lots: 0,
                outcome: OrderOutcome::Rejected,
                matches: vec![],
            };
        }

        // Update resting orders
        let mut fill_qty_lots: LotBalance = 0;
        for fill in matches.iter_mut() {
            let mut maker_order = self.get_order(fill.maker_order_id).unwrap();
            maker_order.open_qty_lots -= fill.fill_qty_lots;

            if maker_order.open_qty_lots == 0 {
                fill.maker_order_removed = Some(true);
                self.remove_order(fill.maker_order_id);
            } else {
                fill.maker_order_removed = Some(false);
                match maker_order.unwrap_side() {
                    Side::Buy => self.bids.save_order(maker_order),
                    Side::Sell => self.asks.save_order(maker_order),
                }
            }

            // update running totals
            fill_qty_lots += fill.fill_qty_lots;
        }

        let can_post = !matches!(
            order.order_type,
            OrderType::FillOrKill | OrderType::ImmediateOrCancel | OrderType::Market
        );

        let outcome = match unfilled_qty_lots {
            0 => OrderOutcome::Filled,
            _ if order.order_type == OrderType::Market => OrderOutcome::Filled,
            _ if unfilled_qty_lots == order.max_qty_lots && can_post => OrderOutcome::Posted,
            _ => OrderOutcome::PartialFill,
        };

        if unfilled_qty_lots > 0 && can_post {
            self.insert_order(OpenLimitOrder {
                sequence_number: order.sequence_number,
                owner_id: user_id.clone(),
                limit_price_lots: _expect!(order, limit_price_lots, errors::MISSING_LIMIT_PRICE)
                    .into(),
                open_qty_lots: unfilled_qty_lots,
                client_id: order.client_id,
                side: order.side.into(),
            });
        }

        let open_qty_lots = if can_post { unfilled_qty_lots } else { 0 };

        PlaceOrderResult {
            id: order_id,
            fill_qty_lots,
            open_qty_lots,
            quote_amount_lots: order
                .available_quote_lots
                .unwrap_or_default()
                .checked_sub(unused_quote_lots.unwrap_or_default())
                .unwrap_or_default(),
            outcome,
            matches,
        }
    }

    /// Match orders. The result can be used to alter the orderbook, settle
    /// balance changes, etc.
    fn match_order(&self, user_id: &AccountId, order: &NewOrder) -> MatchOrderResult {
        let mut unfilled_qty_lots = order.max_qty_lots;
        let mut unused_quote_lots = order.available_quote_lots;

        let check_if_crossed: fn(LotBalance, LotBalance) -> bool = match order.side {
            Side::Buy => |p1, p2| p1 <= p2,
            Side::Sell => |p1, p2| p1 >= p2,
        };

        let mut matches: Vec<Match> = vec![];
        let resting_orders = match order.side {
            Side::Buy => self.asks.iter(),
            Side::Sell => self.bids.iter(),
        };

        for best_match in resting_orders {
            let trade_price_lots = *best_match.unwrap_price();

            let crossed = order.limit_price_lots.is_none()
                || check_if_crossed(trade_price_lots, order.limit_price_lots.unwrap());
            if !crossed {
                break;
            }

            if unfilled_qty_lots == 0 {
                break;
            }

            if best_match.owner_id == *user_id {
                near_sdk::env::panic_str(errors::SELF_TRADE)
            }

            let trade_qty_lots = match unused_quote_lots {
                // buying
                Some(remaining_quote_lots) => {
                    let max_based_on_remaining_quote = (U256::from(remaining_quote_lots)
                        * U256::from(order.base_denomination)
                        / U256::from(trade_price_lots)
                        / U256::from(order.base_lot_size))
                    .as_u64();
                    best_match
                        .open_qty_lots
                        .min(unfilled_qty_lots)
                        .min(max_based_on_remaining_quote)
                }
                // selling
                _ => best_match.open_qty_lots.min(unfilled_qty_lots),
            };

            if trade_qty_lots == 0 {
                break;
            }

            let native_quote_paid = ({
                U256::from(trade_price_lots)
                    * U256::from(order.quote_lot_size)
                    * U256::from(trade_qty_lots)
                    * U256::from(order.base_lot_size)
                    / U256::from(order.base_denomination)
            })
            .as_u128();
            unfilled_qty_lots -= trade_qty_lots;
            if unused_quote_lots.is_some() {
                // buying
                let quote_lots_paid = (native_quote_paid / order.quote_lot_size as u128) as u64;
                unused_quote_lots = Some(unused_quote_lots.unwrap() - quote_lots_paid);
            }

            matches.push(Match {
                maker_order_id: best_match.id(),
                maker_user_id: best_match.owner_id.clone(),
                fill_qty_lots: trade_qty_lots,
                fill_price_lots: trade_price_lots,
                native_quote_paid,
                maker_order_removed: None,
            });
        }

        MatchOrderResult {
            unfilled_qty_lots,
            unused_quote_lots,
            matches,
        }
    }

    /// Fetch an [OpenLimitOrder], if it exists
    pub fn get_order(&self, order_id: OrderId) -> Option<OpenLimitOrder> {
        let (side, price_lots, seq) = get_order_id_parts(order_id);
        let order = match side {
            Side::Buy => self.bids.get_order(price_lots, seq),
            Side::Sell => self.asks.get_order(price_lots, seq),
        };
        if let Some(mut order) = order {
            order.side = side.into();
            Some(order)
        } else {
            None
        }
    }

    /// Remove an order from the book
    pub fn remove_order(&mut self, order_id: OrderId) -> Option<OpenLimitOrder> {
        let (side, price_lots, seq) = get_order_id_parts(order_id);
        let order = match side {
            Side::Buy => self.bids.delete_order(price_lots, seq),
            Side::Sell => self.asks.delete_order(price_lots, seq),
        };
        if let Some(mut order) = order {
            order.side = side.into();
            Some(order)
        } else {
            None
        }
    }

    pub fn cancel_order(&mut self, order_id: OrderId) -> Option<OpenLimitOrder> {
        self.remove_order(order_id)
    }

    pub fn cancel_orders(&mut self, order_ids: Vec<OrderId>) -> Vec<OpenLimitOrder> {
        let mut deleted: Vec<OpenLimitOrder> = vec![];
        for order_id in order_ids.into_iter() {
            if let Some(order) = self.remove_order(order_id) {
                deleted.push(order)
            } else {
                debug_log!("Order bug: user had non-existent order ID");
            }
        }
        deleted
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use near_sdk::AccountId;

    fn add_orders(ob: &mut VecOrderbook, orders: Vec<NewOrder>) {
        for (_, order) in orders.into_iter().enumerate() {
            ob.place_order(&AccountId::new_unchecked("test_user".to_string()), order);
        }
    }

    fn orderbook() -> VecOrderbook {
        VecOrderbook::default()
    }

    fn place_order(ob: &mut VecOrderbook, account_id: &AccountId, order: NewOrder) -> OrderId {
        let res = ob.place_order(account_id, order);
        res.id
    }

    #[derive(Default)]
    struct Counter {
        pub prev: u64,
    }

    impl Counter {
        pub fn next(&mut self) -> u64 {
            self.prev += 1;
            self.prev
        }
    }

    fn new_counter() -> Counter {
        Counter::default()
    }

    #[test]
    fn add_order() {
        let mut counter = new_counter();
        let mut ob = orderbook();

        let res = ob.place_order(
            &AccountId::new_unchecked("test_user".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(100),
                max_qty_lots: 5,
                side: Side::Buy,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 10,
                base_lot_size: 1,
            },
        );
        assert_eq!(res.fill_qty_lots, 0);
        assert_eq!(ob.find_bbo(Side::Buy).unwrap().open_qty_lots, 5);
    }

    #[test]
    fn no_fill() {
        let mut counter = new_counter();
        let mut ob = orderbook();

        add_orders(
            &mut ob,
            vec![
                NewOrder {
                    sequence_number: counter.next(),
                    limit_price_lots: Some(1),
                    max_qty_lots: 1,
                    side: Side::Buy,
                    order_type: OrderType::Limit,
                    client_id: None,
                    available_quote_lots: None,
                    quote_lot_size: 1,
                    base_denomination: 1,
                    base_lot_size: 1,
                },
                NewOrder {
                    sequence_number: counter.next(),
                    limit_price_lots: Some(2),
                    max_qty_lots: 2,
                    side: Side::Buy,
                    order_type: OrderType::Limit,
                    client_id: None,
                    available_quote_lots: None,
                    quote_lot_size: 1,
                    base_denomination: 1,
                    base_lot_size: 1,
                },
                NewOrder {
                    sequence_number: counter.next(),
                    limit_price_lots: Some(3),
                    max_qty_lots: 3,
                    side: Side::Buy,
                    order_type: OrderType::Limit,
                    client_id: None,
                    available_quote_lots: None,
                    quote_lot_size: 1,
                    base_denomination: 1,
                    base_lot_size: 1,
                },
                NewOrder {
                    sequence_number: counter.next(),
                    limit_price_lots: Some(4),
                    max_qty_lots: 4,
                    side: Side::Sell,
                    order_type: OrderType::Limit,
                    client_id: None,
                    available_quote_lots: None,
                    quote_lot_size: 1,
                    base_denomination: 1,
                    base_lot_size: 1,
                },
                NewOrder {
                    sequence_number: counter.next(),
                    limit_price_lots: Some(5),
                    max_qty_lots: 5,
                    side: Side::Sell,
                    order_type: OrderType::Limit,
                    client_id: None,
                    available_quote_lots: None,
                    quote_lot_size: 1,
                    base_denomination: 1,
                    base_lot_size: 1,
                },
                NewOrder {
                    sequence_number: counter.next(),
                    limit_price_lots: Some(6),
                    max_qty_lots: 6,
                    side: Side::Sell,
                    order_type: OrderType::Limit,
                    client_id: None,
                    available_quote_lots: None,
                    quote_lot_size: 1,
                    base_denomination: 1,
                    base_lot_size: 1,
                },
            ],
        );

        assert_eq!(*ob.find_bbo(Side::Buy).unwrap().unwrap_price(), 3);
    }

    #[test]
    fn basic_fill() {
        let mut counter = new_counter();
        let mut ob = orderbook();

        let res = ob.place_order(
            &AccountId::new_unchecked("maker".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(100),
                max_qty_lots: 5,
                side: Side::Buy,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );
        assert_eq!(res.fill_qty_lots, 0);
        assert_eq!(ob.find_bbo(Side::Buy).unwrap().open_qty_lots, 5);

        ob.place_order(
            &AccountId::new_unchecked("maker".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(101),
                max_qty_lots: 1,
                side: Side::Sell,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );
        assert_eq!(*ob.find_bbo(Side::Sell).unwrap().unwrap_price(), 101);

        let res2 = ob.place_order(
            &AccountId::new_unchecked("taker".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(99),
                max_qty_lots: 4,
                side: Side::Sell,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );
        assert_eq!(res2.fill_qty_lots, 4);
        assert_eq!(res2.matches.len(), 1);
        assert_eq!(res2.matches[0].fill_qty_lots, 4);
        assert_eq!(res2.matches[0].fill_price_lots, 100);
    }

    #[test]
    fn partial_fill() {
        let mut counter = new_counter();
        let mut ob = orderbook();

        add_orders(
            &mut ob,
            vec![
                NewOrder {
                    sequence_number: counter.next(),
                    limit_price_lots: Some(5),
                    max_qty_lots: 5,
                    side: Side::Sell,
                    order_type: OrderType::Limit,
                    client_id: None,
                    available_quote_lots: None,
                    quote_lot_size: 1,
                    base_denomination: 1,
                    base_lot_size: 1,
                },
                NewOrder {
                    sequence_number: counter.next(),
                    limit_price_lots: Some(10),
                    max_qty_lots: 5,
                    side: Side::Sell,
                    order_type: OrderType::Limit,
                    client_id: None,
                    available_quote_lots: None,
                    quote_lot_size: 1,
                    base_denomination: 1,
                    base_lot_size: 1,
                },
                NewOrder {
                    sequence_number: counter.next(),
                    limit_price_lots: Some(15),
                    max_qty_lots: 5,
                    side: Side::Sell,
                    order_type: OrderType::Limit,
                    client_id: None,
                    available_quote_lots: None,
                    quote_lot_size: 1,
                    base_denomination: 1,
                    base_lot_size: 1,
                },
            ],
        );
        let res = ob.place_order(
            &AccountId::new_unchecked("taker".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(10),
                max_qty_lots: 7,
                side: Side::Buy,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );

        assert_eq!(res.fill_qty_lots, 7);
        assert_eq!(res.matches.len(), 2);

        // assert_eq!(ob.asks.len(), 2);
        assert_eq!(ob.find_bbo(Side::Sell).unwrap().open_qty_lots, 3);
        assert_eq!(*ob.find_bbo(Side::Sell).unwrap().unwrap_price(), 10);
    }

    #[test]
    fn find_order() {
        let mut counter = new_counter();
        let mut ob = orderbook();

        let oid1 = place_order(
            &mut ob,
            &AccountId::new_unchecked("maker".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(100),
                max_qty_lots: 5,
                side: Side::Buy,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );

        let oid2 = place_order(
            &mut ob,
            &AccountId::new_unchecked("taker".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(125),
                max_qty_lots: 10,
                side: Side::Sell,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );

        let bid = ob.get_order(oid1).unwrap();
        assert_eq!(*bid.unwrap_side(), Side::Buy);
        let ask = ob.get_order(oid2).unwrap();
        assert_eq!(*ask.unwrap_side(), Side::Sell);

        // let invalid = ob.get_order(3);
        // assert_eq!(invalid, None);
    }

    #[test]
    fn test_post_only() {
        let mut counter = new_counter();
        let mut ob = orderbook();

        add_orders(
            &mut ob,
            vec![NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(5),
                max_qty_lots: 5,
                side: Side::Sell,
                order_type: OrderType::PostOnly,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            }],
        );
        let res = ob.place_order(
            &AccountId::new_unchecked("test_user".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(4),
                max_qty_lots: 5,
                side: Side::Buy,
                order_type: OrderType::PostOnly,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );
        assert_eq!(res.outcome, OrderOutcome::Posted);
        assert_eq!(res.fill_qty_lots, 0);
        assert_eq!(res.matches.len(), 0);

        let res = ob.place_order(
            &AccountId::new_unchecked("taker".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(5),
                max_qty_lots: 2,
                side: Side::Buy,
                order_type: OrderType::PostOnly,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );
        assert_eq!(res.outcome, OrderOutcome::Rejected);
        assert_eq!(res.fill_qty_lots, 0);
        assert_eq!(res.matches.len(), 0);
    }

    #[test]
    fn test_ioc() {
        let mut counter = new_counter();
        let mut ob = orderbook();

        add_orders(
            &mut ob,
            vec![NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(5),
                max_qty_lots: 4,
                side: Side::Sell,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            }],
        );
        let res = ob.place_order(
            &AccountId::new_unchecked("taker".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(5),
                max_qty_lots: 5,
                side: Side::Buy,
                order_type: OrderType::ImmediateOrCancel,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );
        assert_eq!(res.outcome, OrderOutcome::PartialFill);
        assert_eq!(res.fill_qty_lots, 4);
        assert_eq!(res.matches.len(), 1);

        // assert_eq!(ob.bids.len(), 0);
    }

    #[test]
    fn test_fill_or_kill() {
        let mut counter = new_counter();
        let mut ob = orderbook();

        add_orders(
            &mut ob,
            vec![NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(5),
                max_qty_lots: 5,
                side: Side::Sell,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            }],
        );
        let res = ob.place_order(
            &AccountId::new_unchecked("taker".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(4),
                max_qty_lots: 5,
                side: Side::Buy,
                order_type: OrderType::FillOrKill,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );
        assert_eq!(res.outcome, OrderOutcome::Rejected);
        assert_eq!(res.fill_qty_lots, 0);
        assert_eq!(res.matches.len(), 0);

        let res = ob.place_order(
            &AccountId::new_unchecked("taker".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(5),
                max_qty_lots: 10,
                side: Side::Buy,
                order_type: OrderType::FillOrKill,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );
        assert_eq!(res.outcome, OrderOutcome::Rejected);
        assert_eq!(res.fill_qty_lots, 0);
        assert_eq!(res.matches.len(), 0);

        let res = ob.place_order(
            &AccountId::new_unchecked("taker".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(5),
                max_qty_lots: 5,
                side: Side::Buy,
                order_type: OrderType::FillOrKill,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );
        assert_eq!(res.outcome, OrderOutcome::Filled);
        assert_eq!(res.fill_qty_lots, 5);
        assert_eq!(res.matches.len(), 1);
    }

    #[test]
    fn test_cancel() {
        let mut counter = new_counter();
        let user = AccountId::new_unchecked("test".to_string());
        let mut ob = orderbook();

        let res = ob.place_order(
            &user,
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(5),
                max_qty_lots: 5,
                side: Side::Buy,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );

        ob.cancel_order(res.id);
        let order = ob.get_order(res.id);
        assert_eq!(order, None);
    }

    #[test]
    fn test_cancel_multiple() {
        let mut counter = new_counter();
        let mut ob = orderbook();

        let oid1 = place_order(
            &mut ob,
            &AccountId::new_unchecked("dont_remove".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(5),
                max_qty_lots: 5,
                side: Side::Buy,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );

        let oid2 = place_order(
            &mut ob,
            &AccountId::new_unchecked("remove".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(5),
                max_qty_lots: 5,
                side: Side::Buy,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );

        let oid3 = place_order(
            &mut ob,
            &AccountId::new_unchecked("remove".to_string()),
            NewOrder {
                sequence_number: counter.next(),
                limit_price_lots: Some(5),
                max_qty_lots: 5,
                side: Side::Buy,
                order_type: OrderType::Limit,
                client_id: None,
                available_quote_lots: None,
                quote_lot_size: 1,
                base_denomination: 1,
                base_lot_size: 1,
            },
        );

        ob.cancel_orders(vec![oid2, oid3]);
        assert!(
            ob.get_order(oid1).is_some(),
            "Deleted order that didn't belong to user"
        );
        assert_eq!(ob.get_order(oid2), None, "Missed a spot (order 2)");
        assert_eq!(ob.get_order(oid3), None, "Missed a spot (order 3)");
    }
}
