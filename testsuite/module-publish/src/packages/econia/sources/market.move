/// Market functionality for order book operations.
///
/// For each registered market, Econia has an order book stored under a
/// global resource account. When someone registers a market, a new
/// order book entry is added under the resource account at a new market
/// ID.
///
/// Once a market is registered, signing users and delegated custodians
/// can place limit orders and market orders, and cancel or change the
/// size of any open orders. Swaps can be placed permissionlessly
/// without a market account.
///
/// Econia implements an atomic matching engine, and emits events in
/// response to changes in order book state as well as assorted market
/// operations. Notably, Econia evicts the ask or bid with the lowest
/// price-time priority when inserting a limit order to a binary search
/// tree that exceeds a critical height.
///
/// Multiple API variants are supported for market registration and
/// order management function, to enable diagnostic function returns,
/// public entry calls, etc.
///
/// All orders are issued an order ID upon placement, which is unique to
/// the given market. The order ID encodes a counter fo the number of
/// orders that have been placed on the corresponding market. For orders
/// that result in a post to the book, the market order ID additionally
/// encodes an "AVL queue access key" (essentially a pointer into
/// order book memory), which is required for order lookup during order
/// size change and/or order cancellation operations.
///
/// Note that the terms "order ID" and "market order ID" are used
/// interchangeably.
///
/// # General overview sections
///
/// [View functions](#view-functions)
///
/// * [Constant getters](#constant-getters)
/// * [Market order ID decoders](#market-order-id-decoders)
/// * [Order lookup](#order-lookup)
///
/// [Public function index](#public-function-index)
///
/// * [Market registration](#market-registration)
/// * [Limit orders](#limit-orders)
/// * [Passive advance limit orders](#passive-advance-limit-orders)
/// * [Market orders](#market-orders)
/// * [Swaps](#swaps)
/// * [Change order size](#change-order-size)
/// * [Cancel orders](#cancel-orders)
///
/// [Dependency charts](#dependency-charts)
///
/// * [Internal dependencies](#internal-dependencies)
/// * [External module dependencies](#external-module-dependencies)
///
/// [Order management testing](#order-management-testing)
///
/// * [Functions with aborts](#functions-with-aborts)
/// * [Return proxies](#return-proxies)
/// * [Invocation proxies](#invocation-proxies)
/// * [Branching functions](#branching-functions)
///
/// [Complete DocGen index](#complete-docgen-index)
///
/// # View functions
///
/// ## Constant getters
///
/// * `get_ABORT()`
/// * `get_ASK()`
/// * `get_BID()`
/// * `get_BUY()`
/// * `get_CANCEL_BOTH()`
/// * `get_CANCEL_MAKER()`
/// * `get_CANCEL_TAKER()`
/// * `get_FILL_OR_ABORT()`
/// * `get_HI_PRICE()`
/// * `get_IMMEDIATE_OR_CANCEL()`
/// * `get_MAX_POSSIBLE()`
/// * `get_NO_CUSTODIAN()`
/// * `get_NO_RESTRICTION()`
/// * `get_NO_UNDERWRITER()`
/// * `get_POST_OR_ABORT()`
/// * `get_PERCENT()`
/// * `get_SELL()`
/// * `get_TICKS()`
///
/// ## Market order ID decoders
///
/// * `did_order_post()`
/// * `get_market_order_id_counter()`
/// * `get_market_order_id_price()`
/// * `get_posted_order_id_side()`
///
/// ## Event handle lookup
///
/// * `get_market_event_handle_creation_info()`
/// * `get_swapper_event_handle_creation_numbers()`
///
/// ## Order lookup
///
/// * `get_open_order()`
/// * `get_open_orders()`
/// * `get_open_orders_all()`
/// * `get_open_orders_paginated()`
/// * `get_price_levels()`
/// * `get_price_levels_all()`
/// * `get_price_levels_paginated()`
/// * `has_open_order()`
///
/// # Public function index
///
/// See the [dependency charts](#dependency-charts) for a visual map of
/// associated function wrappers.
///
/// ## Market registration
///
/// * `register_market_base_coin()`
/// * `register_market_base_coin_from_coinstore()`
/// * `register_market_base_generic()`
///
/// ## Limit orders
///
/// * `place_limit_order_custodian()`
/// * `place_limit_order_user()`
/// * `place_limit_order_user_entry()`
///
/// ## Passive advance limit orders
///
/// * `place_limit_order_passive_advance_custodian()`
/// * `place_limit_order_passive_advance_user()`
/// * `place_limit_order_passive_advance_user_entry()`
///
/// ## Market orders
///
/// * `place_market_order_custodian()`
/// * `place_market_order_user()`
/// * `place_market_order_user_entry()`
///
/// ## Swaps
///
/// * `swap_between_coinstores()`
/// * `swap_between_coinstores_entry()`
/// * `swap_coins()`
/// * `swap_generic()`
///
/// ## Change order size
///
/// * `change_order_size_custodian()`
/// * `change_order_size_user()`
///
/// ## Cancel orders
///
/// * `cancel_order_custodian()`
/// * `cancel_order_user()`
/// * `cancel_all_orders_custodian()`
/// * `cancel_all_orders_user()`
///
/// # Dependency charts
///
/// The below dependency charts use `mermaid.js` syntax, which can be
/// automatically rendered into a diagram (depending on the browser)
/// when viewing the documentation file generated from source code. If
/// a browser renders the diagrams with coloring that makes it difficult
/// to read, try a different browser.
///
/// ## Internal dependencies
///
/// These charts describe dependencies between `market` functions.
///
/// Market registration:
///
/// ```mermaid
///
/// flowchart LR
///
/// register_market_base_coin --> register_market
///
/// register_market_base_generic --> register_market
///
/// register_market_base_coin_from_coinstore -->
///     register_market_base_coin
///
/// ```
///
/// Placing orders:
///
/// ```mermaid
///
/// flowchart LR
///
/// place_limit_order ---> match
///
/// place_limit_order --> range_check_trade
///
/// place_market_order ---> match
///
/// place_market_order --> range_check_trade
///
/// swap ---> match
///
/// swap_between_coinstores ---> range_check_trade
///
/// subgraph Swaps
///
/// swap_between_coinstores_entry --> swap_between_coinstores
///
/// swap_between_coinstores --> swap
///
/// swap_coins --> swap
///
/// swap_generic --> swap
///
/// end
///
/// swap_coins ---> range_check_trade
///
/// swap_generic ---> range_check_trade
///
/// place_limit_order_passive_advance --> place_limit_order
///
/// subgraph Market orders
///
/// place_market_order_user_entry --> place_market_order_user
///
/// place_market_order_user --> place_market_order
///
/// place_market_order_custodian --> place_market_order
///
/// end
///
/// subgraph Limit orders
///
/// place_limit_order_user_entry --> place_limit_order_user
///
/// place_limit_order_user --> place_limit_order
///
/// place_limit_order_custodian --> place_limit_order
///
/// end
///
/// subgraph Passive advance limit orders
///
/// place_limit_order_passive_advance_user_entry -->
///     place_limit_order_passive_advance_user
///
/// place_limit_order_passive_advance_user -->
///     place_limit_order_passive_advance
///
/// place_limit_order_passive_advance_custodian -->
///     place_limit_order_passive_advance
///
/// end
///
/// ```
///
/// Cancel reasons:
///
/// ```mermaid
///
/// flowchart LR
///
/// place_market_order -->
///     get_cancel_reason_option_for_market_order_or_swap
/// swap --> get_cancel_reason_option_for_market_order_or_swap
///
/// ```
///
/// Changing order size:
///
/// ```mermaid
///
/// flowchart LR
///
/// change_order_size_custodian --> change_order_size
///
/// change_order_size_user --> change_order_size
///
/// ```
///
/// Cancelling orders:
///
/// ```mermaid
///
/// flowchart LR
///
/// cancel_all_orders_custodian --> cancel_all_orders
///
/// cancel_order_custodian --> cancel_order
///
/// cancel_all_orders_user --> cancel_all_orders
///
/// cancel_order_user --> cancel_order
///
/// cancel_all_orders --> cancel_order
///
/// ```
///
/// View functions:
///
/// ```mermaid
///
/// flowchart LR
///
/// get_open_orders --> get_open_orders_for_side
/// get_open_orders_all --> get_open_orders
/// get_price_levels --> get_open_orders
/// get_price_levels --> get_price_levels_for_side
/// get_market_order_id_price --> did_order_post
/// get_price_levels_all --> get_price_levels
/// get_open_order --> has_open_order
/// get_open_order --> get_posted_order_id_side
/// get_open_order --> get_order_id_avl_queue_access_key
/// get_posted_order_id_side --> did_order_post
/// get_posted_order_id_side --> get_order_id_avl_queue_access_key
/// has_open_order --> get_posted_order_id_side
/// has_open_order --> get_order_id_avl_queue_access_key
/// get_open_orders_paginated --> get_open_orders_for_side_paginated
/// get_open_orders_paginated --> verify_pagination_order_ids
/// get_open_orders_for_side_paginated -->
///     get_order_id_avl_queue_access_key
/// get_price_levels_paginated --> get_price_levels_for_side_paginated
/// get_price_levels_paginated --> verify_pagination_order_ids
/// get_price_levels_for_side_paginated -->
///     get_order_id_avl_queue_access_key
/// verify_pagination_order_ids --> has_open_order
/// verify_pagination_order_ids --> get_posted_order_id_side
///
/// ```
///
/// ## External module dependencies
///
/// These charts describe `market` function dependencies on functions
/// from other Econia modules, other than `avl_queue` and `tablist`,
/// which are essentially data structure libraries.
///
/// `incentives`:
///
/// ``` mermaid
///
/// flowchart LR
///
/// register_market_base_coin_from_coinstore -->
///     incentives::get_market_registration_fee
///
/// register_market --> incentives::register_econia_fee_store_entry
///
/// match --> incentives::get_taker_fee_divisor
/// match --> incentives::calculate_max_quote_match
/// match --> incentives::assess_taker_fees
///
/// ```
///
/// `registry`:
///
/// ``` mermaid
///
/// flowchart LR
///
/// register_market_base_coin -->
///     registry::register_market_base_coin_internal
///
/// register_market_base_generic -->
///     registry::register_market_base_generic_internal
/// register_market_base_generic -->
///     registry::get_underwriter_id
///
/// place_limit_order_custodian --> registry::get_custodian_id
///
/// place_market_order_custodian --> registry::get_custodian_id
///
/// swap_generic --> registry::get_underwriter_id
///
/// change_order_size_custodian --> registry::get_custodian_id
///
/// cancel_order_custodian --> registry::get_custodian_id
///
/// cancel_all_orders_custodian --> registry::get_custodian_id
///
/// ```
///
/// `resource_account`:
///
/// ``` mermaid
///
/// flowchart LR
///
/// init_module --> resource_account::get_signer
///
/// register_market --> resource_account::get_signer
///
/// place_limit_order --> resource_account::get_address
///
/// place_market_order --> resource_account::get_address
///
/// swap --> resource_account::get_address
/// swap --> resource_account::get_signer
///
/// change_order_size --> resource_account::get_address
///
/// cancel_order --> resource_account::get_address
///
/// get_open_order --> resource_account::get_address
///
/// get_open_orders --> resource_account::get_address
///
/// has_open_order --> resource_account::get_address
///
/// get_price_levels --> resource_account::get_address
///
/// get_market_event_handle_creation_info -->
///     resource_account::get_address
///
/// get_open_orders_paginated --> resource_account::get_address
///
/// get_price_levels_paginated --> resource_account::get_address
///
/// ```
///
/// `user`:
///
/// ``` mermaid
///
/// flowchart LR
///
/// place_limit_order --> user::get_asset_counts_internal
/// place_limit_order --> user::withdraw_assets_internal
/// place_limit_order --> user::deposit_assets_internal
/// place_limit_order --> user::get_next_order_access_key_internal
/// place_limit_order --> user::place_order_internal
/// place_limit_order --> user::cancel_order_internal
/// place_limit_order --> user::emit_limit_order_events_internal
///
/// place_market_order --> user::get_asset_counts_internal
/// place_market_order --> user::withdraw_assets_internal
/// place_market_order --> user::deposit_assets_internal
/// place_market_order --> user::emit_market_order_events_internal
///
/// match --> user::fill_order_internal
/// match --> user::create_fill_event_internal
///
/// change_order_size --> user::change_order_size_internal
///
/// cancel_order --> user::cancel_order_internal
///
/// cancel_all_orders --> user::get_active_market_order_ids_internal
///
/// has_open_order --> user::get_open_order_id_internal
///
/// get_open_orders_for_side --> user::get_open_order_id_internal
///
/// swap --> user::create_cancel_order_event_internal
/// swap --> user::emit_swap_maker_fill_events_internal
///
/// get_open_orders_for_side_paginated -->
///     user::get_open_order_id_internal
///
/// get_price_levels_for_side_paginated -->
///     user::get_open_order_id_internal
///
/// ```
///
/// # Order management testing
///
/// While market registration functions can be simply verified with
/// straightforward tests, order management functions are more
/// comprehensively tested through integrated tests that verify multiple
/// logical branches, returns, and state updates. Aborts are tested
/// individually for each function.
///
/// ## Functions with aborts
///
/// Function aborts to test:
///
/// * [x] `cancel_order()`
/// * [x] `change_order_size()`
/// * [x] `match()`
/// * [x] `place_limit_order()`
/// * [x] `place_limit_order_passive_advance()`
/// * [x] `place_market_order()`
/// * [x] `range_check_trade()`
/// * [x] `swap()`
///
/// ## Return proxies
///
/// Various order management functions have returns, and verifying the
/// returns of some functions verifies the returns of associated inner
/// functions. For example, the collective verification of the returns
/// of `swap_coins()` and `swap_generic()` verifies the returns of both
/// `swap()` and `match()`, such that the combination of `swap_coins()`
/// and `swap_generic()` can be considered a "return proxy" of both
/// `swap()` and of `match()`. Hence the most efficient test suite
/// involves return verification for the minimal return proxy set:
///
/// | Function                         | Return proxy                |
/// |----------------------------------|-----------------------------|
/// | `match()`                   | `swap_coins()`, `swap_generic()` |
/// | `place_limit_order()`            | `place_limit_order_user()`  |
/// | `place_limit_order_custodian()`  | None                        |
/// | `place_limit_order_user()`       | None                        |
/// | `place_market_order()`           | `place_market_order_user()` |
/// | `place_market_order_custodian()` | None                        |
/// | `place_market_order_user()`      | None                        |
/// | `swap()`                    | `swap_coins()`, `swap_generic()` |
/// | `swap_between_coinstores()`      | None                        |
/// | `swap_coins()`                   | None                        |
/// | `swap_generic()`                 | None                        |
///
/// Passive advance limit order functions do not fit in the above table
/// without excessive line length, and are thus presented here:
///
/// * Function `place_limit_order_passive_advance()` has return proxy
///   `place_limit_order_passive_advance_user()`.
/// * Function `place_limit_order_passive_advance_user()` has no return
///   proxy.
/// * Function `place_limit_order_passive_advance_custodian()` has no
///   return proxy.
///
/// Function returns to test:
///
/// * [x] `place_limit_order_custodian()`
/// * [x] `place_limit_order_passive_advance_custodian()`
/// * [x] `place_limit_order_passive_advance_user()`
/// * [x] `place_limit_order_user()`
/// * [x] `place_market_order_custodian()`
/// * [x] `place_market_order_user()`
/// * [x] `swap_between_coinstores()`
/// * [x] `swap_coins()`
/// * [x] `swap_generic()`
///
/// ## Invocation proxies
///
/// Similarly, verifying the invocation of some functions verifies the
/// invocation of associated inner functions. For example,
/// `cancel_all_orders_user()` can be considered an invocation proxy
/// of `cancel_all_orders()` and of `cancel_order()`. Here, to provide
/// 100% invocation coverage, only functions at the top of the
/// dependency stack must be verified.
///
/// Function invocations to test:
///
/// * [x] `cancel_all_orders_custodian()`
/// * [x] `cancel_all_orders_user()`
/// * [x] `cancel_order_custodian()`
/// * [x] `cancel_order_user()`
/// * [x] `change_order_size_custodian()`
/// * [x] `change_order_size_user()`
/// * [x] `place_limit_order_user_entry()`
/// * [x] `place_limit_order_custodian()`
/// * [x] `place_limit_order_passive_advance_custodian()`
/// * [x] `place_limit_order_passive_advance_user_entry()`
/// * [x] `place_market_order_user_entry()`
/// * [x] `place_market_order_custodian()`
/// * [x] `swap_between_coinstores_entry()`
/// * [x] `swap_coins()`
/// * [x] `swap_generic()`
///
/// ## Branching functions
///
/// Functions with logical branches to test:
///
/// * [x] `cancel_all_orders()`
/// * [x] `cancel_order()`
/// * [x] `change_order_size()`
/// * [x] `match()`
/// * [x] `place_limit_order()`
/// * [x] `place_limit_order_passive_advance()`
/// * [x] `place_market_order()`
/// * [x] `range_check_trade()`
/// * [x] `swap_between_coinstores()`
/// * [x] `swap_coins()`
/// * [x] `swap_generic()`
/// * [x] `swap()`
///
/// See each function for its logical branches.
///
/// # Complete DocGen index
///
/// The below index is automatically generated from source code:
module econia::market {

    // Uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::guid;
    use aptos_framework::table::{Self, Table};
    use aptos_framework::type_info::{Self, TypeInfo};
    use econia::avl_queue::{Self, AVLqueue};
    use econia::incentives;
    use econia::registry::{
        Self, CustodianCapability, GenericAsset, UnderwriterCapability};
    use econia::resource_account;
    use econia::tablist::{Self, Tablist};
    use econia::user::{Self, CancelOrderEvent, FillEvent};
    use std::option::{Self, Option};
    use std::signer::address_of;
    use std::string::{Self, String};
    use std::vector;

    // Uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    use econia::assets::{Self, BC, QC, UC};

    // Test-only uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Structs >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// View function return for getting event handle creation info of a
    /// particular `MarketEventHandlesForMarket`.
    struct MarketEventHandleCreationInfo has copy, drop {
        /// Econia resource account address, corresponding to event
        /// handle creator address.
        resource_account_address: address,
        /// Creation number of `cancel_order_events` handle in a
        /// `MarketEventHandlesForMarket`.
        cancel_order_events_handle_creation_num: u64,
        /// Creation number of `place_swap_order_events` handle in a
        /// `MarketEventHandlesForMarket`.
        place_swap_order_events_handle_creation_num: u64
    }

    /// All of the Econia resource account's
    /// `MarketEventHandlesForMarket`.
    struct MarketEventHandles has key {
        /// Map from market ID to `MarketEventHandlesForMarket`.
        map: Table<u64, MarketEventHandlesForMarket>
    }

    /// Within a given market, event handles for market events that are
    /// not emitted elsewhere when associated with a swap order placed
    /// by a non-signing swapper.
    struct MarketEventHandlesForMarket has store {
        /// Event handle for `user::CancelOrderEvent`s.
        cancel_order_events: EventHandle<CancelOrderEvent>,
        /// Event handle for `PlaceSwapOrderEvent`s.
        place_swap_order_events: EventHandle<PlaceSwapOrderEvent>
    }

