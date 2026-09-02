spec aptos_experimental::bulk_order_utils {

    spec discard_price_crossing_levels(
        prices: &vector<u64>, best_price: 0x1::option::Option<u64>, is_bid: bool
    ): u64 {
        use 0x1::option;
        pragma opaque = true;
        // `result` is the first price level that no longer crosses the
        // opposite-side best price, or the vector length when every level
        // crosses. The prefix and boundary clauses determine that index.
        ensures [inferred]!option::is_some<u64>(best_price) ==> result == 0;
        ensures [inferred] option::is_some<u64>(best_price) ==>
            result <= len(prices);
        ensures [inferred] option::is_some<u64>(best_price) ==>
            (
                forall x in 0..result:
                    is_bid ==>
                        prices[x] >= option::destroy_some<u64>(best_price)
            );
        ensures [inferred] option::is_some<u64>(best_price) ==>
            (
                forall x in 0..result:
                    !is_bid ==>
                        prices[x] <= option::destroy_some<u64>(best_price)
            );
        ensures [inferred] option::is_some<u64>(best_price) && result < len(prices) ==>
            (is_bid ==>
                prices[result] < option::destroy_some<u64>(best_price));
        ensures [inferred] option::is_some<u64>(best_price) && result < len(prices) ==>
            (!is_bid ==>
                prices[result] > option::destroy_some<u64>(best_price));
        aborts_if [inferred] false;
    }

    spec match_order_and_get_next_from_bulk_order<M: copy + drop + store>(
        order: &mut 0x5::bulk_order_types::BulkOrder<M>, is_bid: bool, matched_size: u64
    ): (0x1::option::Option<u64>, 0x1::option::Option<u64>) {
        use 0x1::option;
        use 0x5::bulk_order_types;
        pragma opaque = true;
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local52: bool:
                forall _local53: bool:
                    matched_size > a_1[0]
                        && (_local52
                            && !_local53) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                >> (order)
                            )
        });
        ensures [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0]
                && (a_1[0] != 0
                    && len(a_1) == 0) ==>
                result_1 == option::none<u64>()
        });
        ensures [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0]
                && (a_1[0] != 0
                    && len(a_1) == 0) ==>
                result_2 == option::none<u64>()
        });
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local34: bool:
                forall _local35: bool:
                    matched_size <= a_1[0]
                        && (a_1[0] != 0
                            && (len(a_1) == 0
                                && (!_local34
                                    && _local35))) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                update_field(
                                    update_field(
                                        update_field(
                                            ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                            >> (order),
                                            bid_prices,
                                            a_0
                                        ),
                                        ask_prices,
                                        a_0
                                    ),
                                    ask_sizes,
                                    a_1
                                )
                            )
        });
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local34: bool:
                forall _local35: bool:
                    matched_size <= a_1[0]
                        && (a_1[0] != 0
                            && (len(a_1) == 0
                                && (_local34
                                    && _local35))) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                update_field(
                                    update_field(
                                        update_field(
                                            update_field(
                                                ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                                >> (order),
                                                bid_prices,
                                                a_0
                                            ),
                                            ask_prices,
                                            a_0
                                        ),
                                        bid_sizes,
                                        a_1
                                    ),
                                    ask_sizes,
                                    a_1
                                )
                            )
        });
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local34: bool:
                forall _local35: bool:
                    matched_size <= a_1[0]
                        && (a_1[0] != 0
                            && (len(a_1) == 0
                                && (_local34
                                    && !_local35))) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                update_field(
                                    update_field(
                                        update_field(
                                            ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                            >> (order),
                                            bid_prices,
                                            a_0
                                        ),
                                        ask_prices,
                                        a_0
                                    ),
                                    bid_sizes,
                                    a_1
                                )
                            )
        });
        ensures [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0]
                && (a_1[0] != 0
                    && len(a_1) != 0) ==>
                result_1 == option::some<u64>(a_0[0])
        });
        ensures [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0]
                && (a_1[0] != 0
                    && len(a_1) != 0) ==>
                result_2 == option::some<u64>(a_1[0])
        });
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local47: bool:
                forall _local48: bool:
                    matched_size <= a_1[0]
                        && (a_1[0] != 0
                            && (len(a_1) != 0
                                && (!_local47
                                    && _local48))) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                update_field(
                                    update_field(
                                        update_field(
                                            ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                            >> (order),
                                            bid_prices,
                                            a_0
                                        ),
                                        ask_prices,
                                        a_0
                                    ),
                                    ask_sizes,
                                    a_1
                                )
                            )
        });
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local47: bool:
                forall _local48: bool:
                    matched_size <= a_1[0]
                        && (a_1[0] != 0
                            && (len(a_1) != 0
                                && (_local47
                                    && _local48))) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                update_field(
                                    update_field(
                                        update_field(
                                            update_field(
                                                ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                                >> (order),
                                                bid_prices,
                                                a_0
                                            ),
                                            ask_prices,
                                            a_0
                                        ),
                                        bid_sizes,
                                        a_1
                                    ),
                                    ask_sizes,
                                    a_1
                                )
                            )
        });
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local47: bool:
                forall _local48: bool:
                    matched_size <= a_1[0]
                        && (a_1[0] != 0
                            && (len(a_1) != 0
                                && (_local47
                                    && !_local48))) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                update_field(
                                    update_field(
                                        update_field(
                                            ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                            >> (order),
                                            bid_prices,
                                            a_0
                                        ),
                                        ask_prices,
                                        a_0
                                    ),
                                    bid_sizes,
                                    a_1
                                )
                            )
        });
        ensures [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0]
                && (a_1[0] == 0
                    && len(concat(a_1[0..0], a_1[1..len(a_1)])) == 0) ==>
                result_1 == option::none<u64>()
        });
        ensures [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0]
                && (a_1[0] == 0
                    && len(concat(a_1[0..0], a_1[1..len(a_1)])) == 0) ==>
                result_2 == option::none<u64>()
        });
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local34: bool:
                forall _local35: bool:
                    matched_size <= a_1[0]
                        && (
                            a_1[0] == 0
                                && (
                                    len(concat(a_1[0..0], a_1[1..len(a_1)])) == 0
                                        && (!_local34
                                            && _local35)
                                )
                        ) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                update_field(
                                    update_field(
                                        update_field(
                                            ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                            >> (order),
                                            bid_prices,
                                            concat(a_0[0..0], a_0[1..len(a_0)])
                                        ),
                                        ask_prices,
                                        concat(a_0[0..0], a_0[1..len(a_0)])
                                    ),
                                    ask_sizes,
                                    concat(a_1[0..0], a_1[1..len(a_1)])
                                )
                            )
        });
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local34: bool:
                forall _local35: bool:
                    matched_size <= a_1[0]
                        && (
                            a_1[0] == 0
                                && (
                                    len(concat(a_1[0..0], a_1[1..len(a_1)])) == 0
                                        && (_local34
                                            && _local35)
                                )
                        ) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                update_field(
                                    update_field(
                                        update_field(
                                            update_field(
                                                ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                                >> (order),
                                                bid_prices,
                                                concat(a_0[0..0], a_0[1..len(a_0)])
                                            ),
                                            ask_prices,
                                            concat(a_0[0..0], a_0[1..len(a_0)])
                                        ),
                                        bid_sizes,
                                        concat(a_1[0..0], a_1[1..len(a_1)])
                                    ),
                                    ask_sizes,
                                    concat(a_1[0..0], a_1[1..len(a_1)])
                                )
                            )
        });
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local34: bool:
                forall _local35: bool:
                    matched_size <= a_1[0]
                        && (
                            a_1[0] == 0
                                && (
                                    len(concat(a_1[0..0], a_1[1..len(a_1)])) == 0
                                        && (_local34
                                            && !_local35)
                                )
                        ) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                update_field(
                                    update_field(
                                        update_field(
                                            ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                            >> (order),
                                            bid_prices,
                                            concat(a_0[0..0], a_0[1..len(a_0)])
                                        ),
                                        ask_prices,
                                        concat(a_0[0..0], a_0[1..len(a_0)])
                                    ),
                                    bid_sizes,
                                    concat(a_1[0..0], a_1[1..len(a_1)])
                                )
                            )
        });
        ensures [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0]
                && (a_1[0] == 0
                    && len(concat(a_1[0..0], a_1[1..len(a_1)])) != 0) ==>
                result_1
                    == option::some<u64>(concat(a_0[0..0], a_0[1..len(a_0)])[0])
        });
        ensures [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0]
                && (a_1[0] == 0
                    && len(concat(a_1[0..0], a_1[1..len(a_1)])) != 0) ==>
                result_2
                    == option::some<u64>(concat(a_1[0..0], a_1[1..len(a_1)])[0])
        });
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local47: bool:
                forall _local48: bool:
                    matched_size <= a_1[0]
                        && (
                            a_1[0] == 0
                                && (
                                    len(concat(a_1[0..0], a_1[1..len(a_1)])) != 0
                                        && (!_local47
                                            && _local48)
                                )
                        ) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                update_field(
                                    update_field(
                                        update_field(
                                            ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                            >> (order),
                                            bid_prices,
                                            concat(a_0[0..0], a_0[1..len(a_0)])
                                        ),
                                        ask_prices,
                                        concat(a_0[0..0], a_0[1..len(a_0)])
                                    ),
                                    ask_sizes,
                                    concat(a_1[0..0], a_1[1..len(a_1)])
                                )
                            )
        });
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local47: bool:
                forall _local48: bool:
                    matched_size <= a_1[0]
                        && (
                            a_1[0] == 0
                                && (
                                    len(concat(a_1[0..0], a_1[1..len(a_1)])) != 0
                                        && (_local47
                                            && _local48)
                                )
                        ) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                update_field(
                                    update_field(
                                        update_field(
                                            update_field(
                                                ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                                >> (order),
                                                bid_prices,
                                                concat(a_0[0..0], a_0[1..len(a_0)])
                                            ),
                                            ask_prices,
                                            concat(a_0[0..0], a_0[1..len(a_0)])
                                        ),
                                        bid_sizes,
                                        concat(a_1[0..0], a_1[1..len(a_1)])
                                    ),
                                    ask_sizes,
                                    concat(a_1[0..0], a_1[1..len(a_1)])
                                )
                            )
        });
        ensures [inferred = sathard]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            forall _local47: bool:
                forall _local48: bool:
                    matched_size <= a_1[0]
                        && (
                            a_1[0] == 0
                                && (
                                    len(concat(a_1[0..0], a_1[1..len(a_1)])) != 0
                                        && (_local47
                                            && !_local48)
                                )
                        ) ==>
                        order
                            == update_field(
                                order,
                                order_request,
                                update_field(
                                    update_field(
                                        update_field(
                                            ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M
                                            >> (order),
                                            bid_prices,
                                            concat(a_0[0..0], a_0[1..len(a_0)])
                                        ),
                                        ask_prices,
                                        concat(a_0[0..0], a_0[1..len(a_0)])
                                    ),
                                    bid_sizes,
                                    concat(a_1[0..0], a_1[1..len(a_1)])
                                )
                            )
        });
        aborts_if [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size > a_1[0]
        });
        aborts_if [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0]
                && (a_1[0] != 0
                    && (len(a_1) != 0
                        && !in_range(a_0, 0)))
        });
        aborts_if [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0]
                && (
                    a_1[0] == 0
                        && (
                            len(concat(a_1[0..0], a_1[1..len(a_1)])) != 0
                                && !in_range(concat(a_1[0..0], a_1[1..len(a_1)]), 0)
                        )
                )
        });
        aborts_if [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0]
                && (
                    a_1[0] == 0
                        && (
                            len(concat(a_1[0..0], a_1[1..len(a_1)])) != 0
                                && !in_range(concat(a_0[0..0], a_0[1..len(a_0)]), 0)
                        )
                )
        });
        aborts_if [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0] && (a_1[0] == 0 && !in_range(a_0, 0))
        });
        aborts_if [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            matched_size <= a_1[0] && a_1[0] - matched_size < 0
        });
        aborts_if [inferred]({
            let (a_0, a_1) = {
                let b =
                    ..S1 |~ result_of<bulk_order_types::get_order_request_mut<M>> (order);
                S1.. |~ result_of<bulk_order_types::get_prices_and_sizes_mut<M>> (b, is_bid)
            };
            !in_range(a_1, 0)
        });
    }

    spec new_bulk_order_request_with_sanitization<M: copy + drop + store>(
        account: address,
        sequence_number: u64,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
        metadata: M
    ): 0x5::bulk_order_types::BulkOrderRequest<M> {
        use 0x5::bulk_order_types;
        pragma opaque = true;
        /* Superseded WP clauses: the two loop validators now have exact,
        * body-proved contracts, allowing this single validity
        * predicate rather than path-labelled SAT-hard candidates.
        */
        /*
        ensures [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            let d = S3.. |~ result_of<validate_price_ordering>(ask_prices, false);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) > 0 && (len(bid_prices) <= 30 && (a && (b && (c && (d && len(ask_prices) == 0)))))))) ==> result == bulk_order_types::new_bulk_order_request<M>(account, sequence_number, bid_prices, bid_sizes, ask_prices, ask_sizes, metadata)
        });
        ensures [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            let d = S3.. |~ result_of<validate_price_ordering>(ask_prices, false);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) > 0 && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && (b && (c && (d && (len(ask_prices) > 0 && bid_prices[0] < ask_prices[0])))))))))) ==> result == bulk_order_types::new_bulk_order_request<M>(account, sequence_number, bid_prices, bid_sizes, ask_prices, ask_sizes, metadata)
        });
        ensures [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            let d = S3.. |~ result_of<validate_price_ordering>(ask_prices, false);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) == 0 && (len(ask_prices) > 0 && (len(ask_prices) <= 30 && (a && (b && (c && d)))))))) ==> result == bulk_order_types::new_bulk_order_request<M>(account, sequence_number, bid_prices, bid_sizes, ask_prices, ask_sizes, metadata)
        });
        aborts_if [inferred] sequence_number == 0;
        aborts_if [inferred] sequence_number > 0 && len(bid_prices) != len(bid_sizes);
        aborts_if [inferred] sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && len(ask_prices) != len(ask_sizes));
        aborts_if [inferred] sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && len(bid_prices) > 30));
        aborts_if [inferred] sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) > 0 && (len(bid_prices) <= 30 && len(ask_prices) > 30))));
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) > 0 && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && !a)))))
        });
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) > 0 && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && !b))))))
        });
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) > 0 && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && (b && !c)))))))
        });
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            let d = S3.. |~ result_of<validate_price_ordering>(ask_prices, false);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) > 0 && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && (b && (c && !d))))))))
        });
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            let d = S3.. |~ result_of<validate_price_ordering>(ask_prices, false);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && (b && (c && (d && (len(bid_prices) > 0 && (len(ask_prices) > 0 && bid_prices[0] >= ask_prices[0]))))))))))
        });
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            let d = S3.. |~ result_of<validate_price_ordering>(ask_prices, false);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && (b && (c && (d && (len(bid_prices) > 0 && (len(ask_prices) > 0 && !in_range(ask_prices, 0)))))))))))
        });
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            let d = S3.. |~ result_of<validate_price_ordering>(ask_prices, false);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && (b && (c && (d && (len(bid_prices) > 0 && (len(ask_prices) > 0 && !in_range(bid_prices, 0)))))))))))
        });
        aborts_if [inferred] sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) == 0 && len(ask_prices) == 0)));
        aborts_if [inferred] sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) == 0 && (len(ask_prices) > 0 && len(bid_prices) > 30))));
        aborts_if [inferred] sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) == 0 && (len(bid_prices) <= 30 && len(ask_prices) > 30))));
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) == 0 && (len(ask_prices) > 0 && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && !a))))))
        });
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) == 0 && (len(ask_prices) > 0 && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && !b)))))))
        });
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) == 0 && (len(ask_prices) > 0 && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && (b && !c))))))))
        });
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            let d = S3.. |~ result_of<validate_price_ordering>(ask_prices, false);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) == 0 && (len(ask_prices) > 0 && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && (b && (c && !d)))))))))
        });
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            let d = S3.. |~ result_of<validate_price_ordering>(ask_prices, false);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) == 0 && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && (b && (c && (d && (len(bid_prices) > 0 && (len(ask_prices) > 0 && bid_prices[0] >= ask_prices[0])))))))))))
        });
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            let d = S3.. |~ result_of<validate_price_ordering>(ask_prices, false);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) == 0 && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && (b && (c && (d && (len(bid_prices) > 0 && (len(ask_prices) > 0 && !in_range(ask_prices, 0))))))))))))
        });
        aborts_if [inferred = sathard] ({
            let a = ..S1 |~ result_of<validate_not_zero_sizes>(bid_sizes);
            let b = S1..S2 |~ result_of<validate_not_zero_sizes>(ask_sizes);
            let c = S2..S3 |~ result_of<validate_price_ordering>(bid_prices, true);
            let d = S3.. |~ result_of<validate_price_ordering>(ask_prices, false);
            sequence_number > 0 && (len(bid_prices) == len(bid_sizes) && (len(ask_prices) == len(ask_sizes) && (len(bid_prices) == 0 && (len(bid_prices) <= 30 && (len(ask_prices) <= 30 && (a && (b && (c && (d && (len(bid_prices) > 0 && (len(ask_prices) > 0 && !in_range(bid_prices, 0))))))))))))
        });
        */
        let bids_are_nonzero = result_of<validate_not_zero_sizes>(bid_sizes);
        let asks_are_nonzero = result_of<validate_not_zero_sizes>(ask_sizes);
        let bid_prices_are_ordered = result_of<validate_price_ordering>(bid_prices, true);
        let ask_prices_are_ordered = result_of<validate_price_ordering>(ask_prices, false);
        let valid = sequence_number > 0
            && len(bid_prices) == len(bid_sizes)
            && len(ask_prices) == len(ask_sizes)
            && (len(bid_prices) > 0
                || len(ask_prices) > 0)
            && len(bid_prices) <= 30
            && len(ask_prices) <= 30
            && bids_are_nonzero
            && asks_are_nonzero
            && bid_prices_are_ordered
            && ask_prices_are_ordered
            && (
                len(bid_prices) == 0
                    || len(ask_prices) == 0
                    || bid_prices[0] < ask_prices[0]
            );
        ensures valid ==>
            result
                == bulk_order_types::new_bulk_order_request<M>(
                    account,
                    sequence_number,
                    bid_prices,
                    bid_sizes,
                    ask_prices,
                    ask_sizes,
                    metadata
                );
        aborts_if !valid;
    }

    spec new_bulk_order_with_sanitization<M: copy + drop + store>(
        order_id: 0x5::order_book_types::OrderId,
        unique_priority_idx: 0x5::order_book_types::IncreasingIdx,
        order_req: 0x5::bulk_order_types::BulkOrderRequest<M>,
        best_bid_price: 0x1::option::Option<u64>,
        best_ask_price: 0x1::option::Option<u64>
    ): (
        0x5::bulk_order_types::BulkOrder<M>,
        vector<u64>,
        vector<u64>,
        vector<u64>,
        vector<u64>
    ) {
        use 0x1::timestamp;
        pragma opaque = true, aborts_if_is_partial = false;
        let bid_start = result_of<discard_price_crossing_levels>(
            order_req.bid_prices, best_ask_price, true
        );
        let ask_start = result_of<discard_price_crossing_levels>(
            order_req.ask_prices, best_bid_price, false
        );
        let kept_bid_prices = order_req.bid_prices[bid_start
            ..len(order_req.bid_prices)];
        let kept_bid_sizes = order_req.bid_sizes[bid_start..len(order_req.bid_sizes)];
        let kept_ask_prices = order_req.ask_prices[ask_start
            ..len(order_req.ask_prices)];
        let kept_ask_sizes = order_req.ask_sizes[ask_start..len(order_req.ask_sizes)];
        let sanitized_request = update_field(
            update_field(
                update_field(
                    update_field(order_req, bid_prices, kept_bid_prices),
                    bid_sizes,
                    kept_bid_sizes
                ),
                ask_prices,
                kept_ask_prices
            ),
            ask_sizes,
            kept_ask_sizes
        );
        ensures result_1.order_request == sanitized_request;
        ensures result_1.order_id == order_id;
        ensures result_1.unique_priority_idx == unique_priority_idx;
        ensures result_1.creation_time_micros == timestamp::spec_now_microseconds();
        ensures result_2 == order_req.bid_prices[0..bid_start];
        ensures result_3 == order_req.bid_sizes[0..bid_start];
        ensures result_4 == order_req.ask_prices[0..ask_start];
        ensures result_5 == order_req.ask_sizes[0..ask_start];
        aborts_if !exists<timestamp::CurrentTimeMicroseconds>(@aptos_framework);
        aborts_if bid_start > 0 && bid_start > len(order_req.bid_prices);
        aborts_if bid_start > 0 && bid_start > len(order_req.bid_sizes);
        aborts_if ask_start > 0 && ask_start > len(order_req.ask_prices);
        aborts_if ask_start > 0 && ask_start > len(order_req.ask_sizes);
    }

    spec trim_start<Element>(v: &mut vector<Element>, new_start: u64): vector<Element> {
        use 0x1::vector;
        pragma opaque = true, aborts_if_is_partial = false;
        ensures [inferred] ensures_of<vector::move_range<Element>> (
            v, 0, new_start, vec<Element>(), 0, v, result
        );
        ensures result == old(v)[0..new_start];
        ensures v == old(v)[new_start..len(old(v))];
        aborts_if new_start > len(v);
    }

    spec validate_not_zero_sizes(sizes: &vector<u64>): bool {
        pragma opaque = true;
        ensures result == (forall x in 0..len(sizes): sizes[x] > 0);
        aborts_if false;
    }

    spec validate_price_ordering(
        prices: &vector<u64>, is_descending: bool
    ): bool {
        pragma opaque = true;
        ensures result == prices_are_ordered(prices, is_descending);
        aborts_if false;
    }

    spec fun prices_are_ordered(prices: vector<u64>, is_descending: bool): bool {
        forall x in 0..len(prices) - 1:
            if (is_descending) prices[x] > prices[x + 1]
            else prices[x] < prices[x + 1]
    }
}
