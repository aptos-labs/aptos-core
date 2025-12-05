// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_native_interface::{
    SafeNativeError,
    SafeNativeResult,
};
use move_vm_types::{
    values::{Struct, Value},
};
use move_core_types::{
    account_address::AccountAddress};

pub mod abort_codes {
    pub const ECANNOT_PARSE_VARIANT: u64 = 1;
    pub const EORDER_ALREADY_EXISTS: u64 = 2;
    pub const EINVALID_MAKER_ORDER: u64 = 3;
}


#[derive(Debug, Clone)]
pub(crate) enum TriggerCondition {
    PriceMoveAbove(u64),
    PriceMoveBelow(u64),
    TimeBased(u64)
}

/// Order time in force
#[derive(Debug, Clone)]
pub(crate) enum TimeInForce {
    /// Good till cancelled order type
    GTC,
    /// Post Only order type - ensures that the order is not a taker order
    POST_ONLY,
    /// Immediate or Cancel order type - ensures that the order is a taker order. Try to match as much of the
    /// order as possible as taker order and cancel the rest.
    IOC
}

impl TimeInForce {
    pub(crate) fn parse(value: Struct) -> SafeNativeResult<TimeInForce> {
        let (tag, _values) = value.unpack_with_tag()?;
        match tag {
            0 => Ok(TimeInForce::GTC),
            1 => Ok(TimeInForce::POST_ONLY),
            2 => Ok(TimeInForce::IOC),
            _ => Err(SafeNativeError::Abort { abort_code: abort_codes::ECANNOT_PARSE_VARIANT }),
        }
    }

    pub(crate) fn pack(self: &Self) -> Struct {
        match self {
            TimeInForce::GTC => Struct::pack_variant(0, vec![]),
            TimeInForce::POST_ONLY => Struct::pack_variant(1, vec![]),
            TimeInForce::IOC => Struct::pack_variant(2, vec![]),
        }
    }
}

pub(crate) struct SingleOrderRequest {
    pub(crate) account: AccountAddress,
    pub(crate) order_id: OrderIdType,
    pub(crate) client_order_id: Option<String>,
    pub(crate) price: u64,
    pub(crate) orig_size: u64,
    pub(crate) remaining_size: u64,
    pub(crate) is_bid: bool,
    pub(crate) trigger_condition: Option<TriggerCondition>,
    pub(crate) time_in_force: TimeInForce,
    pub(crate) metadata: Value
}