    /// An order on the order book.
    struct Order has store {
        /// Number of lots to be filled.
        size: u64,
        /// Order price, in ticks per lot.
        price: u64,
        /// Address of user holding order.
        user: address,
        /// For given user, ID of custodian required to approve order
        /// operations and withdrawals on given market account.
        custodian_id: u64,
        /// User-side access key for storage-optimized lookup.
        order_access_key: u64
    }

    /// An order book for a given market. Contains
    /// `registry::MarketInfo` field duplicates to reduce global storage
    /// item queries against the registry.
    struct OrderBook has store {
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
        /// Asks AVL queue.
        asks: AVLqueue<Order>,
        /// Bids AVL queue.
        bids: AVLqueue<Order>,
        /// Cumulative number of orders placed.
        counter: u64,
        /// Deprecated field retained for compatible upgrade policy.
        maker_events: EventHandle<MakerEvent>,
        /// Deprecated field retained for compatible upgrade policy.
        taker_events: EventHandle<TakerEvent>
    }

    /// Order book map for all Econia order books.
    struct OrderBooks has key {
        /// Map from market ID to corresponding order book. Enables
        /// off-chain iterated indexing by market ID.
        map: Tablist<u64, OrderBook>
    }

    /// User-friendly representation of an open order on the order book.
    struct OrderView has copy, drop {
        /// Market ID for open order.
        market_id: u64,
        /// `ASK` or `BID`.
        side: bool,
        /// The order ID for the posted order.
        order_id: u128,
        /// Remaining number of lots to be filled.
        remaining_size: u64,
        /// Order price, in ticks per lot.
        price: u64,
        /// Address of user holding order.
        user: address,
        /// For given user, ID of custodian required to approve order
        /// operations and withdrawals on given market account.
        custodian_id: u64
    }

    /// `OrderView` instances from an `OrderBook`, indexed by side and
    /// sorted by price-time priority.
    struct OrdersView has copy, drop {
        /// Asks sorted by price-time priority: oldest order at lowest
        /// price first in vector.
        asks: vector<OrderView>,
        /// Bids sorted by price-time priority: oldest order at highest
        /// price first in vector.
        bids: vector<OrderView>
    }

    /// Emitted when a swap order is placed.
    struct PlaceSwapOrderEvent has copy, drop, store {
        /// Market ID for order.
        market_id: u64,
        /// Signing account if swap is placed by a signing swapper, else
        /// `NO_TAKER_ADDRESS`.
        signing_account: address,
        /// Integrator address passed during swap order placement,
        /// eligible for a portion of any generated taker fees.
        integrator: address,
        /// Either `BUY` or `SELL`.
        direction: bool,
        /// Indicated minimum base subunits to trade.
        min_base: u64,
        /// Indicated maximum base subunits to trade.
        max_base: u64,
        /// Indicated minimum quote subunits to trade.
        min_quote: u64,
        /// Indicated maximum quote subunits to trade.
        max_quote: u64,
        /// Indicated limit price.
        limit_price: u64,
        /// Unique ID for order within market.
        order_id: u128
    }

    /// A price level from an `OrderBook`.
    struct PriceLevel has copy, drop {
        /// Price, in ticks per lot.
        price: u64,
        /// Cumulative size of open orders at price level, in lots.
        size: u128
    }

    /// `PriceLevel` instances from an `OrderBook`, indexed by side and
    /// sorted by price-time priority.
    struct PriceLevels has copy, drop {
        /// Market ID of corresponding market.
        market_id: u64,
        /// Ask price levels sorted by price-time priority: lowest price
        /// level first in vector.
        asks: vector<PriceLevel>,
        /// Ask price levels sorted by price-time priority: highest
        /// price level first in vector.
        bids: vector<PriceLevel>
    }

    /// View function return for getting event handle creation numbers
    /// for a signing swapper's `SwapperEventHandlesForMarket`.
    struct SwapperEventHandleCreationNumbers has copy, drop {
        /// Creation number of `cancel_order_events` handle in a
        /// `SwapperEventHandlesForMarket`.
        cancel_order_events_handle_creation_num: u64,
        /// Creation number of `fill_events` handle in a
        /// `SwapperEventHandlesForMarket`.
        fill_events_handle_creation_num: u64,
        /// Creation number of `place_swap_order_events` handle in a
        /// `SwapperEventHandlesForMarket`.
        place_swap_order_events_handle_creation_num: u64
    }

    /// All of a signing swapper's `SwapperEventHandlesForMarket`.
    struct SwapperEventHandles has key {
        /// Map from market ID to `SwapperEventHandlesForMarket`.
        map: Table<u64, SwapperEventHandlesForMarket>
    }

    /// Event handles for market events associated with a signing
    /// swapper on a particular market. Stored under a signing swapper's
    /// account (not market account), since swaps are processed outside
    /// of an Econia-style market account.
    struct SwapperEventHandlesForMarket has store {
        /// Event handle for `user::CancelOrderEvent`s.
        cancel_order_events: EventHandle<CancelOrderEvent>,
        /// Event handle for `user::FillEvent`s.
        fill_events: EventHandle<FillEvent>,
        /// Event handle for `PlaceSwapOrderEvent`s.
        place_swap_order_events: EventHandle<PlaceSwapOrderEvent>
    }

    // Structs <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Error codes >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Maximum base trade amount specified as 0.
    const E_MAX_BASE_0: u64 = 0;
    /// Maximum quote trade amount specified as 0.
    const E_MAX_QUOTE_0: u64 = 1;
    /// Minimum base trade amount exceeds maximum base trade amount.
    const E_MIN_BASE_EXCEEDS_MAX: u64 = 2;
    /// Minimum quote trade amount exceeds maximum quote trade amount.
    const E_MIN_QUOTE_EXCEEDS_MAX: u64 = 3;
    /// Filling order would overflow asset received from trade.
    const E_OVERFLOW_ASSET_IN: u64 = 4;
    /// Not enough asset to trade away.
    const E_NOT_ENOUGH_ASSET_OUT: u64 = 5;
    /// No market with given ID.
    const E_INVALID_MARKET_ID: u64 = 6;
    /// Base asset type is invalid.
    const E_INVALID_BASE: u64 = 7;
    /// Quote asset type is invalid.
    const E_INVALID_QUOTE: u64 = 8;
    /// Minimum base asset trade amount requirement not met.
    const E_MIN_BASE_NOT_TRADED: u64 = 9;
    /// Minimum quote coin trade amount requirement not met.
    const E_MIN_QUOTE_NOT_TRADED: u64 = 10;
    /// Order price specified as 0.
    const E_PRICE_0: u64 = 11;
    /// Order price exceeds maximum allowable price.
    const E_PRICE_TOO_HIGH: u64 = 12;
    /// Post-or-abort limit order price crosses spread.
    const E_POST_OR_ABORT_CROSSES_SPREAD: u64 = 13;
    /// Order size does not meet minimum size for market.
    const E_SIZE_TOO_SMALL: u64 = 14;
    /// Limit order size results in base asset amount overflow.
    const E_SIZE_BASE_OVERFLOW: u64 = 15;
    /// Limit order size and price results in ticks amount overflow.
    const E_SIZE_PRICE_TICKS_OVERFLOW: u64 = 16;
    /// Limit order size and price results in quote amount overflow.
    const E_SIZE_PRICE_QUOTE_OVERFLOW: u64 = 17;
    /// Invalid restriction flag.
    const E_INVALID_RESTRICTION: u64 = 18;
    /// A self match occurs when self match behavior is abort.
    const E_SELF_MATCH: u64 = 19;
    /// No room to insert order with such low price-time priority.
    const E_PRICE_TIME_PRIORITY_TOO_LOW: u64 = 20;
    /// Underwriter invalid for given market.
    const E_INVALID_UNDERWRITER: u64 = 21;
    /// Market order ID invalid.
    const E_INVALID_MARKET_ORDER_ID: u64 = 22;
    /// Custodian not authorized for operation.
    const E_INVALID_CUSTODIAN: u64 = 23;
    /// Invalid user indicated for operation.
    const E_INVALID_USER: u64 = 24;
    /// Fill-or-abort price does not cross the spread.
    const E_FILL_OR_ABORT_NOT_CROSS_SPREAD: u64 = 25;
    /// AVL queue head price does not match head order price.
    const E_HEAD_KEY_PRICE_MISMATCH: u64 = 26;
    /// Simulation query called by invalid account.
    const E_NOT_SIMULATION_ACCOUNT: u64 = 27;
    /// Invalid self match behavior flag.
    const E_INVALID_SELF_MATCH_BEHAVIOR: u64 = 28;
    /// Passive advance percent is not less than or equal to 100.
    const E_INVALID_PERCENT: u64 = 29;
    /// Order size change requiring insertion resulted in an AVL queue
    /// access key mismatch.
    const E_SIZE_CHANGE_INSERTION_ERROR: u64 = 30;
    /// Order ID corresponds to an order that did not post.
    const E_ORDER_DID_NOT_POST: u64 = 31;
    /// Order price field does not match AVL queue insertion key price.
    const E_ORDER_PRICE_MISMATCH: u64 = 32;
    /// New order size is less than the minimum order size for market.
    const E_SIZE_CHANGE_BELOW_MIN_SIZE: u64 = 33;

    // Error codes <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Constants >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Ascending AVL queue flag, for asks AVL queue.
    const ASCENDING: bool = true;
    /// Flag to abort during a self match.
    const ABORT: u8 = 0;
    /// Flag for ask side.
    const ASK: bool = true;
    /// Flag for bid side.
    const BID: bool = false;
    /// Flag for buy direction.
    const BUY: bool = false;
    /// Flag to cancel maker and taker order during a self match.
    const CANCEL_BOTH: u8 = 1;
    /// Flag to cancel maker order only during a self match.
    const CANCEL_MAKER: u8 = 2;
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
    /// Swap order cancelled because the remaining base asset amount to
    /// match was too small to fill a single lot.
    const CANCEL_REASON_TOO_SMALL_TO_FILL_LOT: u8 = 8;
    /// Swap order cancelled because the next order on the book to match
    /// against violated the swap order limit price.
    const CANCEL_REASON_VIOLATED_LIMIT_PRICE: u8 = 9;
    /// Flag to cancel taker order only during a self match.
    const CANCEL_TAKER: u8 = 3;
    /// Critical tree height above which evictions may take place.
    const CRITICAL_HEIGHT: u8 = 18;
    /// Descending AVL queue flag, for bids AVL queue.
    const DESCENDING: bool = false;
    /// Flag for fill-or-abort order restriction.
    const FILL_OR_ABORT: u8 = 1;
    /// `u64` bitmask with all bits set, generated in Python via
    /// `hex(int('1' * 64, 2))`.
    const HI_64: u64 = 0xffffffffffffffff;
    /// All bits set in integer of width required to encode price.
    /// Generated in Python via `hex(int('1' * 32, 2))`.
    const HI_PRICE: u64 = 0xffffffff;
    /// Flag for immediate-or-cancel order restriction.
    const IMMEDIATE_OR_CANCEL: u8 = 2;
    /// Flag to trade max possible asset amount: `u64` bitmask with all
    /// bits set, generated in Python via `hex(int('1' * 64, 2))`.
    const MAX_POSSIBLE: u64 = 0xffffffffffffffff;
    /// Number of restriction flags.
    const N_RESTRICTIONS: u8 = 3;
    /// Flag for null value when null defined as 0.
    const NIL: u64 = 0;
    /// Custodian ID flag for no custodian.
    const NO_CUSTODIAN: u64 = 0;
    /// Flag for no order restriction.
    const NO_RESTRICTION: u8 = 0;
    /// Taker address flag for when taker order does not originate from
    /// a market account or a signing swapper.
    const NO_TAKER_ADDRESS: address = @0x0;
    /// Underwriter ID flag for no underwriter.
    const NO_UNDERWRITER: u64 = 0;
    /// Flag for passive order specified by percent advance.
    const PERCENT: bool = true;
    /// Maximum percentage passive advance.
    const PERCENT_100: u64 = 100;
    /// Flag for post-or-abort order restriction.
    const POST_OR_ABORT: u8 = 3;
    /// Flag for sell direction.
    const SELL: bool = true;
    /// Number of bits order counter is shifted in an order ID.
    const SHIFT_COUNTER: u8 = 64;
    /// Flag for passive order specified by advance in ticks.
    const TICKS: bool = false;

    // Constants <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // View functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>


    #[view]
    /// Public constant getter for `ABORT`.
    ///
    /// # Testing
    ///
    /// * `test_get_ABORT()`
    public fun get_ABORT(): u8 {ABORT}

    #[view]
    /// Public constant getter for `ASK`.
    ///
    /// # Testing
    ///
    /// * `test_direction_side_polarities()`
    /// * `test_get_ASK()`
    public fun get_ASK(): bool {ASK}

    #[view]
    /// Public constant getter for `BID`.
    ///
    /// # Testing
    ///
    /// * `test_direction_side_polarities()`
    /// * `test_get_BID()`
    public fun get_BID(): bool {BID}

    #[view]
    /// Public constant getter for `BUY`.
    ///
    /// # Testing
    ///
    /// * `test_direction_side_polarities()`
    /// * `test_get_BUY()`
    public fun get_BUY(): bool {BUY}

    #[view]
    /// Public constant getter for `CANCEL_BOTH`.
    ///
    /// # Testing
    ///
    /// * `test_get_CANCEL_BOTH()`
    public fun get_CANCEL_BOTH(): u8 {CANCEL_BOTH}

    #[view]
    /// Public constant getter for `CANCEL_MAKER`.
    ///
    /// # Testing
    ///
    /// * `test_get_CANCEL_MAKER()`
    public fun get_CANCEL_MAKER(): u8 {CANCEL_MAKER}

    #[view]
    /// Public constant getter for `CANCEL_TAKER`.
    ///
    /// # Testing
    ///
    /// * `test_get_CANCEL_TAKER()`
    public fun get_CANCEL_TAKER(): u8 {CANCEL_TAKER}

    #[view]
    /// Public constant getter for `FILL_OR_ABORT`.
    ///
    /// # Testing
    ///
    /// * `test_get_FILL_OR_ABORT()`
    public fun get_FILL_OR_ABORT(): u8 {FILL_OR_ABORT}

    #[view]
    /// Public constant getter for `HI_PRICE`.
    ///
    /// # Testing
    ///
    /// * `test_get_HI_PRICE()`
    public fun get_HI_PRICE(): u64 {HI_PRICE}

    #[view]
    /// Public constant getter for `IMMEDIATE_OR_CANCEL`.
    ///
    /// # Testing
    ///
    /// * `test_get_IMMEDIATE_OR_CANCEL()`
    public fun get_IMMEDIATE_OR_CANCEL(): u8 {IMMEDIATE_OR_CANCEL}

    #[view]
    /// Public constant getter for `MAX_POSSIBLE`.
    ///
    /// # Testing
    ///
    /// * `test_get_MAX_POSSIBLE()`
    public fun get_MAX_POSSIBLE(): u64 {MAX_POSSIBLE}

    #[view]
    /// Public constant getter for `NO_CUSTODIAN`.
    ///
    /// # Testing
    ///
    /// * `test_get_NO_CUSTODIAN()`
    public fun get_NO_CUSTODIAN(): u64 {NO_CUSTODIAN}

    #[view]
    /// Public constant getter for `NO_RESTRICTION`.
    ///
    /// # Testing
    ///
    /// * `test_get_NO_RESTRICTION()`
    public fun get_NO_RESTRICTION(): u8 {NO_RESTRICTION}

    #[view]
    /// Public constant getter for `NO_UNDERWRITER`.
    ///
    /// # Testing
    ///
    /// * `test_get_NO_UNDERWRITER()`
    public fun get_NO_UNDERWRITER(): u64 {NO_UNDERWRITER}

    #[view]
    /// Public constant getter for `POST_OR_ABORT`.
    ///
    /// # Testing
    ///
    /// * `test_get_POST_OR_ABORT()`
    public fun get_POST_OR_ABORT(): u8 {POST_OR_ABORT}

    #[view]
    /// Public constant getter for `PERCENT`.
    ///
    /// # Testing
    ///
    /// * `test_get_PERCENT()`
    public fun get_PERCENT(): bool {PERCENT}

    #[view]
    /// Public constant getter for `SELL`.
    ///
    /// # Testing
    ///
    /// * `test_direction_side_polarities()`
    /// * `test_get_SELL()`
    public fun get_SELL(): bool {SELL}

    #[view]
    /// Public constant getter for `TICKS`.
    ///
    /// # Testing
    ///
    /// * `test_get_TICKS()`
    public fun get_TICKS(): bool {TICKS}

    #[view]
    /// Return a `MarketEventHandleCreationInfo` for `market_id`, if
    /// Econia resource account has event handles for indicated market.
    ///
    /// Restricted to private view function to prevent runtime handle
    /// contention.
    ///
    /// # Testing
    ///
    /// * `test_swap_between_coinstores_register_base_store()`
    fun get_market_event_handle_creation_info(
        market_id: u64
    ): Option<MarketEventHandleCreationInfo>
    acquires MarketEventHandles {
        // Return none if Econia resource account does not have market
        // event handles map.
        let resource_account_address = resource_account::get_address();
        if (!exists<MarketEventHandles>(resource_account_address))
            return option::none();
        // Return none if no handles exist for market.
        let market_event_handles_map_ref =
            &borrow_global<MarketEventHandles>(resource_account_address).map;
        let has_handles = table::contains(
            market_event_handles_map_ref, market_id);
        if (!has_handles) return option::none();
        let market_handles_ref = table::borrow(
            market_event_handles_map_ref, market_id);
        // Return option-packed creation info for market.
        option::some(MarketEventHandleCreationInfo{
            resource_account_address: resource_account_address,
            cancel_order_events_handle_creation_num:
                guid::creation_num(event::guid(
                    &market_handles_ref.cancel_order_events)),
            place_swap_order_events_handle_creation_num:
                guid::creation_num(event::guid(
                    &market_handles_ref.place_swap_order_events))
        })
    }

    // View functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Public function wrapper for `cancel_all_orders()` for cancelling
    /// orders under authority of delegated custodian.
    ///
    /// # Invocation testing
    ///
    /// * `test_cancel_all_orders_ask_custodian()`
    public fun cancel_all_orders_custodian(
        user_address: address,
        market_id: u64,
        side: bool,
        custodian_capability_ref: &CustodianCapability
    ) acquires OrderBooks {
        cancel_all_orders(
            user_address,
            market_id,
            registry::get_custodian_id(custodian_capability_ref),
            side);
    }

    /// Public function wrapper for `cancel_order()` for cancelling
    /// order under authority of delegated custodian.
    ///
    /// # Invocation testing
    ///
    /// * `test_cancel_order_ask_custodian()`
    public fun cancel_order_custodian(
        user_address: address,
        market_id: u64,
        side: bool,
        market_order_id: u128,
        custodian_capability_ref: &CustodianCapability
    ) acquires OrderBooks {
        cancel_order(
            user_address,
            market_id,
            registry::get_custodian_id(custodian_capability_ref),
            side,
            market_order_id);
    }

