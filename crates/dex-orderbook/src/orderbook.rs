/// Implements the matching engine.
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{AccountId, Balance};
use std::fmt::Debug;

use tonic_sdk_dex_errors as errors;
use tonic_sdk_dex_types::*;
use tonic_sdk_macros::*;

use crate::orderbook_math::OrderbookCalculator;
use crate::*;

/// The immediate outcome of creating a new order.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
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
#[derive(Debug, Clone)]
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

// useful for integrity checks
impl NewOrder {
    pub fn value_locked(&self) -> Tvl {
        match self.side {
            Side::Buy => Tvl {
                base_locked: 0,
                quote_locked: self.available_quote_lots.unwrap() as u128 * self.quote_lot_size,
            },
            Side::Sell => Tvl {
                base_locked: self.max_qty_lots as u128 * self.base_lot_size,
                quote_locked: 0,
            },
        }
    }

    pub fn assert_valid(&self) {
        if self.order_type != OrderType::Market {
            let limit_price = _expect!(self.limit_price_lots, "missing limit price");
            _assert!(limit_price > 0, "limit price is 0");
        }
        _assert!(self.max_qty_lots > 0, "missing quantity");
    }
}

/// Internal struct representing a match ready to be executed.
#[derive(Clone, Debug)]
pub struct Match {
    pub maker_order_id: OrderId,
    pub maker_user_id: AccountId,
    pub fill_qty_lots: LotBalance,
    pub fill_price_lots: LotBalance,
    pub native_quote_paid: Balance,
    pub maker_order_price_rank: u32,

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
    /// Price rank of the new order. `None` if the order didn't post.
    pub price_rank: Option<u32>,
    /// Best resting bid before the order was placed. [None] if bid side was
    /// empty.
    pub best_bid: Option<LotBalance>,
    /// Best resting ask before the order was placed. [None] if ask side was
    /// empty.
    pub best_ask: Option<LotBalance>,
}

impl PlaceOrderResult {
    pub fn is_posted(&self) -> bool {
        self.open_qty_lots > 0
    }
}

impl ValueLocked for PlaceOrderResult {
    fn value_locked(
        &self,
        base_lot_size: Balance,
        _quote_lot_size: Balance,
        _base_denomination: Balance,
    ) -> Tvl {
        // calculate from matches
        // calculate from the *_lots fields
        // calculate diff... :skull:
        let quote_traded = self.matches.iter().map(|m| m.native_quote_paid).sum();
        let base_traded = self
            .matches
            .iter()
            .map(|m| m.fill_qty_lots as u128 * base_lot_size)
            .sum();

        Tvl {
            quote_locked: quote_traded,
            base_locked: base_traded,
        }
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

impl<T: L2> ValueLocked for Orderbook<T> {
    fn value_locked(
        &self,
        base_lot_size: Balance,
        quote_lot_size: Balance,
        base_denomination: Balance,
    ) -> Tvl {
        self.asks
            .value_locked(base_lot_size, quote_lot_size, base_denomination)
            + self
                .bids
                .value_locked(base_lot_size, quote_lot_size, base_denomination)
    }
}

impl<T: L2> Orderbook<T> {
    // Get midmarket price in native quote amount.
    //
    // Returns [None] if the orderbook is completely empty. If one side is
    // empty, return the best price from the other side.
    // pub fn get_midmarket_price(&self, calc: &OrderbookCalculator) -> Option<Balance> {
    //     let asks_empty = self.asks.is_empty();
    //     let bids_empty = self.bids.is_empty();

    //     let best_bid = self.find_bbo(Side::Buy);
    //     let best_ask = self.find_bbo(Side::Sell);

    //     if asks_empty && bids_empty {
    //         None
    //     } else if asks_empty {
    //         Some(calc.quote_lots_to_native(best_bid.unwrap().unwrap_price()))
    //     } else if bids_empty {
    //         Some(calc.quote_lots_to_native(best_ask.unwrap().unwrap_price()))
    //     } else {
    //         // compute average of best bid and best ask
    //         let best_bid_price = calc.quote_lots_to_native(best_bid.unwrap().unwrap_price());
    //         let best_ask_price = calc.quote_lots_to_native(best_ask.unwrap().unwrap_price());
    //         Some(BN!(best_bid_price).add(best_ask_price).div(2).as_u128())
    //     }
    // }

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

    fn get_price_rank(&self, side: Side, price_lots: LotBalance) -> u32 {
        match side {
            Side::Buy => self.bids.get_price_rank(price_lots),
            Side::Sell => self.asks.get_price_rank(price_lots),
        }
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
            // orderbook unchanged
            let best_bid = self.find_bbo(Side::Buy).map(|o| o.unwrap_price());
            let best_ask = self.find_bbo(Side::Sell).map(|o| o.unwrap_price());
            // no orderbook state modified at this point, return to cancel
            return PlaceOrderResult {
                id: order_id,
                fill_qty_lots: 0,
                open_qty_lots: 0,
                quote_amount_lots: 0,
                outcome: OrderOutcome::Rejected,
                matches: vec![],
                price_rank: None,
                best_bid,
                best_ask,
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
                price_rank: None,
            });
        }

        let open_qty_lots = if can_post { unfilled_qty_lots } else { 0 };

        // return price rank if order posted
        let price_rank = if open_qty_lots > 0 {
            Some(self.get_price_rank(
                order.side,
                _expect!(order, limit_price_lots, errors::MISSING_LIMIT_PRICE),
            ))
        } else {
            None
        };

        // orderbook has been mutated!
        let best_bid = self.find_bbo(Side::Buy).map(|o| o.unwrap_price());
        let best_ask = self.find_bbo(Side::Sell).map(|o| o.unwrap_price());

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
            price_rank,
            best_bid,
            best_ask,
        }
    }

    /// Match orders. The result can be used to alter the orderbook, settle
    /// balance changes, etc.
    fn match_order(&self, user_id: &AccountId, order: &NewOrder) -> MatchOrderResult {
        let calculator = OrderbookCalculator {
            base_lot_size: order.base_lot_size,
            quote_lot_size: order.quote_lot_size,
            base_denomination: order.base_denomination,
        };
        // let midmarket_price = self.get_midmarket_price(&calculator);

        let mut unfilled_qty_lots = order.max_qty_lots;
        let mut unused_quote = order
            .available_quote_lots
            .map(|l| calculator.quote_lot_size as u128 * l as u128);

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
            let trade_price_lots = best_match.unwrap_price();

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

            let trade_qty_lots = match unused_quote {
                // buying
                Some(remaining_quote) => {
                    let max_based_on_remaining_quote =
                        calculator.get_base_purchasable(remaining_quote, trade_price_lots);
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

            let native_quote_paid =
                calculator.get_bid_quote_value(trade_qty_lots, trade_price_lots);
            unfilled_qty_lots -= trade_qty_lots;
            if unused_quote.is_some() {
                // buying
                unused_quote = Some(unused_quote.unwrap() - native_quote_paid);
            }

            matches.push(Match {
                maker_order_id: best_match.id(),
                maker_user_id: best_match.owner_id.clone(),
                fill_qty_lots: trade_qty_lots,
                fill_price_lots: trade_price_lots,
                native_quote_paid,
                maker_order_removed: None,
                maker_order_price_rank: best_match.unwrap_price_rank(),
            });
        }

        MatchOrderResult {
            unfilled_qty_lots,
            // TODO: change this to use full native size
            unused_quote_lots: unused_quote.map(|n| (n / calculator.quote_lot_size) as u64),
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
