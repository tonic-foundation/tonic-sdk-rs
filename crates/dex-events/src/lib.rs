use std::fmt;

use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, AccountId};

use tonic_sdk_dex_types::*;

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Event {
    #[serde(flatten)] // due to tagging options, this adds a "type" key and a "data" key
    pub data: EventType,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| fmt::Error)?)
    }
}

// we tag this with type/content and flatten it into the event struct. this is
// because serde sometimes has trouble figuring out which enum member the json
// corresponds to
#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde", tag = "type", content = "data")]
pub enum EventType {
    Order(NewOrderEvent),
    Fill(NewFillEvent),
    Cancel(NewCancelEvent),
    NewMarket(NewMarketEvent),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde", rename = "new_order")]
pub struct NewOrderEvent {
    pub account_id: AccountId,
    pub order_id: OrderId,
    /// `Some(0)` if the order didn't post. (uses [Option] for backwards
    /// compatibility; [None] implies the event is an old version.)
    pub open_quantity: Option<U128>,
    pub market_id: MarketId,
    /// Price specified in the order. Zero (0) if market order
    pub limit_price: U128,
    /// Price rank. `None` if the order didn't post
    #[deprecated(note = "use best_bid/best_ask instead")]
    pub price_rank: Option<u32>, // TODO: make this Option. new indexer can't index old events if this is required
    /// Best resting bid before the order was placed. [None] if bid side was
    /// empty.
    pub best_bid: Option<U128>,
    /// Best resting ask before the order was placed. [None] if ask side was
    /// empty.
    pub best_ask: Option<U128>,
    /// Quantity specified in the order; may not be the same as amount traded
    pub quantity: U128,
    pub side: Side,
    pub order_type: OrderType,
    /// Taker fee denominated in the quote currency
    pub taker_fee: U128,
    pub referrer_id: Option<AccountId>,
    /// Referrer rebate denominated in the quote currency
    pub referrer_rebate: U128,
    /// True if order created by an [Action::Swap]
    #[serde(default)] // backwards compatibility
    pub is_swap: bool,
    pub client_id: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde", rename = "cancel_order")]
pub struct NewCancelEvent {
    pub market_id: MarketId,
    pub cancels: Vec<CancelEventData>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde", rename = "cancel_order")]
pub struct CancelEventData {
    pub order_id: OrderId,
    /// Amount of locked token refunded.
    pub refund_amount: U128,
    /// The token that was locked in the open order. Quote if bid, base if ask.
    pub refund_token: TokenType,
    // TODO: named this way to match fills, etc, but there's no reason for those
    // fields to be named with this abbreviation
    /// The remaining open order quantity when the order was cancelled.
    pub cancelled_qty: U128,
    /// The order's price rank before it was cancelled.
    ///
    /// Deprecated
    #[deprecated(note = "use best_bid/best_ask instead")]
    pub price_rank: u32, // TODO: make this Option. new indexer can't index old events if this is required
    /// Best resting bid before the order was placed. [None] if bid side was
    /// empty.
    pub best_bid: Option<U128>,
    /// Best resting ask before the order was placed. [None] if ask side was
    /// empty.
    pub best_ask: Option<U128>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde", rename = "new_market")]
pub struct NewMarketEvent {
    pub creator_id: AccountId,
    pub market_id: MarketId,
    pub base_token: TokenType,
    pub quote_token: TokenType,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde", rename = "new_fill")]
pub struct NewFillEvent {
    pub market_id: MarketId,
    pub order_id: OrderId,
    pub fills: Vec<FillEventData>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct FillEventData {
    pub maker_order_id: OrderId,
    pub fill_qty: U128,
    pub fill_price: U128,
    pub quote_qty: U128,
    pub maker_rebate: U128,
    // the taker side
    pub side: Side,
    pub taker_account_id: AccountId,
    pub maker_account_id: AccountId,
    /// Price rank of the maker order right before it was filled. This is always
    /// Always zero (0) for now, since when an order is filled, it is necessarily at
    /// the top of the book
    #[deprecated(note = "use best_bid/best_ask on the associated maker order instead")]
    pub maker_price_rank: u32, // TODO: make this Option. new indexer can't index old events if this is required
}

pub fn emit_event(data: EventType) {
    #[cfg(not(feature = "no_emit"))]
    env::log_str(&Event { data }.to_string());
}