    /// Public function wrapper for `change_order_size()` for changing
    /// order size under authority of delegated custodian.
    ///
    /// # Invocation testing
    ///
    /// * `test_change_order_size_ask_custodian()`
    public fun change_order_size_custodian(
        user_address: address,
        market_id: u64,
        side: bool,
        market_order_id: u128,
        new_size: u64,
        custodian_capability_ref: &CustodianCapability
    ) acquires OrderBooks {
        change_order_size(
            user_address,
            market_id,
            registry::get_custodian_id(custodian_capability_ref),
            side,
            market_order_id,
            new_size);
    }

    /// Public function wrapper for `place_limit_order()` for placing
    /// order under authority of delegated custodian.
    ///
    /// # Invocation and return testing
    ///
    /// * `test_place_limit_order_no_cross_bid_custodian()`
    public fun place_limit_order_custodian<
        BaseType,
        QuoteType
    >(
        user_address: address,
        market_id: u64,
        integrator: address,
        side: bool,
        size: u64,
        price: u64,
        restriction: u8,
        self_match_behavior: u8,
        custodian_capability_ref: &CustodianCapability
    ): (
        u128,
        u64,
        u64,
        u64
    ) acquires OrderBooks {
        place_limit_order<
            BaseType,
            QuoteType
        >(
            user_address,
            market_id,
            registry::get_custodian_id(custodian_capability_ref),
            integrator,
            side,
            size,
            price,
            restriction,
            self_match_behavior,
            CRITICAL_HEIGHT)
    }

    /// Public function wrapper for
    /// `place_limit_order_passive_advance()` for placing order under
    /// authority of delegated custodian.
    ///
    /// # Invocation and return testing
    ///
    /// * `test_place_limit_order_passive_advance_ticks_bid()`
    public fun place_limit_order_passive_advance_custodian<
        BaseType,
        QuoteType
    >(
        user_address: address,
        market_id: u64,
        integrator: address,
        side: bool,
        size: u64,
        advance_style: bool,
        target_advance_amount: u64,
        custodian_capability_ref: &CustodianCapability
    ): u128
    acquires OrderBooks {
        place_limit_order_passive_advance<
            BaseType,
            QuoteType
        >(
            user_address,
            market_id,
            registry::get_custodian_id(custodian_capability_ref),
            integrator,
            side,
            size,
            advance_style,
            target_advance_amount)
    }

    /// Public function wrapper for
    /// `place_limit_order_passive_advance()` for placing order under
    /// authority of signing user.
    ///
    /// # Invocation and return testing
    ///
    /// * `test_place_limit_order_passive_advance_no_cross_price_ask()`
    /// * `test_place_limit_order_passive_advance_no_cross_price_bid()`
    /// * `test_place_limit_order_passive_advance_no_full_advance()`
    /// * `test_place_limit_order_passive_advance_no_start_price()`.
    /// * `test_place_limit_order_passive_advance_no_target_advance()`
    /// * `test_place_limit_order_passive_advance_percent_ask()`
    /// * `test_place_limit_order_passive_advance_percent_bid()`
    /// * `test_place_limit_order_passive_advance_ticks_ask()`
    public fun place_limit_order_passive_advance_user<
        BaseType,
        QuoteType
    >(
        user: &signer,
        market_id: u64,
        integrator: address,
        side: bool,
        size: u64,
        advance_style: bool,
        target_advance_amount: u64
    ): u128
    acquires OrderBooks {
        place_limit_order_passive_advance<
            BaseType,
            QuoteType
        >(
            address_of(user),
            market_id,
            NO_CUSTODIAN,
            integrator,
            side,
            size,
            advance_style,
            target_advance_amount)
    }

    /// Public function wrapper for `place_limit_order()` for placing
    /// order under authority of signing user.
    ///
    /// # Invocation and return testing
    ///
    /// * `test_place_limit_order_crosses_ask_exact()`
    /// * `test_place_limit_order_crosses_ask_partial()`
    /// * `test_place_limit_order_crosses_ask_partial_cancel()`
    /// * `test_place_limit_order_crosses_ask_self_match_cancel()`
    /// * `test_place_limit_order_crosses_bid_exact()`
    /// * `test_place_limit_order_crosses_bid_partial()`
    /// * `test_place_limit_order_evict()`
    /// * `test_place_limit_order_no_cross_ask_user()`
    public fun place_limit_order_user<
        BaseType,
        QuoteType
    >(
        user: &signer,
        market_id: u64,
        integrator: address,
        side: bool,
        size: u64,
        price: u64,
        restriction: u8,
        self_match_behavior: u8
    ): (
        u128,
        u64,
        u64,
        u64
    ) acquires OrderBooks {
        place_limit_order<
            BaseType,
            QuoteType
        >(
            address_of(user),
            market_id,
            NO_CUSTODIAN,
            integrator,
            side,
            size,
            price,
            restriction,
            self_match_behavior,
            CRITICAL_HEIGHT)
    }

    /// Public function wrapper for `place_market_order()` for placing
    /// order under authority of delegated custodian.
    ///
    /// # Invocation and return testing
    ///
    /// * `test_place_market_order_max_base_sell_custodian()`
    /// * `test_place_market_order_max_quote_buy_custodian()`
    public fun place_market_order_custodian<
        BaseType,
        QuoteType
    >(
        user_address: address,
        market_id: u64,
        integrator: address,
        direction: bool,
        size: u64,
        self_match_behavior: u8,
        custodian_capability_ref: &CustodianCapability
    ): (
        u64,
        u64,
        u64
    ) acquires OrderBooks {
        place_market_order<BaseType, QuoteType>(
            user_address,
            market_id,
            registry::get_custodian_id(custodian_capability_ref),
            integrator,
            direction,
            size,
            self_match_behavior)
    }

    /// Public function wrapper for `place_market_order()` for placing
    /// order under authority of signing user.
    ///
    /// # Invocation and return testing
    ///
    /// * `test_place_market_order_max_base_buy_user()`
    /// * `test_place_market_order_max_quote_sell_user()`
    public fun place_market_order_user<
        BaseType,
        QuoteType
    >(
        user: &signer,
        market_id: u64,
        integrator: address,
        direction: bool,
        size: u64,
        self_match_behavior: u8
    ): (
        u64,
        u64,
        u64
    ) acquires OrderBooks {
        place_market_order<BaseType, QuoteType>(
            address_of(user),
            market_id,
            NO_CUSTODIAN,
            integrator,
            direction,
            size,
            self_match_behavior)
    }

    /// Register pure coin market, return resultant market ID.
    ///
    /// See inner function `register_market()`.
    ///
    /// # Type parameters
    ///
    /// * `BaseType`: Base coin type for market.
    /// * `QuoteType`: Quote coin type for market.
    /// * `UtilityType`: Utility coin type, specified at
    ///   `incentives::IncentiveParameters.utility_coin_type_info`.
    ///
    /// # Parameters
    ///
    /// * `lot_size`: `registry::MarketInfo.lot_size` for market.
    /// * `tick_size`: `registry::MarketInfo.tick_size` for market.
    /// * `min_size`: `registry::MarketInfo.min_size` for market.
    /// * `utility_coins`: Utility coins paid to register a market. See
    ///   `incentives::IncentiveParameters.market_registration_fee`.
    ///
    /// # Returns
    ///
    /// * `u64`: Market ID for new market.
    ///
    /// # Testing
    ///
    /// * `test_register_markets()`
    public fun register_market_base_coin<
        BaseType,
        QuoteType,
        UtilityType
    >(
        lot_size: u64,
        tick_size: u64,
        min_size: u64,
        utility_coins: Coin<UtilityType>
    ): u64
    acquires OrderBooks {
        // Register market in global registry, storing market ID.
        let market_id = registry::register_market_base_coin_internal<
            BaseType, QuoteType, UtilityType>(lot_size, tick_size, min_size,
            utility_coins);
        // Register order book and quote coin fee store, return market
        // ID.
        register_market<BaseType, QuoteType>(
            market_id, string::utf8(b""), lot_size, tick_size, min_size,
            NO_UNDERWRITER)
    }

    /// Register generic market, return resultant market ID.
    ///
    /// See inner function `register_market()`.
    ///
    /// Generic base name restrictions described at
    /// `registry::register_market_base_generic_internal()`.
    ///
    /// # Type parameters
    ///
    /// * `QuoteType`: Quote coin type for market.
    /// * `UtilityType`: Utility coin type, specified at
    ///   `incentives::IncentiveParameters.utility_coin_type_info`.
    ///
    /// # Parameters
    ///
    /// * `base_name_generic`: `registry::MarketInfo.base_name_generic`
    ///   for market.
    /// * `lot_size`: `registry::MarketInfo.lot_size` for market.
    /// * `tick_size`: `registry::MarketInfo.tick_size` for market.
    /// * `min_size`: `registry::MarketInfo.min_size` for market.
    /// * `utility_coins`: Utility coins paid to register a market. See
    ///   `incentives::IncentiveParameters.market_registration_fee`.
    /// * `underwriter_capability_ref`: Immutable reference to market
    ///   underwriter capability.
    ///
    /// # Returns
    ///
    /// * `u64`: Market ID for new market.
    ///
    /// # Testing
    ///
    /// * `test_register_markets()`
    public fun register_market_base_generic<
        QuoteType,
        UtilityType
    >(
        base_name_generic: String,
        lot_size: u64,
        tick_size: u64,
        min_size: u64,
        utility_coins: Coin<UtilityType>,
        underwriter_capability_ref: &UnderwriterCapability
    ): u64
    acquires OrderBooks {
        // Register market in global registry, storing market ID.
        let market_id = registry::register_market_base_generic_internal<
            QuoteType, UtilityType>(base_name_generic, lot_size, tick_size,
            min_size, underwriter_capability_ref, utility_coins);
        // Register order book and quote coin fee store, return market
        // ID.
        register_market<GenericAsset, QuoteType>(
            market_id, base_name_generic, lot_size, tick_size, min_size,
            registry::get_underwriter_id(underwriter_capability_ref))
    }

    /// Swap against the order book between a user's coin stores.
    ///
    /// Initializes an `aptos_framework::coin::CoinStore` for each coin
    /// type that does not yet have one.
    ///
    /// # Type Parameters
    ///
    /// * `BaseType`: Same as for `match()`.
    /// * `QuoteType`: Same as for `match()`.
    ///
    /// # Parameters
    ///
    /// * `user`: Account of swapping user.
    /// * `market_id`: Same as for `match()`.
    /// * `integrator`: Same as for `match()`.
    /// * `direction`: Same as for `match()`.
    /// * `min_base`: Same as for `match()`.
    /// * `max_base`: Same as for `match()`. If passed as `MAX_POSSIBLE`
    ///   will attempt to trade maximum possible amount for coin store.
    /// * `min_quote`: Same as for `match()`.
    /// * `max_quote`: Same as for `match()`. If passed as
    ///   `MAX_POSSIBLE` will attempt to trade maximum possible amount
    ///   for coin store.
    /// * `limit_price`: Same as for `match()`.
    ///
    /// # Returns
    ///
    /// * `u64`: Base asset trade amount, same as for `match()`.
    /// * `u64`: Quote coin trade amount, same as for `match()`.
    /// * `u64`: Quote coin fees paid, same as for `match()`.
    ///
    /// # Emits
    ///
    /// * `PlaceSwapOrderEvent`: Information about the swap order.
    /// * `user::FillEvent`(s): Information about fill(s) associated
    ///   with the swap.
    /// * `user::CancelOrderEvent`: Optionally, information about why
    ///   the swap was cancelled without completely filling.
    ///
    /// # Testing
    ///
    /// * `test_swap_between_coinstores_max_possible_base_buy()`
    /// * `test_swap_between_coinstores_max_possible_base_sell()`
    /// * `test_swap_between_coinstores_max_possible_quote_buy()`
    /// * `test_swap_between_coinstores_max_possible_quote_sell()`
    /// * `test_swap_between_coinstores_max_quote_traded()`
    /// * `test_swap_between_coinstores_not_enough_liquidity()`
    /// * `test_swap_between_coinstores_register_base_store()`
    /// * `test_swap_between_coinstores_register_quote_store()`
    /// * `test_swap_between_coinstores_self_match_taker_cancel()`
    public fun swap_between_coinstores<
        BaseType,
        QuoteType
    >(
        user: &signer,
        market_id: u64,
        integrator: address,
        direction: bool,
        min_base: u64,
        max_base: u64,
        min_quote: u64,
        max_quote: u64,
        limit_price: u64
    ): (
        u64,
        u64,
        u64
    ) acquires
        MarketEventHandles,
        OrderBooks,
        SwapperEventHandles
    {
        let user_address = address_of(user); // Get user address.
        // Register base coin store if user does not have one.
        if (!coin::is_account_registered<BaseType>(user_address))
            coin::register<BaseType>(user);
        // Register quote coin store if user does not have one.
        if (!coin::is_account_registered<QuoteType>(user_address))
            coin::register<QuoteType>(user);
        let (base_value, quote_value) = // Get coin value amounts.
            (coin::balance<BaseType>(user_address),
             coin::balance<QuoteType>(user_address));
        // If max base to trade flagged as max possible, update it:
        if (max_base == MAX_POSSIBLE) max_base = if (direction == BUY)
            // If a buy, max to trade is amount that can fit in
            // coin store, else is the amount in the coin store.
            (HI_64 - base_value) else base_value;
        // If max quote to trade flagged as max possible, update it:
        if (max_quote == MAX_POSSIBLE) max_quote = if (direction == BUY)
            // If a buy, max to trade is amount in coin store, else is
            // the amount that could fit in the coin store.
            quote_value else (HI_64 - quote_value);
        range_check_trade( // Range check trade amounts.
            direction, min_base, max_base, min_quote, max_quote,
            base_value, base_value, quote_value, quote_value);
        // Get option-wrapped base coins and quote coins for matching:
        let (optional_base_coins, quote_coins) = if (direction == BUY)
            // If a buy, need no base but need max quote.
            (option::some(coin::zero<BaseType>()),
             coin::withdraw<QuoteType>(user, max_quote)) else
            // If a sell, need max base but not quote.
            (option::some(coin::withdraw<BaseType>(user, max_base)),
             coin::zero<QuoteType>());
        // Swap against the order book, deferring market events.
        let fill_event_queue = vector[];
        let (
            optional_base_coins,
            quote_coins,
            base_traded,
            quote_traded,
            fees,
            place_swap_order_event_option,
            cancel_order_event_option
        ) = swap(
            &mut fill_event_queue,
            user_address,
            market_id,
            NO_UNDERWRITER,
            integrator,
            direction,
            min_base,
            max_base,
            min_quote,
            max_quote,
            limit_price,
            optional_base_coins,
            quote_coins
        );
        // Create swapper event handles for market as needed.
        if (!exists<SwapperEventHandles>(user_address))
            move_to(user, SwapperEventHandles{map: table::new()});
        let swapper_event_handles_map_ref_mut =
            &mut borrow_global_mut<SwapperEventHandles>(user_address).map;
        let has_handles =
            table::contains(swapper_event_handles_map_ref_mut, market_id);
        if (!has_handles) {
            let handles = SwapperEventHandlesForMarket{
                cancel_order_events: account::new_event_handle(user),
                fill_events: account::new_event_handle(user),
                place_swap_order_events: account::new_event_handle(user)
            };
            table::add(
                swapper_event_handles_map_ref_mut, market_id, handles);
        };
        let handles_ref_mut =
            table::borrow_mut(swapper_event_handles_map_ref_mut, market_id);
        // Emit place swap order event.
        event::emit_event(&mut handles_ref_mut.place_swap_order_events,
                          option::destroy_some(place_swap_order_event_option));
        // Emit fill events first-in-first-out.
        vector::for_each_ref(&fill_event_queue, |fill_event_ref| {
            let fill_event: FillEvent = *fill_event_ref;
            event::emit_event(&mut handles_ref_mut.fill_events, fill_event);
        });
        // Optionally emit cancel event.
        if (option::is_some(&cancel_order_event_option))
            event::emit_event(&mut handles_ref_mut.cancel_order_events,
                              option::destroy_some(cancel_order_event_option));
        // Deposit base coins back to user's coin store.
        coin::deposit(user_address, option::destroy_some(optional_base_coins));
        // Deposit quote coins back to user's coin store.
        coin::deposit(user_address, quote_coins);
        (base_traded, quote_traded, fees) // Return match results.
    }

    /// Swap standalone coins against the order book.
    ///
    /// If a buy, attempts to spend all quote coins. If a sell, attempts
    /// to sell all base coins.
    ///
    /// Passes all base coins to matching engine if a buy or a sell, and
    /// passes all quote coins to matching engine if a buy. If a sell,
    /// does not pass any quote coins to matching engine, to avoid
    /// intermediate quote match overflow that could occur prior to fee
    /// assessment.
    ///
    /// # Type Parameters
    ///
    /// * `BaseType`: Same as for `match()`.
    /// * `QuoteType`: Same as for `match()`.
    ///
    /// # Parameters
    ///
    /// * `market_id`: Same as for `match()`.
    /// * `integrator`: Same as for `match()`.
    /// * `direction`: Same as for `match()`.
    /// * `min_base`: Same as for `match()`.
    /// * `max_base`: Same as for `match()`. Ignored if a sell. Else if
    ///   passed as `MAX_POSSIBLE` will attempt to trade maximum
    ///   possible amount for passed coin holdings.
    /// * `min_quote`: Same as for `match()`.
    /// * `max_quote`: Same as for `match()`. Ignored if a buy. Else if
    ///   passed as `MAX_POSSIBLE` will attempt to trade maximum
    ///   possible amount for passed coin holdings.
    /// * `limit_price`: Same as for `match()`.
    /// * `base_coins`: Same as `optional_base_coins` for `match()`, but
    ///   unpacked.
    /// * `quote_coins`: Same as for `match()`.
    ///
    /// # Returns
    ///
    /// * `Coin<BaseType>`: Updated base coin holdings, same as for
    ///   `match()` but unpacked.
    /// * `Coin<QuoteType>`: Updated quote coin holdings, same as for
    ///   `match()`.
    /// * `u64`: Base coin trade amount, same as for `match()`.
    /// * `u64`: Quote coin trade amount, same as for `match()`.
    /// * `u64`: Quote coin fees paid, same as for `match()`.
    ///
    /// # Terminology
    ///
    /// * The "inbound" asset is the asset received from a trade: base
    ///   coins in the case of a buy, quote coins in the case of a sell.
    /// * The "outbound" asset is the asset traded away: quote coins in
    ///   the case of a buy, base coins in the case of a sell.
    ///
    /// # Testing
    ///
    /// * `test_swap_coins_buy_max_base_limiting()`
    /// * `test_swap_coins_buy_no_max_base_limiting()`
    /// * `test_swap_coins_buy_no_max_quote_limiting()`
    /// * `test_swap_coins_sell_max_quote_limiting()`
    /// * `test_swap_coins_sell_no_max_base_limiting()`
    /// * `test_swap_coins_sell_no_max_quote_limiting()`
    public fun swap_coins<
        BaseType,
        QuoteType
    >(
        market_id: u64,
        integrator: address,
        direction: bool,
        min_base: u64,
        max_base: u64,
        min_quote: u64,
        max_quote: u64,
        limit_price: u64,
        base_coins: Coin<BaseType>,
        quote_coins: Coin<QuoteType>
    ): (
        Coin<BaseType>,
        Coin<QuoteType>,
        u64,
        u64,
        u64
    ) acquires
        MarketEventHandles,
        OrderBooks
    {
        let (base_value, quote_value) = // Get coin value amounts.
            (coin::value(&base_coins), coin::value(&quote_coins));
        // Get option wrapped base coins.
        let optional_base_coins = option::some(base_coins);
        // Get quote coins to route through matching engine and update
        // max match amounts based on side. If a swap buy:
        let quote_coins_to_match = if (direction == BUY) {
            // Max quote to trade is amount passed in.
            max_quote = quote_value;
            // If max base amount to trade is max possible flag, update
            // to max amount that can be received.
            if (max_base == MAX_POSSIBLE) max_base = (HI_64 - base_value);
            // Pass all quote coins to matching engine.
            coin::extract(&mut quote_coins, max_quote)
        } else { // If a swap sell:
            // Max base to trade is amount passed in.
            max_base = base_value;
            // If max quote amount to trade is max possible flag, update
            // to max amount that can be received.
            if (max_quote == MAX_POSSIBLE) max_quote = (HI_64 - quote_value);
            // Do not pass any quote coins to matching engine.
            coin::zero()
        };
        range_check_trade( // Range check trade amounts.
            direction, min_base, max_base, min_quote, max_quote,
            base_value, base_value, quote_value, quote_value);
        // Swap against order book, discarding events.
        let (
            optional_base_coins,
            quote_coins_matched,
            base_traded,
            quote_traded,
            fees,
            _,
            _
        ) = swap(
            &mut vector[],
            NO_TAKER_ADDRESS,
            market_id,
            NO_UNDERWRITER,
            integrator,
            direction,
            min_base,
            max_base,
            min_quote,
            max_quote,
            limit_price,
            optional_base_coins,
            quote_coins_to_match
        );
        // Merge matched quote coins back into holdings.
        coin::merge(&mut quote_coins, quote_coins_matched);
        // Get base coins from option.
        let base_coins = option::destroy_some(optional_base_coins);
        // Return all coins.
        (base_coins, quote_coins, base_traded, quote_traded, fees)
    }

