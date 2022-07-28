use near_sdk::serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum OrderType {
    /// Order fills at the specified price or better. Any part of the order not
    /// immediately filled will be posted.
    Limit,

    /// Fill as much as can be immediately filled and cancel the remainder.
    ImmediateOrCancel,

    /// Ensure the order posts to the book. If any part would immediately fill,
    /// cancel the order completely.
    PostOnly,

    /// Immediately fill the whole order or cancel it completely.
    FillOrKill,

    /// Fill as much as possible at market price and refund unused funds.
    ///
    /// Slippage tolerance can be controlled by setting `max_spend`, eg, if the
    /// mid-market price is 10 USDC, place an order to buy 10 NEAR with a
    /// slippage tolerance of 5% with:
    ///
    /// ```ignore
    /// let usdc_denomination: Balance = 1_000_000;
    /// let params = NewOrderParams {
    ///   limit_price: None,
    ///   max_spend: Some(U128(105 * usdc_denomination)),
    ///   quantity: U128(10),
    ///   side: Side::Buy,
    ///   order_type: OrderType::Market,
    ///   ..
    /// }
    /// ```
    Market,
}
