/// Types crossing the `native_order_book` Move ↔ native boundary. The
/// metadata type `M` is last in each value, so the Rust side decodes the
/// header fields and keeps the trailing bytes, which are the BCS of `M`.
module aptos_experimental::native_order_types {
    use std::option::Option;
    use std::string::String;
    use aptos_trading::order_book_types::{OrderId, IncreasingIdx, TimeInForce, TriggerCondition};

    /// A single order, as stored natively. Pending iff `trigger_condition`
    /// is set.
    enum NativeSingleOrder<M: store + copy + drop> has store, copy, drop {
        V1 {
            account: address,
            order_id: OrderId,
            client_order_id: Option<String>,
            price: u64,
            orig_size: u64,
            remaining_size: u64,
            is_bid: bool,
            trigger_condition: Option<TriggerCondition>,
            time_in_force: TimeInForce,
            creation_time_micros: u64,
            unique_priority_idx: IncreasingIdx,
            metadata: M,
        }
    }

    /// A bulk order, as stored natively. Price levels are best first.
    enum NativeBulkOrder<M: store + copy + drop> has store, copy, drop {
        V1 {
            account: address,
            order_id: OrderId,
            sequence_number: u64,
            bid_prices: vector<u64>,
            bid_sizes: vector<u64>,
            ask_prices: vector<u64>,
            ask_sizes: vector<u64>,
            unique_priority_idx: IncreasingIdx,
            creation_time_micros: u64,
            metadata: M,
        }
    }

    public fun new_single_order_v1<M: store + copy + drop>(
        account: address,
        order_id: OrderId,
        client_order_id: Option<String>,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        trigger_condition: Option<TriggerCondition>,
        time_in_force: TimeInForce,
        creation_time_micros: u64,
        unique_priority_idx: IncreasingIdx,
        metadata: M,
    ): NativeSingleOrder<M> {
        NativeSingleOrder::V1 {
            account,
            order_id,
            client_order_id,
            price,
            orig_size,
            remaining_size,
            is_bid,
            trigger_condition,
            time_in_force,
            creation_time_micros,
            unique_priority_idx,
            metadata,
        }
    }

    public fun new_bulk_order_v1<M: store + copy + drop>(
        account: address,
        order_id: OrderId,
        sequence_number: u64,
        bid_prices: vector<u64>,
        bid_sizes: vector<u64>,
        ask_prices: vector<u64>,
        ask_sizes: vector<u64>,
        unique_priority_idx: IncreasingIdx,
        creation_time_micros: u64,
        metadata: M,
    ): NativeBulkOrder<M> {
        NativeBulkOrder::V1 {
            account,
            order_id,
            sequence_number,
            bid_prices,
            bid_sizes,
            ask_prices,
            ask_sizes,
            unique_priority_idx,
            creation_time_micros,
            metadata,
        }
    }
}