    /// Swap against the order book for a generic market, under
    /// authority of market underwriter.
    ///
    /// Passes all quote coins to matching engine if a buy. If a sell,
    /// does not pass any quote coins to matching engine, to avoid
    /// intermediate quote match overflow that could occur prior to fee
    /// assessment.
    ///
    /// # Type Parameters
    ///
    /// * `QuoteType`: Same as for `match()`.
    ///
    /// # Parameters
    ///
    /// * `market_id`: Same as for `match()`.
    /// * `integrator`: Same as for `match()`.
    /// * `direction`: Same as for `match()`.
    /// * `min_base`: Same as for `match()`.
    /// * `max_base`: Same as for `match()`.
    /// * `min_quote`: Same as for `match()`.
    /// * `max_quote`: Same as for `match()`. Ignored if a buy. Else if
    ///   passed as `MAX_POSSIBLE` will attempt to trade maximum
    ///   possible amount for passed coin holdings.
    /// * `limit_price`: Same as for `match()`.
    /// * `quote_coins`: Same as for `match()`.
    /// * `underwriter_capability_ref`: Immutable reference to
    ///   underwriter capability for given market.
    ///
    /// # Returns
    ///
    /// * `Coin<QuoteType>`: Updated quote coin holdings, same as for
    ///   `match()`.
    /// * `u64`: Base asset trade amount, same as for `match()`.
    /// * `u64`: Quote coin trade amount, same as for `match()`.
    /// * `u64`: Quote coin fees paid, same as for `match()`.
    ///
    /// # Testing
    ///
    /// * `test_swap_generic_buy_base_limiting()`
    /// * `test_swap_generic_buy_quote_limiting()`
    /// * `test_swap_generic_sell_max_quote_limiting()`
    /// * `test_swap_generic_sell_no_max_base_limiting()`
    /// * `test_swap_generic_sell_no_max_quote_limiting()`
    public fun swap_generic<
        QuoteType
    >(
        market_id: u64,
        integrator: address,
        direction: bool,
        min_base: u64,
        max_base: u64,
        min_quote: u64,
        max_quote: u64,
        limit_price: u64,
        quote_coins: Coin<QuoteType>,
        underwriter_capability_ref: &UnderwriterCapability
    ): (
        Coin<QuoteType>,
        u64,
        u64,
        u64
    ) acquires
        MarketEventHandles,
        OrderBooks
    {
        let underwriter_id = // Get underwriter ID.
            registry::get_underwriter_id(underwriter_capability_ref);
        // Get quote coin value.
        let quote_value = coin::value(&quote_coins);
        // Get base asset value holdings and quote coins to route
        // through matching engine, and update max match amounts based
        // on side. If a swap buy:
        let (base_value, quote_coins_to_match) = if (direction == BUY) {
            // Max quote to trade is amount passed in.
            max_quote = quote_value;
            // Do not pass in base asset, and pass all quote coins to
            // matching engine.
            (0, coin::extract(&mut quote_coins, max_quote))
        } else { // If a swap sell:
            // If max quote amount to trade is max possible flag, update
            // to max amount that can be received.
            if (max_quote == MAX_POSSIBLE) max_quote = (HI_64 - quote_value);
            // Effective base asset holdings are max trade amount, do
            // not pass and quote coins to matching engine.
            (max_base, coin::zero())
        };
        range_check_trade( // Range check trade amounts.
            direction, min_base, max_base, min_quote, max_quote,
            base_value, base_value, quote_value, quote_value);
        // Swap against order book, discarding events.
        let (
            optional_base_coins,
            quote_coins_matched,
            base_traded,
            quote_traded,
            fees,
            _,
            _
        ) = swap(
            &mut vector[],
            NO_TAKER_ADDRESS,
            market_id,
            underwriter_id,
            integrator,
            direction,
            min_base,
            max_base,
            min_quote,
            max_quote,
            limit_price,
            option::none(),
            quote_coins_to_match
        );
        // Destroy empty base coin option.
        option::destroy_none<Coin<GenericAsset>>(optional_base_coins);
        // Merge matched quote coins back into holdings.
        coin::merge(&mut quote_coins, quote_coins_matched);
        // Return quote coins, amount of base traded, amount of quote
        // traded, and quote fees paid.
        (quote_coins, base_traded, quote_traded, fees)
    }

    // Public functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public entry functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Public entry function wrapper for `cancel_all_orders()` for
    /// cancelling orders under authority of signing user.
    ///
    /// # Invocation testing
    ///
    /// * `test_cancel_all_orders_bid_user()`
    public entry fun cancel_all_orders_user(
        user: &signer,
        market_id: u64,
        side: bool,
    ) acquires OrderBooks {
        cancel_all_orders(
            address_of(user),
            market_id,
            NO_CUSTODIAN,
            side);
    }

    /// Public entry function wrapper for `cancel_order()` for
    /// cancelling order under authority of signing user.
    ///
    /// # Invocation testing
    ///
    /// * `test_cancel_order_bid_user()`
    public entry fun cancel_order_user(
        user: &signer,
        market_id: u64,
        side: bool,
        market_order_id: u128
    ) acquires OrderBooks {
        cancel_order(
            address_of(user),
            market_id,
            NO_CUSTODIAN,
            side,
            market_order_id);
    }

    /// Public entry function wrapper for `change_order_size()` for
    /// changing order size under authority of signing user.
    ///
    /// # Invocation testing
    ///
    /// * `test_change_order_size_bid_user()`
    public entry fun change_order_size_user(
        user: &signer,
        market_id: u64,
        side: bool,
        market_order_id: u128,
        new_size: u64
    ) acquires OrderBooks {
        change_order_size(
            address_of(user),
            market_id,
            NO_CUSTODIAN,
            side,
            market_order_id,
            new_size);
    }

    /// Public entry function wrapper for
    /// `place_limit_order_passive_advance_user()`.
    ///
    /// # Invocation testing
    ///
    /// * `test_place_limit_order_passive_advance_ticks_ask()`
    public entry fun place_limit_order_passive_advance_user_entry<
        BaseType,
        QuoteType
    >(
        user: &signer,
        market_id: u64,
        integrator: address,
        side: bool,
        size: u64,
        advance_style: bool,
        target_advance_amount: u64
    ) acquires OrderBooks {
        place_limit_order_passive_advance_user<
            BaseType,
            QuoteType
        >(
            user,
            market_id,
            integrator,
            side,
            size,
            advance_style,
            target_advance_amount);
    }

    /// Public entry function wrapper for `place_limit_order_user()`.
    ///
    /// # Invocation testing
    ///
    /// * `test_place_limit_order_user_entry()`
    public entry fun place_limit_order_user_entry<
        BaseType,
        QuoteType
    >(
        user: &signer,
        market_id: u64,
        integrator: address,
        side: bool,
        size: u64,
        price: u64,
        restriction: u8,
        self_match_behavior: u8
    ) acquires OrderBooks {
        place_limit_order_user<BaseType, QuoteType>(
            user, market_id, integrator, side, size, price, restriction,
            self_match_behavior);
    }

    /// Public entry function wrapper for `place_market_order_user()`.
    ///
    /// # Invocation testing
    ///
    /// * `test_place_market_order_user_entry()`
    public entry fun place_market_order_user_entry<
        BaseType,
        QuoteType
    >(
        user: &signer,
        market_id: u64,
        integrator: address,
        direction: bool,
        size: u64,
        self_match_behavior: u8
    ) acquires OrderBooks {
        place_market_order_user<BaseType, QuoteType>(
            user, market_id, integrator, direction, size, self_match_behavior);
    }

    /// Wrapped call to `register_market_base_coin()` for paying utility
    /// coins from an `aptos_framework::coin::CoinStore`.
    ///
    /// # Testing
    ///
    /// * `test_register_markets()`
    public entry fun register_market_base_coin_from_coinstore<
        BaseType,
        QuoteType,
        UtilityType
    >(
        user: &signer,
        lot_size: u64,
        tick_size: u64,
        min_size: u64
    ) acquires OrderBooks {
        // Get market registration fee, denominated in utility coins.
        let fee = incentives::get_market_registration_fee();
        // Register market with base coin, paying fees from coin store.
        register_market_base_coin<BaseType, QuoteType, UtilityType>(
            lot_size, tick_size, min_size, coin::withdraw(user, fee));
    }

    /// Public entry function wrapper for `swap_between_coinstores()`.
    ///
    /// # Invocation testing
    ///
    /// * `test_swap_between_coinstores_register_base_store()`
    /// * `test_swap_between_coinstores_register_quote_store()`
    public entry fun swap_between_coinstores_entry<
        BaseType,
        QuoteType
    >(
        user: &signer,
        market_id: u64,
        integrator: address,
        direction: bool,
        min_base: u64,
        max_base: u64,
        min_quote: u64,
        max_quote: u64,
        limit_price: u64
    ) acquires
        MarketEventHandles,
        OrderBooks,
        SwapperEventHandles
    {
        swap_between_coinstores<BaseType, QuoteType>(
            user, market_id, integrator, direction, min_base, max_base,
            min_quote, max_quote, limit_price);
    }

    // Public entry functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Private functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Cancel all of a user's open maker orders.
    ///
    /// # Parameters
    ///
    /// * `user`: Same as for `cancel_order()`.
    /// * `market_id`: Same as for `cancel_order()`.
    /// * `custodian_id`: Same as for `cancel_order()`.
    /// * `side`: Same as for `cancel_order()`.
    ///
    /// # Expected value testing
    ///
    /// * `test_cancel_all_orders_ask_custodian()`
    /// * `test_cancel_all_orders_bid_user()`
    fun cancel_all_orders(
        user: address,
        market_id: u64,
        custodian_id: u64,
        side: bool
    ) acquires OrderBooks {
        // Get user's active market order IDs.
        let market_order_ids = user::get_active_market_order_ids_internal(
            user, market_id, custodian_id, side);
        // Get number of market order IDs, init loop index variable.
        let (n_orders, i) = (vector::length(&market_order_ids), 0);
        while (i < n_orders) { // Loop over all active orders.
            // Cancel market order for current iteration.
            cancel_order(user, market_id, custodian_id, side,
                         *vector::borrow(&market_order_ids, i));
            i = i + 1; // Increment loop counter.
        }
    }

    /// Cancel maker order on order book and in user's market account.
    ///
    /// The market order ID is first checked to see if the AVL queue
    /// access key encoded within can even be used for an AVL queue
    /// removal operation in the first place. Then during the call to
    /// `user::cancel_order_internal()`, the market order ID is again
    /// verified against the order access key derived from the AVL queue
    /// removal operation.
    ///
    /// # Parameters
    ///
    /// * `user`: Address of user holding maker order.
    /// * `market_id`: Market ID of market.
    /// * `custodian_id`: Market account custodian ID.
    /// * `side`: `ASK` or `BID`, the maker order side.
    /// * `market_order_id`: Market order ID of order on order book.
    ///
    /// # Aborts
    ///
    /// * `E_INVALID_MARKET_ORDER_ID`: Market order ID does not
    ///   correspond to a valid order.
    /// * `E_INVALID_MARKET_ID`: No market with given ID.
    /// * `E_INVALID_USER`: Mismatch between `user` and user for order
    ///   on book having given market order ID.
    /// * `E_INVALID_CUSTODIAN`: Mismatch between `custodian_id` and
    ///   custodian ID of order on order book having market order ID.
    ///
    /// # Expected value testing
    ///
    /// * `test_cancel_order_ask_custodian()`
    /// * `test_cancel_order_bid_user()`
    ///
    /// # Failure testing
    ///
    /// * `test_cancel_order_invalid_custodian()`
    /// * `test_cancel_order_invalid_market_id()`
    /// * `test_cancel_order_invalid_market_order_id_bogus()`
    /// * `test_cancel_order_invalid_market_order_id_null()`
    /// * `test_cancel_order_invalid_user()`
    fun cancel_order(
        user: address,
        market_id: u64,
        custodian_id: u64,
        side: bool,
        market_order_id: u128
    ) acquires OrderBooks {
        // Get address of resource account where order books are stored.
        let resource_address = resource_account::get_address();
        let order_books_map_ref_mut = // Mutably borrow order books map.
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        // Assert order books map has order book with given market ID.
        assert!(tablist::contains(order_books_map_ref_mut, market_id),
                E_INVALID_MARKET_ID);
        let order_book_ref_mut = // Mutably borrow market order book.
            tablist::borrow_mut(order_books_map_ref_mut, market_id);
        // Mutably borrow corresponding orders AVL queue.
        let orders_ref_mut = if (side == ASK) &mut order_book_ref_mut.asks
            else &mut order_book_ref_mut.bids;
        // Get AVL queue access key from market order ID.
        let avlq_access_key = ((market_order_id & (HI_64 as u128)) as u64);
        // Check if removal from the AVL queue is even possible.
        let removal_possible = avl_queue::contains_active_list_node_id(
            orders_ref_mut, avlq_access_key);
        // Assert that removal from the AVL queue is possible.
        if (removal_possible) {
            // assert!(removal_possible, E_INVALID_MARKET_ORDER_ID);
            // Remove order from AVL queue, storing its fields.
            let Order{size, price, user: order_user, custodian_id:
                    order_custodian_id, order_access_key} = avl_queue::remove(
                orders_ref_mut, avlq_access_key);
            // Assert passed maker address is user holding order.
            assert!(user == order_user, E_INVALID_USER);
            // Assert passed custodian ID matches that from order.
            assert!(custodian_id == order_custodian_id, E_INVALID_CUSTODIAN);
            // Cancel order user-side, thus verifying market order ID.
            user::cancel_order_internal(
                user, market_id, custodian_id, side, size, price, order_access_key,
                market_order_id, CANCEL_REASON_MANUAL_CANCEL);
        }
    }

    /// Change maker order size on book and in user's market account.
    ///
    /// Priority for given price level is preserved for size decrease,
    /// but lost for size increase.
    ///
    /// The market order ID is first checked to see if the AVL queue
    /// access key encoded within can even be used for an AVL queue
    /// borrow operation in the first place. Then during the call to
    /// `user::change_order_size_internal()`, the market order ID is
    /// again verified against the order access key derived from the AVL
    /// queue borrow operation.
    ///
    /// # Parameters
    ///
    /// * `user`: Address of user holding maker order.
    /// * `market_id`: Market ID of market.
    /// * `custodian_id`: Market account custodian ID.
    /// * `side`: `ASK` or `BID`, the maker order side.
    /// * `market_order_id`: Market order ID of order on order book.
    /// * `new_size`: The new order size to change to.
    ///
    /// # Aborts
    ///
    /// * `E_INVALID_MARKET_ORDER_ID`: Market order ID does not
    ///   correspond to a valid order.
    /// * `E_INVALID_MARKET_ID`: No market with given ID.
    /// * `E_INVALID_USER`: Mismatch between `user` and user for order
    ///   on book having given market order ID.
    /// * `E_INVALID_CUSTODIAN`: Mismatch between `custodian_id` and
    ///   custodian ID of order on order book having market order ID.
    /// * `E_SIZE_CHANGE_BELOW_MIN_SIZE`: New order size is less than
    ///   the minimum order size for market.
    ///
    /// # Expected value testing
    ///
    /// * `test_change_order_size_ask_custodian()`
    /// * `test_change_order_size_bid_user()`
    /// * `test_change_order_size_bid_user_new_tail()`
    ///
    /// # Failure testing
    ///
    /// * `test_change_order_size_below_min_size()`
    /// * `test_change_order_size_insertion_error()`
    /// * `test_change_order_size_invalid_custodian()`
    /// * `test_change_order_size_invalid_market_id()`
    /// * `test_change_order_size_invalid_market_order_id_bogus()`
    /// * `test_change_order_size_invalid_market_order_id_null()`
    /// * `test_change_order_size_invalid_user()`
    fun change_order_size(
        user: address,
        market_id: u64,
        custodian_id: u64,
        side: bool,
        market_order_id: u128,
        new_size: u64
    ) acquires OrderBooks {
        // Get address of resource account where order books are stored.
        let resource_address = resource_account::get_address();
        let order_books_map_ref_mut = // Mutably borrow order books map.
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        // Assert order books map has order book with given market ID.
        assert!(tablist::contains(order_books_map_ref_mut, market_id),
                E_INVALID_MARKET_ID);
        let order_book_ref_mut = // Mutably borrow market order book.
            tablist::borrow_mut(order_books_map_ref_mut, market_id);
        // Assert new size is at least minimum size for market.
        assert!(new_size >= order_book_ref_mut.min_size,
                E_SIZE_CHANGE_BELOW_MIN_SIZE);
        // Mutably borrow corresponding orders AVL queue.
        let orders_ref_mut = if (side == ASK) &mut order_book_ref_mut.asks
            else &mut order_book_ref_mut.bids;
        // Get AVL queue access key from market order ID.
        let avlq_access_key = ((market_order_id & (HI_64 as u128)) as u64);
        // Check if borrowing from the AVL queue is even possible.
        let borrow_possible = avl_queue::contains_active_list_node_id(
            orders_ref_mut, avlq_access_key);
        // Assert that borrow from the AVL queue is possible.
        assert!(borrow_possible, E_INVALID_MARKET_ORDER_ID);
        // Check if order is at tail of queue for given price level.
        let tail_of_price_level_queue =
            avl_queue::is_local_tail(orders_ref_mut, avlq_access_key);
        let order_ref_mut = // Mutably borrow order on order book.
            avl_queue::borrow_mut(orders_ref_mut, avlq_access_key);
        // Assert passed user address is user holding order.
        assert!(user == order_ref_mut.user, E_INVALID_USER);
        // Assert passed custodian ID matches that from order.
        assert!(custodian_id == order_ref_mut.custodian_id,
                E_INVALID_CUSTODIAN);
        // Change order size user-side, thus verifying market order ID
        // and new size.
        user::change_order_size_internal(
            user, market_id, custodian_id, side, order_ref_mut.size, new_size,
            order_ref_mut.price, order_ref_mut.order_access_key,
            market_order_id);
        // Get order price.
        let price = avl_queue::get_access_key_insertion_key(avlq_access_key);
        // If size change is for a size decrease or if order is at tail
        // of given price level:
        if ((new_size < order_ref_mut.size) || tail_of_price_level_queue) {
            // Mutate order on book to reflect new size, preserving spot
            // in queue for the given price level.
            order_ref_mut.size = new_size;
        // If new size is more than old size (user-side function
        // verifies that size is not equal) but order is not tail of
        // queue for the given price level, priority should be lost:
        } else {
            // Remove order from AVL queue, pushing corresponding AVL
            // queue list node onto unused list node stack.
            let order = avl_queue::remove(orders_ref_mut, avlq_access_key);
            order.size = new_size; // Mutate order size.
            // Insert at back of queue for given price level.
            let new_avlq_access_key =
                avl_queue::insert(orders_ref_mut, price, order);
            // Verify that new AVL queue access key is the same as
            // before the size change: since list nodes are re-used, the
            // AVL queue access key should be the same, even though the
            // order is now the new tail of a doubly linked list for the
            // given insertion key (back of queue for the given price
            // level). Eviction is not checked because the AVL queue
            // shape is the same before and after the remove/insert
            // compound operation.
            assert!(new_avlq_access_key == avlq_access_key,
                    E_SIZE_CHANGE_INSERTION_ERROR);
        };
    }

