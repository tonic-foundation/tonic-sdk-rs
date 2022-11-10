pub use crate::*;

use super::test_utils::*;

#[test]
fn add_order() {
    let mut counter = new_counter();
    let mut ob = new_orderbook();

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
    let mut ob = new_orderbook();

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

    assert_eq!(ob.find_bbo(Side::Buy).unwrap().unwrap_price(), 3);
}

#[test]
fn basic_fill() {
    let mut counter = new_counter();
    let mut ob = new_orderbook();

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
    assert_eq!(ob.find_bbo(Side::Sell).unwrap().unwrap_price(), 101);

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
    let mut ob = new_orderbook();

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
    assert_eq!(ob.find_bbo(Side::Sell).unwrap().unwrap_price(), 10);
}

#[test]
fn find_order() {
    let mut counter = new_counter();
    let mut ob = new_orderbook();

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
    assert_eq!(bid.unwrap_side(), Side::Buy);
    let ask = ob.get_order(oid2).unwrap();
    assert_eq!(ask.unwrap_side(), Side::Sell);

    // let invalid = ob.get_order(3);
    // assert_eq!(invalid, None);
}

#[test]
fn test_post_only() {
    let mut counter = new_counter();
    let mut ob = new_orderbook();

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
    let mut ob = new_orderbook();

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
    let mut ob = new_orderbook();

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
    let mut ob = new_orderbook();

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
    let mut ob = new_orderbook();

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
