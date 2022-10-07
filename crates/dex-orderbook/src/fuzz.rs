pub use crate::*;

#[cfg(test)]
mod test {
    use proptest::prelude::*;

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

    fn arb_order_type() -> impl Strategy<Value = OrderType> {
        prop_oneof![Just(OrderType::Limit)]
    }

    fn arb_order_side() -> impl Strategy<Value = Side> {
        prop_oneof![
            Just(Side::Buy),
            // Just(Side::Sell)
        ]
    }

    /// Return arbitrary base lot size, quote lot size, and base denomination
    /// with the assumption that `L_q * L_b >= base_denomination`.
    ///
    /// Returned order is:
    ///
    /// base lot decimals, quote lot decimals, base token decimals
    fn arb_decimals(
        max_base_decimals: u32,
        max_quote_decimals: u32,
    ) -> impl Strategy<Value = (u128, u128, u128)> {
        // generate base and quote lot decimals first, then return base token decimals as strictly less than the sum
        (
            0..max_base_decimals,
            0..max_quote_decimals,
            0..max_base_decimals,
        )
            .prop_filter(
                "base lot decimals + quote lot decimals must be >= base token decimals",
                |(d_bl, d_ql, d_b)| *d_bl + *d_ql >= *d_b,
            )
            .prop_flat_map(|(d_bl, d_ql, d_b)| {
                (
                    Just(10u128.pow(d_bl)),
                    Just(10u128.pow(d_ql)),
                    Just(10u128.pow(d_b)),
                )
            })
    }

    proptest! {
        #[test]
        fn test_arb_decimals((base_lot_size, quote_lot_size, base_denomination) in arb_decimals(24, 18)) {
            assert!(
                BN!(base_lot_size).mul(quote_lot_size).0 >= BN!(base_denomination).0,
                "decimal generation failure base_lot_size {} quote_lot_size {} base_denomination {}",
                base_lot_size,
                quote_lot_size,
                base_denomination
            )
        }
    }

    prop_compose! {
        fn arb_order_req(base_lot_size: u128, quote_lot_size: u128, base_denomination: u128)(
            sequence_number in 0..(u64::MAX - 1),
            order_type in arb_order_type(),
            side in arb_order_side(),
            // these values are probably never going to touch most of this range...
            limit_price_lots in 1..1_000_000u64,
            max_qty_lots in 1..1_000_000u64
        ) -> NewOrder {
            // TODO: move this outside
            let available_quote_lots = (
                get_bid_quote_value(
                    max_qty_lots,
                    limit_price_lots,
                    base_lot_size,
                    quote_lot_size,
                    base_denomination
                ) / quote_lot_size
            ) as u64;

            // TODO: move this outside
            // XXX: this setup happens in each kind of order; may need to rethink how this works
            let max_qty_lots = if side == Side::Sell {
                max_qty_lots
            } else {
                max_qty_lots.min(
                    get_base_purchasable(
                        available_quote_lots,
                        limit_price_lots,
                        base_lot_size,
                        base_denomination
                    )
                )
            };

            NewOrder {
                sequence_number,
                limit_price_lots: if order_type != OrderType::Market { Some(limit_price_lots) } else { None },
                available_quote_lots: if side == Side::Buy {Some(available_quote_lots)} else { None},
                max_qty_lots,
                side,
                order_type,
                base_lot_size,
                quote_lot_size,
                base_denomination,
                client_id: None,
            }
        }
    }

    fn arb_orders_vecs(
        max_base_decimals: u32,
        max_quote_decimals: u32,
        max_orders: usize,
    ) -> impl Strategy<Value = Vec<NewOrder>> {
        arb_decimals(max_base_decimals, max_quote_decimals).prop_flat_map(
            move |(base_lot_decimals, quote_lot_decimals, base_decimals)| {
                prop::collection::vec(
                    arb_order_req(base_lot_decimals, quote_lot_decimals, base_decimals),
                    1..=max_orders,
                )
            },
        )
    }

    fn req_to_string(req: &NewOrder) -> String {
        if req.side == Side::Buy {
            format!(
                "{:?} buy {} @ {} ({} quote)",
                req.order_type,
                req.max_qty_lots,
                req.limit_price_lots.unwrap(),
                req.value_locked().quote_locked
            )
        } else {
            format!(
                "{:?} sell {} @ {} ({} base)",
                req.order_type,
                req.max_qty_lots,
                req.limit_price_lots.unwrap(),
                req.value_locked().base_locked
            )
        }
    }

    proptest! {
        #[test]
        fn test_arb_order_req(order_reqs in arb_orders_vecs(18, 6, 1)) {
            for req in order_reqs {
                assert!(req.max_qty_lots > 0, "invalid order")
            }
        }

        #[test]
        fn fuzz_ob_integrity(order_reqs in arb_orders_vecs(18, 6, 6)) {
            let mut ob = new_orderbook();
            let mut counter = new_counter(); // override sequence number? does it matter?
            let buyer = AccountId::new_unchecked("buyer.near".to_string());
            let seller = AccountId::new_unchecked("seller.near".to_string());

            // TODO: arb_orders_vec should return tuple with lot/denominations

            for mut req in order_reqs {
                // set up the sequence number
                // TODO: might be better to do this in the strategy
                req.sequence_number = counter.next();

                let base_lot_size = req.base_lot_size;
                let quote_lot_size = req.quote_lot_size;
                let base_denomination = req.base_denomination;
                let user = match req.side {
                    Side::Buy => &buyer,
                    Side::Sell => &seller
                };

                let req_clone = req.clone();

                let tvl_before = req.value_locked()
                    + ob.value_locked(base_lot_size, quote_lot_size, base_denomination);
                let _resp = ob.place_order(user, req);

                let tvl_after = ob.value_locked(base_lot_size, quote_lot_size, base_denomination);
                assert_eq!(
                    tvl_before,
                    tvl_after,
                    "rugged, diff {}, {}",
                    tvl_before.quote_locked - tvl_after.quote_locked,
                    req_to_string(&req_clone)
                );
            }
        }
    }
}