    /// Get optional cancel reason for market order or swap.
    ///
    /// # Parameters
    ///
    /// * `self_match_taker_cancel`: If matching resulted in cancelling
    ///   the taker side of an order due to a self match.
    /// * `base_traded`: The amount of base assets traded.
    /// * `max_base`: The maximum indicated amount of base assets to
    ///   match.
    /// * `liquidity_gone`: If the matching engine halted due to
    ///   insufficient liquidity.
    /// * `lot_size`: The lot size for the market.
    /// * `violated_limit_price`: `true` if matching halted due to a
    ///   violated limit price
    ///
    /// # Returns
    ///
    /// * `Option<u8>`: An optional cancel reason, if the order needs
    ///   to be cancelled.
    inline fun get_cancel_reason_option_for_market_order_or_swap(
        self_match_taker_cancel: bool,
        base_traded: u64,
        max_base: u64,
        liquidity_gone: bool,
        lot_size: u64,
        limit_price_violated: bool
    ): Option<u8> {
        let need_to_cancel =
            ((self_match_taker_cancel) || (base_traded < max_base));
        if (need_to_cancel) {
            if (self_match_taker_cancel) {
                option::some(CANCEL_REASON_SELF_MATCH_TAKER)
            } else if (limit_price_violated) {
                option::some(CANCEL_REASON_VIOLATED_LIMIT_PRICE)
            } else if (liquidity_gone) {
                option::some(CANCEL_REASON_NOT_ENOUGH_LIQUIDITY)
            } else if ((max_base - base_traded) < lot_size) {
                option::some(CANCEL_REASON_TOO_SMALL_TO_FILL_LOT)
            } else {
                option::some(CANCEL_REASON_MAX_QUOTE_TRADED)
            }
        } else {
            option::none()
        }
    }

    /// Index specified number of open orders for given side of order
    /// book.
    ///
    /// # Testing
    ///
    /// * `test_get_open_orders()`
    fun get_open_orders_for_side(
        market_id: u64,
        order_book_ref_mut: &mut OrderBook,
        side: bool,
        n_orders_max: u64
    ): vector<OrderView> {
        let orders = vector[]; // Initialize empty vector of orders.
        // Get mutable reference to orders AVL queue for given side.
        let avlq_ref_mut = if (side == ASK) &mut order_book_ref_mut.asks else
            &mut order_book_ref_mut.bids;
        // While there are still orders left to index:
        while((vector::length(&orders) < n_orders_max) &&
              (!avl_queue::is_empty(avlq_ref_mut))) {
            // Remove and unpack order at head of queue.
            let Order{size, price, user, custodian_id, order_access_key} =
                avl_queue::pop_head(avlq_ref_mut);
            // Get order ID from user-side order memory.
            let order_id = option::destroy_some(
                user::get_open_order_id_internal(user, market_id, custodian_id,
                                                 side, order_access_key));
            // Push back an order view to orders view vector.
            vector::push_back(&mut orders, OrderView{
                market_id, side, order_id, remaining_size: size, price, user,
                custodian_id});
        };
        orders // Return vector of view-friendly orders.
    }

    /// Index specified number of open orders for given side of order
    /// book, from given starting order ID.
    ///
    /// See `get_open_orders_paginated()`.
    ///
    /// # Testing
    ///
    /// * `test_get_open_orders_paginated()`
    fun get_open_orders_for_side_paginated(
        order_book_ref: &OrderBook,
        market_id: u64,
        side: bool,
        n_orders_to_index_max: u64,
        starting_order_id: u128
    ): (
        vector<OrderView>,
        u128,
    ) {
        // Get immutable reference to orders AVL queue for given side.
        let avlq_ref = if (side == ASK) &order_book_ref.asks else
            &order_book_ref.bids;
        let orders = vector[]; // Initialize empty vector of orders.
        // Return early if no orders to index.
        if (avl_queue::is_empty(avlq_ref) || n_orders_to_index_max == 0)
            return (orders, (NIL as u128));
        // Get order ID to index. If starting from best bid/ask:
        let order_id = if (starting_order_id == (NIL as u128)) {
            // Lookup order ID from user memory and reassign.
            let order_ref = avl_queue::borrow_head(avlq_ref);
            let optional_order_id = user::get_open_order_id_internal(
                order_ref.user, market_id, order_ref.custodian_id, side,
                order_ref.order_access_key);
            option::destroy_some(optional_order_id)
        } else {
            starting_order_id
        };
        // Get AVL queue access key from order ID.
        let avlq_access_key = get_order_id_avl_queue_access_key(order_id);
        let n_indexed_orders = 0;
        while (n_indexed_orders < n_orders_to_index_max) {
            // Borrow next order to index in AVL queue.
            let order_ref = avl_queue::borrow(avlq_ref, avlq_access_key);
            // Get order ID from user-side order memory.
            let order_id = option::destroy_some(
                user::get_open_order_id_internal(
                    order_ref.user, market_id, order_ref.custodian_id,
                    side, order_ref.order_access_key));
            // Push back an order view to orders view vector.
            vector::push_back(&mut orders, OrderView{
                market_id, side, order_id, remaining_size: order_ref.size,
                price: order_ref.price, user: order_ref.user, custodian_id:
                order_ref.custodian_id});
            // Get access key for next order in AVL queue.
            avlq_access_key = avl_queue::next_list_node_id_in_access_key(
                avlq_ref, avlq_access_key);
            // Stop indexing if no traversals left.
            if (avlq_access_key == NIL) break;
            n_indexed_orders = n_indexed_orders + 1;
        };
        let next_page_start = if (avlq_access_key == NIL) {
            (NIL as u128)
        } else {
            // Borrow order for next page start.
            let order_ref = avl_queue::borrow(avlq_ref, avlq_access_key);
            let optional_order_id = user::get_open_order_id_internal(
                order_ref.user, market_id, order_ref.custodian_id, side,
                order_ref.order_access_key);
            option::destroy_some(optional_order_id)
        };
        (orders, next_page_start)
    }

    /// Get AVL queue access key encoded in `order_id`.
    ///
    /// # Testing
    ///
    /// * `test_get_market_order_id_avl_queue_access_key()`
    fun get_order_id_avl_queue_access_key(
        order_id: u128
    ): u64 {
        ((order_id & (HI_64 as u128)) as u64)
    }

    /// Index specified number of price levels for given side of order
    /// book.
    ///
    /// # Testing
    ///
    /// * `test_get_price_levels()`
    /// * `test_get_price_levels_mismatch()`
    fun get_price_levels_for_side(
        order_book_ref_mut: &mut OrderBook,
        side: bool,
        n_price_levels_max: u64
    ): vector<PriceLevel> {
        // Initialize empty price levels vector.
        let price_levels = vector[];
        // Get mutable reference to orders AVL queue for given side.
        let avlq_ref_mut = if (side == ASK) &mut order_book_ref_mut.asks else
            &mut order_book_ref_mut.bids;
        // While more price levels can be indexed:
        while (vector::length(&price_levels) < n_price_levels_max) {
            let size = 0; // Initialize price level size to 0.
            // Get optional price of order at head of queue.
            let optional_head_price = avl_queue::get_head_key(avlq_ref_mut);
            // If there is an order at the head of the queue:
            if (option::is_some(&optional_head_price)) {
                // Unpack its price as the price tracker for the level.
                let price = option::destroy_some(optional_head_price);
                // While orders still left on book:
                while (!avl_queue::is_empty(avlq_ref_mut)) {
                    // If order at head of the queue is in price level:
                    if (option::contains(
                            &avl_queue::get_head_key(avlq_ref_mut), &price)) {
                        // Pop order, storing only its size and price.
                        let Order{
                            size: order_size,
                            price: order_price,
                            user: _,
                            custodian_id: _,
                            order_access_key: _
                        } = avl_queue::pop_head(avlq_ref_mut);
                        // Verify order price equals insertion key.
                        assert!(order_price == price, E_ORDER_PRICE_MISMATCH);
                        // Increment tracker for price level size. Note
                        // that no overflow is checked because an open
                        // order's size is a u64, and an AVL queue can
                        // hold at most 2 ^ 14 - 1 open orders.
                        size = size + (order_size as u128);
                    } else { // If order at head of queue not in level:
                        break // Break of out loop over head of queue.
                    }
                };
                // Push back price level to price levels vector.
                vector::push_back(&mut price_levels, PriceLevel{price, size});
            } else { // If no order at the head of the queue:
                break // Break of out loop on price level vector length.
            }
        };
        price_levels // Return vector of price levels.
    }

    /// Index specified number of open orders for given side of order
    /// book into price levels, starting from given starting order ID.
    ///
    /// See `get_price_levels_paginated()`.
    ///
    /// # Testing
    ///
    /// * `test_price_levels_paginated()`
    fun get_price_levels_for_side_paginated(
        order_book_ref: &OrderBook,
        market_id: u64,
        side: bool,
        n_orders_to_index_max: u64,
        starting_order_id: u128 // If `NIL`, start from best bid/ask.
    ): (
        vector<PriceLevel>,
        u128, // Order ID for start of next page, `NIL` if done.
    ) {
        // Get immutable reference to orders AVL queue for given side.
        let avlq_ref = if (side == ASK) &order_book_ref.asks else
            &order_book_ref.bids;
        // Initialize empty price levels vector.
        let price_levels = vector[];
        // Return early if no orders to index.
        if (avl_queue::is_empty(avlq_ref) || n_orders_to_index_max == 0)
            return (price_levels, (NIL as u128));
        // Get order ID to index. If starting from best bid/ask:
        let order_id = if (starting_order_id == (NIL as u128)) {
            // Lookup order ID from user memory and reassign.
            let order_ref = avl_queue::borrow_head(avlq_ref);
            let optional_order_id = user::get_open_order_id_internal(
                order_ref.user, market_id, order_ref.custodian_id, side,
                order_ref.order_access_key);
            option::destroy_some(optional_order_id)
        } else {
            starting_order_id
        };
        // Get AVL queue access key from order ID.
        let avlq_access_key = get_order_id_avl_queue_access_key(order_id);
        // Get price for starting price level.
        let price = avl_queue::borrow(avlq_ref, avlq_access_key).price;
        // Initialize size for price level to 0.
        let size = 0;
        let n_indexed_orders = 0;
        while (n_indexed_orders < n_orders_to_index_max) {
            // Borrow next order to index in AVL queue.
            let order_ref = avl_queue::borrow(avlq_ref, avlq_access_key);
            // If in same price level, increment size:
            if (order_ref.price == price) {
                size = size + (order_ref.size as u128);
            // If in new level, push back prior one and start anew.
            } else {
                vector::push_back(&mut price_levels, PriceLevel{price, size});
                price = order_ref.price;
                size = (order_ref.size as u128);
            };
            // Get access key for next order in AVL queue.
            avlq_access_key = avl_queue::next_list_node_id_in_access_key(
                avlq_ref, avlq_access_key);
            // Stop indexing if no traversals left.
            if (avlq_access_key == NIL) break;
            n_indexed_orders = n_indexed_orders + 1;
        };
        // Push back final price level.
        vector::push_back(&mut price_levels, PriceLevel{price, size});
        let next_page_start = if (avlq_access_key == NIL) {
            (NIL as u128)
        } else {
            // Borrow order for next page start.
            let order_ref = avl_queue::borrow(avlq_ref, avlq_access_key);
            let optional_order_id = user::get_open_order_id_internal(
                order_ref.user, market_id, order_ref.custodian_id, side,
                order_ref.order_access_key);
            option::destroy_some(optional_order_id)
        };
        (price_levels, next_page_start)
    }

    /// Initialize the order books map upon module publication.
    fun init_module(
        _econia: &signer
    ) {
        // Get Econia resource account signer.
        let resource_account = resource_account::get_signer();
        // Initialize order books map under resource account.
        move_to(&resource_account, OrderBooks{map: tablist::new()})
    }

