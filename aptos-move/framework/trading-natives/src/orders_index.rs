// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult, safely_get_struct_variant_field_as, safely_pop_arg
};
use move_vm_runtime::{ native_functions::NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use once_cell::sync::Lazy;
use smallvec::{smallvec, SmallVec};
use std::{collections::{BTreeMap, VecDeque}, sync::Mutex};
use move_vm_types::values::Reference;
use crate::order_book_types::{AccountClientOrderId, ActiveMatchedOrder, OrderData, OrderIdType, OrderMatch, OrderMatchDetails, OrderType, OrderWithState, PriceTime, SingleOrder, SingleOrderRequest, TriggerCondition, UniqueIdxType, abort_codes, pack_move_order_match, parse_option, parse_single_order_request, parse_trigger_condition};


/// OrderBook tracking active (i.e. unconditional, immediately executable) limit orders.
///
/// - invariant - all buys are smaller than sells, at all times.
/// - tie_breaker in sells is U128_MAX-value, to make sure largest value in the book
///   that is taken first, is the one inserted first, amongst those with same bid price.
struct PriceTimeIndex {
    buys: BTreeMap<PriceTime, OrderData>,
    sells: BTreeMap<PriceTime, OrderData>
}

impl PriceTimeIndex {
    fn new() -> Self {
        Self {
            buys: BTreeMap::new(),
            sells: BTreeMap::new(),
        }
    }

    fn get_tie_breaker(unique_priority_idx: UniqueIdxType, is_bid: bool) -> UniqueIdxType {
        if is_bid {
            unique_priority_idx.descending_idx()
        } else {
            unique_priority_idx
        }
    }

    fn best_bid_price(self: &Self) -> Option<u64> {
        if self.buys.is_empty() {
            None
        } else {
            Some(self.buys.iter().next_back().unwrap().0.price)
        }
    }

    fn best_ask_price(self: &Self) -> Option<u64> {
        if self.sells.is_empty() {
            None
        } else {
            Some(self.sells.iter().next().unwrap().0.price)
        }
    }

    fn place_maker_order(
        self: &mut Self,
        order_id: OrderIdType,
        order_book_type: OrderType,
        price: u64,
        unique_priority_idx: UniqueIdxType,
        size: u64,
        is_bid: bool
    ) -> SafeNativeResult<()> {
        let tie_breaker = Self::get_tie_breaker(unique_priority_idx, is_bid);
        let key = PriceTime { price, tie_breaker };
        let value = OrderData { order_id, order_book_type, size };
        // Assert that this is not a taker order
        if self.is_taker_order(price, is_bid) {
            return Err(SafeNativeError::Abort { abort_code: abort_codes::EINVALID_MAKER_ORDER });
        }
        if is_bid {
            self.buys.insert(key, value);
        } else {
            self.sells.insert(key, value);
        };
        Ok(())
    }

    fn is_taker_order(self: &Self, price: u64, is_bid: bool) -> bool {
        if is_bid {
            let best_ask_price = self.best_ask_price();
            best_ask_price.is_some() && price >= best_ask_price.unwrap()
        } else {
            let best_bid_price = self.best_bid_price();
            best_bid_price.is_some() && price <= best_bid_price.unwrap()
        }
    }

    fn single_match_with_current_active_order(
        remaining_size: u64,
        cur_key: PriceTime,
        cur_value: OrderData,
        orders: &mut BTreeMap<PriceTime, OrderData>
    ) -> ActiveMatchedOrder {
        let is_cur_match_fully_consumed = cur_value.size <= remaining_size;

        let matched_size_for_this_order =
            if is_cur_match_fully_consumed {
                orders.remove(&cur_key);
                cur_value.size
            } else {
                orders.get_mut(&cur_key).unwrap().size -= remaining_size;
                remaining_size
            };

        ActiveMatchedOrder {
            order_id: cur_value.order_id,
            matched_size: matched_size_for_this_order,  // Matched size on the maker order
            remaining_size: cur_value.size - matched_size_for_this_order, // Remaining size on the maker order
            order_book_type: cur_value.order_book_type
        }
    }

    fn get_single_match_for_buy_order(
        self: &mut PriceTimeIndex, price: u64, size: u64
    ) -> ActiveMatchedOrder {
        let (smallest_key, smallest_value) = self.sells.first_key_value().unwrap();
        assert!(price >= smallest_key.price);
        Self::single_match_with_current_active_order(
            size,
            *smallest_key,
            smallest_value.clone(),
            &mut self.sells
        )
    }

    fn get_single_match_for_sell_order(
        self: &mut PriceTimeIndex, price: u64, size: u64
    ) -> ActiveMatchedOrder {
        let (largest_key, largest_value) = self.buys.last_key_value().unwrap();
        assert!(price <= largest_key.price);
        Self::single_match_with_current_active_order(
            size,
            *largest_key,
            largest_value.clone(),
            &mut self.buys
        )
    }

    fn get_single_match_result(
        self: &mut PriceTimeIndex,
        price: u64,
        size: u64,
        is_bid: bool
    ) -> ActiveMatchedOrder {
        if is_bid {
            self.get_single_match_for_buy_order(price, size)
        } else {
            self.get_single_match_for_sell_order(price, size)
        }
    }
}

struct SingleOrderBook {
    orders: BTreeMap<OrderIdType, OrderWithState>,
    client_order_ids: BTreeMap<AccountClientOrderId, OrderIdType>,
    // pending_orders: PendingOrderBookIndex
}


impl SingleOrderBook {
    fn new() -> Self {
        Self {
            orders: BTreeMap::new(),
            client_order_ids: BTreeMap::new(),
            // pending_orders: PendingOrderBookIndex::new(),
        }
    }

    /// Returns a single match for a taker order. It is responsibility of the caller to first call the `is_taker_order`
    /// API to ensure that the order is a taker order before calling this API, otherwise it will abort.
    fn get_single_match_for_taker(
        self: &mut SingleOrderBook,
        active_matched_order: ActiveMatchedOrder,
    ) -> OrderMatch {
        let ActiveMatchedOrder {
            order_id, matched_size, remaining_size, order_book_type
        } = active_matched_order;

        assert!(order_book_type.is_single_order_type());

        let order_with_state = if remaining_size == 0 {
            let mut order = self.orders.remove(&order_id).unwrap();
            order.order.remaining_size = 0;
            order
        } else {
            let order =self.orders.get_mut(&order_id).unwrap();
            order.order.remaining_size = remaining_size;
            order.clone()
        };


        let OrderWithState { order: SingleOrder { account, order_id, client_order_id, unique_priority_idx, price, orig_size, remaining_size, trigger_condition: _, is_bid, time_in_force }, is_active } = order_with_state;

        assert!(is_active, "EINVALID_INACTIVE_ORDER_STATE");

        if remaining_size == 0 && client_order_id.is_some() {
            self.client_order_ids.remove(&AccountClientOrderId { account, client_order_id: client_order_id.clone().unwrap() });
        };
        OrderMatch {
            order: OrderMatchDetails::SingleOrder {
                order_id,
                account,
                client_order_id,
                unique_priority_idx,
                price,
                orig_size,
                remaining_size, is_bid, time_in_force }, matched_size }
    }

}

struct OrdersIndexNativeState {

    next_order_id: u128,

    single_order_book: SingleOrderBook,
    // bulk_order_book: BulkOrderBook,
    price_time_idx: PriceTimeIndex,

}

impl OrdersIndexNativeState {
    fn new() -> Self {
        Self {
            next_order_id: 0,
            single_order_book: SingleOrderBook::new(),
            // bulk_order_book: BulkOrderBook::new(),
            price_time_idx: PriceTimeIndex::new(),
        }
    }

    /// Places a maker order to the order book. If the order is a pending order, it is added to the pending order book
    /// else it is added to the active order book. The API aborts if it's not a maker order or if the order already exists
    fn place_maker_or_pending_order(self: &mut Self, order_req: SingleOrderRequest) -> SafeNativeResult<()> {
        let ascending_idx = UniqueIdxType { idx: self.next_order_id };
        self.next_order_id += 1;

        if order_req.trigger_condition.is_some() {
            // do something
        } else {
            self.place_ready_maker_order_with_unique_idx(order_req, ascending_idx)?;
        }
        Ok(())
    }

    fn place_ready_maker_order_with_unique_idx(self: &mut Self, order_req: SingleOrderRequest, ascending_idx: UniqueIdxType) -> SafeNativeResult<()> {
        let order = SingleOrder {
            order_id: order_req.order_id,
            account: order_req.account,
            unique_priority_idx: ascending_idx,
            client_order_id: order_req.client_order_id.clone(),
            price: order_req.price,
            orig_size: order_req.orig_size,
            remaining_size: order_req.remaining_size,
            is_bid: order_req.is_bid,
            trigger_condition: order_req.trigger_condition,
            time_in_force: order_req.time_in_force,
            // metadata: order_req.metadata
        };

        let entry = self.single_order_book.orders.entry(order_req.order_id);
        match entry {
            std::collections::btree_map::Entry::Occupied(_occupied) => {
                return Err(SafeNativeError::Abort { abort_code: abort_codes::EORDER_ALREADY_EXISTS });
            }
            std::collections::btree_map::Entry::Vacant(vacant) => {
                vacant.insert(OrderWithState { order, is_active: true });
            }
        };
        if let Some(client_order_id) = order_req.client_order_id {
            self.single_order_book.client_order_ids.insert(
                AccountClientOrderId {
                    account: order_req.account,
                    client_order_id,
                },
                order_req.order_id
            );
        };
        self.price_time_idx.place_maker_order(
            order_req.order_id,
            OrderType::single_order_type(),
            order_req.price,
            ascending_idx,
            order_req.remaining_size,
            order_req.is_bid
        )
    }

    fn is_taker_order(self: &Self, price: u64, is_bid: bool, trigger_condition: Option<TriggerCondition>) -> bool {
        if trigger_condition.is_some() {
            return false;
        };
        return self.price_time_idx.is_taker_order(price, is_bid)
    }

    fn get_single_match_for_taker(self: &mut Self, price: u64, size: u64, is_bid: bool) -> OrderMatch {
        let result = self.price_time_idx.get_single_match_result(price, size, is_bid);
        if result.order_book_type.is_single_order_type() {
            self.single_order_book.get_single_match_for_taker(result)
        } else {
            unimplemented!("Bulk order matching is not implemented yet");
            // self.bulk_order_book.get_single_match_for_taker(&mut self.price_time_idx, result, is_bid)
        }
    }
}

struct OrdersIndexesNativeState {
    orders_indexes: BTreeMap<u128, OrdersIndexNativeState>,
}

impl OrdersIndexesNativeState {
    fn get_orders_index<'s>(&'s mut self, id: u128, version: u64) -> &'s OrdersIndexNativeState {
        self.orders_indexes.entry(id).or_insert_with(|| {
            assert!(version == 0);
            OrdersIndexNativeState::new()
        })
    }


    fn get_mut_orders_index<'s>(&'s mut self, id: u128, version: u64) -> &'s mut OrdersIndexNativeState {
        self.orders_indexes.entry(id).or_insert_with(|| {
            assert!(version == 0);
            OrdersIndexNativeState::new()
        })
    }

    fn get_mut_order_index_and_table<'s>(&'s mut self, orders_index_ref: StructRef) -> SafeNativeResult<(&'s mut OrdersIndexNativeState, ())> {
        let id = safely_get_struct_variant_field_as!(orders_index_ref, &[0], 0, "id", u128);
        let version = safely_get_struct_variant_field_as!(orders_index_ref, &[0], 1, "version", u64);
        // get table
        Ok((self.get_mut_orders_index(id, version), ()))
    }

    fn get_order_index_and_table<'s>(&'s mut self, orders_index_ref: StructRef) -> SafeNativeResult<(&'s OrdersIndexNativeState, ())> {
        let id = safely_get_struct_variant_field_as!(orders_index_ref, &[0], 0, "id", u128);
        let version = safely_get_struct_variant_field_as!(orders_index_ref, &[0], 1, "version", u64);
        // get table
        Ok((self.get_mut_orders_index(id, version), ()))
    }
}

