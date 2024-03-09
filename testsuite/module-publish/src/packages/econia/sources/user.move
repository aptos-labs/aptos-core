/// User-side asset, collateral, and order management.
///
/// Contains data structures and functionality for tracking a user's
/// assets and open orders. Upon market account registration, users can
/// either preside over their own account, or delegate custody to a
/// custodian who manage their orders and withdrawals. For each market,
/// a user can open multiple market accounts, each with a unique
/// custodian.
///
/// # General overview sections
///
/// [Architecture](#architecture)
///
/// * [Market account IDs](#market-account-IDs)
/// * [Market accounts](#market-accounts)
/// * [Orders and access keys](#orders-and-access-keys)
/// * [Market order IDs](#market-order-IDs)
///
/// [Function index](#function-index)
///
/// * [View functions](#view-functions)
/// * [Public functions](#public-functions)
/// * [Public entry functions](#public-entry-functions)
/// * [Public friend functions](#public-friend-functions)
/// * [Dependency charts](#dependency-charts)
///
/// [Complete DocGen index](#complete-docgen-index)
///
/// # Architecture
///
/// ## Market account IDs
///
/// Markets, defined in the global registry, are assigned a 1-indexed
/// `u64` market ID, as are custodians. The concatenated result of a
/// market ID and a custodian ID is known as a market account ID, which
/// is used as a key in assorted user-side lookup operations: the 64
/// least-significant bits in a market account ID are the custodian ID
/// for the given market account (`NIL` if no delegated custodian),
/// while the 64 most-significant bits are the market ID. See
/// `get_custodian_id()`, `get_market_account_id()`, and
/// `get_market_id()` for implementation details.
///
/// ## Market accounts
///
/// When a user opens a market account, a `MarketAccount` entry is
/// added to their `MarketAccounts`, and a coin entry is added to their
/// `Collateral` for the given market's quote coin type. If the market's
/// base asset is a coin, a `Collateral` entry is similarly created for
/// the base coin type.
///
/// ## Orders and access keys
///
/// When users place an order on the order book, an `Order` is added to
/// their corresponding `MarketAccount`. If they then cancel the order,
/// the corresponding `Order` is not deallocated, but rather, marked
/// "inactive" and pushed onto a stack of inactive orders for the
/// corresponding side (`MarketAccount.asks_stack_top` or
/// `MarketAccount.bids_stack_top`). Then, when a user places another
/// order, rather than allocating a new `Order`, the inactive order at
/// the top of the stack is popped off the stack and marked active.
///
/// This approach is motivated by global storage gas costs: as of the
/// time of this writing, per-item creations cost approximately 16.7
/// times as much as per-item writes, and there is no incentive to
/// deallocate from memory. Hence the inactive stack paradigm allows
/// for orders to be recycled in a way that reduces overall storage
/// costs. In practice, however, this means that each `Order` is
/// assigned a static "access key" that persists throughout subsequent
/// active order states: if a user places an order, cancels the order,
/// then places another order, the `Order` will have the same access key
/// in each active instance. In other words, access keys are the lookup
/// ID in the relevant `Order` data structure for the given side
/// (`MarketAccount.asks` or `MarketAccount.bids`), and are not
/// necessarily unique for orders across time.
///
/// ## Market order IDs
///
/// Market order IDs, however, are unique across time for a given market
/// ID, and are tracked in a users' `Order.market_order_id`. A market
/// order ID is a unique identifier for an order on a given order book.
///
/// # Function index
///
/// ## View functions
///
/// Constant getters:
///
/// * `get_ASK()`
/// * `get_BID()`
/// * `get_CANCEL_REASON_EVICTION()`
/// * `get_CANCEL_REASON_IMMEDIATE_OR_CANCEL()`
/// * `get_CANCEL_REASON_MANUAL_CANCEL()`
/// * `get_CANCEL_REASON_MAX_QUOTE_TRADED()`
/// * `get_CANCEL_REASON_NOT_ENOUGH_LIQUIDITY()`
/// * `get_CANCEL_REASON_SELF_MATCH_MAKER()`
/// * `get_CANCEL_REASON_SELF_MATCH_TAKER()`
/// * `get_CANCEL_REASON_TOO_SMALL_TO_FILL_LOT()`
/// * `get_CANCEL_REASON_VIOLATED_LIMIT_PRICE()`
/// * `get_NO_CUSTODIAN()`
///
/// Market account lookup:
///
/// * `get_all_market_account_ids_for_market_id()`
/// * `get_all_market_account_ids_for_user()`
/// * `get_market_account()`
/// * `get_market_accounts()`
/// * `get_market_event_handle_creation_numbers()`
/// * `has_market_account()`
/// * `has_market_account_by_market_account_id()`
/// * `has_market_account_by_market_id()`
///
/// Market account ID lookup:
///
/// * `get_custodian_id()`
/// * `get_market_account_id()`
/// * `get_market_id()`
///
/// ## Public functions
///
/// Market account lookup
///
/// * `get_asset_counts_custodian()`
/// * `get_asset_counts_user()`
/// * `get_market_account_market_info_custodian()`
/// * `get_market_account_market_info_user()`
///
/// Asset transfer:
///
/// * `deposit_coins()`
/// * `deposit_generic_asset()`
/// * `withdraw_coins_custodian()`
/// * `withdraw_coins_user()`
/// * `withdraw_generic_asset_custodian()`
/// * `withdraw_generic_asset_user()`
///
/// ## Public entry functions
///
/// Asset transfer:
///
/// * `deposit_from_coinstore()`
/// * `withdraw_to_coinstore()`
///
/// Account registration:
///
/// * `init_market_event_handles_if_missing()`
/// * `register_market_account()`
/// * `register_market_account_generic_base()`
///
/// ## Public friend functions
///
/// Order management:
///
/// * `cancel_order_internal()`
/// * `change_order_size_internal()`
/// * `get_open_order_id_internal()`
/// * `fill_order_internal()`
/// * `place_order_internal()`
///
/// Asset management:
///
/// * `deposit_assets_internal()`
/// * `get_asset_counts_internal()`
/// * `withdraw_assets_internal()`
///
/// Order identifiers:
///
/// * `get_next_order_access_key_internal()`
/// * `get_active_market_order_ids_internal()`
///
/// Market events:
///
/// * `create_cancel_order_event_internal()`
/// * `create_fill_event_internal()`
/// * `emit_limit_order_events_internal()`
/// * `emit_market_order_events_internal()`
/// * `emit_swap_maker_fill_events_internal()`
///
/// ## Dependency charts
///
/// The below dependency charts use `mermaid.js` syntax, which can be
/// automatically rendered into a diagram (depending on the browser)
/// when viewing the documentation file generated from source code. If
/// a browser renders the diagrams with coloring that makes it difficult
/// to read, try a different browser.
///
/// Deposits:
///
/// ```mermaid
///
/// flowchart LR
///
/// deposit_coins --> deposit_asset
///
/// deposit_from_coinstore --> deposit_coins
///
/// deposit_assets_internal --> deposit_asset
/// deposit_assets_internal --> deposit_coins
///
/// deposit_generic_asset --> deposit_asset
/// deposit_generic_asset --> registry::get_underwriter_id
///
/// ```
///
/// Withdrawals:
///
/// ```mermaid
///
/// flowchart LR
///
/// withdraw_generic_asset_user --> withdraw_generic_asset
///
/// withdraw_generic_asset_custodian --> withdraw_generic_asset
/// withdraw_generic_asset_custodian --> registry::get_custodian_id
///
/// withdraw_coins_custodian --> withdraw_coins
/// withdraw_coins_custodian --> registry::get_custodian_id
///
/// withdraw_coins_user --> withdraw_coins
///
/// withdraw_to_coinstore --> withdraw_coins_user
///
/// withdraw_generic_asset --> withdraw_asset
/// withdraw_generic_asset --> registry::get_underwriter_id
///
/// withdraw_coins --> withdraw_asset
///
/// withdraw_assets_internal --> withdraw_asset
/// withdraw_assets_internal --> withdraw_coins
///
/// ```
///
/// Market account lookup:
///
/// ```mermaid
///
/// flowchart LR
///
/// get_asset_counts_user --> get_asset_counts_internal
///
/// get_asset_counts_custodian --> get_asset_counts_internal
/// get_asset_counts_custodian --> registry::get_custodian_id
///
/// get_market_account_market_info_custodian -->
///     get_market_account_market_info
/// get_market_account_market_info_custodian -->
///     registry::get_custodian_id
///
/// get_market_account_market_info_user -->
///     get_market_account_market_info
///
/// get_market_accounts --> get_all_market_account_ids_for_user
/// get_market_accounts --> get_market_id
/// get_market_accounts --> get_custodian_id
/// get_market_accounts --> get_market_account
///
/// get_market_account --> get_market_account_id
/// get_market_account --> has_market_account_by_market_account_id
/// get_market_account --> vectorize_open_orders
///
/// get_open_order_id_internal --> get_market_account_id
/// get_open_order_id_internal -->
///     has_market_account_by_market_account_id
///
/// has_market_account --> has_market_account_by_market_account_id
/// has_market_account --> get_market_account_id
///
/// get_market_event_handle_creation_numbers --> get_market_account_id
///
/// ```
///
/// Market account registration:
///
/// ```mermaid
///
/// flowchart LR
///
/// register_market_account --> registry::is_registered_custodian_id
/// register_market_account --> register_market_account_account_entries
/// register_market_account --> register_market_account_collateral_entry
/// register_market_account --> init_market_event_handles_if_missing
///
/// register_market_account_generic_base --> register_market_account
///
/// register_market_account_account_entries -->
///     registry::get_market_info_for_market_account
///
/// init_market_event_handles_if_missing --> has_market_account
///
/// ```
///
/// Internal order management:
///
/// ```mermaid
///
/// flowchart LR
///
/// change_order_size_internal --> cancel_order_internal
/// change_order_size_internal --> place_order_internal
///
/// ```
///
/// Market events:
///
/// ```mermaid
///
/// flowchart LR
///
/// emit_limit_order_events_internal --> emit_maker_fill_event
/// emit_market_order_events_internal --> emit_maker_fill_event
/// emit_swap_maker_fill_events_internal --> emit_maker_fill_event
///
/// ```
///
/// # Complete DocGen index
///
/// The below index is automatically generated from source code:
module econia::user {

    // Uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::guid;
    use aptos_framework::table::{Self, Table};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::type_info::{Self, TypeInfo};
    use econia::tablist::{Self, Tablist};
    use econia::registry::{
        Self, CustodianCapability, GenericAsset, UnderwriterCapability};
    use std::option::{Self, Option};
    use std::signer::address_of;
    use std::string::String;
    use std::vector;

    // Uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Friends >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    friend econia::market;

    // Friends <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    use econia::avl_queue::{u_128_by_32, u_64_by_32};
    #[test_only]
    use econia::assets::{Self, BC, QC, UC};
    #[test_only]
    use std::string;

    // Test-only uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Structs >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Emitted when an order is cancelled.
    struct CancelOrderEvent has copy, drop, store {
        /// Market ID for order.
        market_id: u64,
        /// Unique ID for order within market.
        order_id: u128,
        /// User for market account that placed order.
        user: address,
        /// Custodian ID for market account that placed order.
        custodian_id: u64,
        /// Reason for the cancel, for example
        /// `CANCEL_REASON_MANUAL_CANCEL`.
        reason: u8
    }

    /// Emitted when the size of an open order is manually changed.
    struct ChangeOrderSizeEvent has copy, drop, store {
        /// Market ID for order.
        market_id: u64,
        /// Unique ID for order within market.
        order_id: u128,
        /// User for market account that placed order.
        user: address,
        /// Custodian ID for market account that placed order.
        custodian_id: u64,
        /// `ASK` or `BID`.
        side: bool,
        /// Order size after manual size change operation.
        new_size: u64
    }

    /// All of a user's collateral across all market accounts.
    struct Collateral<phantom CoinType> has key {
        /// Map from market account ID to collateral for market account.
        /// Separated into different table entries to reduce transaction
        /// collisions across markets. Enables off-chain iterated
        /// indexing by market account ID.
        map: Tablist<u128, Coin<CoinType>>
    }

    /// Emitted when one order fills against another.
    struct FillEvent has copy, drop, store {
        /// Market ID for fill.
        market_id: u64,
        /// Amount filled, in lots.
        size: u64,
        /// Fill price, in ticks per lot.
        price: u64,
        /// `ASK` or `BID`, the side of the maker order.
        maker_side: bool,
        /// User address associated with market account for maker.
        maker: address,
        /// Custodian ID associated with market account for maker.
        maker_custodian_id: u64,
        /// Order ID for maker, unique within the market.
        maker_order_id: u128,
        /// User address associated with market account for taker.
        taker: address,
        /// Custodian ID associated with market account for taker.
        taker_custodian_id: u64,
        /// Order ID for taker, unique within the market.
        taker_order_id: u128,
        /// Amount of fees paid by taker on the fill, in indivisible
        /// quote subunits.
        taker_quote_fees_paid: u64,
        /// Sequence number (0-indexed) of fill within a single trade,
        /// which may have more than one fill. For example if a market
        /// order results in two fills, the first will have sequence
        /// number 0 and the second will have sequence number 1.
        sequence_number_for_trade: u64
    }

    /// Represents a user's open orders and asset counts for a given
    /// market account ID. Contains `registry::MarketInfo` field
    /// duplicates to reduce global storage item queries against the
    /// registry.
    struct MarketAccount has store {
        /// `registry::MarketInfo.base_type`.
        base_type: TypeInfo,
        /// `registry::MarketInfo.base_name_generic`.
        base_name_generic: String,
        /// `registry::MarketInfo.quote_type`.
        quote_type: TypeInfo,
        /// `registry::MarketInfo.lot_size`.
        lot_size: u64,
        /// `registry::MarketInfo.tick_size`.
        tick_size: u64,
        /// `registry::MarketInfo.min_size`.
        min_size: u64,
        /// `registry::MarketInfo.underwriter_id`.
        underwriter_id: u64,
        /// Map from order access key to open ask order.
        asks: Tablist<u64, Order>,
        /// Map from order access key to open bid order.
        bids: Tablist<u64, Order>,
        /// Access key of ask order at top of inactive stack, if any.
        asks_stack_top: u64,
        /// Access key of bid order at top of inactive stack, if any.
        bids_stack_top: u64,
        /// Total base asset units held as collateral.
        base_total: u64,
        /// Base asset units available to withdraw.
        base_available: u64,
        /// Amount `base_total` will increase to if all open bids fill.
        base_ceiling: u64,
        /// Total quote asset units held as collateral.
        quote_total: u64,
        /// Quote asset units available to withdraw.
        quote_available: u64,
        /// Amount `quote_total` will increase to if all open asks fill.
        quote_ceiling: u64
    }

    /// User-friendly market account view function return.
    struct MarketAccountView has store {
        /// Market ID for given market account.
        market_id: u64,
        /// Custodian ID for given market account.
        custodian_id: u64,
        /// All open asks.
        asks: vector<Order>,
        /// All open bids.
        bids: vector<Order>,
        /// `MarketAccount.base_total`.
        base_total: u64,
        /// `MarketAccount.base_available`.
        base_available: u64,
        /// `MarketAccount.base_ceiling`.
        base_ceiling: u64,
        /// `MarketAccount.quote_total`.
        quote_total: u64,
        /// `MarketAccount.quote_available`.
        quote_available: u64,
        /// `MarketAccount.quote_ceiling`.
        quote_ceiling: u64
    }

    /// All of a user's market accounts.
    struct MarketAccounts has key {
        /// Map from market account ID to `MarketAccount`.
        map: Table<u128, MarketAccount>,
        /// Map from market ID to vector of custodian IDs for which
        /// a market account has been registered on the given market.
        /// Enables off-chain iterated indexing by market account ID and
        /// assorted on-chain queries.
        custodians: Tablist<u64, vector<u64>>
    }

    /// View function return for getting event handle creation numbers
    /// of a particular `MarketEventHandlesForMarketAccount`.
    struct MarketEventHandleCreationNumbers has copy, drop {
        /// Creation number of `cancel_order_events` handle in a
        /// `MarketEventHandlesForMarketAccount`.
        cancel_order_events_handle_creation_num: u64,
        /// Creation number of `change_order_size_events` handle in a
        /// `MarketEventHandlesForMarketAccount`.
        change_order_size_events_handle_creation_num: u64,
        /// Creation number of `fill_events` handle in a
        /// `MarketEventHandlesForMarketAccount`.
        fill_events_handle_creation_num: u64,
        /// Creation number of `place_limit_order_events` handle in a
        /// `MarketEventHandlesForMarketAccount`.
        place_limit_order_events_handle_creation_num: u64,
        /// Creation number of `place_market_order_events` handle in a
        /// `MarketEventHandlesForMarketAccount`.
        place_market_order_events_handle_creation_num: u64
    }

    /// All of a user's `MarketEventHandlesForMarketAccount`.
    struct MarketEventHandles has key {
        /// Map from market account ID to
        /// `MarketEventHandlesForMarketAccount`.
        map: Table<u128, MarketEventHandlesForMarketAccount>
    }

    /// Event handles for market events within a unique market account.
    struct MarketEventHandlesForMarketAccount has store {
        /// Event handle for `CancelOrderEvent`s.
        cancel_order_events: EventHandle<CancelOrderEvent>,
        /// Event handle for `ChangeOrderSizeEvent`s.
        change_order_size_events: EventHandle<ChangeOrderSizeEvent>,
        /// Event handle for `FillEvent`s.
        fill_events: EventHandle<FillEvent>,
        /// Event handle for `PlaceLimitOrderEvent`s.
        place_limit_order_events: EventHandle<PlaceLimitOrderEvent>,
        /// Event handle for `PlaceMarketOrderEvent`s.
        place_market_order_events: EventHandle<PlaceMarketOrderEvent>
    }

    /// An open order, either ask or bid.
    struct Order has store {
        /// Market order ID. `NIL` if inactive.
        market_order_id: u128,
        /// Order size left to fill, in lots. When `market_order_id` is
        /// `NIL`, indicates access key of next inactive order in stack.
        size: u64
    }

    /// Emitted when a limit order is placed.
    struct PlaceLimitOrderEvent has copy, drop, store {
        /// Market ID for order.
        market_id: u64,
        /// User for market account that placed order.
        user: address,
        /// Custodian ID for market account that placed order.
        custodian_id: u64,
        /// Integrator address passed during limit order placement,
        /// eligible for a portion of any generated taker fees.
        integrator: address,
        /// `ASK` or `BID`.
        side: bool,
        /// Size indicated during limit order placement.
        size: u64,
        /// Order limit price.
        price: u64,
        /// Restriction indicated during limit order placement, either
        /// `market::FILL_OR_ABORT`, `market::IMMEDIATE_OR_CANCEL`,
        /// `market::POST_OR_ABORT`, or `market::NO_RESTRICTION`.
        restriction: u8,
        /// Self match behavior indicated during limit order placement,
        /// either `market::ABORT`, `market::CANCEL_BOTH`,
        /// `market::CANCEL_MAKER`, or `market::CANCEL_TAKER`.
        self_match_behavior: u8,
        /// Order size remaining after the function call in which the
        /// order was placed, which may include fills across the spread.
        /// For example if an order of size 10 and restriction
        /// `market::IMMEDIATE_OR_CANCEL` fills 6 lots across the
        /// spread, the order will be cancelled and remaining size is 4.
        remaining_size: u64,
        /// Unique ID for order within market.
        order_id: u128
    }

    /// Emitted when a market order is placed.
    struct PlaceMarketOrderEvent has copy, drop, store {
        /// Market ID for order.
        market_id: u64,
        /// User for market account that placed order.
        user: address,
        /// Custodian ID for market account that placed order.
        custodian_id: u64,
        /// Integrator address passed during market order placement,
        /// eligible for a portion of any generated taker fees.
        integrator: address,
        /// Either `market::BUY` or `market::SELL`.
        direction: bool,
        /// Size indicated during market order placement.
        size: u64,
        /// Self match behavior indicated during market order placement,
        /// either `market::ABORT`, `market::CANCEL_BOTH`,
        /// `market::CANCEL_MAKER`, or `market::CANCEL_TAKER`.
        self_match_behavior: u8,
        /// Unique ID for order within market.
        order_id: u128
    }

    // Structs <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Error codes >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Market account already exists.
    const E_EXISTS_MARKET_ACCOUNT: u64 = 0;
    /// Custodian ID has not been registered.
    const E_UNREGISTERED_CUSTODIAN: u64 = 1;
    /// No market accounts resource found.
    const E_NO_MARKET_ACCOUNTS: u64 = 2;
    /// No market account resource found.
    const E_NO_MARKET_ACCOUNT: u64 = 3;
    /// Asset type is not in trading pair for market.
    const E_ASSET_NOT_IN_PAIR: u64 = 4;
    /// Deposit would overflow asset ceiling.
    const E_DEPOSIT_OVERFLOW_ASSET_CEILING: u64 = 5;
    /// Underwriter is not valid for indicated market.
    const E_INVALID_UNDERWRITER: u64 = 6;
    /// Too little available for withdrawal.
    const E_WITHDRAW_TOO_LITTLE_AVAILABLE: u64 = 7;
    /// Price is zero.
    const E_PRICE_0: u64 = 8;
    /// Price exceeds maximum possible price.
    const E_PRICE_TOO_HIGH: u64 = 9;
    /// Ticks to fill an order overflows a `u64`.
    const E_TICKS_OVERFLOW: u64 = 11;
    /// Filling order would overflow asset received from trade.
    const E_OVERFLOW_ASSET_IN: u64 = 12;
    /// Not enough asset to trade away.
    const E_NOT_ENOUGH_ASSET_OUT: u64 = 13;
    /// No change in order size.
    const E_CHANGE_ORDER_NO_CHANGE: u64 = 14;
    /// Market order ID mismatch with user's open order.
    const E_INVALID_MARKET_ORDER_ID: u64 = 15;
    /// Mismatch between coin value and indicated amount.
    const E_COIN_AMOUNT_MISMATCH: u64 = 16;
    /// Expected order access key does not match assigned order access
    /// key.
    const E_ACCESS_KEY_MISMATCH: u64 = 17;
    /// Coin type is generic asset.
    const E_COIN_TYPE_IS_GENERIC_ASSET: u64 = 18;
    /// Mismatch between expected size before operation and actual size
    /// before operation.
    const E_START_SIZE_MISMATCH: u64 = 19;

    // Error codes <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Constants >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Flag for ask side
    const ASK: bool = true;
    /// Flag for bid side
    const BID: bool = false;
    /// Order cancelled because it was evicted from the price-time
    /// priority queue.
    const CANCEL_REASON_EVICTION: u8 = 1;
    /// Order cancelled because it was an immediate-or-cancel order
    /// that did not immediately fill.
    const CANCEL_REASON_IMMEDIATE_OR_CANCEL: u8 = 2;
    /// Order cancelled because it was manually cancelled by either
    /// signing user or custodian.
    const CANCEL_REASON_MANUAL_CANCEL: u8 = 3;
    /// Order cancelled because no more quote asset could be traded.
    const CANCEL_REASON_MAX_QUOTE_TRADED: u8 = 4;
    /// Order cancelled because there was not enough liquidity to take
    /// from.
    const CANCEL_REASON_NOT_ENOUGH_LIQUIDITY: u8 = 5;
    /// Order cancelled because it was on the maker side of an fill
    /// where self match behavior indicated cancelling the maker order.
    const CANCEL_REASON_SELF_MATCH_MAKER: u8 = 6;
    /// Order cancelled because it was on the taker side of an fill
    /// where self match behavior indicated cancelling the taker order.
    const CANCEL_REASON_SELF_MATCH_TAKER: u8 = 7;
    /// Flag to indicate that order is only temporarily cancelled from
    /// market account memory because it will be subsequently re-placed
    /// as part of a size change.
    const CANCEL_REASON_SIZE_CHANGE_INTERNAL: u8 = 0;
    /// Swap order cancelled because the remaining base asset amount to
    /// match was too small to fill a single lot.
    const CANCEL_REASON_TOO_SMALL_TO_FILL_LOT: u8 = 8;
    /// Swap order cancelled because the next order on the book to match
    /// against violated the swap order limit price.
    const CANCEL_REASON_VIOLATED_LIMIT_PRICE: u8 = 9;
    /// `u64` bitmask with all bits set, generated in Python via
    /// `hex(int('1' * 64, 2))`.
    const HI_64: u64 = 0xffffffffffffffff;
    /// Maximum possible price that can be encoded in 32 bits. Generated
    /// in Python via `hex(int('1' * 32, 2))`.
    const HI_PRICE: u64 = 0xffffffff;
    /// Flag for null value when null defined as 0.
    const NIL: u64 = 0;
    /// Custodian ID flag for no custodian.
    const NO_CUSTODIAN: u64 = 0;
    /// Underwriter ID flag for no underwriter.
    const NO_UNDERWRITER: u64 = 0;
    /// Number of bits market ID is shifted in market account ID.
    const SHIFT_MARKET_ID: u8 = 64;

    // Constants <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // View functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[view]
    /// Public constant getter for `ASK`.
    ///
    /// # Testing
    ///
    /// * `test_get_ASK()`
    public fun get_ASK(): bool {ASK}

    #[view]
    /// Public constant getter for `BID`.
    ///
    /// # Testing
    ///
    /// * `test_get_BID()`
    public fun get_BID(): bool {BID}

    #[view]
    /// Public constant getter for `CANCEL_REASON_EVICTION`.
    ///
    /// # Testing
    ///
    /// * `test_get_cancel_reasons()`
    public fun get_CANCEL_REASON_EVICTION(): u8 {
        CANCEL_REASON_EVICTION
    }