    /// Match a taker order against the order book.
    ///
    /// Calculates maximum amount of quote coins to match, matches, then
    /// assesses taker fees. Matches up until the point of a self match,
    /// then proceeds according to specified self match behavior.
    ///
    /// # Type Parameters
    ///
    /// * `BaseType`: Base asset type for market.
    ///   `registry::GenericAsset` if a generic market.
    /// * `QuoteType`: Quote coin type for market.
    ///
    /// # Parameters
    ///
    /// * `market_id`: Market ID of market.
    /// * `fill_event_queue_ref_mut`: Mutable reference to vector for
    ///   enqueueing deferred `user::FillEvent`(s).
    /// * `order_book_ref_mut`: Mutable reference to market order book.
    /// * `taker`: Address of taker whose order is matched. Passed as
    ///   `NO_TAKER_ADDRESS` when taker order originates from a swap
    ///   without a signature.
    /// * `custodian_id`: Custodian ID associated with a taker market
    ///   account, if any. Should be passed as `NO_CUSTODIAN` if `taker`
    ///   is `NO_TAKER_ADDRESS`.
    /// * `integrator`: The integrator for the taker order, who collects
    ///   a portion of taker fees at their
    ///   `incentives::IntegratorFeeStore` for the given market. May be
    ///   passed as an address known not to be an integrator, for
    ///   example `@0x0` or `@econia`, in the service of diverting all
    ///   fees to Econia.
    /// * `direction`: `BUY` or `SELL`, from the taker's perspective. If
    ///   a `BUY`, fills against asks, else against bids.
    /// * `min_base`: Minimum base asset units to be traded by taker,
    ///   either received or traded away.
    /// * `max_base`: Maximum base asset units to be traded by taker,
    ///   either received or traded away.
    /// * `min_quote`: Minimum quote asset units to be traded by taker,
    ///   either received or traded away. Refers to the net change in
    ///   taker's quote holdings after matching and fees.
    /// * `max_quote`: Maximum quote asset units to be traded by taker,
    ///   either received or traded away. Refers to the net change in
    ///   taker's quote holdings after matching and fees.
    /// * `limit_price`: If direction is `BUY`, the price above which
    ///   matching should halt. If direction is `SELL`, the price below
    ///   which matching should halt. Can be passed as `HI_PRICE` if a
    ///   `BUY` or `0` if a `SELL` to approve matching at any price.
    /// * `self_match_behavior`: `ABORT`, `CANCEL_BOTH`, `CANCEL_MAKER`,
    ///   or `CANCEL_TAKER`. Ignored if no self matching takes place.
    /// * `optional_base_coins`: None if `BaseType` is
    ///   `registry::GenericAsset` (market is generic), else base coin
    ///   holdings for pure coin market, which are incremented if
    ///   `direction` is `BUY` and decremented if `direction` is `SELL`.
    /// * `quote_coins`: Quote coin holdings for market, which are
    ///   decremented if `direction` is `BUY` and incremented if
    ///   `direction` is `SELL`.
    ///
    /// # Returns
    ///
    /// * `Option<Coin<BaseType>>`: None if `BaseType` is
    ///   `registry::GenericAsset`, else updated `optional_base_coins`
    ///   holdings after matching.
    /// * `Coin<QuoteType>`: Updated `quote_coins` holdings after
    ///   matching.
    /// * `u64`: Base asset amount traded by taker: net change in
    ///   taker's base holdings.
    /// * `u64`: Quote coin amount traded by taker, inclusive of fees:
    ///   net change in taker's quote coin holdings.
    /// * `u64`: Amount of quote coin fees paid.
    /// * `bool`: `true` if a self match that results in a taker cancel.
    /// * `bool`: `true` if liquidity is gone from order book on
    ///   corresponding side after matching.
    /// * `bool`: `true` if matching halted due to violated limit price.
    ///
    /// # Aborts
    ///
    /// * `E_PRICE_TOO_HIGH`: Order price exceeds maximum allowable
    ///   price.
    /// * `E_HEAD_KEY_PRICE_MISMATCH`: AVL queue head price does not
    ///   match head order price.
    /// * `E_SELF_MATCH`: A self match occurs when `self_match_behavior`
    ///   is `ABORT`.
    /// * `E_INVALID_SELF_MATCH_BEHAVIOR`: A self match occurs but an
    ///   invalid behavior flag is passed.
    /// * `E_MIN_BASE_NOT_TRADED`: Minimum base asset trade amount
    ///   requirement not met.
    /// * `E_MIN_QUOTE_NOT_TRADED`: Minimum quote asset trade amount
    ///   requirement not met.
    ///
    /// # Expected value testing
    ///
    /// * `test_match_complete_fill_no_lots_buy()`
    /// * `test_match_complete_fill_no_ticks_sell()`
    /// * `test_match_empty()`
    /// * `test_match_fill_size_0()`
    /// * `test_match_loop_twice()`
    /// * `test_match_order_size_0()`
    /// * `test_match_partial_fill_lot_limited_sell()`
    /// * `test_match_partial_fill_tick_limited_buy()`
    /// * `test_match_price_break_buy()`
    /// * `test_match_price_break_sell()`
    /// * `test_match_self_match_cancel_both()`
    /// * `test_match_self_match_cancel_maker()`
    /// * `test_match_self_match_cancel_taker()`
    ///
    /// # Failure testing
    ///
    /// * `test_match_min_base_not_traded()`
    /// * `test_match_min_quote_not_traded()`
    /// * `test_match_price_mismatch()`
    /// * `test_match_price_too_high()`
    /// * `test_match_self_match_abort()`
    /// * `test_match_self_match_invalid()`
    fun match<
        BaseType,
        QuoteType
    >(
        market_id: u64,
        fill_event_queue_ref_mut: &mut vector<FillEvent>,
        order_book_ref_mut: &mut OrderBook,
        taker: address,
        custodian_id: u64,
        integrator: address,
        direction: bool,
        min_base: u64,
        max_base: u64,
        min_quote: u64,
        max_quote: u64,
        limit_price: u64,
        self_match_behavior: u8,
        optional_base_coins: Option<Coin<BaseType>>,
        quote_coins: Coin<QuoteType>,
    ): (
        Option<Coin<BaseType>>,
        Coin<QuoteType>,
        u64,
        u64,
        u64,
        bool,
        bool,
        bool
    ) {
        // Assert price is not too high.
        assert!(limit_price <= HI_PRICE, E_PRICE_TOO_HIGH);
        // Taker buy fills against asks, sell against bids.
        let side = if (direction == BUY) ASK else BID;
        let (lot_size, tick_size) = (order_book_ref_mut.lot_size,
            order_book_ref_mut.tick_size); // Get lot and tick sizes.
        // Get taker fee divisor.
        let taker_fee_divisor = incentives::get_taker_fee_divisor();
        // Get max quote coins to match.
        let max_quote_match = incentives::calculate_max_quote_match(
            direction, taker_fee_divisor, max_quote);
        // Calculate max amounts of lots and ticks to fill.
        let (max_lots, max_ticks) =
            (max_base / lot_size, max_quote_match / tick_size);
        // Initialize counters for number of lots and ticks to fill.
        let (lots_until_max, ticks_until_max) = (max_lots, max_ticks);
        // Mutably borrow corresponding orders AVL queue.
        let orders_ref_mut = if (side == ASK) &mut order_book_ref_mut.asks
            else &mut order_book_ref_mut.bids;
        // Assume it is not the case that a self match led to a taker
        // order cancellation.
        let self_match_taker_cancel = false;
        // Get new order ID before any potential fills.
        order_book_ref_mut.counter = order_book_ref_mut.counter + 1;
        let order_id = ((order_book_ref_mut.counter as u128) << SHIFT_COUNTER);
        // Initialize counters for fill iteration.
        let (fill_count, fees_paid) = (0, 0);
        let violated_limit_price = false; // Assume no price violation.
        // While there are orders to match against:
        while (!avl_queue::is_empty(orders_ref_mut)) {
            let price = // Get price of order at head of AVL queue.
                *option::borrow(&avl_queue::get_head_key(orders_ref_mut));
            // Break if price too high to buy at or too low to sell at.
            if (((direction == BUY ) && (price > limit_price)) ||
                ((direction == SELL) && (price < limit_price))) {
                    violated_limit_price = true;
                    break
            };
            // Calculate max number of lots that could be filled
            // at order price, limited by ticks left to fill until max.
            let max_fill_size_ticks = ticks_until_max / price;
            // Max fill size is lesser of tick-limited fill size and
            // lot-limited fill size.
            let max_fill_size = if (max_fill_size_ticks < lots_until_max)
                max_fill_size_ticks else lots_until_max;
            // Mutably borrow order at head of AVL queue.
            let order_ref_mut = avl_queue::borrow_head_mut(orders_ref_mut);
            // Assert AVL queue head price matches that of order.
            assert!(order_ref_mut.price == price, E_HEAD_KEY_PRICE_MISMATCH);
            // If order at head of queue has size 0, evict it and
            // continue to next order. This should never be reached
            // during production, but is handled here to explicitly
            // verify the assumption of no empty orders on the book.
            if (order_ref_mut.size == 0) {
                let Order{
                    size: evictee_size,
                    price: evictee_price,
                    user: evictee_user,
                    custodian_id: evictee_custodian_id,
                    order_access_key: evictee_order_access_key
                } = avl_queue::pop_head(orders_ref_mut);
                user::cancel_order_internal(
                    evictee_user,
                    market_id,
                    evictee_custodian_id,
                    side,
                    evictee_size,
                    evictee_price,
                    evictee_order_access_key,
                    (NIL as u128),
                    CANCEL_REASON_EVICTION,
                );
                continue
            };
            // Get fill size and if a complete fill against book.
            let (fill_size, complete_fill) =
                // If max fill size is less than order size, fill size
                // is max fill size and is an incomplete fill. Else
                // order gets completely filled.
                if (max_fill_size < order_ref_mut.size)
                   (max_fill_size, false) else (order_ref_mut.size, true);
            if (fill_size == 0) break; // Break if no lots to fill.
            // Get maker user address and custodian ID for maker's
            // market account.
            let (maker, maker_custodian_id) =
                (order_ref_mut.user, order_ref_mut.custodian_id);
            let self_match = // Determine if a self match.
                ((taker == maker) && (custodian_id == maker_custodian_id));
            if (self_match) { // If a self match:
                // Assert self match behavior is not abort.
                assert!(self_match_behavior != ABORT, E_SELF_MATCH);
                // Assume not cancelling maker order.
                let cancel_maker_order = false;
                // If self match behavior is cancel both:
                if (self_match_behavior == CANCEL_BOTH) {
                    (cancel_maker_order, self_match_taker_cancel) =
                        (true, true); // Flag orders for cancellation.
                // If self match behavior is cancel maker order:
                } else if (self_match_behavior == CANCEL_MAKER) {
                    cancel_maker_order = true; // Flag for cancellation.
                // If self match behavior is cancel taker order:
                } else if (self_match_behavior == CANCEL_TAKER) {
                    // Flag for cancellation.
                    self_match_taker_cancel = true;
                // Otherwise invalid self match behavior specified.
                } else abort E_INVALID_SELF_MATCH_BEHAVIOR;
                // If maker order should be canceled:
                if (cancel_maker_order) {
                    // Cancel from maker's market account, storing
                    // market order ID.
                    let market_order_id = user::cancel_order_internal(
                        maker, market_id, maker_custodian_id, side,
                        order_ref_mut.size, price,
                        order_ref_mut.order_access_key, (NIL as u128),
                        CANCEL_REASON_SELF_MATCH_MAKER);
                    // Get AVL queue access key from market order ID.
                    let avlq_access_key =
                        ((market_order_id & (HI_64 as u128)) as u64);
                    // Remove order from AVL queue.
                    let Order{size: _, price: _, user: _, custodian_id: _,
                              order_access_key: _} = avl_queue::remove(
                        orders_ref_mut, avlq_access_key);
                }; // Optional maker order cancellation complete.
                // Break out of loop if a self match taker cancel.
                if (self_match_taker_cancel) break;
            } else { // If not a self match:
                // Get ticks, quote filled.
                let ticks_filled = fill_size * price;
                let quote_filled = ticks_filled * tick_size;
                // Decrement counter for lots to fill until max reached.
                lots_until_max = lots_until_max - fill_size;
                // Decrement counter for ticks to fill until max.
                ticks_until_max = ticks_until_max - ticks_filled;
                // Declare return assignment variable.
                let market_order_id;
                // Fill matched order user side, store market order ID.
                (optional_base_coins, quote_coins, market_order_id) =
                    user::fill_order_internal<BaseType, QuoteType>(
                        maker, market_id, maker_custodian_id, side,
                        order_ref_mut.order_access_key, order_ref_mut.size,
                        fill_size, complete_fill, optional_base_coins,
                        quote_coins, fill_size * lot_size, quote_filled);
                // Enqueue a fill event with the amount of fees paid.
                let fees_paid_for_fill = quote_filled / taker_fee_divisor;
                let fill_event = user::create_fill_event_internal(
                    market_id, fill_size, price, side, maker,
                    maker_custodian_id, market_order_id, taker, custodian_id,
                    order_id, fees_paid_for_fill, fill_count);
                vector::push_back(fill_event_queue_ref_mut, fill_event);
                // Update fill iteration counters.
                fill_count = fill_count + 1;
                fees_paid = fees_paid + fees_paid_for_fill;
                // If order on book completely filled:
                if (complete_fill) {
                    let avlq_access_key = // Get AVL queue access key.
                        ((market_order_id & (HI_64 as u128)) as u64);
                    let order = // Remove order from AVL queue.
                        avl_queue::remove(orders_ref_mut, avlq_access_key);
                    let Order{size: _, price: _, user: _, custodian_id: _,
                            order_access_key: _} = order; // Unpack order.
                    // Break out of loop if no more lots or ticks to fill.
                    if ((lots_until_max == 0) || (ticks_until_max == 0)) break
                } else { // If order on book not completely filled:
                    // Decrement order size by amount filled.
                    order_ref_mut.size = order_ref_mut.size - fill_size;
                    break // Stop matching.
                }
            }; // Done processing counterparty match.
        }; // Done looping over head of AVL queue for given side.
        let (base_fill, quote_fill) = // Calculate base and quote fills.
            (((max_lots  - lots_until_max ) * lot_size),
             ((max_ticks - ticks_until_max) * tick_size));
        // Assess taker fees.
        let (quote_coins, _) = incentives::assess_taker_fees<QuoteType>(
                market_id, integrator, taker_fee_divisor,
                fees_paid * taker_fee_divisor, quote_coins);
        // If a buy, taker pays quote required for fills, and additional
        // fee assessed after matching. If a sell, taker receives quote
        // from fills, then has a portion assessed as fees.
        let quote_traded = if (direction == BUY) (quote_fill + fees_paid)
            else (quote_fill - fees_paid);
        // Assert minimum base asset trade amount met.
        assert!(base_fill >= min_base, E_MIN_BASE_NOT_TRADED);
        // Assert minimum quote coin trade amount met.
        assert!(quote_traded >= min_quote, E_MIN_QUOTE_NOT_TRADED);
        // Return optional base coin, quote coins, trade amounts,
        // self match taker cancel flag, if liquidity is gone, and if
        // limit price was violated.
        (optional_base_coins, quote_coins, base_fill, quote_traded, fees_paid,
         self_match_taker_cancel, avl_queue::is_empty(orders_ref_mut),
         violated_limit_price)
    }

    /// Place limit order against order book from user market account.
    ///
    /// # Type Parameters
    ///
    /// * `BaseType`: Same as for `match()`.
    /// * `QuoteType`: Same as for `match()`.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Same as for `match()`.
    /// * `custodian_id`: Same as for `match()`.
    /// * `integrator`: Same as for `match()`, only receives fees if
    ///   order fills across the spread.
    /// * `side`: `ASK` or `BID`, the side on which to place an order as
    ///   a maker.
    /// * `size`: The size, in lots, to fill.
    /// * `price`: The limit order price, in ticks per lot.
    /// * `restriction`: `FILL_OR_ABORT`, `IMMEDIATE_OR_CANCEL`,
    ///   `POST_OR_ABORT`, or `NO_RESTRICTION`.
    /// * `self_match_behavior`: Same as for `match()`.
    /// * `critical_height`: The AVL queue height above which evictions
    ///   may take place. Should only be passed as `CRITICAL_HEIGHT`.
    ///   Accepted as an argument to simplify testing.
    ///
    /// # Returns
    ///
    /// * `u128`: Order ID assigned to order, unique within a market.
    /// * `u64`: Base asset trade amount as a taker, same as for
    ///   `match()`, if order fills across the spread.
    /// * `u64`: Quote asset trade amount as a taker, same as for
    ///   `match()`, if order fills across the spread.
    /// * `u64`: Quote coin fees paid as a taker, same as for `match()`,
    ///   if order fills across the spread.
    ///
    /// # Aborts
    ///
    /// * `E_INVALID_RESTRICTION`: Invalid restriction flag.
    /// * `E_PRICE_0`: Order price specified as 0.
    /// * `E_PRICE_TOO_HIGH`: Order price exceeds maximum allowed
    ///   price.
    /// * `E_INVALID_BASE`: Base asset type is invalid.
    /// * `E_INVALID_QUOTE`: Quote asset type is invalid.
    /// * `E_SIZE_TOO_SMALL`: Limit order size does not meet minimum
    ///   size for market.
    /// * `E_FILL_OR_ABORT_NOT_CROSS_SPREAD`: Fill-or-abort price does
    ///   not cross the spread.
    /// * `E_POST_OR_ABORT_CROSSES_SPREAD`: Post-or-abort price crosses
    ///   the spread.
    /// * `E_SIZE_BASE_OVERFLOW`: The product of order size and market
    ///   lot size results in a base asset unit overflow.
    /// * `E_SIZE_PRICE_TICKS_OVERFLOW`: The product of order size and
    ///   price results in a tick amount overflow.
    /// * `E_SIZE_PRICE_QUOTE_OVERFLOW`: The product of order size,
    ///   price, and market tick size results in a quote asset unit
    ///   overflow.
    /// * `E_PRICE_TIME_PRIORITY_TOO_LOW`: Order would result in lowest
    ///   price-time priority if inserted to AVL queue, but AVL queue
    ///   does not have room for any more orders.
    ///
    /// # Restrictions
    ///
    /// * A post-or-abort order aborts if its price crosses the spread.
    /// * A fill-or-abort order aborts if it is not completely filled
    ///   as a taker order. Here, a corresponding minimum base trade
    ///   amount is passed to `match()`, which aborts if the minimum
    ///   amount is not filled.
    /// * An immediate-or-cancel order fills as a taker if possible,
    ///   then returns.
    ///
    /// # Self matching
    ///
    /// Fills up until the point of a self match, cancelling remaining
    /// size without posting if:
    ///
    /// 1. Price crosses the spread,
    /// 2. Cross-spread filling is permitted per the indicated
    ///    restriction, and
    /// 3. Self match behavior indicates taker cancellation.
    ///
    /// # Expected value testing
    ///
    /// * `test_place_limit_order_crosses_ask_exact()`
    /// * `test_place_limit_order_crosses_ask_partial()`
    /// * `test_place_limit_order_crosses_ask_partial_cancel()`
    /// * `test_place_limit_order_crosses_ask_partial_maker()`
    /// * `test_place_limit_order_crosses_ask_self_match_cancel()`
    /// * `test_place_limit_order_crosses_bid_exact()`
    /// * `test_place_limit_order_crosses_bid_partial()`
    /// * `test_place_limit_order_crosses_bid_partial_maker()`
    /// * `test_place_limit_order_crosses_bid_partial_post_under_min()`
    /// * `test_place_limit_order_evict()`
    /// * `test_place_limit_order_no_cross_ask_user()`
    /// * `test_place_limit_order_no_cross_ask_user_ioc()`
    /// * `test_place_limit_order_no_cross_bid_custodian()`
    /// * `test_place_limit_order_remove_event_handles()`
    /// * `test_place_limit_order_still_crosses_ask()`
    /// * `test_place_limit_order_still_crosses_bid()`
    ///
    /// # Failure testing
    ///
    /// * `test_place_limit_order_base_overflow()`
    /// * `test_place_limit_order_fill_or_abort_not_cross()`
    /// * `test_place_limit_order_fill_or_abort_partial()`
    /// * `test_place_limit_order_invalid_base()`
    /// * `test_place_limit_order_invalid_quote()`
    /// * `test_place_limit_order_invalid_restriction()`
    /// * `test_place_limit_order_no_price()`
    /// * `test_place_limit_order_post_or_abort_crosses()`
    /// * `test_place_limit_order_price_hi()`
    /// * `test_place_limit_order_price_time_priority_low()`
    /// * `test_place_limit_order_quote_overflow()`
    /// * `test_place_limit_order_size_lo()`
    /// * `test_place_limit_order_ticks_overflow()`
    fun place_limit_order<
        BaseType,
        QuoteType,
    >(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        integrator: address,
        side: bool,
        size: u64,
        price: u64,
        restriction: u8,
        self_match_behavior: u8,
        critical_height: u8
    ): (
        u128,
        u64,
        u64,
        u64
    ) acquires OrderBooks {
        // Assert valid order restriction flag.
        assert!(restriction <= N_RESTRICTIONS, E_INVALID_RESTRICTION);
        assert!(price != 0, E_PRICE_0); // Assert nonzero price.
        // Assert price is not too high.
        assert!(price <= HI_PRICE, E_PRICE_TOO_HIGH);
        // Get user's available and ceiling asset counts.
        let (_, base_available, base_ceiling, _, quote_available,
             quote_ceiling) = user::get_asset_counts_internal(
                user_address, market_id, custodian_id);
        // If asset count check does not abort, then market exists, so
        // get address of resource account for borrowing order book.
        let resource_address = resource_account::get_address();
        let order_books_map_ref_mut = // Mutably borrow order books map.
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        let order_book_ref_mut = // Mutably borrow market order book.
            tablist::borrow_mut(order_books_map_ref_mut, market_id);
        assert!(type_info::type_of<BaseType>() // Assert base type.
                == order_book_ref_mut.base_type, E_INVALID_BASE);
        assert!(type_info::type_of<QuoteType>() // Assert quote type.
                == order_book_ref_mut.quote_type, E_INVALID_QUOTE);
        // Assert order size is at least minimum size for market.
        assert!(size >= order_book_ref_mut.min_size, E_SIZE_TOO_SMALL);
        // Get market underwriter ID.
        let underwriter_id = order_book_ref_mut.underwriter_id;
        // Order crosses spread if an ask and would trail behind bids
        // AVL queue head, or if a bid and would trail behind asks AVL
        // queue head.
        let crosses_spread = if (side == ASK)
            !avl_queue::would_update_head(&order_book_ref_mut.bids, price) else
            !avl_queue::would_update_head(&order_book_ref_mut.asks, price);
        // Assert order crosses spread if fill-or-abort.
        assert!(!((restriction == FILL_OR_ABORT) && !crosses_spread),
                E_FILL_OR_ABORT_NOT_CROSS_SPREAD);
        // Assert order does not cross spread if post-or-abort.
        assert!(!((restriction == POST_OR_ABORT) && crosses_spread),
                E_POST_OR_ABORT_CROSSES_SPREAD);
        // Calculate base asset amount corresponding to size in lots.
        let base = (size as u128) * (order_book_ref_mut.lot_size as u128);
        // Assert corresponding base asset amount fits in a u64.
        assert!(base <= (HI_64 as u128), E_SIZE_BASE_OVERFLOW);
        // Calculate tick amount corresponding to size in lots.
        let ticks = (size as u128) * (price as u128);
        // Assert corresponding tick amount fits in a u64.
        assert!(ticks <= (HI_64 as u128), E_SIZE_PRICE_TICKS_OVERFLOW);
        // Calculate amount of quote required to fill size at price.
        let quote = ticks * (order_book_ref_mut.tick_size as u128);
        // Assert corresponding quote amount fits in a u64.
        assert!(quote <= (HI_64 as u128), E_SIZE_PRICE_QUOTE_OVERFLOW);
        // Max base to trade is amount calculated from size, lot size.
        let max_base = (base as u64);
        // If a fill-or-abort order, must fill as a taker order with
        // a minimum trade amount equal to max base. Else no min.
        let min_base = if (restriction == FILL_OR_ABORT) max_base else 0;
        // No need to specify min quote if filling as a taker order
        // since min base is specified.
        let min_quote = 0;
        // Get max quote to trade. If price crosses spread:
        let max_quote = if (crosses_spread) { // If fills as taker:
            if (side == ASK) { // If an ask, filling as taker sell:
                // Order will fill at prices that are at least as high
                // as specified order price, and user will receive more
                // quote than calculated from order size and price.
                // Hence max quote to trade is amount that will fit in
                // market account.
                (HI_64 - quote_ceiling)
            } else { // If a bid, filling as a taker buy:
                // Order will fill at prices that are at most as high as
                // specified order price, and user will have to pay at
                // most the amount from order size and price, plus fees.
                // Since max base is marked as amount corresponding to
                // order size, matching engine will halt once enough
                // base has been filled. Hence mark that max quote to
                // trade is amount that user has available to spend, to
                // provide a buffer against integer division truncation
                // that may occur when matching engine calculates max
                // quote to match.
                quote_available
            }
        } else { // If no portion of order fills as a taker:
            (quote as u64) // Max quote is amount from size and price.
        };
        // If an ask, trade direction to range check is sell, else buy.
        let direction = if (side == ASK) SELL else BUY;
        range_check_trade( // Range check trade amounts.
            direction, min_base, max_base, min_quote, max_quote,
            base_available, base_ceiling, quote_available, quote_ceiling);
        // Assume no assets traded as a taker.
        let (base_traded, quote_traded, fees) = (0, 0, 0);
        let cancel_reason_option = option::none();
        let fill_event_queue = vector[];
        let remaining_size = size;
        if (crosses_spread) { // If order price crosses spread:
            // Calculate max base and quote to withdraw. If a buy:
            let (base_withdraw, quote_withdraw) = if (direction == BUY)
                // Withdraw quote to buy base, else sell base for quote.
                (0, max_quote) else (max_base, 0);
            // Withdraw optional base coins and quote coins for match,
            // verifying base type and quote type for market.
            let (optional_base_coins, quote_coins) =
                user::withdraw_assets_internal<BaseType, QuoteType>(
                    user_address, market_id, custodian_id, base_withdraw,
                    quote_withdraw, underwriter_id);
            // Declare return assignment variable.
            let self_match_cancel;
            // Match against order book, deferring fill events.
            (
                optional_base_coins,
                quote_coins,
                base_traded,
                quote_traded,
                fees,
                self_match_cancel,
                _,
                _
            ) = match(
                market_id,
                &mut fill_event_queue,
                order_book_ref_mut,
                user_address,
                custodian_id,
                integrator,
                direction,
                min_base,
                max_base,
                min_quote,
                max_quote,
                price,
                self_match_behavior,
                optional_base_coins,
                quote_coins
            );
            // Calculate amount of base deposited back to market account.
            let base_deposit = if (direction == BUY) base_traded else
                base_withdraw - base_traded;
            // Deposit assets back to user's market account.
            user::deposit_assets_internal<BaseType, QuoteType>(
                user_address, market_id, custodian_id, base_deposit,
                optional_base_coins, quote_coins, underwriter_id);
            // Remaining size is amount not traded during matching.
            remaining_size =
                size - (base_traded / order_book_ref_mut.lot_size);
            // Get optional order cancel reason.
            if (self_match_cancel) {
                option::fill(&mut cancel_reason_option,
                             CANCEL_REASON_SELF_MATCH_TAKER);
            } else if (remaining_size > 0) {
                if (restriction == IMMEDIATE_OR_CANCEL) {
                    option::fill(&mut cancel_reason_option,
                                 CANCEL_REASON_IMMEDIATE_OR_CANCEL);
                } else {
                    // Order still crosses spread if an ask and would
                    // trail behind bids AVL queue head, or if a bid and
                    // would trail behind asks AVL queue head: can
                    // happen if an ask (taker sell) and quote ceiling
                    // reached, or if a bid (taker buy) and all
                    // available quote spent.
                    let still_crosses_spread = if (side == ASK)
                        !avl_queue::would_update_head(
                            &order_book_ref_mut.bids, price) else
                        !avl_queue::would_update_head(
                            &order_book_ref_mut.asks, price);
                    if (still_crosses_spread) {
                        option::fill(&mut cancel_reason_option,
                                     CANCEL_REASON_MAX_QUOTE_TRADED);
                    }
                }
            };
        } else { // If spread not crossed (matching engine not called):
            // Order book counter needs to be updated for new order ID.
            order_book_ref_mut.counter = order_book_ref_mut.counter + 1;
            // IOC order needs to be cancelled if no fills took place.
            if (restriction == IMMEDIATE_OR_CANCEL) {
                option::fill(&mut cancel_reason_option,
                             CANCEL_REASON_IMMEDIATE_OR_CANCEL);
            };
        };
        // Assume that limit order will not post.
        let market_order_id =
            ((order_book_ref_mut.counter as u128) << SHIFT_COUNTER);
        // If order eligible to post:
        if (option::is_none(&cancel_reason_option) && (remaining_size > 0)) {
            // Get next order access key for user-side order placement.
            let order_access_key = user::get_next_order_access_key_internal(
                user_address, market_id, custodian_id, side);
            // Get orders AVL queue for maker side.
            let orders_ref_mut = if (side == ASK)
                &mut order_book_ref_mut.asks else &mut order_book_ref_mut.bids;
            // Declare order to insert to book.
            let order = Order{size: remaining_size, price, user: user_address,
                              custodian_id, order_access_key};
            // Get new AVL queue access key, evictee access key, and evictee
            // value by attempting to insert for given critical height.
            let (avlq_access_key, evictee_access_key, evictee_value) =
                avl_queue::insert_check_eviction(
                    orders_ref_mut, price, order, critical_height);
            // Assert that order could be inserted to AVL queue.
            assert!(avlq_access_key != NIL, E_PRICE_TIME_PRIORITY_TOO_LOW);
            // Encode AVL queue access key in market order ID.
            market_order_id = market_order_id | (avlq_access_key as u128);
            user::place_order_internal( // Place order user-side.
                user_address, market_id, custodian_id, side, remaining_size,
                price, market_order_id, order_access_key);
            if (evictee_access_key == NIL) { // If no eviction required:
                // Destroy empty evictee value option.
                option::destroy_none(evictee_value);
            } else { // If had to evict order at AVL queue tail:
                // Unpack evicted order.
                let Order{size, price, user, custodian_id, order_access_key} =
                    option::destroy_some(evictee_value);
                // Cancel order user-side.
                user::cancel_order_internal(
                    user, market_id, custodian_id, side, size, price,
                    order_access_key, (NIL as u128), CANCEL_REASON_EVICTION);
            };
        };
        // Emit relevant events to user event handles.
        user::emit_limit_order_events_internal(
            market_id, user_address, custodian_id, integrator, side, size,
            price, restriction, self_match_behavior, remaining_size,
            market_order_id, &fill_event_queue, &cancel_reason_option);
        // Return market order ID and taker trade amounts.
        return (market_order_id, base_traded, quote_traded, fees)
    }