static ORDERS_INDEXES_NATIVE_STATE: Lazy<Mutex<OrdersIndexesNativeState>> = Lazy::new(|| Mutex::new(OrdersIndexesNativeState { orders_indexes: BTreeMap::new() }));


/***************************************************************************************************
 * native fun place_maker_order<M: store + copy + drop>(
 *    self: &mut OrdersIndex<M>, order_req: SingleOrderRequest<M>
 * )
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
 fn native_place_maker_order(
    _context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);
    debug_assert_eq!(ty_args.len(), 1);

    let mut locked = ORDERS_INDEXES_NATIVE_STATE.lock().unwrap();

    let order_req = safely_pop_arg!(args, Struct);
    let orders_index_ref = safely_pop_arg!(args, StructRef);

    let (orders_index, _table) = locked.get_mut_order_index_and_table(orders_index_ref)?;

    orders_index.place_maker_or_pending_order(parse_single_order_request(order_req)?)?;

    Ok(smallvec![])
}

/***************************************************************************************************
 * Checks if the order is a taker order i.e., matched immediately with the active order book.
 *
 * native fun is_taker_order<M: store + copy + drop>(
 *        self: &OrdersIndex<M>,
 *        price: u64,
 *        is_bid: bool,
 *        trigger_condition: Option<TriggerCondition>
 *     ): bool;
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_is_taker_order(
    _context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 4);
    debug_assert_eq!(ty_args.len(), 1);

    let mut locked = ORDERS_INDEXES_NATIVE_STATE.lock().unwrap();

    let trigger_condition = parse_option(safely_pop_arg!(args, Struct), |v| parse_trigger_condition(v))?;
    let is_bid = safely_pop_arg!(args, bool);
    let price = safely_pop_arg!(args, u64);

    println!("price: {}, is_bid: {}, trigger: {:?}", price, is_bid, trigger_condition);

    let orders_index_ref = safely_pop_arg!(args, StructRef);
    let (orders_index, _table) = locked.get_order_index_and_table(orders_index_ref)?;

    Ok(smallvec![Value::bool(orders_index.is_taker_order(price, is_bid, trigger_condition))])
}


/***************************************************************************************************
 * Checks if the order is a taker order i.e., matched immediately with the active order book.
 *
 * native fun get_single_match_for_taker<M: store + copy + drop>(
 *        self: &OrdersIndex<M>,
 *        price: u64,
 *        size: u64,
 *        is_bid: bool,
 *     ): OrderMatch;
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_get_single_match_for_taker(
    _context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 4);
    debug_assert_eq!(ty_args.len(), 1);

    let mut locked = ORDERS_INDEXES_NATIVE_STATE.lock().unwrap();

    let is_bid = safely_pop_arg!(args, bool);
    let size = safely_pop_arg!(args, u64);
    let price = safely_pop_arg!(args, u64);

    let orders_index_ref = safely_pop_arg!(args, StructRef);
    let (orders_index, _table) = locked.get_mut_order_index_and_table(orders_index_ref)?;

    Ok(smallvec![Value::struct_(pack_move_order_match(orders_index.get_single_match_for_taker(price, size, is_bid)))])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("place_maker_order", native_place_maker_order as RawSafeNative),
        ("is_taker_order", native_is_taker_order as RawSafeNative),
        ("get_single_match_for_taker", native_get_single_match_for_taker as RawSafeNative),
    ];

    builder.make_named_natives(natives)
}
