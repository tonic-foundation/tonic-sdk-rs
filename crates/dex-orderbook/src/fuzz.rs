pub use crate::*;

#[cfg(test)]
mod test {
    use super::test_utils::*;

    // things to vary:
    // lot sizes
    // denomination
    // order params

    #[test]
    fn basic_tvl() {
        let user = AccountId::new_unchecked("a.near".to_string());
        let base_lot_size = 10;
        let quote_lot_size = 100;
        let base_denomination = 1000;

        // test per-order tvl
        let open_bid = OpenLimitOrder {
            open_qty_lots: 5,
            owner_id: user.clone(),
            sequence_number: 1,
            client_id: None,
            side: Some(Side::Buy),
            limit_price_lots: Some(100),
        };
        assert_eq!(
            open_bid.value_locked(base_lot_size, quote_lot_size, base_denomination),
            Tvl {
                base_locked: 0,
                quote_locked: 500
            },
            "bid tvl mismatch"
        );

        let open_ask = OpenLimitOrder {
            open_qty_lots: 5,
            owner_id: user.clone(),
            sequence_number: 1,
            client_id: None,
            side: Some(Side::Sell),
            limit_price_lots: Some(101), // doesn't matter
        };
        assert_eq!(
            open_ask.value_locked(base_lot_size, quote_lot_size, base_denomination),
            Tvl {
                base_locked: 50,
                quote_locked: 0
            },
            "ask tvl mismatch"
        );

        // test whole orderbook tvl
        let mut counter = new_counter();
        let mut ob = new_orderbook();

        let bid_req = NewOrder {
            sequence_number: counter.next(),
            limit_price_lots: Some(100),
            max_qty_lots: 5,
            side: Side::Buy,
            order_type: OrderType::Limit,
            client_id: None,
            available_quote_lots: Some(5), // TODO: formulated to exactly lock the correct balance with no refund
            base_lot_size,
            quote_lot_size,
            base_denomination,
        };
        let ask_req = NewOrder {
            sequence_number: counter.next(),
            limit_price_lots: Some(101), // don't fill
            max_qty_lots: 5,
            side: Side::Sell,
            order_type: OrderType::Limit,
            client_id: None,
            available_quote_lots: None,
            base_lot_size,
            quote_lot_size,
            base_denomination,
        };
        let tvl_before = bid_req.value_locked() + ask_req.value_locked();

        // TODO: PlaceOrderResult doesn't include the amount of unused tokens; until now,
        // the contract simply didn't debit unused tokens from the user, but it will be
        // useful to start returning that amount for these tests.
        let _bid_resp = ob.place_order(&user, bid_req);
        let _ask_resp = ob.place_order(&user, ask_req);
        let tvl_after = ob.value_locked(base_lot_size, quote_lot_size, base_denomination);

        assert_eq!(
            tvl_after,
            Tvl {
                base_locked: 50,
                quote_locked: 500
            },
            "ob TVL check failed"
        );
        assert_eq!(tvl_before, tvl_after, "orderbook rugged");
    }

    #[derive(Clone)]
    enum Operation {
        PlaceBid {
            price: LotBalance,
            quantity: LotBalance,
            // order_type: OrderType,
        },
        PlaceAsk {
            price: LotBalance,
            quantity: LotBalance,
            // order_type: OrderType,
        },
    }
    fn check_ob_integrity(operations: &[Operation]) {
        let mut ob = new_orderbook();
        let mut counter = new_counter();
        let base_lot_size = 10;
        let quote_lot_size = 100;
        let base_denomination = 1000;
        let buyer = AccountId::new_unchecked("buyer.near".to_string());
        let seller = AccountId::new_unchecked("seller.near".to_string());

        for op in operations {
            let user = match op {
                Operation::PlaceBid { .. } => &buyer,
                Operation::PlaceAsk { .. } => &seller,
            };

            let req = match op {
                Operation::PlaceBid { price, quantity } => {
                    // XXX: this seems like it needs a separate test (the lots
                    // part; test needs to be: go from native -> lots -> native
                    // and make sure nothing was lost)
                    let quote_lots_required = (get_bid_quote_value(
                        *quantity,
                        *price,
                        base_lot_size,
                        quote_lot_size,
                        base_denomination,
                    ) / quote_lot_size) as u64;
                    NewOrder {
                        sequence_number: counter.next(),
                        limit_price_lots: Some(*price),
                        max_qty_lots: *quantity,
                        side: Side::Buy,
                        order_type: OrderType::Limit,
                        available_quote_lots: Some(quote_lots_required),
                        base_lot_size,
                        quote_lot_size,
                        base_denomination,
                        client_id: None,
                    }
                }
                Operation::PlaceAsk { price, quantity } => NewOrder {
                    sequence_number: counter.next(),
                    limit_price_lots: Some(*price),
                    max_qty_lots: *quantity,
                    side: Side::Sell,
                    order_type: OrderType::Limit,
                    available_quote_lots: None,
                    base_lot_size,
                    quote_lot_size,
                    base_denomination,
                    client_id: None,
                },
            };

            let tvl_before = req.value_locked()
                + ob.value_locked(base_lot_size, quote_lot_size, base_denomination);
            let _resp = ob.place_order(user, req);

            let tvl_after = ob.value_locked(base_lot_size, quote_lot_size, base_denomination);
            assert_eq!(tvl_before, tvl_after, "bid rugged");
        }
    }
}