    /// Place a limit order, passively advancing from the best price on
    /// the given side.
    ///
    /// Computes limit order price based on a target "advance" amount
    /// specified as a percentage of the spread, or specified in ticks:
    /// if a user places an ask with a 35 percent advance, for example,
    /// the "advance price" will be computed as the minimum ask price
    /// minus 35 percent of the spread. If a bid with a 10 tick advance,
    /// the advance price becomes the maximum bid price plus 10 ticks.
    ///
    /// Returns without posting an order if the order book is empty on
    /// the specified side, or if advance amount is nonzero and the
    /// order book is empty on the other side (since the spread cannot
    /// be computed). If target advance amount, specified in ticks,
    /// exceeds the number of ticks available inside the spread,
    /// advances as much as possible without crossing the spread.
    ///
    /// To ensure passivity, a full advance corresponds to an advance
    /// price just short of completely crossing the spread: for a 100
    /// percent passive advance bid on a market where the minimum ask
    /// price is 400, the advance price is 399.
    ///
    /// After computing the advance price, places a post-or-abort limit
    /// order that aborts for a self match. Advance price is then
    /// range-checked by `place_limit_order()`.
    ///
    /// # Price calculations
    ///
    /// For a limit order to be placed on the book, it must fit in
    /// 32 bits and be nonzero. Hence no underflow checking for the
    /// bid "check price", or overflow checking for the multiplication
    /// operation during the advance amount calculation for the percent
    /// case.
    ///
    /// # Type Parameters
    ///
    /// * `BaseType`: Same as for `match()`.
    /// * `QuoteType`: Same as for `match()`.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Same as for `match()`.
    /// * `custodian_id`: Same as for `match()`.
    /// * `integrator`: Same as for `place_limit_order()`.
    /// * `side`: Same as for `place_limit_order()`.
    /// * `size`: Same as for `place_limit_order()`.
    /// * `advance_style`: `PERCENT` or `TICKS`, denoting a price
    ///   advance into the spread specified as a percent of a full
    ///   advance, or a target number of ticks into the spread.
    /// * `target_advance_amount`: If `advance_style` is `PERCENT` the
    ///   percent of the spread to advance, else the number of ticks to
    ///   advance.
    ///
    /// # Returns
    ///
    /// * `u128`: Market order ID, same as for `place_limit_order()`.
    ///
    /// # Aborts
    ///
    /// * `E_INVALID_MARKET_ID`: No market with given ID.
    /// * `E_INVALID_BASE`: Base asset type is invalid.
    /// * `E_INVALID_QUOTE`: Quote asset type is invalid.
    /// * `E_INVALID_PERCENT`: `advance_style` is `PERCENT` and
    ///   `target_advance_amount` is not less than or equal to 100.
    ///
    /// # Expected value testing
    ///
    /// * `test_place_limit_order_passive_advance_no_cross_price_ask()`
    /// * `test_place_limit_order_passive_advance_no_cross_price_bid()`
    /// * `test_place_limit_order_passive_advance_no_full_advance()`
    /// * `test_place_limit_order_passive_advance_no_start_price()`.
    /// * `test_place_limit_order_passive_advance_no_target_advance()`
    /// * `test_place_limit_order_passive_advance_percent_ask()`
    /// * `test_place_limit_order_passive_advance_percent_bid()`
    /// * `test_place_limit_order_passive_advance_ticks_ask()`
    /// * `test_place_limit_order_passive_advance_ticks_bid()`
    ///
    /// # Failure testing
    ///
    /// * `test_place_limit_order_passive_advance_invalid_base()`
    /// * `test_place_limit_order_passive_advance_invalid_market_id()`
    /// * `test_place_limit_order_passive_advance_invalid_percent()`
    /// * `test_place_limit_order_passive_advance_invalid_quote()`
    fun place_limit_order_passive_advance<
        BaseType,
        QuoteType,
    >(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        integrator: address,
        side: bool,
        size: u64,
        advance_style: bool,
        target_advance_amount: u64
    ): u128
    acquires OrderBooks {
        // Get address of resource account where order books are stored.
        let resource_address = resource_account::get_address();
        let order_books_map_ref = // Immutably borrow order books map.
            &borrow_global<OrderBooks>(resource_address).map;
        // Assert order books map has order book with given market ID.
        assert!(tablist::contains(order_books_map_ref, market_id),
                E_INVALID_MARKET_ID);
        // Immutably borrow market order book.
        let order_book_ref = tablist::borrow(order_books_map_ref, market_id);
        assert!(type_info::type_of<BaseType>() // Assert base type.
                == order_book_ref.base_type, E_INVALID_BASE);
        assert!(type_info::type_of<QuoteType>() // Assert quote type.
                == order_book_ref.quote_type, E_INVALID_QUOTE);
        // Get option-packed maximum bid and minimum ask prices.
        let (max_bid_price_option, min_ask_price_option) =
            (avl_queue::get_head_key(&order_book_ref.bids),
             avl_queue::get_head_key(&order_book_ref.asks));
        // Get best price on given side, and best price on other side.
        let (start_price_option, cross_price_option) = if (side == ASK)
            (min_ask_price_option, max_bid_price_option) else
            (max_bid_price_option, min_ask_price_option);
        // Return if there is no price to advance from.
        if (option::is_none(&start_price_option)) return (NIL as u128);
        // Get price to start advance from.
        let start_price = *option::borrow(&start_price_option);
        // If target advance amount is 0, price is start price. Else:
        let price = if (target_advance_amount == 0) start_price else {
            // Return if no cross price.
            if (option::is_none(&cross_price_option)) return (NIL as u128);
            // Get cross price.
            let cross_price = *option::borrow(&cross_price_option);
            // Calculate full advance price. If an ask:
            let full_advance_price = if (side == ASK) {
                // Check price one tick above max bid price.
                let check_price = cross_price + 1;
                // If check price is less than start price, full advance
                // goes to check price. Otherwise do not advance past
                // start price.
                if (check_price < start_price) check_price else start_price
            } else { // If a bid:
                // Check price one tick below min ask price.
                let check_price = cross_price - 1;
                // If check price greater than start price, full advance
                // goes to check price. Otherwise do not advance past
                // start price.
                if (check_price > start_price) check_price else start_price
            };
            // Calculate price. If full advance price equals start
            // price, do not advance past start price. Otherwise:
            if (full_advance_price == start_price) start_price else {
                // Calculate full advance in ticks:
                let full_advance = if (side == ASK)
                    // If an ask, calculate max decrement.
                    (start_price - full_advance_price) else
                    // If a bid, calculate max increment.
                    (full_advance_price - start_price);
                // Calculate price. If advance specified as percentage:
                if (advance_style == PERCENT) {
                    // Assert target advance amount is a valid percent.
                    assert!(target_advance_amount <= PERCENT_100,
                            E_INVALID_PERCENT);
                    // Calculate price. If target is 100 percent:
                    if (target_advance_amount == PERCENT_100)
                            // Price is full advance price.
                            full_advance_price else { // Otherwise:
                        let advance = full_advance * target_advance_amount /
                            PERCENT_100; // Calculate advance in ticks.
                        // Price is decremented by advance if an ask,
                        if (side == ASK) start_price - advance else
                            start_price + advance // Else incremented.
                    }
                } else { // Advance specified number of ticks.
                    // Calculate price. If target advance amount greater
                    // than or equal to full advance in ticks:
                    if (target_advance_amount >= full_advance)
                        // Price is full advance price. Else if an ask:
                        full_advance_price else if (side == ASK)
                            // Price is decremented by target advance
                            // amount.
                            start_price - target_advance_amount else
                            // If a bid, price incremented instead.
                            start_price + target_advance_amount
                }
            }
        }; // Price now computed.
        // Place post-or-abort limit order that aborts for self match,
        // storing market order ID.
        let (market_order_id, _, _, _) =
            place_limit_order<BaseType, QuoteType>(
                user_address, market_id, custodian_id, integrator, side, size,
                price, POST_OR_ABORT, ABORT, CRITICAL_HEIGHT);
        market_order_id // Return market order ID.
    }

    /// Place market order against order book from user market account.
    ///
    /// # Type Parameters
    ///
    /// * `BaseType`: Same as for `match()`.
    /// * `QuoteType`: Same as for `match()`.
    ///
    /// # Parameters
    ///
    /// * `user_address`: User address for market account.
    /// * `market_id`: Same as for `match()`.
    /// * `custodian_id`: Same as for `match()`.
    /// * `integrator`: Same as for `match()`.
    /// * `direction`: Same as for `match()`.
    /// * `size`: Size, in lots, to fill.
    /// * `self_match_behavior`: Same as for `match()`.
    ///
    /// # Returns
    ///
    /// * `u64`: Base asset trade amount, same as for `match()`.
    /// * `u64`: Quote coin trade amount, same as for `match()`.
    /// * `u64`: Quote coin fees paid, same as for `match()`.
    ///
    /// # Aborts
    ///
    /// * `E_INVALID_BASE`: Base asset type is invalid.
    /// * `E_INVALID_QUOTE`: Quote asset type is invalid.
    /// * `E_SIZE_TOO_SMALL`: Market order size does not meet minimum
    ///   size for market.
    /// * `E_SIZE_BASE_OVERFLOW`: The product of order size and market
    ///   lot size results in a base asset unit overflow.
    ///
    /// # Expected value testing
    ///
    /// * `test_place_market_order_max_base_below_buy_user()`
    /// * `test_place_market_order_max_base_buy_user()`
    /// * `test_place_market_order_max_base_sell_custodian()`
    /// * `test_place_market_order_max_quote_buy_custodian()`
    /// * `test_place_market_order_max_quote_sell_user()`
    /// * `test_place_market_order_max_quote_traded()`
    /// * `test_place_market_order_not_enough_liquidity()`
    /// * `test_place_market_order_remove_event_handles()`
    ///
    /// # Failure testing
    ///
    /// * `test_place_market_order_invalid_base()`
    /// * `test_place_market_order_invalid_quote()`
    /// * `test_place_market_order_size_base_overflow()`
    /// * `test_place_market_order_size_too_small()`
    fun place_market_order<
        BaseType,
        QuoteType
    >(
        user_address: address,
        market_id: u64,
        custodian_id: u64,
        integrator: address,
        direction: bool,
        size: u64,
        self_match_behavior: u8
    ): (
        u64,
        u64,
        u64
    ) acquires OrderBooks {
        // Get user's available and ceiling asset counts.
        let (_, base_available, base_ceiling, _, quote_available,
             quote_ceiling) = user::get_asset_counts_internal(
                user_address, market_id, custodian_id);
        // If asset count check does not abort, then market exists, so
        // get address of resource account for borrowing order book.
        let resource_address = resource_account::get_address();
        let order_books_map_ref_mut = // Mutably borrow order books map.
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        let order_book_ref_mut = // Mutably borrow market order book.
            tablist::borrow_mut(order_books_map_ref_mut, market_id);
        assert!(type_info::type_of<BaseType>() // Assert base type.
                == order_book_ref_mut.base_type, E_INVALID_BASE);
        assert!(type_info::type_of<QuoteType>() // Assert quote type.
                == order_book_ref_mut.quote_type, E_INVALID_QUOTE);
        // Assert order size is at least minimum size for market.
        assert!(size >= order_book_ref_mut.min_size, E_SIZE_TOO_SMALL);
        // Calculate base asset amount corresponding to size in lots.
        let base = (size as u128) * (order_book_ref_mut.lot_size as u128);
        // Assert corresponding base asset amount fits in a u64.
        assert!(base <= (HI_64 as u128), E_SIZE_BASE_OVERFLOW);
        // Get market underwriter ID.
        let underwriter_id = order_book_ref_mut.underwriter_id;
        // Max base to trade is amount calculated from size, lot size.
        let max_base = (base as u64);
        // Calculate max quote that can be traded: if a buy, quote
        // available in market account. If a sell, max quote that can
        // fit in market account.
        let max_quote = if (direction == BUY)
            quote_available else (HI_64 - quote_ceiling);
        // Set min base/quote to match as 0.
        let (min_base, min_quote) = (0, 0);
        range_check_trade( // Range check trade amounts.
            direction, min_base, max_base, min_quote, max_quote,
            base_available, base_ceiling, quote_available, quote_ceiling);
        // Calculate max base and quote to withdraw. If a buy:
        let (base_withdraw, quote_withdraw) = if (direction == BUY)
            // Withdraw quote to buy base, else sell base for quote.
            (0, max_quote) else (max_base, 0);
        // Withdraw optional base coins and quote coins for match,
        // verifying base type and quote type for market.
        let (optional_base_coins, quote_coins) =
            user::withdraw_assets_internal<BaseType, QuoteType>(
                user_address, market_id, custodian_id, base_withdraw,
                quote_withdraw, underwriter_id);
        // Calculate limit price for matching engine: 0 when selling,
        // max price possible when buying.
        let limit_price = if (direction == SELL) 0 else HI_PRICE;
        // Match against order book, deferring fill events.
        let fill_event_queue = vector[];
        let (
            optional_base_coins,
            quote_coins,
            base_traded,
            quote_traded,
            fees,
            self_match_taker_cancel,
            liquidity_gone,
            _
        ) = match(
            market_id,
            &mut fill_event_queue,
            order_book_ref_mut,
            user_address,
            custodian_id,
            integrator,
            direction,
            min_base,
            max_base,
            min_quote,
            max_quote,
            limit_price,
            self_match_behavior,
            optional_base_coins,
            quote_coins
        );
        // Get order ID from order book counter updated during matching.
        let market_order_id =
            ((order_book_ref_mut.counter as u128) << SHIFT_COUNTER);
        // Calculate amount of base deposited back to market account.
        let base_deposit = if (direction == BUY) base_traded else
            (base_withdraw - base_traded);
        // Deposit assets back to user's market account.
        user::deposit_assets_internal<BaseType, QuoteType>(
            user_address, market_id, custodian_id, base_deposit,
            optional_base_coins, quote_coins, underwriter_id);
        // Get optional cancel reason.
        let cancel_reason_option =
            get_cancel_reason_option_for_market_order_or_swap(
                self_match_taker_cancel, base_traded, max_base,
                liquidity_gone, order_book_ref_mut.lot_size, false);
        // Emit relevant events to user event handles.
        user::emit_market_order_events_internal(
            market_id, user_address, custodian_id, integrator, direction, size,
            self_match_behavior, market_order_id, &fill_event_queue,
            &cancel_reason_option);
        // Return base and quote traded by user, fees paid.
        (base_traded, quote_traded, fees)
    }