    #[view]
    /// Public constant getter for `CANCEL_REASON_IMMEDIATE_OR_CANCEL`.
    ///
    /// # Testing
    ///
    /// * `test_get_cancel_reasons()`
    public fun get_CANCEL_REASON_IMMEDIATE_OR_CANCEL(): u8 {
        CANCEL_REASON_IMMEDIATE_OR_CANCEL
    }

    #[view]
    /// Public constant getter for `CANCEL_REASON_MANUAL_CANCEL`.
    ///
    /// # Testing
    ///
    /// * `test_get_cancel_reasons()`
    public fun get_CANCEL_REASON_MANUAL_CANCEL(): u8 {
        CANCEL_REASON_MANUAL_CANCEL
    }

    #[view]
    /// Public constant getter for `CANCEL_REASON_MAX_QUOTE_TRADED`.
    ///
    /// # Testing
    ///
    /// * `test_get_cancel_reasons()`
    public fun get_CANCEL_REASON_MAX_QUOTE_TRADED(): u8 {
        CANCEL_REASON_MAX_QUOTE_TRADED
    }

    #[view]
    /// Public constant getter for `CANCEL_REASON_NOT_ENOUGH_LIQUIDITY`.
    ///
    /// # Testing
    ///
    /// * `test_get_cancel_reasons()`
    public fun get_CANCEL_REASON_NOT_ENOUGH_LIQUIDITY(): u8 {
        CANCEL_REASON_NOT_ENOUGH_LIQUIDITY
    }

    #[view]
    /// Public constant getter for `CANCEL_REASON_SELF_MATCH_MAKER`.
    ///
    /// # Testing
    ///
    /// * `test_get_cancel_reasons()`
    public fun get_CANCEL_REASON_SELF_MATCH_MAKER(): u8 {
        CANCEL_REASON_SELF_MATCH_MAKER
    }

    #[view]
    /// Public constant getter for `CANCEL_REASON_SELF_MATCH_TAKER`.
    ///
    /// # Testing
    ///
    /// * `test_get_cancel_reasons()`
    public fun get_CANCEL_REASON_SELF_MATCH_TAKER(): u8 {
        CANCEL_REASON_SELF_MATCH_TAKER
    }

    #[view]
    /// Public constant getter for
    /// `CANCEL_REASON_TOO_SMALL_TO_FILL_LOT`.
    ///
    /// # Testing
    ///
    /// * `test_get_cancel_reasons()`
    public fun get_CANCEL_REASON_TOO_SMALL_TO_FILL_LOT(): u8 {
        CANCEL_REASON_TOO_SMALL_TO_FILL_LOT
    }

    #[view]
    /// Public constant getter for `CANCEL_REASON_VIOLATED_LIMIT_PRICE`.
    ///
    /// # Testing
    ///
    /// * `test_get_cancel_reasons()`
    public fun get_CANCEL_REASON_VIOLATED_LIMIT_PRICE(): u8 {
        CANCEL_REASON_VIOLATED_LIMIT_PRICE
    }

    #[view]
    /// Public constant getter for `NO_CUSTODIAN`.
    ///
    /// # Testing
    ///
    /// * `test_get_NO_CUSTODIAN()`
    public fun get_NO_CUSTODIAN(): u64 {NO_CUSTODIAN}

    #[view]
    /// Return all market account IDs associated with market ID.
    ///
    /// # Parameters
    ///
    /// * `user`: Address of user to check market account IDs for.
    /// * `market_id`: Market ID to check market accounts for.
    ///
    /// # Returns
    ///
    /// * `vector<u128>`: Vector of user's market account IDs for given
    ///   market, empty if no market accounts.
    ///
    /// # Gas considerations
    ///
    /// Loops over all elements within a vector that is itself a single
    /// item in global storage, and returns a vector via pass-by-value.
    ///
    /// # Testing
    ///
    /// * `test_market_account_getters()`
    public fun get_all_market_account_ids_for_market_id(
        user: address,
        market_id: u64
    ): vector<u128>
    acquires MarketAccounts {
        let market_account_ids = vector::empty(); // Init empty vector.
        // Return empty if user has no market accounts resource.
        if (!exists<MarketAccounts>(user)) return market_account_ids;
        let custodians_map_ref = // Immutably borrow custodians map.
            &borrow_global<MarketAccounts>(user).custodians;
        // Return empty if user has no market accounts for given market.
        if (!tablist::contains(custodians_map_ref, market_id))
            return market_account_ids;
        // Immutably borrow list of custodians for given market.
        let custodians_ref = tablist::borrow(custodians_map_ref, market_id);
        // Initialize loop counter and number of elements in vector.
        let (i, n_custodians) = (0, vector::length(custodians_ref));
        while (i < n_custodians) { // Loop over all elements.
            // Get custodian ID.
            let custodian_id = *vector::borrow(custodians_ref, i);
            // Get market account ID.
            let market_account_id = ((market_id as u128) << SHIFT_MARKET_ID) |
                                    (custodian_id as u128);
            // Push back onto ongoing market account ID vector.
            vector::push_back(&mut market_account_ids, market_account_id);
            i = i + 1; // Increment loop counter
        };
        market_account_ids // Return market account IDs.
    }

    #[view]
    /// Return all of a user's market account IDs.
    ///
    /// # Parameters
    ///
    /// * `user`: Address of user to check market account IDs for.
    ///
    /// # Returns
    ///
    /// * `vector<u128>`: Vector of user's market account IDs, empty if
    ///   no market accounts.
    ///
    /// # Gas considerations
    ///
    /// For each market that a user has market accounts for, loops over
    /// a separate item in global storage, incurring a per-item read
    /// cost. Additionally loops over a vector for each such per-item
    /// read, incurring linearly-scaled vector operation costs. Returns
    /// a vector via pass-by-value.
    ///
    /// # Testing
    ///
    /// * `test_market_account_getters()`
    public fun get_all_market_account_ids_for_user(
        user: address,
    ): vector<u128>
    acquires MarketAccounts {
        let market_account_ids = vector::empty(); // Init empty vector.
        // Return empty if user has no market accounts resource.
        if (!exists<MarketAccounts>(user)) return market_account_ids;
        let custodians_map_ref = // Immutably borrow custodians map.
            &borrow_global<MarketAccounts>(user).custodians;
        // Get market ID option at head of market ID list.
        let market_id_option = tablist::get_head_key(custodians_map_ref);
        // While market IDs left to loop over:
        while (option::is_some(&market_id_option)) {
            // Get market ID.
            let market_id = *option::borrow(&market_id_option);
            // Immutably borrow list of custodians for given market and
            // next market ID option in list.
            let (custodians_ref, _, next) = tablist::borrow_iterable(
                custodians_map_ref, market_id);
            // Initialize loop counter and number of elements in vector.
            let (i, n_custodians) = (0, vector::length(custodians_ref));
            while (i < n_custodians) { // Loop over all elements.
                // Get custodian ID.
                let custodian_id = *vector::borrow(custodians_ref, i);
                let market_account_id = // Get market account ID.
                    ((market_id as u128) << SHIFT_MARKET_ID) |
                    (custodian_id as u128);
                // Push back onto ongoing market account ID vector.
                vector::push_back(&mut market_account_ids, market_account_id);
                i = i + 1; // Increment loop counter
            };
            // Review next market ID option in list.
            market_id_option = next;
        };
        market_account_ids // Return market account IDs.
    }

    #[view]
    /// Return custodian ID encoded in market account ID.
    ///
    /// # Testing
    ///
    /// * `test_market_account_id_getters()`
    public fun get_custodian_id(
        market_account_id: u128
    ): u64 {
        ((market_account_id & (HI_64 as u128)) as u64)
    }

    #[view]
    /// Return human-readable `MarketAccountView`.
    ///
    /// Mutates state, so kept as a private view function.
    ///
    /// # Aborts
    ///
    /// * `E_NO_MARKET_ACCOUNT`: No such specified market account.
    ///
    /// # Testing
    ///
    /// * `test_deposits()`
    /// * `test_get_market_account_no_market_account()`
    /// * `test_get_market_accounts_open_orders()`
    fun get_market_account(
        user: address,
        market_id: u64,
        custodian_id: u64
    ): MarketAccountView
    acquires MarketAccounts {
        // Get market account ID from market ID, custodian ID.
        let market_account_id = get_market_account_id(market_id, custodian_id);
        // Verify user has market account.
        assert!(has_market_account_by_market_account_id(
            user, market_account_id), E_NO_MARKET_ACCOUNT);
        // Mutably borrow market accounts map.
        let market_accounts_map_ref_mut =
            &mut borrow_global_mut<MarketAccounts>(user).map;
        // Mutably borrow market account.
        let market_account_ref_mut = table::borrow_mut(
            market_accounts_map_ref_mut, market_account_id);
        // Return market account view with parsed fields.
        MarketAccountView{
            market_id,
            custodian_id,
            asks: vectorize_open_orders(&mut market_account_ref_mut.asks),
            bids: vectorize_open_orders(&mut market_account_ref_mut.bids),
            base_total: market_account_ref_mut.base_total,
            base_available: market_account_ref_mut.base_available,
            base_ceiling: market_account_ref_mut.base_ceiling,
            quote_total: market_account_ref_mut.quote_total,
            quote_available: market_account_ref_mut.quote_available,
            quote_ceiling: market_account_ref_mut.quote_ceiling
        }
    }

