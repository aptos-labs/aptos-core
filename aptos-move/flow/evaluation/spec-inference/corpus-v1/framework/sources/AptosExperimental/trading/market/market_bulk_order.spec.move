spec aptos_experimental::market_bulk_order {

    spec cancel_bulk_order<M: copy + drop + store, R: copy + drop + store>(
        market: &mut 0x7::market_types::Market<M>,
        user: &signer,
        cancellation_reason: 0x7::market_types::OrderCancellationReason,
        callbacks: &0x7::market_types::MarketClearinghouseCallbacks<M, R>
    ) {
        use 0x1::signer;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] ensures_of<cancel_bulk_order_internal<M, R>> (
            market,
            signer::address_of(user),
            cancellation_reason,
            callbacks,
            market
        );
    }

    spec cancel_bulk_order_internal<M: copy + drop + store, R: copy + drop + store>(
        market: &mut 0x7::market_types::Market<M>,
        user: address,
        cancellation_reason: 0x7::market_types::OrderCancellationReason,
        callbacks: &0x7::market_types::MarketClearinghouseCallbacks<M, R>
    ) {
        use 0x1::option;
        use 0x5::bulk_order_types;
        use 0x7::order_book;
        use 0x7::market_types;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard]({
            let a = ..S1 |~ result_of<market_types::get_order_book_mut<M>> (market);
            market == update_field(market, order_book, a)
        });
        ensures [inferred = sathard]({
            let (a_0, a_1, a_2, a_3) = {
                let b = {
                    let c = {
                        let d = {
                            let e = {
                                let f =
                                    ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                        market
                                    );
                                ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                    update_field(market, order_book, f)
                                )
                            };
                            ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                update_field(market, order_book, e)
                            )
                        };
                        ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                            update_field(market, order_book, d)
                        )
                    };
                    S1..S2 |~ result_of<order_book::cancel_bulk_order<M>> (c, user)
                };
                S2..S3 |~ result_of<bulk_order_types::destroy_bulk_order<M>> (b)
            };
            {
                let (
                    a_1_0, a_1_1, a_1_2, a_1_3, a_1_4, a_1_5, a_1_6
                ) =
                    S3..S4 |~ result_of<bulk_order_types::destroy_bulk_order_request<M>> (a_0);
                let b_1 = {
                    let c_1 = {
                        let d_1 =
                            ..S1 |~ result_of<market_types::get_order_book_mut<M>> (market);
                        ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                            update_field(market, order_book, d_1)
                        )
                    };
                    ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                        update_field(market, order_book, c_1)
                    )
                };
                S4.. |~ ensures_of<market_types::emit_event_for_bulk_order_cancelled<M>> (
                    update_field(market, order_book, b_1),
                    a_1,
                    a_1_1,
                    user,
                    a_1_2,
                    a_1_3,
                    a_1_4,
                    a_1_5,
                    option::some<market_types::OrderCancellationReason>(
                        cancellation_reason
                    )
                )
            }
        });
        ensures [inferred = sathard]({
            let (a_0, a_1, a_2, a_3, a_4, a_5, a_6) = {
                let (b_0, b_1, b_2, b_3) = {
                    let c = {
                        let d = {
                            let e = {
                                let f = {
                                    let a_1 =
                                        ..S1 |~ result_of<market_types::get_order_book_mut<M
                                            >> (market);
                                    ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                        update_field(market, order_book, a_1)
                                    )
                                };
                                ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                    update_field(market, order_book, f)
                                )
                            };
                            ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                update_field(market, order_book, e)
                            )
                        };
                        S1..S2 |~ result_of<order_book::cancel_bulk_order<M>> (d, user)
                    };
                    S2..S3 |~ result_of<bulk_order_types::destroy_bulk_order<M>> (c)
                };
                S3..S4 |~ result_of<bulk_order_types::destroy_bulk_order_request<M>> (b_0)
            };
            forall x: u64:
                S4.. |~ x < len(a_3) ==> {
                    let (b_1_0, b_1_1, b_1_2, b_1_3) = {
                        let c_1 = {
                            let d_1 = {
                                let e_1 = {
                                    let f_1 = {
                                        let a_2 = ..S1 |~ result_of<market_types::get_order_book_mut<M
                                            >> (market);
                                        ..S1 |~ result_of<market_types::get_order_book_mut<M
                                            >> (update_field(market, order_book, a_2))
                                    };
                                    ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                        update_field(market, order_book, f_1)
                                    )
                                };
                                ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                    update_field(market, order_book, e_1)
                                )
                            };
                            S1..S2 |~ result_of<order_book::cancel_bulk_order<M>> (
                                d_1, user
                            )
                        };
                        S2..S3 |~ result_of<bulk_order_types::destroy_bulk_order<M>> (c_1)
                    };
                    ensures_of<market_types::cleanup_bulk_order_at_price<M, R>> (
                        callbacks, user, b_1_1, true, a_2[x], a_3[x]
                    )
                }
        });
        ensures [inferred = sathard]({
            let (a_0, a_1, a_2, a_3, a_4, a_5, a_6) = {
                let (b_0, b_1, b_2, b_3) = {
                    let c = {
                        let d = {
                            let e = {
                                let f = {
                                    let a_1 =
                                        ..S1 |~ result_of<market_types::get_order_book_mut<M
                                            >> (market);
                                    ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                        update_field(market, order_book, a_1)
                                    )
                                };
                                ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                    update_field(market, order_book, f)
                                )
                            };
                            ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                update_field(market, order_book, e)
                            )
                        };
                        S1..S2 |~ result_of<order_book::cancel_bulk_order<M>> (d, user)
                    };
                    S2..S3 |~ result_of<bulk_order_types::destroy_bulk_order<M>> (c)
                };
                S3..S4 |~ result_of<bulk_order_types::destroy_bulk_order_request<M>> (b_0)
            };
            forall x: u64:
                S4.. |~ x < len(a_5) ==> {
                    let (b_1_0, b_1_1, b_1_2, b_1_3) = {
                        let c_1 = {
                            let d_1 = {
                                let e_1 = {
                                    let f_1 = {
                                        let a_2 = ..S1 |~ result_of<market_types::get_order_book_mut<M
                                            >> (market);
                                        ..S1 |~ result_of<market_types::get_order_book_mut<M
                                            >> (update_field(market, order_book, a_2))
                                    };
                                    ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                        update_field(market, order_book, f_1)
                                    )
                                };
                                ..S1 |~ result_of<market_types::get_order_book_mut<M>> (
                                    update_field(market, order_book, e_1)
                                )
                            };
                            S1..S2 |~ result_of<order_book::cancel_bulk_order<M>> (
                                d_1, user
                            )
                        };
                        S2..S3 |~ result_of<bulk_order_types::destroy_bulk_order<M>> (c_1)
                    };
                    ensures_of<market_types::cleanup_bulk_order_at_price<M, R>> (
                        callbacks, user, b_1_1, false, a_4[x], a_5[x]
                    )
                }
        });
        aborts_if [inferred = sathard]({
            let (a_0, a_1, a_2, a_3, a_4, a_5, a_6) = {
                let (b_0, b_1, b_2, b_3) = {
                    let c = {
                        let d =
                            ..S1 |~ result_of<market_types::get_order_book_mut<M>> (market);
                        S1..S2 |~ result_of<order_book::cancel_bulk_order<M>> (d, user)
                    };
                    S2..S3 |~ result_of<bulk_order_types::destroy_bulk_order<M>> (c)
                };
                S3..S4 |~ result_of<bulk_order_types::destroy_bulk_order_request<M>> (b_0)
            };
            len(a_3) == 18446744073709551616
        });
        aborts_if [inferred = sathard]({
            let (a_0, a_1, a_2, a_3, a_4, a_5, a_6) = {
                let (b_0, b_1, b_2, b_3) = {
                    let c = {
                        let d =
                            ..S1 |~ result_of<market_types::get_order_book_mut<M>> (market);
                        S1..S2 |~ result_of<order_book::cancel_bulk_order<M>> (d, user)
                    };
                    S2..S3 |~ result_of<bulk_order_types::destroy_bulk_order<M>> (c)
                };
                S3..S4 |~ result_of<bulk_order_types::destroy_bulk_order_request<M>> (b_0)
            };
            exists x: u64: x < len(a_3) && !in_range(a_3, x)
        });
        aborts_if [inferred = sathard]({
            let (a_0, a_1, a_2, a_3, a_4, a_5, a_6) = {
                let (b_0, b_1, b_2, b_3) = {
                    let c = {
                        let d =
                            ..S1 |~ result_of<market_types::get_order_book_mut<M>> (market);
                        S1..S2 |~ result_of<order_book::cancel_bulk_order<M>> (d, user)
                    };
                    S2..S3 |~ result_of<bulk_order_types::destroy_bulk_order<M>> (c)
                };
                S3..S4 |~ result_of<bulk_order_types::destroy_bulk_order_request<M>> (b_0)
            };
            exists x: u64: x < len(a_3) && !in_range(a_2, x)
        });
        aborts_if [inferred = sathard]({
            let (a_0, a_1, a_2, a_3, a_4, a_5, a_6) = {
                let (b_0, b_1, b_2, b_3) = {
                    let c = {
                        let d =
                            ..S1 |~ result_of<market_types::get_order_book_mut<M>> (market);
                        S1..S2 |~ result_of<order_book::cancel_bulk_order<M>> (d, user)
                    };
                    S2..S3 |~ result_of<bulk_order_types::destroy_bulk_order<M>> (c)
                };
                S3..S4 |~ result_of<bulk_order_types::destroy_bulk_order_request<M>> (b_0)
            };
            len(a_5) == 18446744073709551616
        });
        aborts_if [inferred = sathard]({
            let (a_0, a_1, a_2, a_3, a_4, a_5, a_6) = {
                let (b_0, b_1, b_2, b_3) = {
                    let c = {
                        let d =
                            ..S1 |~ result_of<market_types::get_order_book_mut<M>> (market);
                        S1..S2 |~ result_of<order_book::cancel_bulk_order<M>> (d, user)
                    };
                    S2..S3 |~ result_of<bulk_order_types::destroy_bulk_order<M>> (c)
                };
                S3..S4 |~ result_of<bulk_order_types::destroy_bulk_order_request<M>> (b_0)
            };
            exists x: u64: x >= len(a_3) && x < len(a_5) && !in_range(a_5, x)
        });
        aborts_if [inferred = sathard]({
            let (a_0, a_1, a_2, a_3, a_4, a_5, a_6) = {
                let (b_0, b_1, b_2, b_3) = {
                    let c = {
                        let d =
                            ..S1 |~ result_of<market_types::get_order_book_mut<M>> (market);
                        S1..S2 |~ result_of<order_book::cancel_bulk_order<M>> (d, user)
                    };
                    S2..S3 |~ result_of<bulk_order_types::destroy_bulk_order<M>> (c)
                };
                S3..S4 |~ result_of<bulk_order_types::destroy_bulk_order_request<M>> (b_0)
            };
            exists x: u64: x >= len(a_3) && x < len(a_5) && !in_range(a_4, x)
        });
    }
}