    /// Range check minimum and maximum asset trade amounts.
    ///
    /// Should be called before `match()`.
    ///
    /// # Terminology
    ///
    /// * "Inbound asset" is asset received by user.
    /// * "Outbound asset" is asset traded away by by user.
    /// * "Available asset" is the the user's holdings for either base
    ///   or quote. When trading from a user's market account,
    ///   corresponds to either `user::MarketAccount.base_available` or
    ///   `user::MarketAccount.quote_available`. When trading from a
    ///   user's `aptos_framework::coin::CoinStore` or from standalone
    ///   coins, corresponds to coin value.
    /// * "Asset ceiling" is the amount that the available asset amount
    ///   could increase to beyond its present amount, even if the
    ///   indicated trade were not executed. When trading from a user's
    ///   market account, corresponds to either
    ///   `user::MarketAccount.base_ceiling` or
    ///   `user::MarketAccount.quote_ceiling`. When trading from a
    ///   user's `aptos_framework::coin::CoinStore` or from standalone
    ///   coins, is the same as available amount.
    ///
    /// # Parameters
    ///
    /// * `direction`: `BUY` or `SELL`.
    /// * `min_base`: Minimum amount of change in base holdings after
    ///   trade.
    /// * `max_base`: Maximum amount of change in base holdings after
    ///   trade.
    /// * `min_quote`: Minimum amount of change in quote holdings after
    ///   trade.
    /// * `max_quote`: Maximum amount of change in quote holdings after
    ///   trade.
    /// * `base_available`: Available base asset amount.
    /// * `base_ceiling`: Base asset ceiling, only checked when a `BUY`.
    /// * `quote_available`: Available quote asset amount.
    /// * `quote_ceiling`: Quote asset ceiling, only checked when a
    ///   `SELL`.
    ///
    /// # Aborts
    ///
    /// * `E_MAX_BASE_0`: Maximum base trade amount specified as 0.
    /// * `E_MAX_QUOTE_0`: Maximum quote trade amount specified as 0.
    /// * `E_MIN_BASE_EXCEEDS_MAX`: Minimum base trade amount is larger
    ///   than maximum base trade amount.
    /// * `E_MIN_QUOTE_EXCEEDS_MAX`: Minimum quote trade amount is
    ///   larger than maximum quote trade amount.
    /// * `E_OVERFLOW_ASSET_IN`: Filling order would overflow asset
    ///   received from trade.
    /// * `E_NOT_ENOUGH_ASSET_OUT`: Not enough asset to trade away.
    ///
    /// # Failure testing
    ///
    /// * `test_range_check_trade_asset_in_buy()`
    /// * `test_range_check_trade_asset_in_sell()`
    /// * `test_range_check_trade_asset_out_buy()`
    /// * `test_range_check_trade_asset_out_sell()`
    /// * `test_range_check_trade_base_0()`
    /// * `test_range_check_trade_min_base_exceeds_max()`
    /// * `test_range_check_trade_min_quote_exceeds_max()`
    /// * `test_range_check_trade_quote_0()`
    fun range_check_trade(
        direction: bool,
        min_base: u64,
        max_base: u64,
        min_quote: u64,
        max_quote: u64,
        base_available: u64,
        base_ceiling: u64,
        quote_available: u64,
        quote_ceiling: u64
    ) {
        // Assert nonzero max base trade amount.
        assert!(max_base > 0, E_MAX_BASE_0);
        // Assert nonzero max quote trade amount.
        assert!(max_quote > 0, E_MAX_QUOTE_0);
        // Assert minimum base less than or equal to maximum.
        assert!(min_base <= max_base, E_MIN_BASE_EXCEEDS_MAX);
        // Assert minimum quote less than or equal to maximum.
        assert!(min_quote <= max_quote, E_MIN_QUOTE_EXCEEDS_MAX);
        // Get inbound asset ceiling and max trade amount, outbound
        // asset available and max trade amount.
        let (in_ceiling, in_max, out_available, out_max) =
            if (direction == BUY) // If trade is in buy direction:
                // Getting base and trading away quote.
                (base_ceiling, max_base, quote_available, max_quote) else
                // Else a sell, so getting quote and trading away base.
                (quote_ceiling, max_quote, base_available, max_base);
        // Calculate maximum possible inbound asset ceiling post-match.
        let in_ceiling_max = (in_ceiling as u128) + (in_max as u128);
        // Assert max possible inbound asset ceiling does not overflow.
        assert!(in_ceiling_max <= (HI_64 as u128), E_OVERFLOW_ASSET_IN);
        // Assert enough outbound asset to cover max trade amount.
        assert!(out_max <= out_available, E_NOT_ENOUGH_ASSET_OUT);
    }

    /// Register order book, fee store under Econia resource account.
    ///
    /// Should only be called by `register_market_base_coin()` or
    /// `register_market_base_generic()`.
    ///
    /// See `registry::MarketInfo` for commentary on lot size, tick
    /// size, minimum size, and 32-bit prices.
    ///
    /// # Type parameters
    ///
    /// * `BaseType`: Base type for market.
    /// * `QuoteType`: Quote coin type for market.
    ///
    /// # Parameters
    ///
    /// * `market_id`: Market ID for new market.
    /// * `base_name_generic`: `registry::MarketInfo.base_name_generic`
    ///   for market.
    /// * `lot_size`: `registry::MarketInfo.lot_size` for market.
    /// * `tick_size`: `registry::MarketInfo.tick_size` for market.
    /// * `min_size`: `registry::MarketInfo.min_size` for market.
    /// * `underwriter_id`: `registry::MarketInfo.min_size` for market.
    ///
    /// # Returns
    ///
    /// * `u64`: Market ID for new market.
    ///
    /// # Testing
    ///
    /// * `test_register_markets()`
    fun register_market<
        BaseType,
        QuoteType
    >(
        market_id: u64,
        base_name_generic: String,
        lot_size: u64,
        tick_size: u64,
        min_size: u64,
        underwriter_id: u64
    ): u64
    acquires OrderBooks {
        // Get Econia resource account signer.
        let resource_account = resource_account::get_signer();
        // Get resource account address.
        let resource_address = address_of(&resource_account);
        let order_books_map_ref_mut = // Mutably borrow order books map.
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        // Add order book entry to order books map.
        tablist::add(order_books_map_ref_mut, market_id, OrderBook{
            base_type: type_info::type_of<BaseType>(),
            base_name_generic,
            quote_type: type_info::type_of<QuoteType>(),
            lot_size,
            tick_size,
            min_size,
            underwriter_id,
            asks: avl_queue::new<Order>(ASCENDING, 0, 0),
            bids: avl_queue::new<Order>(DESCENDING, 0, 0),
            counter: 0,
            maker_events:
                account::new_event_handle<MakerEvent>(&resource_account),
            taker_events:
                account::new_event_handle<TakerEvent>(&resource_account)});
        // Register an Econia fee store entry for market quote coin.
        incentives::register_econia_fee_store_entry<QuoteType>(market_id);
        market_id // Return market ID.
    }

    /// Match a taker's swap order against order book for given market.
    ///
    /// # Type Parameters
    ///
    /// * `BaseType`: Same as for `match()`.
    /// * `QuoteType`: Same as for `match()`.
    ///
    /// # Parameters
    ///
    /// * `fill_event_queue_ref_mut`: Mutable reference to vector for
    ///   enqueueing deferred `user::FillEvent`(s).
    /// * `signer_address`: Address of signing user if applicable, else
    ///   `NO_TAKER_ADDRESS`.
    /// * `market_id`: Same as for `match()`.
    /// * `underwriter_id`: ID of underwriter to verify if `BaseType`
    ///   is `registry::GenericAsset`, else may be passed as
    ///   `NO_UNDERWRITER`.
    /// * `integrator`: Same as for `match()`.
    /// * `direction`: Same as for `match()`.
    /// * `min_base`: Same as for `match()`.
    /// * `max_base`: Same as for `match()`.
    /// * `min_quote`: Same as for `match()`.
    /// * `max_quote`: Same as for `match()`.
    /// * `limit_price`: Same as for `match()`.
    /// * `optional_base_coins`: Same as for `match()`.
    /// * `quote_coins`: Same as for `match()`.
    ///
    /// # Returns
    ///
    /// * `Option<Coin<BaseType>>`: Optional updated base coin holdings,
    ///   same as for `match()`.
    /// * `Coin<QuoteType>`: Updated quote coin holdings, same as for
    ///   `match()`.
    /// * `u64`: Base asset trade amount, same as for `match()`.
    /// * `u64`: Quote coin trade amount, same as for `match()`.
    /// * `u64`: Quote coin fees paid, same as for `match()`.
    /// * `Option<PlaceSwapOrderEvent>`: `PlaceSwapOrderEvent` to emit
    ///   if swap is from a signing swapper.
    /// * `Option<user::CancelOrderEvent>`: Optional
    ///   `user::CancelOrderEvent` to emit if swap is from a signing
    ///   swapper.
    ///
    /// # Emits
    ///
    /// * `PlaceSwapOrderEvent`: Information about swap order, emitted
    ///   when swap is from a non-signing swapper.
    /// * `user::CancelOrderEvent`: Information about order
    ///   cancellation, if order was cancelled without completely
    ///   filling, when swap is from non-signing swapper.
    ///
    /// # Aborts
    ///
    /// * `E_INVALID_MARKET_ID`: No market with given ID.
    /// * `E_INVALID_UNDERWRITER`: Underwriter invalid for given market.
    /// * `E_INVALID_BASE`: Base asset type is invalid.
    /// * `E_INVALID_QUOTE`: Quote asset type is invalid.
    ///
    /// # Expected value testing
    ///
    /// * Covered by `swap_between_coinstores()`, `swap_coins()`, and
    ///   `swap_generic()` testing.
    ///
    /// # Failure testing
    ///
    /// * `test_swap_invalid_base()`
    /// * `test_swap_invalid_market_id()`
    /// * `test_swap_invalid_quote()`
    /// * `test_swap_invalid_underwriter()`
    fun swap<
        BaseType,
        QuoteType
    >(
        fill_event_queue_ref_mut: &mut vector<FillEvent>,
        signer_address: address,
        market_id: u64,
        underwriter_id: u64,
        integrator: address,
        direction: bool,
        min_base: u64,
        max_base: u64,
        min_quote: u64,
        max_quote: u64,
        limit_price: u64,
        optional_base_coins: Option<Coin<BaseType>>,
        quote_coins: Coin<QuoteType>
    ): (
        Option<Coin<BaseType>>,
        Coin<QuoteType>,
        u64,
        u64,
        u64,
        Option<PlaceSwapOrderEvent>,
        Option<CancelOrderEvent>
    ) acquires
        MarketEventHandles,
        OrderBooks
    {
        // Get address of resource account where order books are stored.
        let resource_address = resource_account::get_address();
        let order_books_map_ref_mut = // Mutably borrow order books map.
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        // Assert order books map has order book with given market ID.
        assert!(tablist::contains(order_books_map_ref_mut, market_id),
                E_INVALID_MARKET_ID);
        let order_book_ref_mut = // Mutably borrow market order book.
            tablist::borrow_mut(order_books_map_ref_mut, market_id);
        // If passed an underwriter ID, verify it matches market.
        if (underwriter_id != NO_UNDERWRITER)
            assert!(underwriter_id == order_book_ref_mut.underwriter_id,
                    E_INVALID_UNDERWRITER);
        assert!(type_info::type_of<BaseType>() // Assert base type.
                == order_book_ref_mut.base_type, E_INVALID_BASE);
        assert!(type_info::type_of<QuoteType>() // Assert quote type.
                == order_book_ref_mut.quote_type, E_INVALID_QUOTE);
        // Match against order book, deferring fill events.
        let (
            optional_base_coins,
            quote_coins,
            base_traded,
            quote_traded,
            fees,
            self_match_taker_cancel,
            liquidity_gone,
            violated_limit_price
        ) = match(
            market_id,
            fill_event_queue_ref_mut,
            order_book_ref_mut,
            signer_address,
            NO_CUSTODIAN,
            integrator,
            direction,
            min_base,
            max_base,
            min_quote,
            max_quote,
            limit_price,
            CANCEL_TAKER,
            optional_base_coins,
            quote_coins
        );
        // Get order ID from order book counter updated during matching.
        let market_order_id =
            ((order_book_ref_mut.counter as u128) << SHIFT_COUNTER);
        // Create market event handles for market as needed.
        if (!exists<MarketEventHandles>(resource_address))
            move_to(&resource_account::get_signer(),
                    MarketEventHandles{map: table::new()});
        let market_event_handles_map_ref_mut =
            &mut borrow_global_mut<MarketEventHandles>(resource_address).map;
        let has_handles =
            table::contains(market_event_handles_map_ref_mut, market_id);
        if (!has_handles) {
            let resource_signer = resource_account::get_signer();
            let handles = MarketEventHandlesForMarket{
                cancel_order_events:
                    account::new_event_handle(&resource_signer),
                place_swap_order_events:
                    account::new_event_handle(&resource_signer)
            };
            table::add(
                market_event_handles_map_ref_mut, market_id, handles);
        };
        let handles_ref_mut =
            table::borrow_mut(market_event_handles_map_ref_mut, market_id);
        // Create market events as necessary.
        let place_swap_order_event = PlaceSwapOrderEvent{
            market_id,
            signing_account: signer_address,
            integrator,
            direction,
            min_base,
            max_base,
            min_quote,
            max_quote,
            limit_price,
            order_id: market_order_id
        };
        let cancel_reason_option =
            get_cancel_reason_option_for_market_order_or_swap(
                self_match_taker_cancel, base_traded, max_base,
                liquidity_gone, order_book_ref_mut.lot_size,
                violated_limit_price);
        let need_to_cancel = option::is_some(&cancel_reason_option);
        let cancel_order_event_option = if (need_to_cancel)
            option::some(user::create_cancel_order_event_internal(
                market_id, market_order_id, signer_address, NO_CUSTODIAN,
                option::destroy_some(cancel_reason_option))) else
            option::none();
        // Assume do not need to return place swap order event.
        let place_swap_order_event_option = option::none();
        // If swap not placed by a signing swapper:
        if (signer_address == NO_TAKER_ADDRESS) {
            event::emit_event(&mut handles_ref_mut.place_swap_order_events,
                              place_swap_order_event);
            if (need_to_cancel) event::emit_event(
                &mut handles_ref_mut.cancel_order_events,
                option::extract(&mut cancel_order_event_option));
        } else { // Otherwise swap order placed by signing swapper.
            option::fill(&mut place_swap_order_event_option,
                         place_swap_order_event);
        };
        user::emit_swap_maker_fill_events_internal(fill_event_queue_ref_mut);
        // Return optionally modified asset inputs, trade amounts, fees,
        // place swap order event option, and cancel order event option.
        (optional_base_coins, quote_coins, base_traded, quote_traded, fees,
         place_swap_order_event_option, cancel_order_event_option)
    }

    /// Verify pagination function order IDs are valid for market.
    ///
    /// # Failure testing
    ///
    /// * `test_verify_pagination_order_ids_ask_does_not_exist()`
    /// * `test_verify_pagination_order_ids_ask_wrong_side()`
    /// * `test_verify_pagination_order_ids_bid_does_not_exist()`
    /// * `test_verify_pagination_order_ids_bid_wrong_side()`
    // fun verify_pagination_order_ids(
    //     market_id: u64,
    //     starting_ask_order_id: u128,
    //     starting_bid_order_id: u128,
    // ) acquires OrderBooks {
    //     if (starting_ask_order_id != (NIL as u128)) {
    //         assert!(has_open_order(market_id, starting_ask_order_id),
    //                 E_INVALID_MARKET_ORDER_ID);
    //         assert!(get_posted_order_id_side(starting_ask_order_id) == ASK,
    //                 E_INVALID_MARKET_ORDER_ID);
    //     };
    //     if (starting_bid_order_id != (NIL as u128)) {
    //         assert!(has_open_order(market_id, starting_bid_order_id),
    //                 E_INVALID_MARKET_ORDER_ID);
    //         assert!(get_posted_order_id_side(starting_bid_order_id) == BID,
    //                 E_INVALID_MARKET_ORDER_ID);
    //     };
    // }

    // Private functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Deprecated structs >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Deprecated struct retained for compatible upgrade policy.
    struct MakerEvent has drop, store {
        market_id: u64,
        side: bool,
        market_order_id: u128,
        user: address,
        custodian_id: u64,
        type: u8,
        size: u64,
        price: u64
    }

    /// Deprecated struct retained for compatible upgrade policy.
    struct Orders has key {asks: vector<Order>, bids: vector<Order>}

    /// Deprecated struct retained for compatible upgrade policy.
    struct TakerEvent has drop, store {
        market_id: u64,
        side: bool,
        market_order_id: u128,
        maker: address,
        custodian_id: u64,
        size: u64,
        price: u64
    }

    // Deprecated structs <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Deprecated functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Deprecated functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Deprecated function retained for compatible upgrade policy.
    ///
    /// # Coverage testing
    ///
    /// * `test_index_orders_sdk_coverage()`
    public entry fun index_orders_sdk(_0: &signer, _1: u64) {}

    // Deprecated functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
    // Tests <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

}