#[derive(Debug, Clone)]
pub(crate) struct SingleOrder {
    pub(crate) account: AccountAddress,
    pub(crate) order_id: OrderIdType,
    pub(crate) client_order_id: Option<String>,
    pub(crate) unique_priority_idx: UniqueIdxType,
    pub(crate) price: u64,
    pub(crate) orig_size: u64,
    pub(crate) remaining_size: u64,
    pub(crate) is_bid: bool,
    pub(crate) trigger_condition: Option<TriggerCondition>,
    pub(crate) time_in_force: TimeInForce,
    // pub(crate) metadata: Value
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub(crate) struct OrderIdType {
    pub(crate) order_id: u128
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub(crate) struct AccountClientOrderId {
    pub(crate) account: AccountAddress,
    pub(crate) client_order_id: String
}

// Internal type representing order in which trades are placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub(crate) struct UniqueIdxType {
    pub(crate) idx: u128
}

impl UniqueIdxType {
    pub(crate) fn descending_idx(self: &Self) -> Self {
        Self { idx: u128::MAX - self.idx }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum OrderType {
    SingleOrder,
    BulkOrder,
}

impl OrderType {
    pub(crate) fn single_order_type() -> OrderType {
        OrderType::SingleOrder
    }

    pub(crate) fn bulk_order_type() -> OrderType {
        OrderType::BulkOrder
    }

    pub(crate) fn is_bulk_order_type(self: &Self) -> bool {
        match self {
            OrderType::BulkOrder => true,
            OrderType::SingleOrder => false,
        }
    }

    pub(crate) fn is_single_order_type(self: &Self) -> bool {
        match self {
            OrderType::SingleOrder => true,
            OrderType::BulkOrder => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub(crate) struct PriceTime  {
    pub(crate) price: u64,
    pub(crate) tie_breaker: UniqueIdxType
}

#[derive(Debug, Clone)]
pub(crate) struct OrderData {
    pub(crate) order_id: OrderIdType,
    // Used to track either the order is a single order or a bulk order
    pub(crate) order_book_type: OrderType,
    pub(crate) size: u64
}


#[derive(Debug, Clone)]
pub(crate) struct OrderWithState {
    pub(crate) order: SingleOrder,
    pub(crate) is_active: bool // i.e. where to find it.
}

/// Represents the details of a matched order.
///
/// Contains information about an order that was matched, including its
/// identifier, account, priority index, price, sizes, and side.
///
/// # Fields:
/// - `order_id`: Unique identifier for the order
/// - `account`: Account that placed the order
/// - `unique_priority_idx`: Priority index for time-based ordering
/// - `price`: Price at which the order was matched
/// - `orig_size`: Original size of the order
/// - `remaining_size`: Remaining size after the match
/// - `is_bid`: True if this was a bid order, false if ask order
pub(crate) enum OrderMatchDetails {
    SingleOrder {
        order_id: OrderIdType,
        account: AccountAddress,
        client_order_id: Option<String>, // for client to track orders
        unique_priority_idx: UniqueIdxType,
        price: u64,
        orig_size: u64,
        remaining_size: u64,
        is_bid: bool,
        time_in_force: TimeInForce,
        // metadata: M,
    },
    BulkOrder {
        order_id: OrderIdType,
        account: AccountAddress,
        unique_priority_idx: UniqueIdxType,
        price: u64,
        remaining_size: u64,
        is_bid: bool,
        sequence_number: u64,
        // metadata: M,
    }
}

/// Represents a single match between a taker order and a maker order.
///
/// Contains the matched order details and the size that was matched in this
/// particular match operation.
///
/// # Fields:
/// - `order`: The matched order result
/// - `matched_size`: The size that was matched in this operation
pub(crate) struct OrderMatch {
    pub(crate) order: OrderMatchDetails,
    pub(crate) matched_size: u64
}

pub(crate) struct ActiveMatchedOrder {
    pub(crate) order_id: OrderIdType,
    pub(crate) matched_size: u64,
    /// Remaining size of the maker order
    pub(crate) remaining_size: u64,
    pub(crate) order_book_type: OrderType,
}

pub(crate) fn parse_order_id(value: Struct) -> SafeNativeResult<OrderIdType> {
    let order_id = value.unpack()?.next().unwrap().value_as::<u128>()?;
    Ok(OrderIdType { order_id })
}

pub(crate) fn parse_trigger_condition(value: Struct) -> SafeNativeResult<TriggerCondition> {
    let (tag, mut values) = value.unpack_with_tag()?;
    match tag {
        0 => Ok(TriggerCondition::PriceMoveAbove(values.next().unwrap().value_as::<u64>()?)),
        1 => Ok(TriggerCondition::PriceMoveBelow(values.next().unwrap().value_as::<u64>()?)),
        2 => Ok(TriggerCondition::TimeBased(values.next().unwrap().value_as::<u64>()?)),
        _ => Err(SafeNativeError::Abort { abort_code: abort_codes::ECANNOT_PARSE_VARIANT }),
    }
}

pub(crate) fn parse_option<T>(value: Struct, f: impl Fn(Struct) -> SafeNativeResult<T>) -> SafeNativeResult<Option<T>> {
    let (tag, mut values) = value.unpack_with_tag()?;
    match tag {
        0 => Ok(Option::None),
        1 => Ok(Option::Some(f(values.next().unwrap().value_as::<Struct>()?)?)),
        _ => Err(SafeNativeError::Abort { abort_code: abort_codes::ECANNOT_PARSE_VARIANT }),
    }
}

pub(crate) fn parse_string(value: Struct) -> SafeNativeResult<String> {
    let bytes = value.unpack()?.next().unwrap();
    Ok(std::str::from_utf8(&bytes.value_as::<Vec<u8>>()?).unwrap().to_string())
}

pub(crate) fn parse_single_order_request(order_req: Struct) -> SafeNativeResult<SingleOrderRequest> {
    let mut values = order_req.unpack_variant(0, |_v| "V1".to_string())?;

    let account = values.next().unwrap().value_as::<AccountAddress>()?;
    let order_id = parse_order_id(values.next().unwrap().value_as::<Struct>()?)?;
    let client_order_id = parse_option(values.next().unwrap().value_as::<Struct>()?, |v| parse_string(v))?;
    let price = values.next().unwrap().value_as::<u64>()?;
    let orig_size = values.next().unwrap().value_as::<u64>()?;
    let remaining_size = values.next().unwrap().value_as::<u64>()?;
    let is_bid = values.next().unwrap().value_as::<bool>()?;
    let trigger_condition = parse_option(values.next().unwrap().value_as::<Struct>()?, |v| parse_trigger_condition(v))?;
    let time_in_force = TimeInForce::parse(values.next().unwrap().value_as::<Struct>()?)?;
    let metadata = values.next().unwrap();
    Ok(SingleOrderRequest {
        account,
        order_id,
        client_order_id,
        price,
        orig_size,
        remaining_size,
        is_bid,
        trigger_condition,
        time_in_force,
        metadata,
    })
}


pub(crate) fn pack_option(value: Option<Value>) -> Value {
    match value {
        Option::None => Value::struct_(Struct::pack_variant(0, vec![])),
        Option::Some(value) => Value::struct_(Struct::pack_variant(1, vec![value])),
    }
}

pub(crate) fn pack_string_value(value: String) -> Value {
    Value::struct_(Struct::pack(vec![Value::vector_u8(value.into_bytes())]))
}

pub(crate) fn pack_move_order_match(order_match: OrderMatch) -> Struct {
    Struct::pack_variant(0, vec![
        Value::struct_(pack_order_match_details(order_match.order)),
        Value::u64(order_match.matched_size),
    ])
}

pub(crate) fn pack_order_match_details(order_match_details: OrderMatchDetails) -> Struct {
    match order_match_details {
        OrderMatchDetails::SingleOrder { order_id, account, client_order_id, unique_priority_idx, price, orig_size, remaining_size, is_bid, time_in_force } => {
            Struct::pack_variant(0, vec![
                Value::address(account),
                Value::u128(order_id.order_id),
                pack_option(client_order_id.map(|v| pack_string_value(v))),
                Value::u128(unique_priority_idx.idx),
                Value::u64(price),
                Value::u64(orig_size),
                Value::u64(remaining_size),
                Value::bool(is_bid),
                Value::struct_(time_in_force.pack()),
                Value::struct_(Struct::pack(vec![])), // empty metadata
            ])
        }
        OrderMatchDetails::BulkOrder { order_id, account, unique_priority_idx, price, remaining_size, is_bid, sequence_number } => {
            Struct::pack_variant(1, vec![
                Value::address(account),
                Value::u128(order_id.order_id),
                Value::u128(unique_priority_idx.idx),
                Value::u64(price),
                Value::u64(remaining_size),
                Value::bool(is_bid),
                Value::u64(sequence_number),
                Value::struct_(Struct::pack(vec![])), // empty metadata
            ])
        }
    }
}