    #[view]
    /// Return market account ID with encoded market and custodian IDs.
    ///
    /// # Testing
    ///
    /// * `test_market_account_id_getters()`
    public fun get_market_account_id(
        market_id: u64,
        custodian_id: u64
    ): u128 {
        ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128)
    }

    #[view]
    /// Get user-friendly views of all of a `user`'s market accounts.
    ///
    /// Mutates state, so kept as a private view function.
    ///
    /// # Testing
    ///
    /// * `test_get_market_accounts_open_orders()`
    fun get_market_accounts(
        user: address
    ): vector<MarketAccountView>
    acquires MarketAccounts {
        // Get all of user's market account IDs.
        let market_account_ids = get_all_market_account_ids_for_user(user);
        // Initialize empty vector for open order IDs.
        let market_accounts = vector::empty();
        // If no market account IDs, return empty vector.
        if (vector::is_empty(&market_account_ids)) return market_accounts;
        // For each market account ID:
        vector::for_each(market_account_ids, |market_account_id| {
            // Get encoded market ID.
            let market_id = get_market_id(market_account_id);
            // Get encoded custodian ID.
            let custodian_id = get_custodian_id(market_account_id);
            // Push back struct onto vector of ongoing structs.
            vector::push_back(&mut market_accounts, get_market_account(
                user, market_id, custodian_id));
        });
        market_accounts // Return market account views.
    }

    #[view]
    /// Return a `MarketEventHandleCreationNumbers` for `market_id` and
    /// `custodian_id`, if `user` has event handles for indicated market
    /// account.
    ///
    /// Restricted to private view function to prevent runtime handle
    /// contention.
    ///
    /// # Testing
    ///
    /// * `test_register_market_accounts()`
    fun get_market_event_handle_creation_numbers(
        user: address,
        market_id: u64,
        custodian_id: u64
    ): Option<MarketEventHandleCreationNumbers>
    acquires MarketEventHandles {
        // Return none if user does not have market event handles map,
        if (!exists<MarketEventHandles>(user)) return option::none();
        // Return none if user has no handles for market account.
        let market_event_handles_map_ref =
            &borrow_global<MarketEventHandles>(user).map;
        let market_account_id = get_market_account_id(market_id, custodian_id);
        let has_handles = table::contains(
            market_event_handles_map_ref, market_account_id);
        if (!has_handles) return option::none();
        // Return option-packed creation numbers for all event handles.
        let market_account_handles_ref = table::borrow(
            market_event_handles_map_ref, market_account_id);
        option::some(MarketEventHandleCreationNumbers{
            cancel_order_events_handle_creation_num:
                guid::creation_num(event::guid(
                    &market_account_handles_ref.cancel_order_events)),
            change_order_size_events_handle_creation_num:
                guid::creation_num(event::guid(
                    &market_account_handles_ref.change_order_size_events)),
            fill_events_handle_creation_num:
                guid::creation_num(event::guid(
                    &market_account_handles_ref.fill_events)),
            place_limit_order_events_handle_creation_num:
                guid::creation_num(event::guid(
                    &market_account_handles_ref.place_limit_order_events)),
            place_market_order_events_handle_creation_num:
                guid::creation_num(event::guid(
                    &market_account_handles_ref.place_market_order_events))
        })
    }

    #[view]
    /// Return market ID encoded in market account ID.
    ///
    /// # Testing
    ///
    /// * `test_market_account_id_getters()`
    public fun get_market_id(
        market_account_id: u128
    ): u64 {
        (market_account_id >> SHIFT_MARKET_ID as u64)
    }

    #[view]
    /// Return `true` if `user` has market account registered with
    /// given `market_id` and `custodian_id`.
    ///
    /// # Testing
    ///
    /// * `test_market_account_getters()`
    public fun has_market_account(
        user: address,
        market_id: u64,
        custodian_id: u64
    ): bool
    acquires MarketAccounts {
        has_market_account_by_market_account_id(
            user, get_market_account_id(market_id, custodian_id))
    }

    #[view]
    /// Return `true` if `user` has market account registered with
    /// given `market_account_id`.
    ///
    /// # Testing
    ///
    /// * `test_market_account_getters()`
    public fun has_market_account_by_market_account_id(
        user: address,
        market_account_id: u128
    ): bool
    acquires MarketAccounts {
        // Return false if user has no market accounts resource.
        if (!exists<MarketAccounts>(user)) return false;
        // Immutably borrow market accounts map.
        let market_accounts_map_ref = &borrow_global<MarketAccounts>(user).map;
        // Return if map has entry for given market account ID.
        table::contains(market_accounts_map_ref, market_account_id)
    }

    #[view]
    /// Return `true` if `user` has at least one market account
    /// registered with given `market_id`.
    ///
    /// # Testing
    ///
    /// * `test_market_account_getters()`
    public fun has_market_account_by_market_id(
        user: address,
        market_id: u64
    ): bool
    acquires MarketAccounts {
        // Return false if user has no market accounts resource.
        if (!exists<MarketAccounts>(user)) return false;
        let custodians_map_ref = // Immutably borrow custodians map.
            &borrow_global<MarketAccounts>(user).custodians;
        // Return if custodians map has entry for given market ID.
        tablist::contains(custodians_map_ref, market_id)
    }

    // View functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Wrapped call to `deposit_asset()` for depositing coins.
    ///
    /// # Aborts
    ///
    /// * `E_COIN_TYPE_IS_GENERIC_ASSET`: Coin type is generic asset,
    ///   corresponding to the Econia account having initialized a coin
    ///   of type `GenericAsset`.
    ///
    /// # Testing
    ///
    /// * `test_deposit_coins_generic()`
    /// * `test_deposits()`
    public fun deposit_coins<
        CoinType
    >(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        coins: Coin<CoinType>
    ) acquires
        Collateral,
        MarketAccounts
    {
        // Check if coin type is generic asset.
        let coin_type_is_generic_asset = type_info::type_of<CoinType>() ==
                                         type_info::type_of<GenericAsset>();
        // Assert coin type is not generic asset.
        assert!(!coin_type_is_generic_asset, E_COIN_TYPE_IS_GENERIC_ASSET);
        deposit_asset<CoinType>( // Deposit asset.
            user_address,
            market_id,
            custodian_id,
            coin::value(&coins),
            option::some(coins),
            NO_UNDERWRITER);
    }

    /// Wrapped call to `deposit_asset()` for depositing generic asset.
    ///
    /// # Testing
    ///
    /// * `test_deposits()`
    public fun deposit_generic_asset(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        amount: u64,
        underwriter_capability_ref: &UnderwriterCapability
    ) acquires
        Collateral,
        MarketAccounts
    {
        deposit_asset<GenericAsset>(
            user_address,
            market_id,
            custodian_id,
            amount,
            option::none(),
            registry::get_underwriter_id(underwriter_capability_ref));
    }

    /// Wrapped call to `get_asset_counts_internal()` for custodian.
    ///
    /// Restricted to custodian for given market account to prevent
    /// excessive public queries and thus transaction collisions.
    ///
    /// # Testing
    ///
    /// * `test_deposits()`
    public fun get_asset_counts_custodian(
        user_address: address,
        market_id: u64,
        custodian_capability_ref: &CustodianCapability
    ): (
        u64,
        u64,
        u64,
        u64,
        u64,
        u64
    ) acquires MarketAccounts {
        get_asset_counts_internal(
            user_address, market_id,
            registry::get_custodian_id(custodian_capability_ref))
    }

    /// Wrapped call to `get_asset_counts_internal()` for signing user.
    ///
    /// Restricted to signing user for given market account to prevent
    /// excessive public queries and thus transaction collisions.
    ///
    /// # Testing
    ///
    /// * `test_deposits()`
    public fun get_asset_counts_user(
        user: &signer,
        market_id: u64
    ): (
        u64,
        u64,
        u64,
        u64,
        u64,
        u64
    ) acquires MarketAccounts {
        get_asset_counts_internal(address_of(user), market_id, NO_CUSTODIAN)
    }

    /// Wrapped call to `get_market_account_market_info()` for
    /// custodian.
    ///
    /// Restricted to custodian for given market account to prevent
    /// excessive public queries and thus transaction collisions.
    ///
    /// # Testing
    ///
    /// * `test_register_market_accounts()`
    public fun get_market_account_market_info_custodian(
        user_address: address,
        market_id: u64,
        custodian_capability_ref: &CustodianCapability
    ): (
        TypeInfo,
        String,
        TypeInfo,
        u64,
        u64,
        u64,
        u64
    ) acquires MarketAccounts {
        get_market_account_market_info(
            user_address, market_id,
            registry::get_custodian_id(custodian_capability_ref))
    }

    /// Wrapped call to `get_market_account_market_info()` for signing
    /// user.
    ///
    /// Restricted to signing user for given market account to prevent
    /// excessive public queries and thus transaction collisions.
    ///
    /// # Testing
    ///
    /// * `test_register_market_accounts()`
    public fun get_market_account_market_info_user(
        user: &signer,
        market_id: u64
    ): (
        TypeInfo,
        String,
        TypeInfo,
        u64,
        u64,
        u64,
        u64
    ) acquires MarketAccounts {
        get_market_account_market_info(
            address_of(user), market_id, NO_CUSTODIAN)
    }

    /// Wrapped call to `withdraw_coins()` for withdrawing under
    /// authority of delegated custodian.
    ///
    /// # Testing
    ///
    /// * `test_withdrawals()`
    public fun withdraw_coins_custodian<
        CoinType
    >(
        user_address: address,
        market_id: u64,
        amount: u64,
        custodian_capability_ref: &CustodianCapability
    ): Coin<CoinType>
    acquires
        Collateral,
        MarketAccounts
    {
        withdraw_coins<CoinType>(
            user_address,
            market_id,
            registry::get_custodian_id(custodian_capability_ref),
            amount)
    }

    /// Wrapped call to `withdraw_coins()` for withdrawing under
    /// authority of signing user.
    ///
    /// # Testing
    ///
    /// * `test_withdrawals()`
    public fun withdraw_coins_user<
        CoinType
    >(
        user: &signer,
        market_id: u64,
        amount: u64,
    ): Coin<CoinType>
    acquires
        Collateral,
        MarketAccounts
    {
        withdraw_coins<CoinType>(
            address_of(user),
            market_id,
            NO_CUSTODIAN,
            amount)
    }

    /// Wrapped call to `withdraw_generic_asset()` for withdrawing under
    /// authority of delegated custodian.
    ///
    /// # Testing
    ///
    /// * `test_withdrawals()`
    public fun withdraw_generic_asset_custodian(
        user_address: address,
        market_id: u64,
        amount: u64,
        custodian_capability_ref: &CustodianCapability,
        underwriter_capability_ref: &UnderwriterCapability
    ) acquires
        Collateral,
        MarketAccounts
    {
        withdraw_generic_asset(
            user_address,
            market_id,
            registry::get_custodian_id(custodian_capability_ref),
            amount,
            underwriter_capability_ref)
    }

    /// Wrapped call to `withdraw_generic_asset()` for withdrawing under
    /// authority of signing user.
    ///
    /// # Testing
    ///
    /// * `test_withdrawals()`
    public fun withdraw_generic_asset_user(
        user: &signer,
        market_id: u64,
        amount: u64,
        underwriter_capability_ref: &UnderwriterCapability
    ) acquires
        Collateral,
        MarketAccounts
    {
        withdraw_generic_asset(
            address_of(user),
            market_id,
            NO_CUSTODIAN,
            amount,
            underwriter_capability_ref)
    }

    // Public functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public entry functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Wrapped call to `deposit_coins()` for depositing from an
    /// `aptos_framework::coin::CoinStore`.
    ///
    /// # Testing
    ///
    /// * `test_deposits()`
    public entry fun deposit_from_coinstore<
        CoinType
    >(
        user: &signer,
        market_id: u64,
        custodian_id: u64,
        amount: u64
    ) acquires
        Collateral,
        MarketAccounts
    {
        deposit_coins<CoinType>(
            address_of(user),
            market_id,
            custodian_id,
            coin::withdraw<CoinType>(user, amount));
    }

    /// Initialize market event handles for a market account if missing.
    ///
    /// Since market event handles were implemented as part of a
    /// compatible upgrade policy, it is possible for a user to have a
    /// market account without associated market event handles, if they
    /// registered a market account before an on-chain upgrade.
    ///
    /// # Parameters
    ///
    /// * `user`: User for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    ///
    /// # Aborts
    ///
    /// * `E_NO_MARKET_ACCOUNT`: No such specified market account.
    ///
    /// # Testing
    ///
    /// * `test_init_market_event_handles_if_missing_no_account()`
    /// * `test_register_market_accounts()`
    public entry fun init_market_event_handles_if_missing(
        user: &signer,
        market_id: u64,
        custodian_id: u64
    ) acquires
        MarketAccounts,
        MarketEventHandles
    {
        // Verify user has specified market account.
        let user_address = address_of(user);
        assert!(has_market_account(user_address, market_id, custodian_id),
                E_NO_MARKET_ACCOUNT);
        // Create market event handles map if user doesn't have one,
        // and fill with handles for market account as needed.
        if (!exists<MarketEventHandles>(address_of(user)))
            move_to(user, MarketEventHandles{map: table::new()});
        let market_event_handles_map_ref_mut =
            &mut borrow_global_mut<MarketEventHandles>(user_address).map;
        let market_account_id =
            ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128);
        let has_handles = table::contains(
            market_event_handles_map_ref_mut, market_account_id);
        if (!has_handles) {
            let handles = MarketEventHandlesForMarketAccount{
                cancel_order_events: account::new_event_handle(user),
                change_order_size_events: account::new_event_handle(user),
                fill_events: account::new_event_handle(user),
                place_limit_order_events: account::new_event_handle(user),
                place_market_order_events: account::new_event_handle(user)
            };
            table::add(
                market_event_handles_map_ref_mut, market_account_id, handles);
        };
    }

    /// Register market account for indicated market and custodian.
    ///
    /// Verifies market ID and asset types via internal call to
    /// `register_market_account_account_entries()`.
    ///
    /// # Type parameters
    ///
    /// * `BaseType`: Base type for indicated market. If base asset is
    ///   a generic asset, must be passed as `registry::GenericAsset`
    ///   (alternatively use `register_market_account_base_generic()`).
    /// * `QuoteType`: Quote type for indicated market.
    ///
    /// # Parameters
    ///
    /// * `user`: User registering a market account.
    /// * `market_id`: Market ID for given market.
    /// * `custodian_id`: Custodian ID to register account with, or
    ///   `NO_CUSTODIAN`.
    ///
    /// # Aborts
    ///
    /// * `E_UNREGISTERED_CUSTODIAN`: Custodian ID has not been
    ///   registered.
    ///
    /// # Testing
    ///
    /// * `test_register_market_account_unregistered_custodian()`
    /// * `test_register_market_accounts()`
    public entry fun register_market_account<
        BaseType,
        QuoteType
    >(
        user: &signer,
        market_id: u64,
        custodian_id: u64
    ) acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // If custodian ID indicated, assert it is registered.
        if (custodian_id != NO_CUSTODIAN) assert!(
            registry::is_registered_custodian_id(custodian_id),
            E_UNREGISTERED_CUSTODIAN);
        let market_account_id = // Get market account ID.
            ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128);
        // Register market accounts map entries, verifying market ID and
        // asset types.
        register_market_account_account_entries<BaseType, QuoteType>(
            user, market_account_id, market_id, custodian_id);
        // Register collateral entry if base type is coin (otherwise
        // is a generic asset and no collateral entry required).
        if (coin::is_coin_initialized<BaseType>())
            register_market_account_collateral_entry<BaseType>(
                user, market_account_id);
        // Register quote asset collateral entry for quote coin type
        // (quote type for a verified market must be a coin).
        register_market_account_collateral_entry<QuoteType>(
            user, market_account_id);
        init_market_event_handles_if_missing(user, market_id, custodian_id);
    }

    /// Wrapped `register_market_account()` call for generic base asset.
    ///
    /// # Testing
    ///
    /// * `test_register_market_accounts()`
    public entry fun register_market_account_generic_base<
        QuoteType
    >(
        user: &signer,
        market_id: u64,
        custodian_id: u64
    ) acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        register_market_account<GenericAsset, QuoteType>(
            user, market_id, custodian_id);
    }

    /// Wrapped call to `withdraw_coins_user()` for withdrawing from
    /// market account to user's `aptos_framework::coin::CoinStore`.
    ///
    /// # Testing
    ///
    /// * `test_withdrawals()`
    public entry fun withdraw_to_coinstore<
        CoinType
    >(
        user: &signer,
        market_id: u64,
        amount: u64,
    ) acquires
        Collateral,
        MarketAccounts
    {
        // Register coin store if user does not have one.
        if (!coin::is_account_registered<CoinType>(address_of(user)))
            coin::register<CoinType>(user);
        // Deposit to coin store coins withdrawn from market account.
        coin::deposit<CoinType>(address_of(user), withdraw_coins_user(
            user, market_id, amount));
    }

    // Public entry functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public friend functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Cancel order from a user's tablist of open orders on given side.
    ///
    /// Updates asset counts, pushes order onto top of inactive orders
    /// stack, and overwrites its fields accordingly.
    ///
    /// Accepts as an argument a market order ID, which is checked
    /// against the market order ID in the user's corresponding `Order`.
    /// This check is bypassed when the market order ID is passed as
    /// `NIL`, which should only happen when cancellation is motivated
    /// by an eviction or by a self match cancel: market order IDs are
    /// not tracked in order book state, so during these two operations,
    /// `cancel_order_internal()` is simply called with a `NIL` market
    /// order ID argument. Custodians or users who manually trigger
    /// order cancellations for their own order do have to pass market
    /// order IDs, however, to verify that they are not passing a
    /// malicious market order ID (portions of which essentially
    /// function as pointers into AVL queue state).
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    /// * `side`: `ASK` or `BID`, the side on which an order was placed.
    /// * `start_size`: The open order size before filling.
    /// * `price`: Order price, in ticks per lot.
    /// * `order_access_key`: Order access key for user order lookup.
    /// * `market_order_id`: `NIL` if order cancellation originates from
    ///   an eviction or a self match cancel, otherwise the market order
    ///   ID encoded in the user's `Order`.
    /// * `cancel_reason`: The reason for the cancel. Note that
    ///   user-side open order size changes are processed via
    ///   `change_order_size_internal()` as a cancellation followed by
    ///   immediate re-placement, corresponding to the cancel reason
    ///   `CANCEL_REASON_SIZE_CHANGE_INTERNAL`. When this is the case
    ///   no cancel event is emitted.
    ///
    /// # Returns
    ///
    /// * `u128`: Market order ID for corresponding order.
    ///
    /// # Terminology
    ///
    /// * The "inbound" asset is the asset that would have been received
    ///   from a trade if the cancelled order had been filled.
    /// * The "outbound" asset is the asset that would have been traded
    ///   away if the cancelled order had been filled.
    ///
    /// # Aborts
    ///
    /// * `E_START_SIZE_MISMATCH`: Mismatch between expected size before
    ///   operation and actual size before operation.
    /// * `E_INVALID_MARKET_ORDER_ID`: Market order ID mismatch with
    ///   user's open order, when market order ID not passed as `NIL`.
    ///
    /// # Emits
    ///
    /// * `CancelOrderEvent`: Information about a cancelled order.
    ///
    /// # Assumptions
    ///
    /// * Only called when also cancelling an order from the order book.
    /// * User has an open order under indicated market account with
    ///   provided access key, but not necessarily with provided market
    ///   order ID (if market order ID is not `NIL`): if order
    ///   cancellation is manually actuated by a custodian or user,
    ///   then it had to have been successfully placed on the book to
    ///   begin with for the given access key. Market order IDs,
    ///   however, are not maintained in order book state and so could
    ///   be potentially be passed by a malicious user or custodian who
    ///   intends to alter order book state per above.
    /// * If market order ID is `NIL`, is only called during an eviction
    ///   or a self match cancel.
    /// * `price` matches that encoded in market order ID from cancelled
    ///   order if market order ID is not `NIL`.
    ///
    /// # Expected value testing
    ///
    /// * `test_place_cancel_order_ask()`
    /// * `test_place_cancel_order_bid()`
    /// * `test_place_cancel_order_stack()`
    ///
    /// # Failure testing
    ///
    /// * `test_cancel_order_internal_invalid_market_order_id()`
    /// * `test_cancel_order_internal_start_size_mismatch()`
    public(friend) fun cancel_order_internal(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        side: bool,
        start_size: u64,
        price: u64,
        order_access_key: u64,
        market_order_id: u128,
        reason: u8
    ): u128
    acquires
        MarketAccounts,
        MarketEventHandles
    {
        // Mutably borrow market accounts map.
        let market_accounts_map_ref_mut =
            &mut borrow_global_mut<MarketAccounts>(user_address).map;
        let market_account_id = // Get market account ID.
            ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128);
        let market_account_ref_mut = // Mutably borrow market account.
            table::borrow_mut(market_accounts_map_ref_mut, market_account_id);
        // Mutably borrow orders tablist, inactive orders stack top,
        // inbound asset ceiling, and outbound asset available fields,
        // and determine size multiplier for calculating change in
        // available and ceiling fields, based on order side.
        let (orders_ref_mut, stack_top_ref_mut, in_ceiling_ref_mut,
             out_available_ref_mut, size_multiplier_ceiling,
             size_multiplier_available) = if (side == ASK) (
                &mut market_account_ref_mut.asks,
                &mut market_account_ref_mut.asks_stack_top,
                &mut market_account_ref_mut.quote_ceiling,
                &mut market_account_ref_mut.base_available,
                price * market_account_ref_mut.tick_size,
                market_account_ref_mut.lot_size
            ) else (
                &mut market_account_ref_mut.bids,
                &mut market_account_ref_mut.bids_stack_top,
                &mut market_account_ref_mut.base_ceiling,
                &mut market_account_ref_mut.quote_available,
                market_account_ref_mut.lot_size,
                price * market_account_ref_mut.tick_size);
        let order_ref_mut = // Mutably borrow order to remove.
            tablist::borrow_mut(orders_ref_mut, order_access_key);
        let size = order_ref_mut.size; // Store order's size field.
        // Assert order starts off with expected size.
        assert!(size == start_size, E_START_SIZE_MISMATCH);
        // If passed market order ID is null, reassign its value to the
        // market order ID encoded in the order. Else assert that it is
        // equal to market order ID in user's order.
        if (market_order_id == (NIL as u128))
            market_order_id = order_ref_mut.market_order_id else
            assert!(order_ref_mut.market_order_id == market_order_id,
                    E_INVALID_MARKET_ORDER_ID);
        // Clear out order's market order ID field.
        order_ref_mut.market_order_id = (NIL as u128);
        // Mark order's size field to indicate top of inactive stack.
        order_ref_mut.size = *stack_top_ref_mut;
        // Reassign stack top field to indicate newly inactive order.
        *stack_top_ref_mut = order_access_key;
        // Calculate increment amount for outbound available field.
        let available_increment_amount = size * size_multiplier_available;
        *out_available_ref_mut = // Increment available field.
            *out_available_ref_mut + available_increment_amount;
        // Calculate decrement amount for inbound ceiling field.
        let ceiling_decrement_amount = size * size_multiplier_ceiling;
        *in_ceiling_ref_mut = // Decrement ceiling field.
            *in_ceiling_ref_mut - ceiling_decrement_amount;
        // If order is actually being cancelled and user has market
        // event handles for the market account, emit a cancel event.
        let changing_size = reason == CANCEL_REASON_SIZE_CHANGE_INTERNAL;
        if (!changing_size && exists<MarketEventHandles>(user_address)) {
            let market_event_handles_map_ref_mut =
                &mut borrow_global_mut<MarketEventHandles>(user_address).map;
            let has_handles_for_market_account = table::contains(
                market_event_handles_map_ref_mut, market_account_id);
            if (has_handles_for_market_account) {
                let handles_ref_mut = table::borrow_mut(
                    market_event_handles_map_ref_mut, market_account_id);
                event::emit_event(
                    &mut handles_ref_mut.cancel_order_events,
                    CancelOrderEvent{
                        market_id, order_id: market_order_id,
                        user: user_address, custodian_id, reason});
            }
        };
        market_order_id // Return market order ID.
    }

    /// Change the size of a user's open order on given side.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    /// * `side`: `ASK` or `BID`, the side on which an order was placed.
    /// * `start_size`: The open order size before size change.
    /// * `new_size`: New order size, in lots, checked during inner call
    ///   to `place_order_internal()`.
    /// * `price`: Order price, in ticks per lot.
    /// * `order_access_key`: Order access key for user order lookup.
    /// * `market_order_id`: Market order ID for order book lookup.
    ///
    /// # Aborts
    ///
    /// * `E_CHANGE_ORDER_NO_CHANGE`: No change in order size.
    ///
    /// # Emits
    ///
    /// * `ChangeOrderSizeEvent`: Information about an order that had a
    ///   manual size change.
    ///
    /// # Assumptions
    ///
    /// * Only called when also changing order size on the order book.
    /// * User has an open order under indicated market account with
    ///   provided access key, but not necessarily with provided market
    ///   order ID, which is checked in `cancel_order_internal()`.
    /// * `price` matches that encoded in market order ID for changed
    ///   order.
    ///
    /// # Testing
    ///
    /// * `test_change_order_size_internal_ask()`
    /// * `test_change_order_size_internal_bid()`
    /// * `test_change_order_size_internal_no_change()`
    public(friend) fun change_order_size_internal(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        side: bool,
        start_size: u64,
        new_size: u64,
        price: u64,
        order_access_key: u64,
        market_order_id: u128
    ) acquires
        MarketAccounts,
        MarketEventHandles
    {
        // Mutably borrow market accounts map.
        let market_accounts_map_ref_mut =
            &mut borrow_global_mut<MarketAccounts>(user_address).map;
        let market_account_id = // Get market account ID.
            ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128);
        let market_account_ref_mut = // Mutably borrow market account.
            table::borrow_mut(market_accounts_map_ref_mut, market_account_id);
        // Immutably borrow corresponding orders tablist based on side.
        let orders_ref = if (side == ASK)
            &market_account_ref_mut.asks else &market_account_ref_mut.bids;
        // Immutably borrow order.
        let order_ref = tablist::borrow(orders_ref, order_access_key);
        // Assert change in size.
        assert!(order_ref.size != new_size, E_CHANGE_ORDER_NO_CHANGE);
        cancel_order_internal( // Cancel order with size to be changed.
            user_address, market_id, custodian_id, side, start_size, price,
            order_access_key, market_order_id,
            CANCEL_REASON_SIZE_CHANGE_INTERNAL);
        place_order_internal( // Place order with new size.
            user_address, market_id, custodian_id, side, new_size, price,
            market_order_id, order_access_key);
        // If user has market event handles for the market account, emit
        // a change order size event.
        if (exists<MarketEventHandles>(user_address)) {
            let market_event_handles_map_ref_mut =
                &mut borrow_global_mut<MarketEventHandles>(user_address).map;
            let has_handles_for_market_account = table::contains(
                market_event_handles_map_ref_mut, market_account_id);
            if (has_handles_for_market_account) {
                let handles_ref_mut = table::borrow_mut(
                    market_event_handles_map_ref_mut, market_account_id);
                event::emit_event(
                    &mut handles_ref_mut.change_order_size_events,
                    ChangeOrderSizeEvent{
                        market_id, order_id: market_order_id,
                        user: user_address, custodian_id, side, new_size});
            }
        }
    }

    /// Return a `CancelOrderEvent` with the indicated fields.
    public(friend) fun create_cancel_order_event_internal(
        market_id: u64,
        order_id: u128,
        user: address,
        custodian_id: u64,
        reason: u8
    ): CancelOrderEvent {
        CancelOrderEvent{
            market_id,
            order_id,
            user,
            custodian_id,
            reason
        }
    }

    /// Return a `FillEvent` with the indicated fields.
    public(friend) fun create_fill_event_internal(
        market_id: u64,
        size: u64,
        price: u64,
        maker_side: bool,
        maker: address,
        maker_custodian_id: u64,
        maker_order_id: u128,
        taker: address,
        taker_custodian_id: u64,
        taker_order_id: u128,
        taker_quote_fees_paid: u64,
        sequence_number_for_trade: u64
    ): FillEvent {
        FillEvent{
            market_id,
            size,
            price,
            maker_side,
            maker,
            maker_custodian_id,
            maker_order_id,
            taker,
            taker_custodian_id,
            taker_order_id,
            taker_quote_fees_paid,
            sequence_number_for_trade
        }
    }

    /// Deposit base asset and quote coins when matching.
    ///
    /// Should only be called by the matching engine when matching from
    /// a user's market account.
    ///
    /// # Type parameters
    ///
    /// * `BaseType`: Base type for market.
    /// * `QuoteType`: Quote type for market.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    /// * `base_amount`: Base asset amount to deposit.
    /// * `optional_base_coins`: Optional base coins to deposit.
    /// * `quote_coins`: Quote coins to deposit.
    /// * `underwriter_id`: Underwriter ID for market.
    ///
    /// # Testing
    ///
    /// * `test_deposit_withdraw_assets_internal()`
    public(friend) fun deposit_assets_internal<
        BaseType,
        QuoteType
    >(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        base_amount: u64,
        optional_base_coins: Option<Coin<BaseType>>,
        quote_coins: Coin<QuoteType>,
        underwriter_id: u64
    ) acquires
        Collateral,
        MarketAccounts
    {
        deposit_asset<BaseType>( // Deposit base asset.
            user_address, market_id, custodian_id, base_amount,
            optional_base_coins, underwriter_id);
        deposit_coins<QuoteType>( // Deposit quote coins.
            user_address, market_id, custodian_id, quote_coins);
    }

    /// Emit limit order events to a user's market event handles.
    ///
    /// # Parameters
    ///
    /// * `market_id`: `PlaceLimitOrderEvent.market_id`.
    /// * `user`: `PlaceLimitOrderEvent.user`.
    /// * `custodian_id`: `PlaceLimitOrderEvent.custodian_id`.
    /// * `integrator`: `PlaceLimitOrderEvent.integrator`.
    /// * `side`: `PlaceLimitOrderEvent.side`.
    /// * `size`: `PlaceLimitOrderEvent.size`.
    /// * `price`: `PlaceLimitOrderEvent.price`.
    /// * `restriction`: `PlaceLimitOrderEvent.restriction`.
    /// * `self_match_behavior`:
    ///   `PlaceLimitOrderEvent.self_match_behavior`.
    /// * `remaining_size`: `PlaceLimitOrderEvent.remaining_size`.
    /// * `order_id`: `PlaceLimitOrderEvent.order_id`.
    /// * `fill_event_queue_ref`: Immutable reference to a vector of
    ///   `FillEvent`s to emit as part of a limit order that filled
    ///   across the spread, may be empty.
    /// * `cancel_reason_option_ref`: Immutable reference to an optional
    ///   cancel reason associated with a `CancelOrderEvent`.
    ///
    /// # Emits
    ///
    /// * `PlaceLimitOrderEvent`: Information about the limit order that
    ///   was placed.
    /// * `FillEvent`(s): Information about fill(s) across the spread as
    ///   a taker.
    /// * `CancelOrderEvent`: Optionally, information about why the
    ///   limit order may have had to be cancelled during the
    ///   transaction in which it was placed.
    public(friend) fun emit_limit_order_events_internal(
        market_id: u64,
        user: address,
        custodian_id: u64,
        integrator: address,
        side: bool,
        size: u64,
        price: u64,
        restriction: u8,
        self_match_behavior: u8,
        remaining_size: u64,
        order_id: u128,
        fill_event_queue_ref: &vector<FillEvent>,
        cancel_reason_option_ref: &Option<u8>
    ) acquires MarketEventHandles {
        // Only emit events to handles for the market account that
        // placed the order if they have been initialized.
        if (exists<MarketEventHandles>(user)) {
            let market_event_handles_map_ref_mut =
                &mut borrow_global_mut<MarketEventHandles>(user).map;
            let market_account_id = (((market_id as u128) << SHIFT_MARKET_ID) |
                                     (custodian_id as u128));
            let has_handles_for_market_account = table::contains(
                market_event_handles_map_ref_mut, market_account_id);
            if (has_handles_for_market_account) {
                let handles_ref_mut = table::borrow_mut(
                    market_event_handles_map_ref_mut, market_account_id);
                event::emit_event(
                    &mut handles_ref_mut.place_limit_order_events,
                    PlaceLimitOrderEvent{
                        market_id, user, custodian_id, integrator, side, size,
                        price, restriction, self_match_behavior,
                        remaining_size, order_id});
                // Loop over fill events, substituting order ID in case
                // order posted after fill event creation. Looping here
                // minimizes borrows from the user's account, but will
                // require looping again later to emit maker fill events
                // because the borrow checker prohibits simultaneous
                // borrowing of the same resource from two addresses.
                vector::for_each_ref(fill_event_queue_ref, |event_ref| {
                    let event: FillEvent = *event_ref;
                    event.taker_order_id = order_id;
                    event::emit_event(&mut handles_ref_mut.fill_events, event);
                });
                if (option::is_some(cancel_reason_option_ref)) {
                    let event = CancelOrderEvent{
                        market_id, order_id, user, custodian_id,
                        reason: *option::borrow(cancel_reason_option_ref)};
                    event::emit_event(
                        &mut handles_ref_mut.cancel_order_events, event);
                };
            };
        };
        // Emit fill events for all makers, similarly substituting
        // order ID in case order posted after fill event creation.
        vector::for_each_ref(fill_event_queue_ref, |event_ref| {
            let event: FillEvent = *event_ref;
            event.taker_order_id = order_id;
            emit_maker_fill_event(&event);
        });
    }

    /// Emit market order events to a user's market event handles.
    ///
    /// # Parameters
    ///
    /// * `market_id`: `PlaceMarketOrderEvent.market_id`.
    /// * `user`: `PlaceMarketOrderEvent.user`.
    /// * `custodian_id`: `PlaceMarketOrderEvent.custodian_id`.
    /// * `integrator`: `PlaceMarketOrderEvent.integrator`.
    /// * `direction`: `PlaceMarketOrderEvent.direction`.
    /// * `size`: `PlaceMarketOrderEvent.size`.
    /// * `self_match_behavior`:
    ///   `PlaceMarketOrderEvent.self_match_behavior`.
    /// * `order_id`: `PlaceMarketOrderEvent.order_id`.
    /// * `fill_event_queue_ref`: Immutable reference to a vector of
    ///   `FillEvent`s to emit, may be empty.
    /// * `cancel_reason_option_ref`: Immutable reference to an optional
    ///   cancel reason associated with a `CancelOrderEvent`.
    ///
    /// # Emits
    ///
    /// * `PlaceMarketOrderEvent`: Information about the market order
    ///   that was placed.
    /// * `FillEvent`(s): Information about fill(s).
    /// * `CancelOrderEvent`: Optionally, information about why the
    ///   market order was cancelled without completely filling.
    public(friend) fun emit_market_order_events_internal(
        market_id: u64,
        user: address,
        custodian_id: u64,
        integrator: address,
        direction: bool,
        size: u64,
        self_match_behavior: u8,
        order_id: u128,
        fill_event_queue_ref: &vector<FillEvent>,
        cancel_reason_option_ref: &Option<u8>
    ) acquires MarketEventHandles {
        // Only emit events to handles for the market account that
        // placed the order if they have been initialized.
        if (exists<MarketEventHandles>(user)) {
            let market_event_handles_map_ref_mut =
                &mut borrow_global_mut<MarketEventHandles>(user).map;
            let market_account_id = (((market_id as u128) << SHIFT_MARKET_ID) |
                                     (custodian_id as u128));
            let has_handles_for_market_account = table::contains(
                market_event_handles_map_ref_mut, market_account_id);
            if (has_handles_for_market_account) {
                let handles_ref_mut = table::borrow_mut(
                    market_event_handles_map_ref_mut, market_account_id);
                event::emit_event(
                    &mut handles_ref_mut.place_market_order_events,
                    PlaceMarketOrderEvent{
                        market_id, user, custodian_id, integrator, direction,
                        size, self_match_behavior, order_id});
                // Loop over fill events. Looping here minimizes borrows
                // from the user's account, but will require looping
                // again later to emit maker fill events because the
                // borrow checker prohibits simultaneous borrowing of
                // the same resource from two addresses.
                vector::for_each_ref(fill_event_queue_ref, |event_ref| {
                    event::emit_event(
                        &mut handles_ref_mut.fill_events, *event_ref);
                });
                if (option::is_some(cancel_reason_option_ref)) {
                    let event = CancelOrderEvent{
                        market_id, order_id, user, custodian_id,
                        reason: *option::borrow(cancel_reason_option_ref)};
                    event::emit_event(
                        &mut handles_ref_mut.cancel_order_events, event);
                };
            };
        };
        // Emit fill events for all makers.
        vector::for_each_ref(fill_event_queue_ref, |event_ref| {
            emit_maker_fill_event(event_ref);
        });
    }

    /// Emit a `FillEvent` for each maker associated with a swap.
    public(friend) fun emit_swap_maker_fill_events_internal(
        fill_event_queue_ref: &vector<FillEvent>
    ) acquires MarketEventHandles {
        vector::for_each_ref(fill_event_queue_ref, |event_ref| {
            emit_maker_fill_event(event_ref);
        });
    }

    /// Fill a user's order, routing collateral appropriately.
    ///
    /// Updates asset counts in a user's market account. Transfers
    /// coins as needed between a user's collateral, and an external
    /// source of coins passing through the matching engine. If a
    /// complete fill, pushes the newly inactive order to the top of the
    /// inactive orders stack for the given side.
    ///
    /// Should only be called by the matching engine, which has already
    /// calculated the corresponding amount of assets to fill. If the
    /// matching engine gets to this stage, then the user has an open
    /// order as indicated with sufficient assets to fill it. Hence no
    /// error checking.
    ///
    /// # Type parameters
    ///
    /// * `BaseType`: Base type for indicated market.
    /// * `QuoteType`: Quote type for indicated market.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    /// * `side`: `ASK` or `BID`, the side of the open order.
    /// * `order_access_key`: The open order's access key.
    /// * `start_size`: The open order size before filling.
    /// * `fill_size`: The number of lots filled.
    /// * `complete_fill`: `true` if order is completely filled.
    /// * `optional_base_coins`: Optional external base coins passing
    ///   through the matching engine.
    /// * `quote_coins`: External quote coins passing through the
    ///   matching engine.
    /// * `base_to_route`: Amount of base asset filled.
    /// * `quote_to_route`: Amount of quote asset filled.
    ///
    /// # Returns
    ///
    /// * `Option<Coin<BaseType>>`: Optional external base coins passing
    ///   through the matching engine.
    /// * `Coin<QuoteType>`: External quote coins passing through the
    ///   matching engine.
    /// * `u128`: Market order ID just filled against.
    ///
    /// # Aborts
    ///
    /// * `E_START_SIZE_MISMATCH`: Mismatch between expected size before
    ///   operation and actual size before operation.
    ///
    /// # Assumptions
    ///
    /// * Only called by the matching engine as described above.
    ///
    /// # Testing
    ///
    /// * `test_fill_order_internal_ask_complete_base_coin()`
    /// * `test_fill_order_internal_bid_complete_base_coin()`
    /// * `test_fill_order_internal_bid_partial_base_generic()`
    /// * `test_fill_order_internal_start_size_mismatch()`
    public(friend) fun fill_order_internal<
        BaseType,
        QuoteType
    >(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        side: bool,
        order_access_key: u64,
        start_size: u64,
        fill_size: u64,
        complete_fill: bool,
        optional_base_coins: Option<Coin<BaseType>>,
        quote_coins: Coin<QuoteType>,
        base_to_route: u64,
        quote_to_route: u64
    ): (
        Option<Coin<BaseType>>,
        Coin<QuoteType>,
        u128
    ) acquires
        Collateral,
        MarketAccounts
    {
        // Mutably borrow market accounts map.
        let market_accounts_map_ref_mut =
            &mut borrow_global_mut<MarketAccounts>(user_address).map;
        let market_account_id = // Get market account ID.
            ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128);
        let market_account_ref_mut = // Mutably borrow market account.
            table::borrow_mut(market_accounts_map_ref_mut, market_account_id);
        let ( // Mutably borrow corresponding orders tablist,
            orders_ref_mut,
            stack_top_ref_mut, // Inactive orders stack top,
            asset_in, // Amount of inbound asset,
            asset_in_total_ref_mut, // Inbound asset total field,
            asset_in_available_ref_mut, // Available field,
            asset_out, // Amount of outbound asset,
            asset_out_total_ref_mut, // Outbound asset total field,
            asset_out_ceiling_ref_mut, // And ceiling field.
        ) = if (side == ASK) ( // If an ask is matched:
            &mut market_account_ref_mut.asks,
            &mut market_account_ref_mut.asks_stack_top,
            quote_to_route,
            &mut market_account_ref_mut.quote_total,
            &mut market_account_ref_mut.quote_available,
            base_to_route,
            &mut market_account_ref_mut.base_total,
            &mut market_account_ref_mut.base_ceiling,
        ) else ( // If a bid is matched
            &mut market_account_ref_mut.bids,
            &mut market_account_ref_mut.bids_stack_top,
            base_to_route,
            &mut market_account_ref_mut.base_total,
            &mut market_account_ref_mut.base_available,
            quote_to_route,
            &mut market_account_ref_mut.quote_total,
            &mut market_account_ref_mut.quote_ceiling,
        );
        let order_ref_mut = // Mutably borrow corresponding order.
            tablist::borrow_mut(orders_ref_mut, order_access_key);
        // Store market order ID.
        let market_order_id = order_ref_mut.market_order_id;
        // Assert order starts off with expected size.
        assert!(order_ref_mut.size == start_size, E_START_SIZE_MISMATCH);
        if (complete_fill) { // If completely filling order:
            // Clear out order's market order ID field.
            order_ref_mut.market_order_id = (NIL as u128);
            // Mark order's size field to indicate inactive stack top.
            order_ref_mut.size = *stack_top_ref_mut;
            // Reassign stack top field to indicate new inactive order.
            *stack_top_ref_mut = order_access_key;
        } else { // If only partially filling the order:
            // Decrement amount still unfilled on order.
            order_ref_mut.size = order_ref_mut.size - fill_size;
        };
        // Increment asset in total amount by asset in amount.
        *asset_in_total_ref_mut = *asset_in_total_ref_mut + asset_in;
        // Increment asset in available amount by asset in amount.
        *asset_in_available_ref_mut = *asset_in_available_ref_mut + asset_in;
        // Decrement asset out total amount by asset out amount.
        *asset_out_total_ref_mut = *asset_out_total_ref_mut - asset_out;
        // Decrement asset out ceiling amount by asset out amount.
        *asset_out_ceiling_ref_mut = *asset_out_ceiling_ref_mut - asset_out;
        // If base coins to route:
        if (option::is_some(&optional_base_coins)) {
            // Mutably borrow base collateral map.
            let collateral_map_ref_mut =
                &mut borrow_global_mut<Collateral<BaseType>>(user_address).map;
            let collateral_ref_mut = // Mutably borrow base collateral.
                tablist::borrow_mut(collateral_map_ref_mut, market_account_id);
            let base_coins_ref_mut = // Mutably borrow external coins.
                option::borrow_mut(&mut optional_base_coins);
            // If filling as ask, merge to external coins those
            // extracted from user's collateral. Else if a bid, merge to
            // user's collateral those extracted from external coins.
            if (side == ASK)
                coin::merge(base_coins_ref_mut,
                    coin::extract(collateral_ref_mut, base_to_route)) else
                coin::merge(collateral_ref_mut,
                    coin::extract(base_coins_ref_mut, base_to_route));
        };
        // Mutably borrow quote collateral map.
        let collateral_map_ref_mut =
            &mut borrow_global_mut<Collateral<QuoteType>>(user_address).map;
        let collateral_ref_mut = // Mutably borrow quote collateral.
            tablist::borrow_mut(collateral_map_ref_mut, market_account_id);
        // If filling an ask, merge to user's collateral coins extracted
        // from external coins. Else if a bid, merge to external coins
        // those extracted from user's collateral.
        if (side == ASK)
            coin::merge(collateral_ref_mut,
                coin::extract(&mut quote_coins, quote_to_route)) else
            coin::merge(&mut quote_coins,
                coin::extract(collateral_ref_mut, quote_to_route));
        // Return optional base coins, quote coins, and market order ID.
        (optional_base_coins, quote_coins, market_order_id)
    }

    /// Return asset counts for specified market account.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    ///
    /// # Returns
    ///
    /// * `MarketAccount.base_total`
    /// * `MarketAccount.base_available`
    /// * `MarketAccount.base_ceiling`
    /// * `MarketAccount.quote_total`
    /// * `MarketAccount.quote_available`
    /// * `MarketAccount.quote_ceiling`
    ///
    /// # Aborts
    ///
    /// * `E_NO_MARKET_ACCOUNTS`: No market accounts resource found.
    /// * `E_NO_MARKET_ACCOUNT`: No market account resource found.
    ///
    /// # Testing
    ///
    /// * `test_deposits()`
    /// * `test_get_asset_counts_internal_no_account()`
    /// * `test_get_asset_counts_internal_no_accounts()`
    public(friend) fun get_asset_counts_internal(
        user_address: address,
        market_id: u64,
        custodian_id: u64
    ): (
        u64,
        u64,
        u64,
        u64,
        u64,
        u64
    ) acquires MarketAccounts {
        // Assert user has market accounts resource.
        assert!(exists<MarketAccounts>(user_address), E_NO_MARKET_ACCOUNTS);
        // Immutably borrow market accounts map.
        let market_accounts_map_ref =
            &borrow_global<MarketAccounts>(user_address).map;
        let market_account_id = // Get market account ID.
            ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128);
        // Assert user has market account for given market account ID.
        assert!(table::contains(market_accounts_map_ref, market_account_id),
                E_NO_MARKET_ACCOUNT);
        let market_account_ref = // Immutably borrow market account.
            table::borrow(market_accounts_map_ref, market_account_id);
        (market_account_ref.base_total,
         market_account_ref.base_available,
         market_account_ref.base_ceiling,
         market_account_ref.quote_total,
         market_account_ref.quote_available,
         market_account_ref.quote_ceiling) // Return asset count fields.
    }

    /// Return all active market order IDs for given market account.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    /// * `side`: `ASK` or `BID`, the side on which to check.
    ///
    /// # Returns
    ///
    /// * `vector<u128>`: Vector of all active market order IDs for
    ///   given market account and side, empty if none.
    ///
    /// # Aborts
    ///
    /// * `E_NO_MARKET_ACCOUNTS`: No market accounts resource found.
    /// * `E_NO_MARKET_ACCOUNT`: No market account resource found.
    ///
    /// # Testing
    ///
    /// * `test_get_active_market_order_ids_internal()`
    /// * `test_get_active_market_order_ids_internal_no_account()`
    /// * `test_get_active_market_order_ids_internal_no_accounts()`
    public(friend) fun get_active_market_order_ids_internal(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        side: bool,
    ): vector<u128>
    acquires MarketAccounts {
        // Assert user has market accounts resource.
        assert!(exists<MarketAccounts>(user_address), E_NO_MARKET_ACCOUNTS);
        // Immutably borrow market accounts map.
        let market_accounts_map_ref =
            &borrow_global<MarketAccounts>(user_address).map;
        let market_account_id = // Get market account ID.
            ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128);
        // Assert user has market account for given market account ID.
        assert!(table::contains(market_accounts_map_ref, market_account_id),
                E_NO_MARKET_ACCOUNT);
        let market_account_ref = // Immutably borrow market account.
            table::borrow(market_accounts_map_ref, market_account_id);
        // Immutably borrow corresponding orders tablist based on side.
        let orders_ref = if (side == ASK)
            &market_account_ref.asks else &market_account_ref.bids;
        // Initialize empty vector of market order IDs.
        let market_order_ids = vector::empty();
        // Initialize 1-indexed loop counter and get number of orders.
        let (i, n) = (1, tablist::length(orders_ref));
        while (i <= n) { // Loop over all allocated orders.
            // Immutably borrow order with given access key.
            let order_ref = tablist::borrow(orders_ref, i);
            // If order is active, push back its market order ID.
            if (order_ref.market_order_id != (NIL as u128)) vector::push_back(
                &mut market_order_ids, order_ref.market_order_id);
            i = i + 1; // Increment loop counter.
        };
        market_order_ids // Return market order IDs.
    }

    /// Return order access key for next placed order.
    ///
    /// If inactive orders stack top is empty, will be next 1-indexed
    /// order access key to be allocated. Otherwise is order access key
    /// at top of inactive order stack.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    /// * `side`: `ASK` or `BID`, the side on which an order will be
    ///   placed.
    ///
    /// # Returns
    ///
    /// * `u64`: Order access key of next order to be placed.
    ///
    /// # Aborts
    ///
    /// * `E_NO_MARKET_ACCOUNTS`: No market accounts resource found.
    /// * `E_NO_MARKET_ACCOUNT`: No market account resource found.
    ///
    /// # Testing
    ///
    /// * `test_get_next_order_access_key_internal_no_account()`
    /// * `test_get_next_order_access_key_internal_no_accounts()`
    /// * `test_place_cancel_order_ask()`
    /// * `test_place_cancel_order_stack()`
    public(friend) fun get_next_order_access_key_internal(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        side: bool
    ): u64
    acquires MarketAccounts {
        // Assert user has market accounts resource.
        assert!(exists<MarketAccounts>(user_address), E_NO_MARKET_ACCOUNTS);
        // Immutably borrow market accounts map.
        let market_accounts_map_ref =
            &borrow_global<MarketAccounts>(user_address).map;
        let market_account_id = // Get market account ID.
            ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128);
        let has_market_account = // Check if user has market account.
            table::contains(market_accounts_map_ref, market_account_id);
        // Assert user has market account for given market account ID.
        assert!(has_market_account, E_NO_MARKET_ACCOUNT);
        let market_account_ref = // Mutably borrow market account.
            table::borrow(market_accounts_map_ref, market_account_id);
        // Get orders tablist and inactive order stack top for side.
        let (orders_ref, stack_top_ref) = if (side == ASK)
            (&market_account_ref.asks, &market_account_ref.asks_stack_top) else
            (&market_account_ref.bids, &market_account_ref.bids_stack_top);
        // If empty inactive order stack, return 1-indexed order access
        // key for order that will need to be allocated.
        if (*stack_top_ref == NIL) tablist::length(orders_ref) + 1 else
            *stack_top_ref // Otherwise the top of the inactive stack.
    }

    /// Return optional market order ID corresponding to open order for
    /// `user`, `market_id`, `custodian_id`, `side`, and
    /// `order_access_key`, if one exists.
    ///
    /// Restricted to public friend to prevent runtime user state
    /// contention.
    ///
    /// # Testing
    ///
    /// * `test_market_account_getters()`
    /// * `test_change_order_size_internal_ask()`
    /// * `test_change_order_size_internal_bid()`
    public(friend) fun get_open_order_id_internal(
        user: address,
        market_id: u64,
        custodian_id: u64,
        side: bool,
        order_access_key: u64
    ): Option<u128>
    acquires MarketAccounts {
        // Get market account ID.
        let market_account_id = get_market_account_id(market_id, custodian_id);
        // Return empty option if no corresponding market account.
        if (!has_market_account_by_market_account_id(user, market_account_id))
            return option::none();
        // Immutably borrow market accounts map.
        let market_accounts_map_ref = &borrow_global<MarketAccounts>(user).map;
        // Immutably borrow market account.
        let market_account_ref = table::borrow(
            market_accounts_map_ref, market_account_id);
        // Immutably borrow open orders for given side.
        let open_orders_ref = if (side == ASK) &market_account_ref.asks else
            &market_account_ref.bids;
        // Return empty option if no open order with given access key.
        if (!tablist::contains(open_orders_ref, order_access_key))
            return option::none();
        option::some( // Return option-packed market order ID.
            tablist::borrow(open_orders_ref, order_access_key).market_order_id)
    }

    /// Place order in user's tablist of open orders on given side.
    ///
    /// Range checks order parameters and updates asset counts
    /// accordingly.
    ///
    /// Allocates a new order if the inactive order stack is empty,
    /// otherwise pops one off the top of the stack and overwrites it.
    ///
    /// Should only be called when attempting to place an order on the
    /// order book. Since order book entries list order access keys for
    /// each corresponding user, `get_next_order_access_key_internal()`
    /// needs to be called when generating an entry on the order book:
    /// to insert to the order book, an order access key is first
    /// required. Once an order book entry has been created, a market
    /// order ID will then be made available.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    /// * `side`: `ASK` or `BID`, the side on which an order is placed.
    /// * `size`: Order size, in lots.
    /// * `price`: Order price, in ticks per lot.
    /// * `market_order_id`: Market order ID for order book access.
    /// * `order_access_key_expected`: Expected order access key to be
    ///   assigned to order.
    ///
    /// # Terminology
    ///
    /// * The "inbound" asset is the asset received from a trade.
    /// * The "outbound" asset is the asset traded away.
    ///
    /// # Assumptions
    ///
    /// * Only called when also placing an order on the order book.
    /// * `price` matches that encoded in `market_order_id`.
    /// * Existence of corresponding market account has already been
    ///   verified by `get_next_order_access_key_internal()`.
    ///
    /// # Aborts
    ///
    /// * `E_PRICE_0`: Price is zero.
    /// * `E_PRICE_TOO_HIGH`: Price exceeds maximum possible price.
    /// * `E_TICKS_OVERFLOW`: Ticks to fill order overflows a `u64`.
    /// * `E_OVERFLOW_ASSET_IN`: Filling order would overflow asset
    ///   received from trade.
    /// * `E_NOT_ENOUGH_ASSET_OUT`: Not enough asset to trade away.
    /// * `E_ACCESS_KEY_MISMATCH`: Expected order access key does not
    ///   match assigned order access key.
    ///
    /// # Expected value testing
    ///
    /// * `test_place_cancel_order_ask()`
    /// * `test_place_cancel_order_bid()`
    /// * `test_place_cancel_order_stack()`
    ///
    /// # Failure testing
    ///
    /// * `test_place_order_internal_access_key_mismatch()`
    /// * `test_place_order_internal_in_overflow()`
    /// * `test_place_order_internal_out_underflow()`
    /// * `test_place_order_internal_price_0()`
    /// * `test_place_order_internal_price_hi()`
    /// * `test_place_order_internal_ticks_overflow()`
    public(friend) fun place_order_internal(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        side: bool,
        size: u64,
        price: u64,
        market_order_id: u128,
        order_access_key_expected: u64
    ) acquires MarketAccounts {
        assert!(price > 0, E_PRICE_0); // Assert price is nonzero.
        // Assert price is not too high.
        assert!(price <= HI_PRICE, E_PRICE_TOO_HIGH);
        // Mutably borrow market accounts map.
        let market_accounts_map_ref_mut =
            &mut borrow_global_mut<MarketAccounts>(user_address).map;
        let market_account_id = // Get market account ID.
            ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128);
        let market_account_ref_mut = // Mutably borrow market account.
            table::borrow_mut(market_accounts_map_ref_mut, market_account_id);
        let base_fill = // Calculate base units needed to fill order.
            (size as u128) * (market_account_ref_mut.lot_size as u128);
        // Calculate ticks to fill order.
        let ticks = (size as u128) * (price as u128);
        // Assert ticks to fill order is not too large.
        assert!(ticks <= (HI_64 as u128), E_TICKS_OVERFLOW);
        // Calculate quote units to fill order.
        let quote_fill = ticks * (market_account_ref_mut.tick_size as u128);
        // Mutably borrow orders tablist, inactive orders stack top,
        // inbound asset ceiling, and outbound asset available fields,
        // and assign inbound and outbound asset fill amounts, based on
        // order side.
        let (orders_ref_mut, stack_top_ref_mut, in_ceiling_ref_mut,
             out_available_ref_mut, in_fill, out_fill) = if (side == ASK)
             (&mut market_account_ref_mut.asks,
              &mut market_account_ref_mut.asks_stack_top,
              &mut market_account_ref_mut.quote_ceiling,
              &mut market_account_ref_mut.base_available,
              quote_fill, base_fill) else
             (&mut market_account_ref_mut.bids,
              &mut market_account_ref_mut.bids_stack_top,
              &mut market_account_ref_mut.base_ceiling,
              &mut market_account_ref_mut.quote_available,
              base_fill, quote_fill);
        // Assert no inbound asset overflow.
        assert!((in_fill + (*in_ceiling_ref_mut as u128)) <= (HI_64 as u128),
                E_OVERFLOW_ASSET_IN);
        // Assert enough outbound asset to cover the fill, which also
        // ensures outbound fill amount does not overflow.
        assert!((out_fill <= (*out_available_ref_mut as u128)),
                E_NOT_ENOUGH_ASSET_OUT);
        // Update ceiling for inbound asset.
        *in_ceiling_ref_mut = *in_ceiling_ref_mut + (in_fill as u64);
        // Update available amount for outbound asset.
        *out_available_ref_mut = *out_available_ref_mut - (out_fill as u64);
        // Get order access key. If empty inactive stack:
        let order_access_key = if (*stack_top_ref_mut == NIL) {
            // Get one-indexed order access key for new order.
            let order_access_key = tablist::length(orders_ref_mut) + 1;
            // Allocate new order.
            tablist::add(orders_ref_mut, order_access_key, Order{
                market_order_id, size});
            order_access_key // Store order access key locally.
        } else { // If inactive order stack not empty:
            // Order access key is for inactive order at top of stack.
            let order_access_key = *stack_top_ref_mut;
            let order_ref_mut = // Mutably borrow order at top of stack.
                tablist::borrow_mut(orders_ref_mut, order_access_key);
            // Reassign stack top field to next in stack.
            *stack_top_ref_mut = order_ref_mut.size;
            // Reassign market order ID for active order.
            order_ref_mut.market_order_id = market_order_id;
            order_ref_mut.size = size; // Reassign order size field.
            order_access_key // Store order access key locally.
        };
        // Assert order access key is as expected.
        assert!(order_access_key == order_access_key_expected,
                E_ACCESS_KEY_MISMATCH);
    }

    /// Withdraw base asset and quote coins when matching.
    ///
    /// Should only be called by the matching engine when matching from
    /// a user's market account.
    ///
    /// # Type parameters
    ///
    /// * `BaseType`: Base type for market.
    /// * `QuoteType`: Quote type for market.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    /// * `base_amount`: Base asset amount to withdraw.
    /// * `quote_amount`: Quote asset amount to withdraw.
    /// * `underwriter_id`: Underwriter ID for market.
    ///
    /// # Returns
    ///
    /// * `Option<Coin<BaseType>>`: Optional base coins from user's
    ///   market account.
    /// * `<Coin<QuoteType>`: Quote coins from user's market account.
    ///
    /// # Testing
    ///
    /// * `test_deposit_withdraw_assets_internal()`
    public(friend) fun withdraw_assets_internal<
        BaseType,
        QuoteType,
    >(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        base_amount: u64,
        quote_amount: u64,
        underwriter_id: u64
    ): (
        Option<Coin<BaseType>>,
        Coin<QuoteType>
    ) acquires
        Collateral,
        MarketAccounts
    {
        // Return optional base coins, and quote coins per respective
        // withdrawal functions.
        (withdraw_asset<BaseType>(user_address, market_id, custodian_id,
                                  base_amount, underwriter_id),
         withdraw_coins<QuoteType>(user_address, market_id, custodian_id,
                                   quote_amount))
    }

    // Public friend functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Private functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Deposit an asset to a user's market account.
    ///
    /// Update asset counts, deposit optional coins as collateral.
    ///
    /// # Type parameters
    ///
    /// * `AssetType`: Asset type to deposit, `registry::GenericAsset`
    ///   if a generic asset.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    /// * `amount`: Amount to deposit.
    /// * `optional_coins`: Optional coins to deposit.
    /// * `underwriter_id`: Underwriter ID for market, ignored when
    ///   depositing coins.
    ///
    /// # Aborts
    ///
    /// * `E_NO_MARKET_ACCOUNTS`: No market accounts resource found.
    /// * `E_NO_MARKET_ACCOUNT`: No market account resource found.
    /// * `E_ASSET_NOT_IN_PAIR`: Asset type is not in trading pair for
    ///    market.
    /// * `E_DEPOSIT_OVERFLOW_ASSET_CEILING`: Deposit would overflow
    ///   asset ceiling.
    /// * `E_INVALID_UNDERWRITER`: Underwriter is not valid for
    ///   indicated market, in the case of a generic asset deposit.
    ///
    /// # Assumptions
    ///
    /// * When depositing coins, if a market account exists, then so
    ///   does a corresponding collateral map entry.
    ///
    /// # Testing
    ///
    /// * `test_deposit_asset_amount_mismatch()`
    /// * `test_deposit_asset_no_account()`
    /// * `test_deposit_asset_no_accounts()`
    /// * `test_deposit_asset_not_in_pair()`
    /// * `test_deposit_asset_overflow()`
    /// * `test_deposit_asset_underwriter()`
    /// * `test_deposits()`
    fun deposit_asset<
        AssetType
    >(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        amount: u64,
        optional_coins: Option<Coin<AssetType>>,
        underwriter_id: u64
    ) acquires
        Collateral,
        MarketAccounts
    {
        // Assert user has market accounts resource.
        assert!(exists<MarketAccounts>(user_address), E_NO_MARKET_ACCOUNTS);
        // Mutably borrow market accounts map.
        let market_accounts_map_ref_mut =
            &mut borrow_global_mut<MarketAccounts>(user_address).map;
        let market_account_id = // Get market account ID.
            ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128);
        let has_market_account = // Check if user has market account.
            table::contains(market_accounts_map_ref_mut, market_account_id);
        // Assert user has market account for given market account ID.
        assert!(has_market_account, E_NO_MARKET_ACCOUNT);
        let market_account_ref_mut = // Mutably borrow market account.
            table::borrow_mut(market_accounts_map_ref_mut, market_account_id);
        // Get asset type info.
        let asset_type = type_info::type_of<AssetType>();
        // Get asset total, available, and ceiling amounts based on if
        // asset is base or quote for trading pair, aborting if neither.
        let (total_ref_mut, available_ref_mut, ceiling_ref_mut) =
            if (asset_type == market_account_ref_mut.base_type) (
                &mut market_account_ref_mut.base_total,
                &mut market_account_ref_mut.base_available,
                &mut market_account_ref_mut.base_ceiling
            ) else if (asset_type == market_account_ref_mut.quote_type) (
                &mut market_account_ref_mut.quote_total,
                &mut market_account_ref_mut.quote_available,
                &mut market_account_ref_mut.quote_ceiling
            ) else abort E_ASSET_NOT_IN_PAIR;
        assert!( // Assert deposit does not overflow asset ceiling.
            ((*ceiling_ref_mut as u128) + (amount as u128)) <= (HI_64 as u128),
            E_DEPOSIT_OVERFLOW_ASSET_CEILING);
        *total_ref_mut = *total_ref_mut + amount; // Update total.
        // Update available asset amount.
        *available_ref_mut = *available_ref_mut + amount;
        *ceiling_ref_mut = *ceiling_ref_mut + amount; // Update ceiling.
        // If asset is generic:
        if (asset_type == type_info::type_of<GenericAsset>()) {
            assert!(underwriter_id == market_account_ref_mut.underwriter_id,
                    E_INVALID_UNDERWRITER); // Assert underwriter ID.
            option::destroy_none(optional_coins); // Destroy option.
        } else { // If asset is coin:
            // Extract coins from option.
            let coins = option::destroy_some(optional_coins);
            // Assert passed amount matches coin value.
            assert!(amount == coin::value(&coins), E_COIN_AMOUNT_MISMATCH);
            // Mutably borrow collateral map.
            let collateral_map_ref_mut = &mut borrow_global_mut<
                Collateral<AssetType>>(user_address).map;
            // Mutably borrow collateral for market account.
            let collateral_ref_mut = tablist::borrow_mut(
                collateral_map_ref_mut, market_account_id);
            // Merge coins into collateral.
            coin::merge(collateral_ref_mut, coins);
        };
    }

    /// Emit a `FillEvent` for the market account of the maker
    /// associated with a fill, if market event handles exist for the
    /// indicated market account.
    inline fun emit_maker_fill_event(
        event_ref: &FillEvent
    ) acquires MarketEventHandles {
        let maker = event_ref.maker;
        if (exists<MarketEventHandles>(maker)) {
            let market_event_handles_map_ref_mut =
                &mut borrow_global_mut<MarketEventHandles>(maker).map;
            let market_id = event_ref.market_id;
            let custodian_id = event_ref.maker_custodian_id;
            let market_account_id = (((market_id as u128) << SHIFT_MARKET_ID) |
                                     (custodian_id as u128));
            let has_handles_for_market_account = table::contains(
                market_event_handles_map_ref_mut, market_account_id);
            if (has_handles_for_market_account) {
                let handles_ref_mut = table::borrow_mut(
                    market_event_handles_map_ref_mut, market_account_id);
                event::emit_event(
                    &mut handles_ref_mut.fill_events, *event_ref);
            };
        };
    }

    /// Return `registry::MarketInfo` fields stored in market account.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    ///
    /// # Returns
    ///
    /// * `MarketAccount.base_type`
    /// * `MarketAccount.base_name_generic`
    /// * `MarketAccount.quote_type`
    /// * `MarketAccount.lot_size`
    /// * `MarketAccount.tick_size`
    /// * `MarketAccount.min_size`
    /// * `MarketAccount.underwriter_id`
    ///
    /// # Aborts
    ///
    /// * `E_NO_MARKET_ACCOUNTS`: No market accounts resource found.
    /// * `E_NO_MARKET_ACCOUNT`: No market account resource found.
    ///
    /// # Testing
    ///
    /// * `test_get_market_account_market_info_no_account()`
    /// * `test_get_market_account_market_info_no_accounts()`
    /// * `test_register_market_accounts()`
    fun get_market_account_market_info(
        user_address: address,
        market_id: u64,
        custodian_id: u64
    ): (
        TypeInfo,
        String,
        TypeInfo,
        u64,
        u64,
        u64,
        u64
    ) acquires MarketAccounts {
        // Assert user has market accounts resource.
        assert!(exists<MarketAccounts>(user_address), E_NO_MARKET_ACCOUNTS);
        // Immutably borrow market accounts map.
        let market_accounts_map_ref =
            &borrow_global<MarketAccounts>(user_address).map;
        let market_account_id = // Get market account ID.
            ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128);
        // Assert user has market account for given market account ID.
        assert!(table::contains(market_accounts_map_ref, market_account_id),
                E_NO_MARKET_ACCOUNT);
        let market_account_ref = // Immutably borrow market account.
            table::borrow(market_accounts_map_ref, market_account_id);
         // Return duplicate market info fields.
        (market_account_ref.base_type,
         market_account_ref.base_name_generic,
         market_account_ref.quote_type,
         market_account_ref.lot_size,
         market_account_ref.tick_size,
         market_account_ref.min_size,
         market_account_ref.underwriter_id)
    }

    /// Register market account entries for given market account info.
    ///
    /// Inner function for `register_market_account()`.
    ///
    /// Verifies market ID, base type, and quote type correspond to a
    /// registered market, via call to
    /// `registry::get_market_info_for_market_account()`.
    ///
    /// # Type parameters
    ///
    /// * `BaseType`: Base type for indicated market.
    /// * `QuoteType`: Quote type for indicated market.
    ///
    /// # Parameters
    ///
    /// * `user`: User registering a market account.
    /// * `market_account_id`: Market account ID for given market.
    /// * `market_id`: Market ID for given market.
    /// * `custodian_id`: Custodian ID to register account with, or
    ///   `NO_CUSTODIAN`.
    ///
    /// # Aborts
    ///
    /// * `E_EXISTS_MARKET_ACCOUNT`: Market account already exists.
    ///
    /// # Testing
    ///
    /// * `test_register_market_account_account_entries_exists()`
    /// * `test_register_market_accounts()`
    fun register_market_account_account_entries<
        BaseType,
        QuoteType
    >(
        user: &signer,
        market_account_id: u128,
        market_id: u64,
        custodian_id: u64
    ) acquires MarketAccounts {
        let user_address = address_of(user); // Get user address.
        let (base_type, quote_type) = // Get base and quote types.
            (type_info::type_of<BaseType>(), type_info::type_of<QuoteType>());
        // Get market info and verify market ID, base and quote types.
        let (base_name_generic, lot_size, tick_size, min_size, underwriter_id)
            = registry::get_market_info_for_market_account(
                market_id, base_type, quote_type);
        // If user does not have a market accounts map initialized:
        if (!exists<MarketAccounts>(user_address))
            // Pack an empty one and move it to their account
            move_to<MarketAccounts>(user, MarketAccounts{
                map: table::new(), custodians: tablist::new()});
        // Mutably borrow market accounts map.
        let market_accounts_map_ref_mut =
            &mut borrow_global_mut<MarketAccounts>(user_address).map;
        assert!( // Assert no entry exists for given market account ID.
            !table::contains(market_accounts_map_ref_mut, market_account_id),
            E_EXISTS_MARKET_ACCOUNT);
        table::add( // Add empty market account for market account ID.
            market_accounts_map_ref_mut, market_account_id, MarketAccount{
                base_type, base_name_generic, quote_type, lot_size, tick_size,
                min_size, underwriter_id, asks: tablist::new(),
                bids: tablist::new(), asks_stack_top: NIL, bids_stack_top: NIL,
                base_total: 0, base_available: 0, base_ceiling: 0,
                quote_total: 0, quote_available: 0, quote_ceiling: 0});
        let custodians_ref_mut = // Mutably borrow custodians maps.
            &mut borrow_global_mut<MarketAccounts>(user_address).custodians;
        // If custodians map has no entry for given market ID:
        if (!tablist::contains(custodians_ref_mut, market_id)) {
            // Add new entry indicating new custodian ID.
            tablist::add(custodians_ref_mut, market_id,
                         vector::singleton(custodian_id));
        } else { // If already entry for given market ID:
            // Mutably borrow vector of custodians for given market.
            let market_custodians_ref_mut =
                tablist::borrow_mut(custodians_ref_mut, market_id);
            // Push back custodian ID for given market account.
            vector::push_back(market_custodians_ref_mut, custodian_id);
        }
    }

    /// Create collateral entry upon market account registration.
    ///
    /// Inner function for `register_market_account()`.
    ///
    /// Does not check if collateral entry already exists for given
    /// market account ID, as market account existence check already
    /// performed by `register_market_account_accounts_entries()` in
    /// `register_market_account()`.
    ///
    /// # Type parameters
    ///
    /// * `CoinType`: Phantom coin type for indicated market.
    ///
    /// # Parameters
    ///
    /// * `user`: User registering a market account.
    /// * `market_account_id`: Market account ID for given market.
    ///
    /// # Testing
    ///
    /// * `test_register_market_accounts()`
    fun register_market_account_collateral_entry<
        CoinType
    >(
        user: &signer,
        market_account_id: u128
    ) acquires Collateral {
        let user_address = address_of(user); // Get user address.
        // If user does not have a collateral map initialized, pack an
        // empty one and move it to their account.
        if (!exists<Collateral<CoinType>>(user_address))
            move_to<Collateral<CoinType>>(user, Collateral{
                map: tablist::new()});
        let collateral_map_ref_mut = // Mutably borrow collateral map.
            &mut borrow_global_mut<Collateral<CoinType>>(user_address).map;
        // Add an empty entry for given market account ID.
        tablist::add(collateral_map_ref_mut, market_account_id,
                     coin::zero<CoinType>());
    }

    /// Convert a tablist of `Order` into a vector of only open orders.
    ///
    /// # Testing
    ///
    /// * `test_get_market_accounts_open_orders()`
    fun vectorize_open_orders(
        tablist_ref_mut: &mut Tablist<u64, Order>,
    ): vector<Order> {
        let open_orders = vector::empty(); // Get empty orders vector.
        // Get optional head key.
        let optional_access_key = tablist::get_head_key(tablist_ref_mut);
        // While keys left to iterate on:
        while (option::is_some(&optional_access_key)) {
            // Get open order and next optional access key in tablist.
            let (order, _, next) = tablist::remove_iterable(
                tablist_ref_mut, *option::borrow(&optional_access_key));
            // If market order ID flagged as null:
            if (order.market_order_id == (NIL as u128)) {
                // Unpack order and drop fields.
                let Order{market_order_id: _, size: _} = order;
            } else { // Otherwise, if order is active:
                // Push back onto vector of open orders.
                vector::push_back(&mut open_orders, order);
            };
            // Review next optional access key.
            optional_access_key = next;
        };
        open_orders // Return vectorized open orders.
    }

    /// Withdraw an asset from a user's market account.
    ///
    /// Update asset counts, withdraw optional collateral coins.
    ///
    /// # Type parameters
    ///
    /// * `AssetType`: Asset type to withdraw, `registry::GenericAsset`
    ///   if a generic asset.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Market ID for market account.
    /// * `custodian_id`: Custodian ID for market account.
    /// * `amount`: Amount to withdraw.
    /// * `underwriter_id`: Underwriter ID for market, ignored when
    ///   withdrawing coins.
    ///
    /// # Returns
    ///
    /// * `Option<Coin<AssetType>>`: Optional collateral coins.
    ///
    /// # Aborts
    ///
    /// * `E_NO_MARKET_ACCOUNTS`: No market accounts resource found.
    /// * `E_NO_MARKET_ACCOUNT`: No market account resource found.
    /// * `E_ASSET_NOT_IN_PAIR`: Asset type is not in trading pair for
    ///    market.
    /// * `E_WITHDRAW_TOO_LITTLE_AVAILABLE`: Too little available for
    ///   withdrawal.
    /// * `E_INVALID_UNDERWRITER`: Underwriter is not valid for
    ///   indicated market, in the case of a generic asset withdrawal.
    ///
    /// # Testing
    ///
    /// * `test_withdraw_asset_no_account()`
    /// * `test_withdraw_asset_no_accounts()`
    /// * `test_withdraw_asset_not_in_pair()`
    /// * `test_withdraw_asset_underflow()`
    /// * `test_withdraw_asset_underwriter()`
    /// * `test_withdrawals()`
    fun withdraw_asset<
        AssetType
    >(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        amount: u64,
        underwriter_id: u64
    ): Option<Coin<AssetType>>
    acquires
        Collateral,
        MarketAccounts
    {
        // Assert user has market accounts resource.
        assert!(exists<MarketAccounts>(user_address), E_NO_MARKET_ACCOUNTS);
        // Mutably borrow market accounts map.
        let market_accounts_map_ref_mut =
            &mut borrow_global_mut<MarketAccounts>(user_address).map;
        let market_account_id = // Get market account ID.
            ((market_id as u128) << SHIFT_MARKET_ID) | (custodian_id as u128);
        let has_market_account = // Check if user has market account.
            table::contains(market_accounts_map_ref_mut, market_account_id);
        // Assert user has market account for given market account ID.
        assert!(has_market_account, E_NO_MARKET_ACCOUNT);
        let market_account_ref_mut = // Mutably borrow market account.
            table::borrow_mut(market_accounts_map_ref_mut, market_account_id);
        // Get asset type info.
        let asset_type = type_info::type_of<AssetType>();
        // Get asset total, available, and ceiling amounts based on if
        // asset is base or quote for trading pair, aborting if neither.
        let (total_ref_mut, available_ref_mut, ceiling_ref_mut) =
            if (asset_type == market_account_ref_mut.base_type) (
                &mut market_account_ref_mut.base_total,
                &mut market_account_ref_mut.base_available,
                &mut market_account_ref_mut.base_ceiling
            ) else if (asset_type == market_account_ref_mut.quote_type) (
                &mut market_account_ref_mut.quote_total,
                &mut market_account_ref_mut.quote_available,
                &mut market_account_ref_mut.quote_ceiling
            ) else abort E_ASSET_NOT_IN_PAIR;
        // Assert enough asset available for withdraw.
        assert!(amount <= *available_ref_mut, E_WITHDRAW_TOO_LITTLE_AVAILABLE);
        *total_ref_mut = *total_ref_mut - amount; // Update total.
        // Update available asset amount.
        *available_ref_mut = *available_ref_mut - amount;
        *ceiling_ref_mut = *ceiling_ref_mut - amount; // Update ceiling.
        // Return based on if asset type. If is generic:
        return if (asset_type == type_info::type_of<GenericAsset>()) {
            assert!(underwriter_id == market_account_ref_mut.underwriter_id,
                    E_INVALID_UNDERWRITER); // Assert underwriter ID.
            option::none() // Return empty option.
        } else { // If asset is coin:
            // Mutably borrow collateral map.
            let collateral_map_ref_mut = &mut borrow_global_mut<
                Collateral<AssetType>>(user_address).map;
            // Mutably borrow collateral for market account.
            let collateral_ref_mut = tablist::borrow_mut(
                collateral_map_ref_mut, market_account_id);
            // Withdraw coin and return in an option.
            option::some<Coin<AssetType>>(
                coin::extract(collateral_ref_mut, amount))
        }
    }

    /// Wrapped call to `withdraw_asset()` for withdrawing coins.
    ///
    /// # Testing
    ///
    /// * `test_withdrawals()`
    fun withdraw_coins<
        CoinType
    >(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        amount: u64,
    ): Coin<CoinType>
    acquires
        Collateral,
        MarketAccounts
    {
        option::destroy_some(withdraw_asset<CoinType>(
            user_address,
            market_id,
            custodian_id,
            amount,
            NO_UNDERWRITER))
    }

    /// Wrapped call to `withdraw_asset()` for withdrawing generic
    /// asset.
    ///
    /// # Testing
    ///
    /// * `test_withdrawals()`
    fun withdraw_generic_asset(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        amount: u64,
        underwriter_capability_ref: &UnderwriterCapability
    ) acquires
        Collateral,
        MarketAccounts
    {
        option::destroy_none(withdraw_asset<GenericAsset>(
            user_address,
            market_id,
            custodian_id,
            amount,
            registry::get_underwriter_id(underwriter_capability_ref)))
    }

    // Private functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only constants >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    /// Base asset starting amount for testing.
    const BASE_START: u64 = 7500000000;
    #[test_only]
    /// Quote asset starting amount for testing.
    const QUOTE_START: u64 = 8000000000;

    #[test_only]
    /// Custodian ID for market with delegated custodian.
    const CUSTODIAN_ID: u64 = 123;
    #[test_only]
    /// Market ID for generic test market.
    const MARKET_ID_GENERIC: u64 = 2;
    #[test_only]
    /// Market ID for pure coin test market.
    const MARKET_ID_PURE_COIN: u64 = 1;
    #[test_only]
    /// From `registry::register_markets_test()`. Underwriter ID for
    /// generic test market.
    const UNDERWRITER_ID: u64 = 7;

    #[test_only]
    /// From `registry::register_markets_test()`.
    const LOT_SIZE_PURE_COIN: u64 = 1;
    #[test_only]
    /// From `registry::register_markets_test()`.
    const TICK_SIZE_PURE_COIN: u64 = 2;
    #[test_only]
    /// From `registry::register_markets_test()`.
    const MIN_SIZE_PURE_COIN: u64 = 3;
    #[test_only]
    /// From `registry::register_markets_test()`.
    const LOT_SIZE_GENERIC: u64 = 4;
    #[test_only]
    /// From `registry::register_markets_test()`.
    const TICK_SIZE_GENERIC: u64 = 5;
    #[test_only]
    /// From `registry::register_markets_test()`.
    const MIN_SIZE_GENERIC: u64 = 6;

    // Test-only constants <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    /// Immutably borrow market event handles for a market account.
    inline fun borrow_market_event_handles_for_market_account_test(
        market_id: u64,
        user: address,
        custodian_id: u64
    ): &MarketEventHandlesForMarketAccount
    acquires MarketEventHandles {
        let market_event_handles_map_ref =
            &borrow_global<MarketEventHandles>(user).map;
        let market_account_id = get_market_account_id(market_id, custodian_id);
        table::borrow(market_event_handles_map_ref, market_account_id)
    }

    #[test_only]
    /// Return a `ChangeOrderSizeEvent` with the indicated fields.
    public fun create_change_order_size_event_test(
        market_id: u64,
        order_id: u128,
        user: address,
        custodian_id: u64,
        side: bool,
        new_size: u64
    ): ChangeOrderSizeEvent {
        ChangeOrderSizeEvent{
            market_id,
            order_id,
            user,
            custodian_id,
            side,
            new_size
        }
    }

    #[test_only]
    /// Return a `PlaceLimitOrderEvent` with the indicated fields.
    public fun create_place_limit_order_event_test(
        market_id: u64,
        user: address,
        custodian_id: u64,
        integrator: address,
        side: bool,
        size: u64,
        price: u64,
        restriction: u8,
        self_match_behavior: u8,
        remaining_size: u64,
        order_id: u128
    ): PlaceLimitOrderEvent {
        PlaceLimitOrderEvent{
            market_id,
            user,
            custodian_id,
            integrator,
            side,
            size,
            price,
            restriction,
            self_match_behavior,
            remaining_size,
            order_id
        }
    }

    #[test_only]
    /// Return a `PlaceMarketOrderEvent` with the indicated fields.
    public fun create_place_market_order_event_test(
        market_id: u64,
        user: address,
        custodian_id: u64,
        integrator: address,
        direction: bool,
        size: u64,
        self_match_behavior: u8,
        order_id: u128
    ): PlaceMarketOrderEvent {
        PlaceMarketOrderEvent{
            market_id,
            user,
            custodian_id,
            integrator,
            direction,
            size,
            self_match_behavior,
            order_id
        }
    }

    #[test_only]
    /// Return `HI_PRICE`, for testing synchronization with
    /// `market.move`.
    public fun get_HI_PRICE_test(): u64 {HI_PRICE}

    #[test_only]
    /// Return `NO_UNDERWRITER`, for testing synchronization with
    /// `market.move`.
    public fun get_NO_UNDERWRITER_test(): u64 {NO_UNDERWRITER}

    #[test_only]
    /// Get `CancelOrderEvent`s at a market account handle.
    public fun get_cancel_order_events_test(
        market_id: u64,
        user: address,
        custodian_id: u64
    ): vector<CancelOrderEvent>
    acquires MarketEventHandles {
        event::emitted_events_by_handle(
            &(borrow_market_event_handles_for_market_account_test(
                market_id, user, custodian_id).
                cancel_order_events))
    }

    #[test_only]
    /// Get `ChangeOrderSizeEvent`s at a market account handle.
    public fun get_change_order_size_events_test(
        market_id: u64,
        user: address,
        custodian_id: u64
    ): vector<ChangeOrderSizeEvent>
    acquires MarketEventHandles {
        event::emitted_events_by_handle(
            &(borrow_market_event_handles_for_market_account_test(
                market_id, user, custodian_id).
                change_order_size_events))
    }

    #[test_only]
    /// Like `get_collateral_value_test()`, but accepts market id and
    /// custodian ID.
    public fun get_collateral_value_simple_test<
        CoinType
    >(
        user_address: address,
        market_id: u64,
        custodian_id: u64
    ): u64
    acquires Collateral {
        get_collateral_value_test<CoinType>(
            user_address, get_market_account_id(market_id, custodian_id))
    }

    #[test_only]
    /// Return `Coin.value` of entry in `Collateral` for given
    /// `user_address`, `AssetType` and `market_account_id`.
    public fun get_collateral_value_test<
        CoinType
    >(
        user_address: address,
        market_account_id: u128,
    ): u64
    acquires Collateral {
        let collateral_map_ref = // Immutably borrow collateral map.
            &borrow_global<Collateral<CoinType>>(user_address).map;
        let coin_ref = // Immutably borrow coin collateral.
            tablist::borrow(collateral_map_ref, market_account_id);
        coin::value(coin_ref) // Return coin value.
    }

    #[test_only]
    /// Get `FillEvent`s at a market account handle.
    public fun get_fill_events_test(
        market_id: u64,
        user: address,
        custodian_id: u64
    ): vector<FillEvent>
    acquires MarketEventHandles {
        event::emitted_events_by_handle(
            &(borrow_market_event_handles_for_market_account_test(
                market_id, user, custodian_id).
                fill_events))
    }

    #[test_only]
    /// Get order access key at top of inactive order stack.
    public fun get_inactive_stack_top_test(
        user_address: address,
        market_account_id: u128,
        side: bool,
    ): u64
    acquires MarketAccounts {
        // Immutably borrow market accounts map.
        let market_accounts_map_ref =
            &borrow_global<MarketAccounts>(user_address).map;
        let market_account_ref = // Immutably borrow market account.
            table::borrow(market_accounts_map_ref, market_account_id);
        // Return corresponding stack top field.
        if (side == ASK) market_account_ref.asks_stack_top else
            market_account_ref.bids_stack_top
    }

    #[test_only]
    /// Return next inactive order in inactive orders stack.
    public fun get_next_inactive_order_test(
        user_address: address,
        market_account_id: u128,
        side: bool,
        order_access_key: u64
    ): u64
    acquires MarketAccounts {
        assert!(!is_order_active_test( // Assert order is inactive.
            user_address, market_account_id, side, order_access_key), 0);
        // Get order's size field, indicating next inactive order.
        let (_, next) = get_order_fields_test(
            user_address, market_account_id, side, order_access_key);
        next // Return next inactive order access key.
    }

    #[test_only]
    /// Wrapper for `get_order_fields_test()`, accepting market ID and
    /// custodian ID.
    public fun get_order_fields_simple_test(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        side: bool,
        order_access_key: u64
    ): (
        u128,
        u64
    ) acquires MarketAccounts {
        get_order_fields_test(
            user_address, get_market_account_id(market_id, custodian_id),
            side, order_access_key)
    }

    #[test_only]
    /// Return order fields for given order parameters.
    public fun get_order_fields_test(
        user_address: address,
        market_account_id: u128,
        side: bool,
        order_access_key: u64
    ): (
        u128,
        u64
    ) acquires MarketAccounts {
        // Immutably borrow market accounts map.
        let market_accounts_map_ref =
            &borrow_global<MarketAccounts>(user_address).map;
        let market_account_ref = // Immutably borrow market account.
            table::borrow(market_accounts_map_ref, market_account_id);
        // Immutably borrow corresponding orders tablist based on side.
        let (orders_ref) = if (side == ASK)
            &market_account_ref.asks else &market_account_ref.bids;
        // Immutably borrow order.
        let order_ref = tablist::borrow(orders_ref, order_access_key);
        // Return order fields.
        (order_ref.market_order_id, order_ref.size)
    }

    #[test_only]
    /// Get `PlaceLimitOrderEvent`s at a market account handle.
    public fun get_place_limit_order_events_test(
        market_id: u64,
        user: address,
        custodian_id: u64
    ): vector<PlaceLimitOrderEvent>
    acquires MarketEventHandles {
        event::emitted_events_by_handle(
            &(borrow_market_event_handles_for_market_account_test(
                market_id, user, custodian_id).
                place_limit_order_events))
    }

    #[test_only]
    /// Get `PlaceMarketOrderEvent`s at a market account handle.
    public fun get_place_market_order_events_test(
        market_id: u64,
        user: address,
        custodian_id: u64
    ): vector<PlaceMarketOrderEvent>
    acquires MarketEventHandles {
        event::emitted_events_by_handle(
            &(borrow_market_event_handles_for_market_account_test(
                market_id, user, custodian_id).
                place_market_order_events))
    }

    #[test_only]
    /// Return `true` if `user_adress` has an entry in `Collateral` for
    /// given `AssetType` and `market_account_id`.
    public fun has_collateral_test<
        AssetType
    >(
        user_address: address,
        market_account_id: u128,
    ): bool
    acquires Collateral {
        // Return false if does not even have collateral map.
        if (!exists<Collateral<AssetType>>(user_address)) return false;
        // Immutably borrow collateral map.
        let collateral_map_ref =
            &borrow_global<Collateral<AssetType>>(user_address).map;
        // Return if table contains entry for market account ID.
        tablist::contains(collateral_map_ref, market_account_id)
    }

    #[test_only]
    /// Check if user has allocated order for given parameters.
    public fun has_order_test(
        user_address: address,
        market_account_id: u128,
        side: bool,
        order_access_key: u64
    ): bool
    acquires MarketAccounts {
        // Immutably borrow market accounts map.
        let market_accounts_map_ref =
            &borrow_global<MarketAccounts>(user_address).map;
        let market_account_ref = // Immutably borrow market account.
            table::borrow(market_accounts_map_ref, market_account_id);
        // Immutably borrow corresponding orders tablist based on side.
        let (orders_ref) = if (side == ASK)
            &market_account_ref.asks else &market_account_ref.bids;
        tablist::contains(orders_ref, order_access_key)
    }

    #[test_only]
    /// Register market accounts under test `@user`, return signer and
    /// market account ID of:
    ///
    /// * Pure coin self-custodied market account.
    /// * Pure coin market account with delegated custodian.
    /// * Generic self-custodian market account.
    /// * Generic market account with delegated custodian.
    fun register_market_accounts_test(): (
        signer,
        u128,
        u128,
        u128,
        u128
    ) acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Get signer for test user account.
        let user = account::create_signer_with_capability(
            &account::create_test_signer_cap(@user));
        // Create Aptos account.
        account::create_account_for_test(@user);
        // Register a pure coin and a generic market, storing most
        // returns.
        let (market_id_pure_coin, _, lot_size_pure_coin, tick_size_pure_coin,
             min_size_pure_coin, underwriter_id_pure_coin, market_id_generic,
             _, lot_size_generic, tick_size_generic, min_size_generic,
             underwriter_id_generic) = registry::register_markets_test();
        // Assert market info.
        assert!(market_id_pure_coin      == MARKET_ID_PURE_COIN, 0);
        assert!(lot_size_pure_coin       == LOT_SIZE_PURE_COIN, 0);
        assert!(tick_size_pure_coin      == TICK_SIZE_PURE_COIN, 0);
        assert!(min_size_pure_coin       == MIN_SIZE_PURE_COIN, 0);
        assert!(underwriter_id_pure_coin == NO_UNDERWRITER, 0);
        assert!(market_id_generic        == MARKET_ID_GENERIC, 0);
        assert!(lot_size_generic         == LOT_SIZE_GENERIC, 0);
        assert!(tick_size_generic        == TICK_SIZE_GENERIC, 0);
        assert!(min_size_generic         == MIN_SIZE_GENERIC, 0);
        assert!(underwriter_id_generic   == UNDERWRITER_ID, 0);
        // Register self-custodied pure coin account.
        register_market_account<BC, QC>(
            &user, market_id_pure_coin, NO_CUSTODIAN);
        // Set delegated custodian ID as registered.
        registry::set_registered_custodian_test(CUSTODIAN_ID);
        // Register delegated custody pure coin account.
        register_market_account<BC, QC>(
            &user, market_id_pure_coin, CUSTODIAN_ID);
        // Register self-custodied generic asset account.
        register_market_account_generic_base<QC>(
            &user, market_id_generic, NO_CUSTODIAN);
        // Register delegated custody generic asset account.
        register_market_account_generic_base<QC>(
            &user, market_id_generic, CUSTODIAN_ID);
        // Get market account IDs.
        let market_account_id_coin_self =
            get_market_account_id(market_id_pure_coin, NO_CUSTODIAN);
        let market_account_id_coin_delegated =
            get_market_account_id(market_id_pure_coin, CUSTODIAN_ID);
        let market_account_id_generic_self =
            get_market_account_id(market_id_generic  , NO_CUSTODIAN);
        let market_account_id_generic_delegated =
            get_market_account_id(market_id_generic  , CUSTODIAN_ID);
        (user, // Return signing user and market account IDs.
         market_account_id_coin_self,
         market_account_id_coin_delegated,
         market_account_id_generic_self,
         market_account_id_generic_delegated)
    }

    #[test_only]
    public fun remove_market_event_handles_for_market_account_test(
        user: address,
        market_id: u64,
        custodian_id: u64
    ) acquires MarketEventHandles {
        let market_account_id = get_market_account_id(market_id, custodian_id);
        let market_event_handles_map_ref_mut =
            &mut borrow_global_mut<MarketEventHandles>(user).map;
        let MarketEventHandlesForMarketAccount{
            cancel_order_events,
            change_order_size_events,
            fill_events,
            place_limit_order_events,
            place_market_order_events
        } = table::remove(market_event_handles_map_ref_mut, market_account_id);
        event::destroy_handle(cancel_order_events);
        event::destroy_handle(change_order_size_events);
        event::destroy_handle(fill_events);
        event::destroy_handle(place_limit_order_events);
        event::destroy_handle(place_market_order_events);
    }

    #[test_only]
    public fun remove_market_event_handles_test(
        user: address
    ) acquires MarketEventHandles {
        let MarketEventHandles{map} = move_from(user);
        table::drop_unchecked(map);
    }

    #[test_only]
    /// Return `true` if order is active.
    public fun is_order_active_test(
        user_address: address,
        market_account_id: u128,
        side: bool,
        order_access_key: u64
    ): bool
    acquires MarketAccounts {
        // Get order's market order ID field.
        let (market_order_id, _) = get_order_fields_test(
            user_address, market_account_id, side, order_access_key);
        market_order_id != (NIL as u128) // Return true if non-null ID.
    }

    #[test_only]
    /// Set market order ID for given order.
    public fun set_market_order_id_test(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        side: bool,
        order_access_key: u64,
        market_order_id: u128
    ) acquires MarketAccounts {
        // Mutably borrow market accounts map.
        let market_accounts_map_ref_mut =
            &mut borrow_global_mut<MarketAccounts>(user_address).map;
        // Get market account ID.
        let market_account_id = get_market_account_id(market_id, custodian_id);
        let market_account_ref_mut = // Mutably borrow market account.
            table::borrow_mut(market_accounts_map_ref_mut, market_account_id);
        // Mutably borrow corresponding orders tablist based on side.
        let (orders_ref_mut) = if (side == ASK)
            &mut market_account_ref_mut.asks else
            &mut market_account_ref_mut.bids;
        let order_ref_mut = // Mutably borrow order.
            tablist::borrow_mut(orders_ref_mut, order_access_key);
        // Set new market order ID.
        order_ref_mut.market_order_id = market_order_id;
    }

    // Test-only functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Tests >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ORDER_ID)]
    /// Verify failure for market account ID mismatch.
    fun test_cancel_order_internal_invalid_market_order_id()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        register_market_accounts_test(); // Register test markets.
        // Define order parameters.
        let market_order_id = 123;
        let size            = MIN_SIZE_PURE_COIN;
        let price           = 1;
        let side            = BID;
        // Deposit starting base and quote coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        // Place order
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
        // Attempt invalid cancellation.
        cancel_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                              size, price, 1, market_order_id + 1,
                              CANCEL_REASON_MANUAL_CANCEL);
    }

    #[test]
    #[expected_failure(abort_code = E_START_SIZE_MISMATCH)]
    /// Verify failure for start size mismatch.
    fun test_cancel_order_internal_start_size_mismatch()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        register_market_accounts_test(); // Register test markets.
        // Define order parameters.
        let market_order_id = 123;
        let size            = MIN_SIZE_PURE_COIN;
        let price           = 1;
        let side            = BID;
        // Deposit starting base and quote coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        // Place order
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
        // Attempt invalid cancellation.
        cancel_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                              size + 1, price, 1, market_order_id,
                              CANCEL_REASON_MANUAL_CANCEL);
    }

    #[test]
    /// Verify state updates for changing ask size. Based on
    /// `test_place_cancel_order_ask()`.
    fun test_change_order_size_internal_ask()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        register_market_accounts_test(); // Register test markets.
        // Define order parameters.
        let market_order_id  = 1234;
        let size             = 789;
        let size_old         = size - 1;
        let price            = 321;
        let side             = ASK;
        let order_access_key = 1;
        // Calculate change in base asset and quote asset fields.
        let base_delta = size * LOT_SIZE_PURE_COIN;
        let quote_delta = size * price * TICK_SIZE_PURE_COIN;
        // Deposit starting base and quote coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        // Place order.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size_old, price, market_order_id, 1);
        // Remove market event handles.
        remove_market_event_handles_for_market_account_test(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        change_order_size_internal( // Change order size.
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side, size_old, size,
            price, order_access_key, market_order_id);
        // Assert asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        assert!(base_total      == BASE_START , 0);
        assert!(base_available  == BASE_START - base_delta, 0);
        assert!(base_ceiling    == BASE_START , 0);
        assert!(quote_total     == QUOTE_START, 0);
        assert!(quote_available == QUOTE_START, 0);
        assert!(quote_ceiling   == QUOTE_START + quote_delta, 0);
        // Check market order ID for valid access key.
        let optional_market_order_id = get_open_order_id_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side, order_access_key);
        assert!( // Verify market order ID match.
            *option::borrow(&optional_market_order_id) == market_order_id, 0);
        // Check market order ID for invalid access key.
        optional_market_order_id = get_open_order_id_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
            order_access_key + 1);
        // Verify empty option.
        assert!(option::is_none(&optional_market_order_id), 0);
    }

    #[test]
    /// Verify state updates for changing bid size. Based on
    /// `test_place_cancel_order_bid()`.
    fun test_change_order_size_internal_bid()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        register_market_accounts_test(); // Register test markets.
        // Define order parameters.
        let market_order_id  = 1234;
        let size             = 789;
        let size_old         = size - 1;
        let price            = 321;
        let side             = BID;
        let order_access_key = 1;
        // Calculate change in base asset and quote asset fields.
        let base_delta = size * LOT_SIZE_PURE_COIN;
        let quote_delta = size * price * TICK_SIZE_PURE_COIN;
        // Deposit starting base and quote coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        // Place order.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size_old, price, market_order_id, 1);
        change_order_size_internal( // Change order size.
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side, size_old, size,
            price, order_access_key, market_order_id);
        // Assert asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        assert!(base_total      == BASE_START , 0);
        assert!(base_available  == BASE_START , 0);
        assert!(base_ceiling    == BASE_START + base_delta, 0);
        assert!(quote_total     == QUOTE_START, 0);
        assert!(quote_available == QUOTE_START - quote_delta, 0);
        assert!(quote_ceiling   == QUOTE_START, 0);
        // Check market order ID for valid access key.
        let optional_market_order_id = get_open_order_id_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side, order_access_key);
        assert!( // Verify market order ID match.
            *option::borrow(&optional_market_order_id) == market_order_id, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_CHANGE_ORDER_NO_CHANGE)]
    /// Verify failure for no change in size.
    fun test_change_order_size_internal_no_change()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        register_market_accounts_test(); // Register test markets.
        // Define order parameters.
        let market_order_id = 123;
        let size            = MIN_SIZE_PURE_COIN;
        let price           = 1;
        let side            = BID;
        // Deposit starting base and quote coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        // Place order
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
        change_order_size_internal( // Attempt invalid order size change.
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side, size, size, price,
            1, market_order_id);
    }

    #[test]
    #[expected_failure(abort_code = E_COIN_AMOUNT_MISMATCH)]
    /// Verify failure for amount mismatch.
    fun test_deposit_asset_amount_mismatch()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        register_market_accounts_test();
        // Declore deposit invocation arguments.
        let user_address = @user;
        let market_id = MARKET_ID_PURE_COIN;
        let custodian_id = NO_CUSTODIAN;
        let amount = 123;
        let optional_coins = option::some(assets::mint_test<BC>(amount + 1));
        let underwriter_id = NO_UNDERWRITER;
        // Attempt invalid invocation.
        deposit_asset(user_address, market_id, custodian_id, amount,
                      optional_coins, underwriter_id);
    }

    #[test]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNT)]
    /// Verify failure for no market account.
    fun test_deposit_asset_no_account()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        register_market_accounts_test();
        // Attempt invalid invocation.
        deposit_coins<BC>(@user, 0, 0, coin::zero());
    }

    #[test]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNTS)]
    /// Verify failure for no market accounts.
    fun test_deposit_asset_no_accounts()
    acquires
        Collateral,
        MarketAccounts
    {
        // Attempt invalid invocation.
        deposit_coins<BC>(@user, 0, 0, coin::zero());
    }

    #[test]
    #[expected_failure(abort_code = E_ASSET_NOT_IN_PAIR)]
    /// Verify failure for asset not in pair.
    fun test_deposit_asset_not_in_pair()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        register_market_accounts_test();
        // Attempt invalid invocation.
        deposit_coins<UC>(@user, MARKET_ID_PURE_COIN, NO_CUSTODIAN,
                          coin::zero());
    }

    #[test]
    #[expected_failure(abort_code = E_DEPOSIT_OVERFLOW_ASSET_CEILING)]
    /// Verify failure for ceiling overflow.
    fun test_deposit_asset_overflow()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        register_market_accounts_test();
        let underwriter_capability = // Get underwriter capability.
            registry::get_underwriter_capability_test(UNDERWRITER_ID);
        // Deposit maximum amount of generic asset.
        deposit_generic_asset(@user, MARKET_ID_GENERIC, NO_CUSTODIAN,
                              HI_64, &underwriter_capability);
        // Attempt invalid deposit of one more unit.
        deposit_generic_asset(@user, MARKET_ID_GENERIC, NO_CUSTODIAN,
                              1, &underwriter_capability);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_UNDERWRITER)]
    /// Verify failure for invalid underwriter.
    fun test_deposit_asset_underwriter()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        register_market_accounts_test();
        let underwriter_capability = // Get underwriter capability.
            registry::get_underwriter_capability_test(UNDERWRITER_ID + 1);
        // Attempt deposit with invalid underwriter capability.
        deposit_generic_asset(@user, MARKET_ID_GENERIC, NO_CUSTODIAN,
                              1, &underwriter_capability);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
    }

    #[test(econia = @econia)]
    #[expected_failure(abort_code = E_COIN_TYPE_IS_GENERIC_ASSET)]
    /// Assert failure for coin type is generic asset.
    fun test_deposit_coins_generic(
        econia: &signer
    ) acquires
        Collateral,
        MarketAccounts
    {
        // Initialize coin, storing capabilities.
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<GenericAsset>(
            econia, string::utf8(b""), string::utf8(b""), 1, true);
        // Mint a generic coin.
        let generic_coin = coin::mint<GenericAsset>(1, &mint_cap);
        // Attempt invalid deposit.
        deposit_coins<GenericAsset>(@econia, 0, 0, generic_coin);
        // Destroy capabilities.
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_freeze_cap(freeze_cap);
        coin::destroy_mint_cap(mint_cap);
    }

    #[test]
    /// Verify state updates, returns for pure coin and generic markets.
    fun test_deposit_withdraw_assets_internal()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Get test market account IDs.
        let (_, _, market_account_id_coin_delegated,
                   market_account_id_generic_self, _) =
             register_market_accounts_test();
        // Declare withdrawal amounts for each market accounts.
        let base_amount_0  = 123;
        let quote_amount_0 = 234;
        let base_amount_1  = 345;
        let quote_amount_1 = 456;
        // Deposit starting base and quote asset to each account.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        deposit_asset<GenericAsset>(
            @user, MARKET_ID_GENERIC, NO_CUSTODIAN, BASE_START, option::none(),
            UNDERWRITER_ID);
        deposit_coins<QC>(@user, MARKET_ID_GENERIC, NO_CUSTODIAN,
                          assets::mint_test(QUOTE_START));
        // Withdraw assets from pure coin market account.
        let (optional_base_coins_0, quote_coins_0) = withdraw_assets_internal<
            BC, QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, base_amount_0,
            quote_amount_0, NO_UNDERWRITER);
        // Assert coin values.
        assert!(coin::value(option::borrow(&optional_base_coins_0))
                == base_amount_0, 0);
        assert!(coin::value(&quote_coins_0) == quote_amount_0, 0);
        // Assert asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        assert!(base_total      == BASE_START  - base_amount_0, 0);
        assert!(base_available  == BASE_START  - base_amount_0, 0);
        assert!(base_ceiling    == BASE_START  - base_amount_0, 0);
        assert!(quote_total     == QUOTE_START - quote_amount_0, 0);
        assert!(quote_available == QUOTE_START - quote_amount_0, 0);
        assert!(quote_ceiling   == QUOTE_START - quote_amount_0, 0);
        // Assert collateral amounts.
        assert!(get_collateral_value_test<BC>(
            @user, market_account_id_coin_delegated)
                == BASE_START - base_amount_0, 0);
        assert!(get_collateral_value_test<QC>(
            @user, market_account_id_coin_delegated)
                 == QUOTE_START - quote_amount_0, 0);
        // Deposit assets back to pure coin market account.
        deposit_assets_internal<BC, QC>(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, base_amount_0,
            optional_base_coins_0, quote_coins_0, NO_UNDERWRITER);
        // Assert asset counts.
        (base_total , base_available , base_ceiling,
         quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        assert!(base_total      == BASE_START , 0);
        assert!(base_available  == BASE_START , 0);
        assert!(base_ceiling    == BASE_START , 0);
        assert!(quote_total     == QUOTE_START, 0);
        assert!(quote_available == QUOTE_START, 0);
        assert!(quote_ceiling   == QUOTE_START, 0);
        // Assert collateral amounts.
        assert!(get_collateral_value_test<BC>(
            @user, market_account_id_coin_delegated) == BASE_START, 0);
        assert!(get_collateral_value_test<QC>(
            @user, market_account_id_coin_delegated) == QUOTE_START, 0);
        // Withdraw assets from generic market account.
        let (optional_base_coins_1, quote_coins_1) = withdraw_assets_internal<
            GenericAsset, QC>(@user, MARKET_ID_GENERIC, NO_CUSTODIAN,
            base_amount_1, quote_amount_1, UNDERWRITER_ID);
        // Assert no base asset.
        assert!(option::is_none(&optional_base_coins_1), 0);
        // Assert quote coin amount.
        assert!(coin::value(&quote_coins_1) == quote_amount_1, 0);
        // Assert asset counts.
        (base_total , base_available , base_ceiling,
         quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_GENERIC, NO_CUSTODIAN);
        assert!(base_total      == BASE_START  - base_amount_1, 0);
        assert!(base_available  == BASE_START  - base_amount_1, 0);
        assert!(base_ceiling    == BASE_START  - base_amount_1, 0);
        assert!(quote_total     == QUOTE_START - quote_amount_1, 0);
        assert!(quote_available == QUOTE_START - quote_amount_1, 0);
        assert!(quote_ceiling   == QUOTE_START - quote_amount_1, 0);
        // Assert collateral state.
        assert!(!has_collateral_test<GenericAsset>(
            @user, market_account_id_generic_self), 0);
        assert!(get_collateral_value_test<QC>(
            @user, market_account_id_generic_self)
                 == QUOTE_START - quote_amount_1, 0);
        // Deposit assets back to generic market account.
        deposit_assets_internal<GenericAsset, QC>(
            @user, MARKET_ID_GENERIC, NO_CUSTODIAN, base_amount_1,
            optional_base_coins_1, quote_coins_1, UNDERWRITER_ID);
        // Assert asset counts.
        (base_total , base_available , base_ceiling,
         quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_GENERIC, NO_CUSTODIAN);
        assert!(base_total      == BASE_START , 0);
        assert!(base_available  == BASE_START , 0);
        assert!(base_ceiling    == BASE_START , 0);
        assert!(quote_total     == QUOTE_START, 0);
        assert!(quote_available == QUOTE_START, 0);
        assert!(quote_ceiling   == QUOTE_START, 0);
        // Assert collateral state.
        assert!(!has_collateral_test<GenericAsset>(
            @user, market_account_id_generic_self), 0);
        assert!(get_collateral_value_test<QC>(
            @user, market_account_id_generic_self)
                 == QUOTE_START, 0);
    }

    #[test]
    /// Verify state updates for assorted deposit styles.
    fun test_deposits():
    vector<MarketAccountView>
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Declare deposit parameters
        let coin_amount = 700;
        let generic_amount = 500;
        // Get signing user and test market account IDs.
        let (user, _, market_account_id_coin_delegated,
                      market_account_id_generic_self, _) =
             register_market_accounts_test();
        coin::register<QC>(&user); // Register coin store.
        // Deposit coin asset to user's coin store.
        coin::deposit(@user, assets::mint_test<QC>(coin_amount));
        // Deposit to user's delegated pure coin market account.
        deposit_from_coinstore<QC>(&user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                                   coin_amount);
        let underwriter_capability = // Get underwriter capability.
            registry::get_underwriter_capability_test(UNDERWRITER_ID);
        // Deposit to user's generic market account.
        deposit_generic_asset(@user, MARKET_ID_GENERIC, NO_CUSTODIAN,
                              generic_amount, &underwriter_capability);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
        let custodian_capability = // Get custodian capability.
            registry::get_custodian_capability_test(CUSTODIAN_ID);
        // Assert state for quote deposit.
        let ( base_total,  base_available,  base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            get_asset_counts_custodian(
                @user, MARKET_ID_PURE_COIN, &custodian_capability);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
        assert!(base_total      == 0             , 0);
        assert!(base_available  == 0             , 0);
        assert!(base_ceiling    == 0             , 0);
        assert!(quote_total     == coin_amount   , 0);
        assert!(quote_available == coin_amount   , 0);
        assert!(quote_ceiling   == coin_amount   , 0);
        assert!(get_collateral_value_test<BC>(
            @user, market_account_id_coin_delegated) == 0, 0);
        assert!(get_collateral_value_test<QC>(
            @user, market_account_id_coin_delegated) == coin_amount, 0);
        // Assert state for base deposit.
        let ( base_total,  base_available,  base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            get_asset_counts_user(&user, MARKET_ID_GENERIC);
        assert!(base_total      == generic_amount, 0);
        assert!(base_available  == generic_amount, 0);
        assert!(base_ceiling    == generic_amount, 0);
        assert!(quote_total     == 0             , 0);
        assert!(quote_available == 0             , 0);
        assert!(quote_ceiling   == 0             , 0);
        assert!(!has_collateral_test<GenericAsset>(
            @user, market_account_id_generic_self), 0);
        assert!(get_collateral_value_test<QC>(
            @user, market_account_id_generic_self) == 0, 0);
        // Initialize empty vector to return instead of dropping.
        let return_instead_of_dropping = vector::empty();
        // Get market account view for quote deposit.
        let market_account_view = get_market_account(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        // Assert market account view state.
        assert!(market_account_view.market_id       == MARKET_ID_PURE_COIN, 0);
        assert!(market_account_view.custodian_id    == CUSTODIAN_ID       , 0);
        assert!(market_account_view.base_total      == 0                  , 0);
        assert!(market_account_view.base_available  == 0                  , 0);
        assert!(market_account_view.base_ceiling    == 0                  , 0);
        assert!(market_account_view.quote_total     == coin_amount        , 0);
        assert!(market_account_view.quote_available == coin_amount        , 0);
        assert!(market_account_view.quote_ceiling   == coin_amount        , 0);
        // Push back value to return instead of dropping.
        vector::push_back(
            &mut return_instead_of_dropping, market_account_view);
        // Get market account view for base deposit.
        market_account_view = get_market_account(
            @user, MARKET_ID_GENERIC, NO_CUSTODIAN);
        // Assert market account view state.
        assert!(market_account_view.market_id       == MARKET_ID_GENERIC  , 0);
        assert!(market_account_view.custodian_id    == NO_CUSTODIAN       , 0);
        assert!(market_account_view.base_total      == generic_amount     , 0);
        assert!(market_account_view.base_available  == generic_amount     , 0);
        assert!(market_account_view.base_ceiling    == generic_amount     , 0);
        assert!(market_account_view.quote_total     == 0                  , 0);
        assert!(market_account_view.quote_available == 0                  , 0);
        assert!(market_account_view.quote_ceiling   == 0                  , 0);
        // Push back value to return instead of dropping.
        vector::push_back(
            &mut return_instead_of_dropping, market_account_view);
        return_instead_of_dropping // Return instead of dropping.
    }

    #[test]
    /// Verify state updates for:
    ///
    /// * Filling an ask.
    /// * Fill is complete.
    /// * Inactive stack top not null.
    /// * Base asset is coin.
    fun test_fill_order_internal_ask_complete_base_coin()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test markets, get market account ID for pure coin
        // market with delegated custodian.
        let (_, _, market_account_id, _, _) = register_market_accounts_test();
        // Define order parameters.
        let market_order_id            = 1234;
        let size                       = 789;
        let price                      = 321;
        let side                       = ASK;
        let complete_fill              = true;
        let order_access_key_filled    = 1;
        let order_access_key_cancelled = 2;
        // Calculate base asset and quote asset fill amounts.
        let base_fill = size * LOT_SIZE_PURE_COIN;
        let quote_fill = size * price * TICK_SIZE_PURE_COIN;
        // Deposit starting base and quote coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        // Place order.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
        // Place duplicate order then cancel, so stack is not empty.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 2);
        cancel_order_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side, size, price,
            order_access_key_cancelled, market_order_id,
            CANCEL_REASON_MANUAL_CANCEL);
        // Initialize external coins passing through matching engine.
        let optional_base_coins = option::some(coin::zero());
        let quote_coins = assets::mint_test(quote_fill);
        // Fill order, storing base and quote coins for matching engine,
        // and market order ID.
        (optional_base_coins, quote_coins, market_order_id) =
            fill_order_internal<BC, QC>(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                order_access_key_filled, size, size, complete_fill,
                optional_base_coins, quote_coins, base_fill, quote_fill);
        // Assert market order ID.
        assert!(market_order_id == market_order_id, 0);
        // Assert external coin values after order fill.
        let base_coins = option::destroy_some(optional_base_coins);
        assert!(coin::value(&base_coins) == base_fill, 0);
        assert!(coin::value(&quote_coins) == 0, 0);
        // Destroy external coins.
        assets::burn(base_coins);
        coin::destroy_zero(quote_coins);
        // Assert inactive stack top.
        assert!(get_inactive_stack_top_test(
            @user, market_account_id, side) == order_access_key_filled, 0);
        assert!(!is_order_active_test( // Assert order marked inactive.
            @user, market_account_id, side, order_access_key_filled), 0);
        // Assert next inactive node field.
        assert!(get_next_inactive_order_test(
            @user, market_account_id, side, order_access_key_filled)
            == order_access_key_cancelled, 0);
        // Assert asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        assert!(base_total      == BASE_START - base_fill, 0);
        assert!(base_available  == BASE_START - base_fill, 0);
        assert!(base_ceiling    == BASE_START - base_fill, 0);
        assert!(quote_total     == QUOTE_START + quote_fill, 0);
        assert!(quote_available == QUOTE_START + quote_fill, 0);
        assert!(quote_ceiling   == QUOTE_START + quote_fill, 0);
        // Assert collateral amounts.
        assert!(get_collateral_value_test<BC>(
            @user, market_account_id) == BASE_START - base_fill, 0);
        assert!(get_collateral_value_test<QC>(
            @user, market_account_id) == QUOTE_START + quote_fill, 0);
    }

    #[test]
    /// Verify state updates for:
    ///
    /// * Filling a bid.
    /// * Fill is complete.
    /// * Inactive stack top is null.
    /// * Base asset is coin.
    fun test_fill_order_internal_bid_complete_base_coin()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test markets, get market account ID for pure coin
        // market with delegated custodian.
        let (_, _, market_account_id, _, _) = register_market_accounts_test();
        // Define order parameters.
        let market_order_id            = 1234;
        let size                       = 789;
        let price                      = 321;
        let side                       = BID;
        let complete_fill              = true;
        let order_access_key_filled    = 1;
        // Calculate base asset and quote asset fill amounts.
        let base_fill = size * LOT_SIZE_PURE_COIN;
        let quote_fill = size * price * TICK_SIZE_PURE_COIN;
        // Deposit starting base and quote coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        // Place order.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
        // Initialize external coins passing through matching engine.
        let optional_base_coins = option::some(assets::mint_test(base_fill));
        let quote_coins = coin::zero();
        // Fill order, storing base and quote coins for matching engine,
        // and market order ID.
        (optional_base_coins, quote_coins, market_order_id) =
            fill_order_internal<BC, QC>(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                order_access_key_filled, size, size, complete_fill,
                optional_base_coins, quote_coins, base_fill, quote_fill);
        // Assert market order ID.
        assert!(market_order_id == market_order_id, 0);
        // Assert external coin values after order fill.
        let base_coins = option::destroy_some(optional_base_coins);
        assert!(coin::value(&base_coins) == 0, 0);
        assert!(coin::value(&quote_coins) == quote_fill, 0);
        // Destroy external coins.
        coin::destroy_zero(base_coins);
        assets::burn(quote_coins);
        // Assert inactive stack top.
        assert!(get_inactive_stack_top_test(
            @user, market_account_id, side) == order_access_key_filled, 0);
        assert!(!is_order_active_test( // Assert order marked inactive.
            @user, market_account_id, side, order_access_key_filled), 0);
        // Assert next inactive node field.
        assert!(get_next_inactive_order_test(
            @user, market_account_id, side, order_access_key_filled)
            == NIL, 0);
        // Assert asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        assert!(base_total      == BASE_START + base_fill, 0);
        assert!(base_available  == BASE_START + base_fill, 0);
        assert!(base_ceiling    == BASE_START + base_fill, 0);
        assert!(quote_total     == QUOTE_START - quote_fill, 0);
        assert!(quote_available == QUOTE_START - quote_fill, 0);
        assert!(quote_ceiling   == QUOTE_START - quote_fill, 0);
        // Assert collateral amounts.
        assert!(get_collateral_value_test<BC>(
            @user, market_account_id) == BASE_START + base_fill, 0);
        assert!(get_collateral_value_test<QC>(
            @user, market_account_id) == QUOTE_START - quote_fill, 0);
    }

    #[test]
    /// Verify state updates for:
    ///
    /// * Filling a bid.
    /// * Fill is not complete.
    /// * Base asset is generic.
    fun test_fill_order_internal_bid_partial_base_generic()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test markets, get market account ID for generic
        // market with delegated custodian.
        let (_, _, _, _, market_account_id) = register_market_accounts_test();
        // Define order parameters.
        let market_order_id            = 1234;
        let size                       = 789;
        let fill_size                  = size - 1;
        let price                      = 321;
        let side                       = BID;
        let complete_fill              = false;
        let order_access_key_filled    = 1;
        // Calculate change in base ceiling and quote available if
        // order were completely filled.
        let base_ceiling_delta = size * LOT_SIZE_GENERIC;
        let quote_available_delta = size * price * TICK_SIZE_GENERIC;
        // Calculate base asset and quote asset fill amounts.
        let base_fill = fill_size * LOT_SIZE_GENERIC;
        let quote_fill = fill_size * price * TICK_SIZE_GENERIC;
        // Deposit starting quote coins.
        deposit_coins<QC>(@user, MARKET_ID_GENERIC, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        // Place order.
        place_order_internal(@user, MARKET_ID_GENERIC, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
        // Initialize external coins passing through matching engine.
        let optional_base_coins = option::none();
        let quote_coins = coin::zero();
        // Fill order, storing base and quote coins for matching engine,
        // and market order ID.
        (optional_base_coins, quote_coins, market_order_id) =
            fill_order_internal<GenericAsset, QC>(
                @user, MARKET_ID_GENERIC, CUSTODIAN_ID, side,
                order_access_key_filled, size, fill_size, complete_fill,
                optional_base_coins, quote_coins, base_fill, quote_fill);
        // Assert market order ID.
        assert!(market_order_id == market_order_id, 0);
        // Assert quote coin values after order fill.
        assert!(coin::value(&quote_coins) == quote_fill, 0);
        // Destroy external coins.
        option::destroy_none(optional_base_coins);
        assets::burn(quote_coins);
        // Assert inactive stack top.
        assert!(get_inactive_stack_top_test(
            @user, market_account_id, side) == NIL, 0);
        assert!(is_order_active_test( // Assert order marked active.
            @user, market_account_id, side, order_access_key_filled), 0);
        // Assert order field returns.
        let (market_order_id_r, size_r) = get_order_fields_test(
            @user, market_account_id, side, order_access_key_filled);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r == size - fill_size, 0);
        // Assert asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_GENERIC, CUSTODIAN_ID);
        assert!(base_total      == base_fill, 0);
        assert!(base_available  == base_fill, 0);
        assert!(base_ceiling    == base_ceiling_delta, 0);
        assert!(quote_total     == QUOTE_START - quote_fill, 0);
        assert!(quote_available == QUOTE_START - quote_available_delta, 0);
        assert!(quote_ceiling   == QUOTE_START - quote_fill, 0);
        assert!(!has_collateral_test<GenericAsset>(
            @user, market_account_id), 0);
        assert!(get_collateral_value_test<QC>(
            @user, market_account_id) == QUOTE_START - quote_fill, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_START_SIZE_MISMATCH)]
    /// Verify failure for start size mismatch. Based on
    /// `test_fill_order_internal_ask_complete_base_coin()`.
    fun test_fill_order_internal_start_size_mismatch()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        register_market_accounts_test(); // Register test markets.
        // Define order parameters.
        let market_order_id = 1234;
        let size            = 789;
        let price           = 321;
        let side            = ASK;
        let complete_fill   = true;
        let access_key      = 1;
        // Calculate base asset and quote asset fill amounts.
        let base_fill = size * LOT_SIZE_PURE_COIN;
        let quote_fill = size * price * TICK_SIZE_PURE_COIN;
        // Deposit starting base and quote coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        place_order_internal( // Place order.
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side, size, price,
            market_order_id, access_key);
        // Initialize external coins passing through matching engine.
        let optional_base_coins = option::some(coin::zero());
        let quote_coins = assets::mint_test(quote_fill);
        // Fill order, storing base and quote coins for matching engine.
        (optional_base_coins, quote_coins, _) = fill_order_internal<BC, QC>(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side, access_key,
                size + 1, size, complete_fill, optional_base_coins,
                quote_coins, base_fill, quote_fill);
        // Destroy external coins.
        assets::burn(option::destroy_some(optional_base_coins));
        assets::burn(quote_coins);
    }

    #[test]
    /// Verify expected returns.
    fun test_get_active_market_order_ids_internal()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        register_market_accounts_test();
        // Define order parameters.
        let market_order_id_1 = 123;
        let market_order_id_2 = 234;
        let market_order_id_3 = 345;
        let market_order_id_4 = 456;
        let size              = MIN_SIZE_PURE_COIN;
        let price             = 1;
        // Deposit starting base and quote coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        // Assert empty returns.
        assert!(get_active_market_order_ids_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, ASK) == vector[], 0);
        assert!(get_active_market_order_ids_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, BID) == vector[], 0);
        // Place three asks, then cancel second ask.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, ASK,
                             size, price, market_order_id_1, 1);
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, ASK,
                             size, price, market_order_id_2, 2);
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, ASK,
                             size, price, market_order_id_3, 3);
        cancel_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, ASK,
                              size, price, 2, market_order_id_2,
                              CANCEL_REASON_MANUAL_CANCEL);
        // Get expected market order IDs vector.
        let expected = vector[market_order_id_1, market_order_id_3];
        // Assert expected return.
        assert!(get_active_market_order_ids_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, ASK) == expected, 0);
        // Place single bid.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, BID,
                             size, price, market_order_id_4, 1);
        // Get expected market order IDs vector.
        expected = vector[market_order_id_4];
        // Assert expected return.
        assert!(get_active_market_order_ids_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, BID) == expected, 0);
        // Cancel order.
        cancel_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, BID,
                              size, price, 1, market_order_id_4,
                              CANCEL_REASON_MANUAL_CANCEL);
        // Assert expected return.
        assert!(get_active_market_order_ids_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, BID) == vector[], 0);
    }

    #[test]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNT)]
    /// Verify failure for no market account resource.
    fun test_get_active_market_order_ids_internal_no_account()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        register_market_accounts_test();
        // Attempt invalid invocation.
        get_active_market_order_ids_internal(
            @user, MARKET_ID_PURE_COIN + 10, CUSTODIAN_ID, BID);
    }

    #[test]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNTS)]
    /// Verify failure for no market accounts resource.
    fun test_get_active_market_order_ids_internal_no_accounts()
    acquires MarketAccounts {
        // Attempt invalid invocation.
        get_active_market_order_ids_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, BID);
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_ASK() {assert!(get_ASK() == ASK, 0)}

    #[test]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNT)]
    /// Verify failure for no market account resource.
    fun test_get_asset_counts_internal_no_account()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        register_market_accounts_test();
        // Attempt invalid invocation.
        get_asset_counts_internal(@user, 0, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNTS)]
    /// Verify failure for no market accounts resource.
    fun test_get_asset_counts_internal_no_accounts()
    acquires MarketAccounts {
        // Attempt invalid invocation.
        get_asset_counts_internal(@user, 0, 0);
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_BID() {assert!(get_BID() == BID, 0)}

    #[test]
    /// Verify constant getter returns.
    fun test_get_cancel_reasons() {
        assert!(get_CANCEL_REASON_EVICTION() ==
                    CANCEL_REASON_EVICTION, 0);
        assert!(get_CANCEL_REASON_IMMEDIATE_OR_CANCEL() ==
                    CANCEL_REASON_IMMEDIATE_OR_CANCEL, 0);
        assert!(get_CANCEL_REASON_MANUAL_CANCEL() ==
                    CANCEL_REASON_MANUAL_CANCEL, 0);
        assert!(get_CANCEL_REASON_MAX_QUOTE_TRADED() ==
                    CANCEL_REASON_MAX_QUOTE_TRADED, 0);
        assert!(get_CANCEL_REASON_NOT_ENOUGH_LIQUIDITY() ==
                    CANCEL_REASON_NOT_ENOUGH_LIQUIDITY, 0);
        assert!(get_CANCEL_REASON_SELF_MATCH_MAKER() ==
                    CANCEL_REASON_SELF_MATCH_MAKER, 0);
        assert!(get_CANCEL_REASON_SELF_MATCH_TAKER() ==
                    CANCEL_REASON_SELF_MATCH_TAKER, 0);
        assert!(get_CANCEL_REASON_TOO_SMALL_TO_FILL_LOT() ==
                    CANCEL_REASON_TOO_SMALL_TO_FILL_LOT, 0);
        assert!(get_CANCEL_REASON_VIOLATED_LIMIT_PRICE() ==
                    CANCEL_REASON_VIOLATED_LIMIT_PRICE, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNT)]
    /// Verify failure for no market account resource.
    fun test_get_market_account_market_info_no_account()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        register_market_accounts_test();
        // Attempt invalid invocation.
        get_market_account_market_info(@user, 0, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNTS)]
    /// Verify failure for no market accounts resource.
    fun test_get_market_account_market_info_no_accounts()
    acquires MarketAccounts {
        // Attempt invalid invocation.
        get_market_account_market_info(@user, 0, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNT)]
    /// Verify failure for no market accounts resource.
    fun test_get_market_account_no_market_account():
    MarketAccountView
    acquires MarketAccounts {
        // Attempt invalid invocation.
        get_market_account(@user, 0, 0)
    }

    #[test]
    /// Verify returns for open order indexing.
    fun test_get_market_accounts_open_orders():
    vector<vector<MarketAccountView>>
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Initialize empty vector to return instead of dropping.
        let return_instead_of_dropping = vector::empty();
        // Define order parameters.
        let market_order_id_ask_0  = 123;
        let market_order_id_bid_0  = 456;
        let market_order_id_bid_1  = 789;
        let order_access_key_ask_0 = 1;
        let order_access_key_bid_0 = 1;
        let order_access_key_bid_1 = 2;
        let size                   = MIN_SIZE_PURE_COIN;
        let price                  = 1;
        let market_id              = MARKET_ID_PURE_COIN;
        let user                   = @user;
        // Get user's market account views.
        let market_account_views = get_market_accounts(user);
        // Assert empty vector.
        assert!(vector::is_empty(&market_account_views), 0);
        // Push back value to return instead of dropping.
        vector::push_back(
            &mut return_instead_of_dropping, market_account_views);
        // Register test market accounts.
        register_market_accounts_test();
        // Deposit starting base and quote coins.
        deposit_coins<BC>(
            user, market_id, NO_CUSTODIAN, assets::mint_test(BASE_START));
        deposit_coins<BC>(
            user, market_id, CUSTODIAN_ID, assets::mint_test(BASE_START));
        deposit_coins<QC>(
            user, market_id, NO_CUSTODIAN, assets::mint_test(QUOTE_START));
        deposit_coins<QC>(
            user, market_id, CUSTODIAN_ID, assets::mint_test(QUOTE_START));
        // Place single ask for market account without custodian.
        place_order_internal(
            user, market_id, NO_CUSTODIAN, ASK, size, price,
            market_order_id_ask_0, order_access_key_ask_0);
        // Get user's market account views.
        market_account_views = get_market_accounts(user);
        // Immutably borrow first market account open orders element.
        let market_account_view_ref = vector::borrow(&market_account_views, 0);
        // Assert element state.
        assert!(market_account_view_ref.market_id == market_id, 0);
        assert!(market_account_view_ref.custodian_id == NO_CUSTODIAN, 0);
        let asks_ref = &market_account_view_ref.asks;
        assert!(vector::length(asks_ref) == 1, 0);
        let bids_ref = &market_account_view_ref.bids;
        assert!(vector::length(bids_ref) == 0, 0);
        let order_ref = vector::borrow(asks_ref, 0);
        assert!(order_ref.market_order_id == market_order_id_ask_0, 0);
        assert!(order_ref.size == size, 0);
        // Push back value to return instead of dropping.
        vector::push_back(
            &mut return_instead_of_dropping, market_account_views);
        // Place single ask for market account without custodian, since
        // consumed during previous operation.
        place_order_internal(
            user, market_id, NO_CUSTODIAN, ASK, size, price,
            market_order_id_ask_0, order_access_key_ask_0);
        // Place bids for market account with custodian.
        place_order_internal(
            user, market_id, CUSTODIAN_ID, BID, size, price,
            market_order_id_bid_0, order_access_key_bid_0);
        place_order_internal(
            user, market_id, CUSTODIAN_ID, BID, size, price,
            market_order_id_bid_1, order_access_key_bid_1);
        // Cancel the first placed bid.
        cancel_order_internal(
            user, market_id, CUSTODIAN_ID, BID, size, price,
            order_access_key_bid_0, market_order_id_bid_0,
            CANCEL_REASON_MANUAL_CANCEL);
        // Get all of user's market account views.
        market_account_views = get_market_accounts(user);
        // Immutably borrow first market account view element.
        market_account_view_ref = vector::borrow(&market_account_views, 0);
        // Assert element state.
        assert!(market_account_view_ref.market_id == market_id, 0);
        assert!(market_account_view_ref.custodian_id == NO_CUSTODIAN, 0);
        let asks_ref = &market_account_view_ref.asks;
        assert!(vector::length(asks_ref) == 1, 0);
        let bids_ref = &market_account_view_ref.bids;
        assert!(vector::length(bids_ref) == 0, 0);
        let order_ref = vector::borrow(asks_ref, 0);
        assert!(order_ref.market_order_id == market_order_id_ask_0, 0);
        assert!(order_ref.size == size, 0);
        // Immutably borrow second market account view element.
        market_account_view_ref = vector::borrow(&market_account_views, 1);
        // Assert element state.
        assert!(market_account_view_ref.market_id == market_id, 0);
        assert!(market_account_view_ref.custodian_id == CUSTODIAN_ID, 0);
        let asks_ref = &market_account_view_ref.asks;
        assert!(vector::length(asks_ref) == 0, 0);
        let bids_ref = &market_account_view_ref.bids;
        assert!(vector::length(bids_ref) == 1, 0);
        let order_ref = vector::borrow(bids_ref, 0);
        assert!(order_ref.market_order_id == market_order_id_bid_1, 0);
        assert!(order_ref.size == size, 0);
        // Push back value to return instead of dropping.
        vector::push_back(
            &mut return_instead_of_dropping, market_account_views);
        return_instead_of_dropping // Return instead of dropping.
    }

    #[test]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNT)]
    /// Verify failure for no market account.
    fun test_get_next_order_access_key_internal_no_account()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        register_market_accounts_test();
        // Attempt invalid invocation.
        get_next_order_access_key_internal(@user, 0, 0, ASK);
    }

    #[test]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNTS)]
    /// Verify failure for no market accounts.
    fun test_get_next_order_access_key_internal_no_accounts()
    acquires MarketAccounts {
        // Attempt invalid invocation.
        get_next_order_access_key_internal(@user, 0, 0, ASK);
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_NO_CUSTODIAN() {
        assert!(get_NO_CUSTODIAN() == NO_CUSTODIAN, 0);
        assert!(get_NO_CUSTODIAN() == registry::get_NO_CUSTODIAN(), 0)
    }

    #[test(user = @user)]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNT)]
    /// Verify abort for user has no market account.
    fun test_init_market_event_handles_if_missing_no_account(
        user: &signer
    ) acquires
        MarketAccounts,
        MarketEventHandles
    {
        init_market_event_handles_if_missing(user, 0, 0);
    }

    #[test]
    /// Verify valid returns.
    fun test_market_account_getters()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Get market account IDs for test accounts.
        let market_account_id_coin_self = get_market_account_id(
            MARKET_ID_PURE_COIN, NO_CUSTODIAN);
        let market_account_id_coin_delegated = get_market_account_id(
            MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        let market_account_id_generic_self = get_market_account_id(
            MARKET_ID_GENERIC  , NO_CUSTODIAN);
        let market_account_id_generic_delegated = get_market_account_id(
            MARKET_ID_GENERIC  , CUSTODIAN_ID);
        // Assert empty returns.
        assert!(get_all_market_account_ids_for_market_id(
                @user, MARKET_ID_PURE_COIN) == vector[], 0);
        assert!(get_all_market_account_ids_for_market_id(
                @user, MARKET_ID_GENERIC) == vector[], 0);
        assert!(get_all_market_account_ids_for_user(
                @user) == vector[], 0);
        assert!(option::is_none(&get_open_order_id_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, ASK, 0)), 0);
        // Assert false returns.
        assert!(!has_market_account_by_market_account_id(
                @user, market_account_id_coin_self), 0);
        assert!(!has_market_account_by_market_account_id(
                @user, market_account_id_coin_delegated), 0);
        assert!(!has_market_account_by_market_account_id(
                @user, market_account_id_generic_self), 0);
        assert!(!has_market_account_by_market_account_id(
                @user, market_account_id_generic_delegated), 0);
        assert!(!has_market_account_by_market_id(
                @user, MARKET_ID_PURE_COIN), 0);
        assert!(!has_market_account_by_market_id(
                @user, MARKET_ID_GENERIC), 0);
        assert!(!has_market_account(
                @user, MARKET_ID_PURE_COIN, NO_CUSTODIAN), 0);
        assert!(!has_market_account(
                @user, MARKET_ID_GENERIC  , NO_CUSTODIAN), 0);
        assert!(!has_market_account(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID), 0);
        assert!(!has_market_account(
                @user, MARKET_ID_GENERIC  , CUSTODIAN_ID), 0);
        register_market_accounts_test(); // Register market accounts.
        // Assert empty returns.
        assert!(get_all_market_account_ids_for_market_id(
                @user, 123) == vector[], 0);
        // Get signer for another test user account.
        let user_1 = account::create_signer_with_capability(
            &account::create_test_signer_cap(@user_1));
        // Move to another user empty market accounts resource.
        move_to<MarketAccounts>(&user_1, MarketAccounts{
            map: table::new(), custodians: tablist::new()});
        // Assert empty returns.
        assert!(get_all_market_account_ids_for_user(
                @user_1) == vector[], 0);
        // Assert non-empty returns.
        let expected_ids = vector[market_account_id_coin_self,
                                  market_account_id_coin_delegated];
        assert!(get_all_market_account_ids_for_market_id(
                @user, MARKET_ID_PURE_COIN) == expected_ids, 0);
        expected_ids = vector[market_account_id_generic_self,
                              market_account_id_generic_delegated];
        assert!(get_all_market_account_ids_for_market_id(
                @user, MARKET_ID_GENERIC) == expected_ids, 0);
        expected_ids = vector[market_account_id_coin_self,
                              market_account_id_coin_delegated,
                              market_account_id_generic_self,
                              market_account_id_generic_delegated];
        assert!(get_all_market_account_ids_for_user(
                @user) == expected_ids, 0);
        // Assert true returns.
        assert!(has_market_account_by_market_account_id(
                @user, market_account_id_coin_self), 0);
        assert!(has_market_account_by_market_account_id(
                @user, market_account_id_coin_delegated), 0);
        assert!(has_market_account_by_market_account_id(
                @user, market_account_id_generic_self), 0);
        assert!(has_market_account_by_market_account_id(
                @user, market_account_id_generic_delegated), 0);
        assert!(has_market_account_by_market_id(
                @user, MARKET_ID_PURE_COIN), 0);
        assert!(has_market_account_by_market_id(
                @user, MARKET_ID_GENERIC), 0);
        assert!(has_market_account(
                @user, MARKET_ID_PURE_COIN, NO_CUSTODIAN), 0);
        assert!(has_market_account(
                @user, MARKET_ID_GENERIC  , NO_CUSTODIAN), 0);
        assert!(has_market_account(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID), 0);
        assert!(has_market_account(
                @user, MARKET_ID_GENERIC  , CUSTODIAN_ID), 0);
        // Assert false returns.
        assert!(!has_market_account_by_market_account_id(
                @user_1, market_account_id_coin_self), 0);
        assert!(!has_market_account_by_market_account_id(
                @user_1, market_account_id_coin_delegated), 0);
        assert!(!has_market_account_by_market_account_id(
                @user_1, market_account_id_generic_self), 0);
        assert!(!has_market_account_by_market_account_id(
                @user_1, market_account_id_generic_delegated), 0);
        assert!(!has_market_account_by_market_id(
                @user_1, MARKET_ID_PURE_COIN), 0);
        assert!(!has_market_account_by_market_id(
                @user_1, MARKET_ID_GENERIC), 0);
    }

    #[test]
    /// Verify valid returns
    fun test_market_account_id_getters() {
        let market_id =    u_64_by_32(b"10000000000000000000000000000000",
                                      b"00000000000000000000000000000001");
        let custodian_id = u_64_by_32(b"11000000000000000000000000000000",
                                      b"00000000000000000000000000000011");
        let market_account_id = get_market_account_id(market_id, custodian_id);
        assert!(market_account_id ==
                          u_128_by_32(b"10000000000000000000000000000000",
                                      b"00000000000000000000000000000001",
                                      b"11000000000000000000000000000000",
                                      b"00000000000000000000000000000011"), 0);
        assert!(get_market_id(market_account_id) == market_id, 0);
        assert!(get_custodian_id(market_account_id) == custodian_id, 0);
    }

    #[test]
    /// Verify valid state updates for placing and cancelling an ask,
    /// and next order access key lookup returns.
    fun test_place_cancel_order_ask()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test markets, get market account ID for pure coin
        // market with delegated custodian.
        let (_, _, market_account_id, _, _) = register_market_accounts_test();
        // Define order parameters.
        let market_order_id  = 1234;
        let size             = 789;
        let price            = 321;
        let side             = ASK;
        let order_access_key = 1;
        // Calculate change in base asset and quote asset fields.
        let base_delta = size * LOT_SIZE_PURE_COIN;
        let quote_delta = size * price * TICK_SIZE_PURE_COIN;
        // Deposit starting base and quote coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        // Assert inactive stack top on given side.
        assert!(get_inactive_stack_top_test(@user, market_account_id, side)
                == NIL, 0);
        // Assert next order access key.
        assert!(get_next_order_access_key_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side) == 1, 0);
        // Place order.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
        // Assert asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        assert!(base_total      == BASE_START , 0);
        assert!(base_available  == BASE_START - base_delta, 0);
        assert!(base_ceiling    == BASE_START , 0);
        assert!(quote_total     == QUOTE_START, 0);
        assert!(quote_available == QUOTE_START, 0);
        assert!(quote_ceiling   == QUOTE_START + quote_delta, 0);
        // Assert inactive stack top on given side.
        assert!(get_inactive_stack_top_test(@user, market_account_id, side)
                == NIL, 0);
        // Assert next order access key.
        assert!(get_next_order_access_key_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side) == 2, 0);
        // Assert order fields.
        let (market_order_id_r, size_r) = get_order_fields_test(
            @user, market_account_id, side, order_access_key);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r == size, 0);
        // Remove market event handles.
        remove_market_event_handles_for_market_account_test(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        // Evict order, storing returned market order ID.
        market_order_id_r = cancel_order_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side, size, price,
            order_access_key, (NIL as u128), CANCEL_REASON_MANUAL_CANCEL);
        // Assert returned market order ID.
        assert!(market_order_id_r == market_order_id, 0);
        // Assert next order access key.
        assert!(get_next_order_access_key_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side) == 1, 0);
        // Assert asset counts.
        (base_total , base_available , base_ceiling,
         quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        assert!(base_total      == BASE_START , 0);
        assert!(base_available  == BASE_START , 0);
        assert!(base_ceiling    == BASE_START , 0);
        assert!(quote_total     == QUOTE_START, 0);
        assert!(quote_available == QUOTE_START, 0);
        assert!(quote_ceiling   == QUOTE_START, 0);
        // Assert inactive stack top on given side.
        assert!(get_inactive_stack_top_test(@user, market_account_id, side)
                == order_access_key, 0);
        // Assert order marked inactive.
        assert!(!is_order_active_test(
            @user, market_account_id, side, order_access_key), 0);
        // Assert next inactive node field.
        assert!(get_next_inactive_order_test(@user, market_account_id, side,
                                             order_access_key) == NIL, 0);
    }

    #[test]
    /// Verify valid state updates for placing and cancelling a bid.
    fun test_place_cancel_order_bid()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test markets, get market account ID for pure coin
        // market with delegated custodian.
        let (_, _, market_account_id, _, _) = register_market_accounts_test();
        // Define order parameters.
        let market_order_id  = 1234;
        let size             = 789;
        let price            = 321;
        let side             = BID;
        let order_access_key = 1;
        // Calculate change in base asset and quote asset fields.
        let base_delta = size * LOT_SIZE_PURE_COIN;
        let quote_delta = size * price * TICK_SIZE_PURE_COIN;
        // Deposit starting base and quote coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        // Assert inactive stack top on given side.
        assert!(get_inactive_stack_top_test(@user, market_account_id, side)
                == NIL, 0);
        // Place order.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
        // Assert asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        assert!(base_total      == BASE_START , 0);
        assert!(base_available  == BASE_START , 0);
        assert!(base_ceiling    == BASE_START + base_delta, 0);
        assert!(quote_total     == QUOTE_START, 0);
        assert!(quote_available == QUOTE_START - quote_delta, 0);
        assert!(quote_ceiling   == QUOTE_START, 0);
        // Assert inactive stack top on given side.
        assert!(get_inactive_stack_top_test(@user, market_account_id, side)
                == NIL, 0);
        // Assert order fields.
        let (market_order_id_r, size_r) = get_order_fields_test(
            @user, market_account_id, side, order_access_key);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r == size, 0);
        // Cancel order, storing returned market order ID.
        market_order_id_r = cancel_order_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side, size, price,
            order_access_key, market_order_id, CANCEL_REASON_MANUAL_CANCEL);
        // Assert returned market order ID.
        assert!(market_order_id_r == market_order_id, 0);
        // Assert asset counts.
        (base_total , base_available , base_ceiling,
         quote_total, quote_available, quote_ceiling) =
            get_asset_counts_internal(
                @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID);
        assert!(base_total      == BASE_START , 0);
        assert!(base_available  == BASE_START , 0);
        assert!(base_ceiling    == BASE_START , 0);
        assert!(quote_total     == QUOTE_START, 0);
        assert!(quote_available == QUOTE_START, 0);
        assert!(quote_ceiling   == QUOTE_START, 0);
        // Assert inactive stack top on given side.
        assert!(get_inactive_stack_top_test(@user, market_account_id, side)
                == order_access_key, 0);
        // Assert order marked inactive.
        assert!(!is_order_active_test(
            @user, market_account_id, side, order_access_key), 0);
        // Assert next inactive node field.
        assert!(get_next_inactive_order_test(@user, market_account_id, side,
                                             order_access_key) == NIL, 0);
    }

    #[test]
    /// Verify state updates for multiple pushes and pops from stack,
    /// and next order access key lookup returns.
    fun test_place_cancel_order_stack()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test markets, get market account ID for pure coin
        // market with delegated custodian.
        let (_, _, market_account_id, _, _) = register_market_accounts_test();
        // Define order parameters.
        let market_order_id_1  = 123;
        let market_order_id_2  = 234;
        let market_order_id_3  = 345;
        let size             = MIN_SIZE_PURE_COIN;
        let price            = 1;
        let side             = BID;
        // Deposit starting base and quote coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(BASE_START));
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(QUOTE_START));
        // Assert inactive stack top on given side.
        assert!(get_inactive_stack_top_test(@user, market_account_id, side)
                == NIL, 0);
        // Assert next order access key.
        assert!(get_next_order_access_key_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side) == 1, 0);
        // Place two orders.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id_1, 1);
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id_2, 2);
        // Assert inactive stack top on given side.
        assert!(get_inactive_stack_top_test(@user, market_account_id, side)
                == NIL, 0);
        // Assert next order access key.
        assert!(get_next_order_access_key_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side) == 3, 0);
        // Cancel first order, storing market order ID.
        let market_order_id = cancel_order_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side, size, price, 1,
            market_order_id_1, CANCEL_REASON_MANUAL_CANCEL);
        // Assert returned market order ID.
        assert!(market_order_id == market_order_id_1, 0);
        // Assert inactive stack top on given side.
        assert!(get_inactive_stack_top_test(@user, market_account_id, side)
                == 1, 0);
        // Assert next order access key.
        assert!(get_next_order_access_key_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side) == 1, 0);
        // Cancel second order, storting market order ID.
        market_order_id = cancel_order_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side, size, price, 2,
            market_order_id_2, CANCEL_REASON_MANUAL_CANCEL);
        // Assert returned market order ID.
        assert!(market_order_id == market_order_id_2, 0);
        // Assert inactive stack top on given side.
        assert!(get_inactive_stack_top_test(@user, market_account_id, side)
                == 2, 0);
        // Assert next order access key.
        assert!(get_next_order_access_key_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side) == 2, 0);
        // Assert both orders marked inactive.
        assert!(!is_order_active_test(@user, market_account_id, side, 1), 0);
        assert!(!is_order_active_test(@user, market_account_id, side, 2), 0);
        // Assert next inactive node fields.
        assert!(get_next_inactive_order_test(
            @user, market_account_id, side, 2) == 1, 0);
        assert!(get_next_inactive_order_test(
            @user, market_account_id, side, 1) == NIL, 0);
        // Place an order, assigning access key at top of stack.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id_3, 2);
        // Assert inactive stack top on given side.
        assert!(get_inactive_stack_top_test(@user, market_account_id, side)
                == 1, 0);
        // Assert next order access key.
        assert!(get_next_order_access_key_internal(
            @user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side) == 1, 0);
        // Assert order fields.
        let (market_order_id_r, size_r) = get_order_fields_test(
            @user, market_account_id, side, 2);
        assert!(market_order_id_r == market_order_id_3, 0);
        assert!(size_r == size, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_ACCESS_KEY_MISMATCH)]
    /// Verify failure for access key mismatch.
    fun test_place_order_internal_access_key_mismatch()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        register_market_accounts_test(); // Register market accounts.
        // Declare order parameters
        let market_order_id  = 123;
        let size             = MIN_SIZE_PURE_COIN;
        let price            = 1;
        let side             = BID;
        // Calculate minimum base fill amount for price of 1.
        let min_fill_base = MIN_SIZE_PURE_COIN * LOT_SIZE_PURE_COIN;
        // Calculate starting base coin amount for fill to ceiling.
        let base_start = HI_64 - min_fill_base;
        // Deposit base coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(base_start));
        // Deposit max quote coins.
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(HI_64));
        // Attempt invalid invocation.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 2);
    }
    #[test]
    #[expected_failure(abort_code = E_OVERFLOW_ASSET_IN)]
    /// Verify failure for overflowed inbound asset.
    fun test_place_order_internal_in_overflow()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        register_market_accounts_test(); // Register market accounts.
        // Declare order parameters
        let market_order_id  = 123;
        let size             = MIN_SIZE_PURE_COIN;
        let price            = 1;
        let side             = BID;
        // Calculate minimum base fill amount for price of 1.
        let min_fill_base = MIN_SIZE_PURE_COIN * LOT_SIZE_PURE_COIN;
        // Calculate starting base coin amount for barely overflowing.
        let base_start = HI_64 - min_fill_base + 1;
        // Deposit base coins.
        deposit_coins<BC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(base_start));
        // Deposit max quote coins.
        deposit_coins<QC>(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID,
                          assets::mint_test(HI_64));
        // Attempt invalid invocation.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
    }

    #[test]
    #[expected_failure(abort_code = E_NOT_ENOUGH_ASSET_OUT)]
    /// Verify failure for underflowed outbound asset.
    fun test_place_order_internal_out_underflow()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        register_market_accounts_test(); // Register market accounts.
        // Declare order parameters
        let market_order_id  = 123;
        let size             = MIN_SIZE_PURE_COIN;
        let price            = 1;
        let side             = BID;
        // Attempt invalid invocation.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
    }

    #[test]
    #[expected_failure(abort_code = E_PRICE_0)]
    /// Verify failure for price 0.
    fun test_place_order_internal_price_0()
    acquires
        MarketAccounts
    {
        // Declare order parameters
        let market_order_id  = 123;
        let size             = MIN_SIZE_PURE_COIN;
        let price            = 0;
        let side             = ASK;
        // Attempt invalid invocation.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
    }

    #[test]
    #[expected_failure(abort_code = E_PRICE_TOO_HIGH)]
    /// Verify failure for price too high.
    fun test_place_order_internal_price_hi()
    acquires
        MarketAccounts
    {
        // Declare order parameters
        let market_order_id  = 123;
        let size             = MIN_SIZE_PURE_COIN;
        let price            = HI_PRICE + 1;
        let side             = ASK;
        // Attempt invalid invocation.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
    }

    #[test]
    #[expected_failure(abort_code = E_TICKS_OVERFLOW)]
    /// Verify failure for overflowed ticks.
    fun test_place_order_internal_ticks_overflow()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        register_market_accounts_test(); // Register market accounts.
        // Declare order parameters
        let market_order_id  = 123;
        let size             = HI_64 / HI_PRICE + 1;
        let price            = HI_PRICE;
        let side             = ASK;
        // Attempt invalid invocation.
        place_order_internal(@user, MARKET_ID_PURE_COIN, CUSTODIAN_ID, side,
                             size, price, market_order_id, 1);
    }

    #[test(user = @user)]
    #[expected_failure(abort_code = E_EXISTS_MARKET_ACCOUNT)]
    /// Verify failure for market account already exists.
    fun test_register_market_account_account_entries_exists(
        user: &signer
    ) acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        account::create_account_for_test(address_of(user));
        // Register test markets, storing pure coin market ID.
        let (market_id_pure_coin, _, _, _, _, _, _, _, _, _, _, _) =
            registry::register_markets_test();
        // Register user with market account.
        register_market_account<BC, QC>(
            user, market_id_pure_coin, NO_CUSTODIAN);
        // Attempt invalid re-registration.
        register_market_account<BC, QC>(
            user, market_id_pure_coin, NO_CUSTODIAN);
    }

    #[test(user = @user)]
    #[expected_failure(abort_code = E_UNREGISTERED_CUSTODIAN)]
    /// Verify failure for unregistered custodian.
    fun test_register_market_account_unregistered_custodian(
        user: &signer
    ) acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        registry::init_test(); // Initialize registry.
        // Attempt invalid invocation.
        register_market_account<BC, QC>(user, 1, 123);
    }

    #[test(user = @user)]
    /// Verify state updates for market account registration.
    ///
    /// Exercises all non-assert conditional branches for:
    ///
    /// * `get_market_event_handle_creation_numbers()`
    /// * `init_market_event_handles_if_missing()`
    /// * `register_market_account()`
    /// * `register_market_account_account_entries()`
    /// * `register_market_account_collateral_entry()`
    fun test_register_market_accounts(
        user: &signer
    ) acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        account::create_account_for_test(address_of(user));
        // Register test markets, storing market info.
        let (market_id_pure_coin, base_name_generic_pure_coin,
             lot_size_pure_coin, tick_size_pure_coin, min_size_pure_coin,
             underwriter_id_pure_coin, market_id_generic,
             base_name_generic_generic, lot_size_generic, tick_size_generic,
             min_size_generic, underwriter_id_generic) =
             registry::register_markets_test();
        // Verify no event handle creation numbers.
        assert!(get_market_event_handle_creation_numbers(
            @user, market_id_pure_coin, NO_CUSTODIAN) == option::none(), 0);
        // Set custodian ID as registered.
        registry::set_registered_custodian_test(CUSTODIAN_ID);
        // Register pure coin market account.
        register_market_account<BC, QC>(
            user, market_id_pure_coin, NO_CUSTODIAN);
        assert!(get_market_event_handle_creation_numbers(
            @user, market_id_pure_coin, NO_CUSTODIAN) == option::some(
                MarketEventHandleCreationNumbers{
                    cancel_order_events_handle_creation_num: 2,
                    change_order_size_events_handle_creation_num: 3,
                    fill_events_handle_creation_num: 4,
                    place_limit_order_events_handle_creation_num: 5,
                    place_market_order_events_handle_creation_num: 6}), 0);
        // Invoke init call for handles already initialized.
        init_market_event_handles_if_missing(
            user, market_id_pure_coin, NO_CUSTODIAN);
        assert!(get_market_event_handle_creation_numbers(
            @user, market_id_pure_coin, CUSTODIAN_ID) == option::none(), 0);
        register_market_account<BC, QC>( // Register delegated account.
            user, market_id_pure_coin, CUSTODIAN_ID);
        assert!(get_market_event_handle_creation_numbers(
            @user, market_id_pure_coin, CUSTODIAN_ID) == option::some(
                MarketEventHandleCreationNumbers{
                    cancel_order_events_handle_creation_num: 7,
                    change_order_size_events_handle_creation_num: 8,
                    fill_events_handle_creation_num: 9,
                    place_limit_order_events_handle_creation_num: 10,
                    place_market_order_events_handle_creation_num: 11}), 0);
        // Register generic asset account.
        register_market_account_generic_base<QC>(
            user, market_id_generic, NO_CUSTODIAN);
        // Get market account IDs.
        let market_account_id_self = get_market_account_id(
            market_id_pure_coin, NO_CUSTODIAN);
        let market_account_id_delegated = get_market_account_id(
            market_id_pure_coin, CUSTODIAN_ID);
        let market_account_id_generic = get_market_account_id(
            market_id_generic, NO_CUSTODIAN);
        // Immutably borrow base coin collateral.
        let collateral_map_ref = &borrow_global<Collateral<BC>>(@user).map;
        // Assert entries only made for pure coin market accounts.
        assert!(coin::value(tablist::borrow(
            collateral_map_ref, market_account_id_self)) == 0, 0);
        assert!(coin::value(tablist::borrow(
            collateral_map_ref, market_account_id_delegated)) == 0, 0);
        assert!(!tablist::contains(
            collateral_map_ref, market_account_id_generic), 0);
        // Immutably borrow quote coin collateral.
        let collateral_map_ref = &borrow_global<Collateral<QC>>(@user).map;
        // Assert entries made for all market accounts.
        assert!(coin::value(tablist::borrow(
            collateral_map_ref, market_account_id_self)) == 0, 0);
        assert!(coin::value(tablist::borrow(
            collateral_map_ref, market_account_id_delegated)) == 0, 0);
        assert!(coin::value(tablist::borrow(
            collateral_map_ref, market_account_id_generic)) == 0, 0);
        let custodians_map_ref = // Immutably borrow custodians map.
            &borrow_global<MarketAccounts>(@user).custodians;
        // Immutably borrow custodians entry for pure coin market.
        let custodians_ref =
            tablist::borrow(custodians_map_ref, market_id_pure_coin);
        // Assert listed custodians.
        assert!(*custodians_ref
                == vector[NO_CUSTODIAN, CUSTODIAN_ID], 0);
        // Immutably borrow custodians entry for generic market.
        custodians_ref =
            tablist::borrow(custodians_map_ref, market_id_generic);
        assert!( // Assert listed custodian.
            *custodians_ref == vector[NO_CUSTODIAN], 0);
        // Immutably borrow market accounts map.
        let market_accounts_map_ref =
            &borrow_global<MarketAccounts>(@user).map;
        // Immutably borrow pure coin self-custodied market account.
        let market_account_ref =
            table::borrow(market_accounts_map_ref, market_account_id_self);
        // Assert state.
        assert!(market_account_ref.base_type == type_info::type_of<BC>(), 0);
        assert!(market_account_ref.base_name_generic
                == base_name_generic_pure_coin, 0);
        assert!(market_account_ref.quote_type == type_info::type_of<QC>(), 0);
        assert!(market_account_ref.lot_size == lot_size_pure_coin, 0);
        assert!(market_account_ref.tick_size == tick_size_pure_coin, 0);
        assert!(market_account_ref.min_size == min_size_pure_coin, 0);
        assert!(market_account_ref.underwriter_id
                == underwriter_id_pure_coin, 0);
        assert!(tablist::is_empty(&market_account_ref.asks), 0);
        assert!(tablist::is_empty(&market_account_ref.bids), 0);
        assert!(market_account_ref.asks_stack_top == NIL, 0);
        assert!(market_account_ref.bids_stack_top == NIL, 0);
        assert!(market_account_ref.base_total == 0, 0);
        assert!(market_account_ref.base_available == 0, 0);
        assert!(market_account_ref.base_ceiling == 0, 0);
        assert!(market_account_ref.quote_total == 0, 0);
        assert!(market_account_ref.quote_available == 0, 0);
        assert!(market_account_ref.quote_ceiling == 0, 0);
        // Immutably borrow pure coin delegated market account.
        market_account_ref = table::borrow(market_accounts_map_ref,
                                           market_account_id_delegated);
        // Assert state.
        assert!(market_account_ref.base_type == type_info::type_of<BC>(), 0);
        assert!(market_account_ref.base_name_generic
                == base_name_generic_pure_coin, 0);
        assert!(market_account_ref.quote_type == type_info::type_of<QC>(), 0);
        assert!(market_account_ref.lot_size == lot_size_pure_coin, 0);
        assert!(market_account_ref.tick_size == tick_size_pure_coin, 0);
        assert!(market_account_ref.min_size == min_size_pure_coin, 0);
        assert!(market_account_ref.underwriter_id
                == underwriter_id_pure_coin, 0);
        assert!(tablist::is_empty(&market_account_ref.asks), 0);
        assert!(tablist::is_empty(&market_account_ref.bids), 0);
        assert!(market_account_ref.asks_stack_top == NIL, 0);
        assert!(market_account_ref.bids_stack_top == NIL, 0);
        assert!(market_account_ref.base_total == 0, 0);
        assert!(market_account_ref.base_available == 0, 0);
        assert!(market_account_ref.base_ceiling == 0, 0);
        assert!(market_account_ref.quote_total == 0, 0);
        assert!(market_account_ref.quote_available == 0, 0);
        assert!(market_account_ref.quote_ceiling == 0, 0);
        // Immutably borrow generic market account.
        market_account_ref =
            table::borrow(market_accounts_map_ref, market_account_id_generic);
        // Assert state.
        assert!(market_account_ref.base_type
                == type_info::type_of<GenericAsset>(), 0);
        assert!(market_account_ref.base_name_generic
                == base_name_generic_generic, 0);
        assert!(market_account_ref.quote_type == type_info::type_of<QC>(), 0);
        assert!(market_account_ref.lot_size == lot_size_generic, 0);
        assert!(market_account_ref.tick_size == tick_size_generic, 0);
        assert!(market_account_ref.min_size == min_size_generic, 0);
        assert!(market_account_ref.underwriter_id
                == underwriter_id_generic, 0);
        assert!(tablist::is_empty(&market_account_ref.asks), 0);
        assert!(tablist::is_empty(&market_account_ref.bids), 0);
        assert!(market_account_ref.asks_stack_top == NIL, 0);
        assert!(market_account_ref.bids_stack_top == NIL, 0);
        assert!(market_account_ref.base_total == 0, 0);
        assert!(market_account_ref.base_available == 0, 0);
        assert!(market_account_ref.base_ceiling == 0, 0);
        assert!(market_account_ref.quote_total == 0, 0);
        assert!(market_account_ref.quote_available == 0, 0);
        assert!(market_account_ref.quote_ceiling == 0, 0);
        // Verify market info getter returns for self-custodied pure
        // coin market account.
        let (base_type_r, base_name_generic_r, quote_type_r, lot_size_r,
             tick_size_r, min_size_r, underwriter_id_r) =
            get_market_account_market_info_user(user, market_id_pure_coin);
        assert!(base_type_r == type_info::type_of<BC>(), 0);
        assert!(base_name_generic_r == base_name_generic_pure_coin, 0);
        assert!(quote_type_r == type_info::type_of<QC>(), 0);
        assert!(lot_size_r == lot_size_pure_coin, 0);
        assert!(tick_size_r == tick_size_pure_coin, 0);
        assert!(min_size_r == min_size_pure_coin, 0);
        assert!(underwriter_id_r == underwriter_id_pure_coin, 0);
        let custodian_capability = registry::get_custodian_capability_test(
            CUSTODIAN_ID); // Get custodian capability.
        // Verify market info getter returns for delegated pure coin
        // market account.
        let (base_type_r, base_name_generic_r, quote_type_r, lot_size_r,
             tick_size_r, min_size_r, underwriter_id_r) =
            get_market_account_market_info_custodian(
                @user, market_id_pure_coin, &custodian_capability);
        assert!(base_type_r == type_info::type_of<BC>(), 0);
        assert!(base_name_generic_r == base_name_generic_pure_coin, 0);
        assert!(quote_type_r == type_info::type_of<QC>(), 0);
        assert!(lot_size_r == lot_size_pure_coin, 0);
        assert!(tick_size_r == tick_size_pure_coin, 0);
        assert!(min_size_r == min_size_pure_coin, 0);
        assert!(underwriter_id_r == underwriter_id_pure_coin, 0);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
        // Verify market info getter returns for self-custodied generic
        // market account.
        let (base_type_r, base_name_generic_r, quote_type_r, lot_size_r,
             tick_size_r, min_size_r, underwriter_id_r) =
            get_market_account_market_info_user(user, market_id_generic);
        assert!(base_type_r == type_info::type_of<GenericAsset>(), 0);
        assert!(base_name_generic_r == base_name_generic_generic, 0);
        assert!(quote_type_r == type_info::type_of<QC>(), 0);
        assert!(lot_size_r == lot_size_generic, 0);
        assert!(tick_size_r == tick_size_generic, 0);
        assert!(min_size_r == min_size_generic, 0);
        assert!(underwriter_id_r == underwriter_id_generic, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNT)]
    /// Verify failure for no market account.
    fun test_withdraw_asset_no_account()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        let (user, _, _, _, _) = register_market_accounts_test();
        // Attempt invalid invocation, burning returned coins.
        assets::burn(withdraw_coins_user<BC>(&user, 0, 0));
    }

    #[test(user = @user)]
    #[expected_failure(abort_code = E_NO_MARKET_ACCOUNTS)]
    /// Verify failure for no market accounts.
    fun test_withdraw_asset_no_accounts(
        user: &signer
    ) acquires
        Collateral,
        MarketAccounts
    {
        // Attempt invalid invocation, burning returned coins.
        assets::burn(withdraw_coins_user<BC>(user, 0, 0));
    }

    #[test]
    #[expected_failure(abort_code = E_ASSET_NOT_IN_PAIR)]
    /// Verify failure for asset not in pair.
    fun test_withdraw_asset_not_in_pair()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        let (user, _, _, _, _) = register_market_accounts_test();
        // Attempt invalid invocation, burning returned coins.
        assets::burn(withdraw_coins_user<UC>(&user, MARKET_ID_PURE_COIN, 0));
    }

    #[test]
    #[expected_failure(abort_code = E_WITHDRAW_TOO_LITTLE_AVAILABLE)]
    /// Verify failure for not enough asset available to withdraw.
    fun test_withdraw_asset_underflow()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        let (user, _, _, _, _) = register_market_accounts_test();
        // Attempt invalid invocation, burning returned coins.
        assets::burn(withdraw_coins_user<QC>(&user, MARKET_ID_PURE_COIN, 1));
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_UNDERWRITER)]
    /// Verify failure for invalid underwriter.
    fun test_withdraw_asset_underwriter()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Register test market accounts.
        let (user, _, _, _, _) = register_market_accounts_test();
        let underwriter_capability = // Get underwriter capability.
            registry::get_underwriter_capability_test(UNDERWRITER_ID + 1);
        // Attempt invalid invocation.
        withdraw_generic_asset_user(&user, MARKET_ID_GENERIC, 0,
                                    &underwriter_capability);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
    }

    #[test]
    /// Verify state updates for assorted withdrawal styles.
    fun test_withdrawals()
    acquires
        Collateral,
        MarketAccounts,
        MarketEventHandles
    {
        // Declare start amount parameters.
        let amount_start_coin = 700;
        let amount_start_generic = 500;
        // Declare withdrawal amount parameters.
        let amount_withdraw_coin_0 = 350;
        let amount_withdraw_generic_0 = 450;
        let amount_withdraw_coin_1 = 300;
        let amount_withdraw_generic_1 = 400;
        // Declare final amounts.
        let amount_final_coin_0 = amount_start_coin - amount_withdraw_coin_0;
        let amount_final_generic_0 = amount_start_generic
                                     - amount_withdraw_generic_0;
        let amount_final_coin_1 = amount_start_coin - amount_withdraw_coin_1;
        let amount_final_generic_1 = amount_start_generic
                                     - amount_withdraw_generic_1;
        // Get signing user and test market account IDs.
        let (user, _, _, market_account_id_generic_self,
                         market_account_id_generic_delegated) =
             register_market_accounts_test();
        let custodian_capability = // Get custodian capability.
            registry::get_custodian_capability_test(CUSTODIAN_ID);
        let underwriter_capability = // Get underwriter capability.
            registry::get_underwriter_capability_test(UNDERWRITER_ID);
        // Deposit to both market accounts.
        deposit_coins<QC>(@user, MARKET_ID_GENERIC, NO_CUSTODIAN,
                          assets::mint_test(amount_start_coin));
        deposit_coins<QC>(@user, MARKET_ID_GENERIC, CUSTODIAN_ID,
                          assets::mint_test(amount_start_coin));
        deposit_generic_asset(@user, MARKET_ID_GENERIC, NO_CUSTODIAN,
                              amount_start_generic, &underwriter_capability);
        deposit_generic_asset(@user, MARKET_ID_GENERIC, CUSTODIAN_ID,
                              amount_start_generic, &underwriter_capability);
        // Withdraw coins to coin store under authority of signing user.
        withdraw_to_coinstore<QC>(&user, MARKET_ID_GENERIC, 1);
        withdraw_to_coinstore<QC>(&user, MARKET_ID_GENERIC,
                                  amount_withdraw_coin_0 - 1);
        // Assert coin store balance.
        assert!(coin::balance<QC>(@user) == amount_withdraw_coin_0, 0);
        // Withdraw coins under authority of delegated custodian.
        let coins = withdraw_coins_custodian<QC>(
            @user, MARKET_ID_GENERIC, amount_withdraw_coin_1,
            &custodian_capability);
        // Assert withdrawn coin value.
        assert!(coin::value(&coins) == amount_withdraw_coin_1, 0);
        assets::burn(coins); // Burn coins.
        // Withdraw generic asset under authority of signing user.
        withdraw_generic_asset_user(
            &user, MARKET_ID_GENERIC, amount_withdraw_generic_0,
            &underwriter_capability);
        // Withdraw generic asset under authority of delegated
        // custodian.
        withdraw_generic_asset_custodian(
            @user, MARKET_ID_GENERIC, amount_withdraw_generic_1,
            &custodian_capability, &underwriter_capability);
        // Assert state for self-custodied account.
        let ( base_total,  base_available,  base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            get_asset_counts_user(&user, MARKET_ID_GENERIC);
        assert!(base_total      == amount_final_generic_0, 0);
        assert!(base_available  == amount_final_generic_0, 0);
        assert!(base_ceiling    == amount_final_generic_0, 0);
        assert!(quote_total     == amount_final_coin_0   , 0);
        assert!(quote_available == amount_final_coin_0   , 0);
        assert!(quote_ceiling   == amount_final_coin_0   , 0);
        assert!(!has_collateral_test<GenericAsset>(
            @user, market_account_id_generic_self), 0);
        assert!(get_collateral_value_test<QC>(
            @user, market_account_id_generic_self) == amount_final_coin_0, 0);
        // Assert state for delegated custody account.
        let ( base_total,  base_available,  base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            get_asset_counts_custodian(
                @user, MARKET_ID_GENERIC, &custodian_capability);
        assert!(base_total      == amount_final_generic_1, 0);
        assert!(base_available  == amount_final_generic_1, 0);
        assert!(base_ceiling    == amount_final_generic_1, 0);
        assert!(quote_total     == amount_final_coin_1   , 0);
        assert!(quote_available == amount_final_coin_1   , 0);
        assert!(quote_ceiling   == amount_final_coin_1   , 0);
        assert!(!has_collateral_test<GenericAsset>(
            @user, market_account_id_generic_delegated), 0);
        assert!(get_collateral_value_test<QC>(
            @user, market_account_id_generic_delegated) ==
            amount_final_coin_1, 0);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
    }

    // Tests <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

}