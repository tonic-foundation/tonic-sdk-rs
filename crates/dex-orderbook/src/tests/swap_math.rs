pub use crate::*;

use super::test_utils::*;

/// Test for subtraction overflow as in
/// https://explorer.mainnet.near.org/transactions/2rgowiKymgrduTEqdN7LoR6rPB7pr1zrEUQVidXdrcpj
///
/// Deployed OB commit was tonic-sdk-rs@16b76de6ac04a742dd232ffc1b6f89e8a291e8a5
#[test]
fn swap_math_bug() {
    let mut counter = new_counter();
    let mut ob = new_orderbook();

    let base_lot_size = 10000000000000000;
    let quote_lot_size = 1000;
    let base_denomination = 10u128.pow(18);

    let maker_order_req_1 = NewOrder {
        // order 111114r4E9zucyiwpv3Z, maker 1
        sequence_number: counter.next(),
        side: Side::Sell,
        order_type: OrderType::Limit,
        limit_price_lots: Some(480),
        max_qty_lots: 998, // based on fill event, order only had this much left at time of swap
        available_quote_lots: None,

        quote_lot_size,
        base_denomination,
        base_lot_size,
        client_id: None,
    };

    let maker_order_req_2 = NewOrder {
        // order 111114r4Fddx272CNF2X, maker 2
        sequence_number: counter.next(),
        side: Side::Sell,
        order_type: OrderType::Limit,
        limit_price_lots: Some(488),
        max_qty_lots: 8568,
        available_quote_lots: None,

        quote_lot_size,
        base_denomination,
        base_lot_size,
        client_id: None,
    };
    ob.place_order(
        &AccountId::new_unchecked("maker".to_string()),
        maker_order_req_1,
    );
    ob.place_order(
        &AccountId::new_unchecked("maker".to_string()),
        maker_order_req_2,
    );

    let res = ob.place_order(
        &AccountId::new_unchecked("taker".to_string()),
        NewOrder {
            // order GokLUshrueJnF6dXzpCuuZ, taker (swap)
            sequence_number: counter.next(),

            side: Side::Buy,
            order_type: OrderType::Market,
            limit_price_lots: None,
            max_qty_lots: u64::MAX,
            available_quote_lots: Some(4795), // 4.80 - 0.1% is 4.7952, last 2 is dropped due to lots

            quote_lot_size,
            base_denomination,
            base_lot_size,
            client_id: None,
        },
    );
    // quick rundown of what's happening
    // - the first fill costs 9.98 @ 0.480 = 4.790400
    // - after that, we have (4.7952 - (4.8 * .998)) = 0.0048 usdc left
    //   - notice that this can only buy 0.00983606557 Aurora, which is < 1 lot,
    //   - notice that this the quote lot size is 0.001, so we should have 4 lots available,
    //     but after first fill, ob shows 5 lots available, which *is* enough to buy one more
    //     lot. there's a rounded error on ob:407; converting native paid back to lots is a rounding error

    let native_quote_paid = res
        .matches
        .iter()
        .map(|m| m.native_quote_paid)
        .sum::<u128>();

    assert_eq!(
        9980000000000000000, // + 10000000000000000, // so the bug is fixed if that second fill doesn't happens
        res.fill_qty_lots as u128 * base_lot_size,
        "didn't match real txn"
    );

    assert_eq!(
        native_quote_paid,
        4790400, // and the amount paid should be what we calculated above
    );
}
