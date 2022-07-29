/// This file contains error messages for normal runtime errors. These messages
/// can be parsed by user-facing clients to show friendly error messages.

/////////////////////////////
// miscellaneous errors (E0X)
/////////////////////////////
pub const INVALID_TOKEN_ID: &str = "E01: invalid token ID";
pub const INVALID_ACTION: &str = "E02: Invalid batch action";

///////////////////////
// account errors (E1X)
///////////////////////
pub const INSUFFICIENT_BALANCE: &str = "E11: insufficient balance";
pub const INSUFFICIENT_STORAGE_BALANCE: &str = "E12: insufficient storage balance";
pub const ACCOUNT_NOT_FOUND: &str = "E13: account not found";

/////////////////////
// order errors (E2X)
/////////////////////
pub const MISSING_LIMIT_PRICE: &str = "E21: missing limit price";
pub const ZERO_ORDER_AMOUNT: &str = "E22: zero order amount";
pub const EXCEEDED_ORDER_LIMIT: &str = "E23: exceeded order limit";
pub const ORDER_NOT_FOUND: &str = "E24: order not found";
pub const EXCEEDED_SLIPPAGE_TOLERANCE: &str = "E25: exceeded slippage tolerance";

///////////////////////////////
// market creation errors (E3X)
///////////////////////////////
pub const MARKET_EXISTS: &str = "E31: market exists";
pub const INVALID_QUOTE_LOT_SIZE: &str = "E32: invalid quote lot size";
pub const INVALID_BASE_LOT_SIZE: &str = "E33: invalid base lot size";
pub const INSUFFICIENT_MARKET_DEPOSIT: &str = "E34: insufficient market deposit";
