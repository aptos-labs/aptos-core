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
    /// Return true if the order ID corresponds to an order that
    /// resulted in a post to the order book (including an order that
    /// filled across the spread as a taker before posting as a maker).
    ///
    /// # Testing
    ///
    /// * `test_get_market_order_id_price_did_not_post()`
    /// * `test_get_market_order_id_side_did_not_post()`
    /// * `test_place_limit_order_no_cross_ask_user()`
    /// * `test_place_limit_order_no_cross_bid_custodian()`
    public fun did_order_post(
        order_id: u128
    ): bool {
        (order_id & (HI_64 as u128)) != (NIL as u128)
    }

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

    #[view]
    /// Return order counter encoded in market order ID.
    ///
    /// # Testing
    ///
    /// * `test_place_limit_order_no_cross_ask_user()`
    /// * `test_place_limit_order_no_cross_bid_custodian()`
    public fun get_market_order_id_counter(
        market_order_id: u128
    ): u64 {
        (((market_order_id >> SHIFT_COUNTER) & (HI_64 as u128)) as u64)
    }

    #[view]
    /// For an order that resulted in a post to the order book, return
    /// the order price encoded in its market order ID, corresponding to
    /// the price that the maker portion of the order posted to the book
    /// at.
    ///
    /// # Aborts
    ///
    /// * `E_ORDER_DID_NOT_POST`: Order ID corresponds to an order that
    ///   did not post to the book.
    ///
    /// # Testing
    ///
    /// * `test_get_market_order_id_price_did_not_post()`
    /// * `test_place_limit_order_no_cross_ask_user()`
    /// * `test_place_limit_order_no_cross_bid_custodian()`
    public fun get_market_order_id_price(
        market_order_id: u128
    ): u64 {
        // Assert order posted to the order book.
        assert!(did_order_post(market_order_id), E_ORDER_DID_NOT_POST);
        // Extract encoded price.
        ((market_order_id & (HI_PRICE as u128)) as u64)
    }

    #[view]
    /// Return `OrderView` for `market_id` and `order_id`.
    ///
    /// Mutates state, so kept as a private view function.
    ///
    /// # Testing
    ///
    /// * `test_change_order_size_ask_custodian()`
    /// * `test_change_order_size_bid_user()`
    /// * `test_get_open_order_no_such_order()`
    fun get_open_order(
        market_id: u64,
        order_id: u128
    ): Option<OrderView>
    acquires OrderBooks {
        // Return empty option if no such order.
        if (!has_open_order(market_id, order_id)) return option::none();
        // Get address of resource account where order books are stored.
        let resource_address = resource_account::get_address();
        // Immutably borrow order books map.
        let order_books_map_ref =
            &borrow_global<OrderBooks>(resource_address).map;
        // Immutably borrow market order book.
        let order_book_ref = tablist::borrow(
            order_books_map_ref, market_id);
        // Get order ID side.
        let side = get_posted_order_id_side(order_id);
        // Get open orders for given side.
        let orders_ref = if (side == ASK) &order_book_ref.asks else
            &order_book_ref.bids;
        let avlq_access_key = // Get AVL queue access key.
            get_order_id_avl_queue_access_key(order_id);
        // Immutably borrow order with given access key.
        let order_ref = avl_queue::borrow(orders_ref, avlq_access_key);
        // Pack and return an order view in an option.
        option::some(OrderView{
            market_id,
            side,
            order_id,
            remaining_size: order_ref.size,
            price: order_ref.price,
            user: order_ref.user,
            custodian_id: order_ref.custodian_id})
    }

    #[view]
    /// Index order book for given market ID into ask and bid vectors.
    ///
    /// Vectors sorted by price-time priority per `OrdersView` schema.
    ///
    /// Mutates state, so kept as a private view function.
    ///
    /// # Parameters
    ///
    /// * `market_id`: Market ID of maker orders to index.
    /// * `n_asks_max`: Maximum number of asks to index.
    /// * `n_bids_max`: Maximum number of bids to index.
    ///
    /// # Aborts
    ///
    /// * `E_INVALID_MARKET_ID`: No market with given ID.
    ///
    /// # Testing
    ///
    /// * `test_get_open_orders()`
    /// * `test_get_open_orders_invalid_market_id()`
    fun get_open_orders(
        market_id: u64,
        n_asks_max: u64,
        n_bids_max: u64
    ): OrdersView
    acquires OrderBooks {
        // Get address of resource account where order books are stored.
        let resource_address = resource_account::get_address();
        let order_books_map_ref_mut = // Mutably borrow order books map.
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        // Assert order books map has order book with given market ID.
        assert!(tablist::contains(order_books_map_ref_mut, market_id),
                E_INVALID_MARKET_ID);
        // Mutably borrow order book with given market ID.
        let order_book_ref_mut =
            tablist::borrow_mut(order_books_map_ref_mut, market_id);
        OrdersView{ // Return indexed orders.
            asks: get_open_orders_for_side(
                market_id, order_book_ref_mut, ASK, n_asks_max),
            bids: get_open_orders_for_side(
                market_id, order_book_ref_mut, BID, n_bids_max)
        }
    }

    #[view]
    /// Wrapped call to `get_open_orders()` for getting all open orders
    /// on both sides.
    ///
    /// For a sufficiently large order book this function may fail due
    /// to execution gas limits. Hence `get_open_orders_paginated()` is
    /// recommended during production.
    ///
    /// # Testing
    ///
    /// * `test_get_open_orders()`
    fun get_open_orders_all(
        market_id: u64
    ): OrdersView
    acquires OrderBooks {
        get_open_orders(market_id, HI_64, HI_64)
    }

    #[view]
    /// Like `get_open_orders()`, but paginated.
    ///
    /// Kept as private view function to prevent runtime state
    /// contention.
    ///
    /// When paginating via an SDK, specify the same transaction version
    /// number for each function call until done paginating.
    ///
    /// # Parameters
    ///
    /// * `market_id`: Market ID of maker orders to index.
    /// * `n_asks_to_index_max`: Maximum number of asks to index.
    /// * `n_bids_to_index_max`: Maximum number of bids to index.
    /// * `starting_ask_order_id`: Order ID of ask to start indexing
    ///   from. If `NIL`, start from best ask.
    /// * `starting_bid_order_id`: Order ID of bid to start indexing
    ///   from. If `NIL`, start from best bid.
    ///
    /// # Returns
    ///
    /// * `PriceLevels`: Price level vectors.
    /// * `u128`: Order ID for next ask to start indexing from. `NIL` if
    ///   done indexing asks.
    /// * `u128`: Order ID for next bid to start indexing from. `NIL` if
    ///   done indexing bids.
    ///
    /// # Expected value testing
    ///
    /// * `test_get_open_orders_paginated()`
    ///
    /// # Failure testing
    ///
    /// * `test_get_open_orders_paginated_invalid_market_id()`
    fun get_open_orders_paginated(
        market_id: u64,
        n_asks_to_index_max: u64,
        n_bids_to_index_max: u64,
        starting_ask_order_id: u128,
        starting_bid_order_id: u128,
    ): (
        OrdersView,
        u128,
        u128,
    ) acquires OrderBooks {
        verify_pagination_order_ids( // Verify order IDs.
            market_id, starting_ask_order_id, starting_bid_order_id);
        // Get address of resource account where order books are stored.
        let resource_address = resource_account::get_address();
        let order_books_map_ref = // Immutably borrow order books map.
            &borrow_global<OrderBooks>(resource_address).map;
        // Assert order books map has order book with given market ID.
        assert!(tablist::contains(order_books_map_ref, market_id),
                E_INVALID_MARKET_ID);
        // Immutably borrow order book with given market ID.
        let order_book_ref =
            tablist::borrow(order_books_map_ref, market_id);
        let (asks, ask_next) = get_open_orders_for_side_paginated(
            order_book_ref, market_id, ASK, n_asks_to_index_max,
            starting_ask_order_id);
        let (bids, bid_next) = get_open_orders_for_side_paginated(
            order_book_ref, market_id, BID, n_bids_to_index_max,
            starting_bid_order_id);
        (OrdersView{asks, bids}, ask_next, bid_next)
    }

    #[view]
    /// For an order that resulted in a post to the order book, return
    /// the order side encoded in its order ID, corresponding to the
    /// side that the maker portion of the order posted to the book at.
    ///
    /// # Aborts
    ///
    /// * `E_ORDER_DID_NOT_POST`: Order ID corresponds to an order that
    ///   did not post to the book.
    ///
    /// # Testing
    ///
    /// * `test_get_market_order_id_side_did_not_post()`
    /// * `test_place_limit_order_no_cross_ask_user()`
    /// * `test_place_limit_order_no_cross_bid_custodian()`
    public fun get_posted_order_id_side(
        order_id: u128
    ): bool {
        // Assert order posted to the order book.
        assert!(did_order_post(order_id), E_ORDER_DID_NOT_POST);
        // Get AVL queue access key encoded in order ID.
        let avlq_access_key =
            get_order_id_avl_queue_access_key(order_id);
        // If ascending AVL queue indicated is an ask, else a bid.
        if (avl_queue::is_ascending_access_key(avlq_access_key)) ASK else BID
    }

    #[view]
    /// Index order book for given market ID into price level vectors.
    ///
    /// Vectors sorted by price priority per `PriceLevels` schema.
    ///
    /// Mutates state, so kept as a private view function.
    ///
    /// # Parameters
    ///
    /// * `market_id`: Market ID of price levels to index.
    /// * `n_ask_levels_max`: Maximum number of ask price levels to
    ///   index.
    /// * `n_bid_levels_max`: Maximum number of bid price levels to
    ///   index.
    ///
    /// # Testing
    ///
    /// * `test_get_price_levels()`
    /// * `test_get_price_levels_invalid_market_id()`
    fun get_price_levels(
        market_id: u64,
        n_ask_levels_max: u64,
        n_bid_levels_max: u64
    ): PriceLevels
    acquires OrderBooks {
        // Get address of resource account where order books are stored.
        let resource_address = resource_account::get_address();
        let order_books_map_ref_mut = // Mutably borrow order books map.
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        // Assert order books map has order book with given market ID.
        assert!(tablist::contains(order_books_map_ref_mut, market_id),
                E_INVALID_MARKET_ID);
        // Mutably borrow order book with given market ID.
        let order_book_ref_mut =
            tablist::borrow_mut(order_books_map_ref_mut, market_id);
        PriceLevels{ // Return indexed price levels.
            market_id,
            asks: get_price_levels_for_side(
                order_book_ref_mut, ASK, n_ask_levels_max),
            bids: get_price_levels_for_side(
                order_book_ref_mut, BID, n_bid_levels_max)
        }
    }

    #[view]
    /// Wrapped call to `get_price_levels()` for getting all price
    /// levels on both sides.
    ///
    /// For a sufficiently large order book this function may fail due
    /// to execution gas limits. Hence `get_price_levels_paginated()` is
    /// recommended during production.
    ///
    /// # Testing
    ///
    /// * `test_get_price_levels()`
    fun get_price_levels_all(
        market_id: u64
    ): PriceLevels
    acquires OrderBooks {
        get_price_levels(market_id, HI_64, HI_64)
    }

    #[view]
    /// Like `get_price_levels()`, but paginated.
    ///
    /// Kept as private view function to prevent runtime state
    /// contention.
    ///
    /// When paginating via an SDK, specify the same transaction version
    /// number for each function call until done paginating.
    ///
    /// # Parameters
    ///
    /// * `market_id`: Market ID of price levels to index.
    /// * `n_asks_to_index_max`: Maximum number of asks to index.
    /// * `n_bids_to_index_max`: Maximum number of bids to index.
    /// * `starting_ask_order_id`: Order ID of ask to start indexing
    ///   from. If `NIL`, start from best ask.
    /// * `starting_bid_order_id`: Order ID of bid to start indexing
    ///   from. If `NIL`, start from best bid.
    ///
    /// # Returns
    ///
    /// * `PriceLevels`: Price level vectors.
    /// * `u128`: Order ID for next ask to start indexing from. `NIL` if
    ///   done indexing asks.
    /// * `u128`: Order ID for next bid to start indexing from. `NIL` if
    ///   done indexing bids.
    ///
    /// # Expected value testing
    ///
    /// * `test_get_price_levels_paginated()`
    ///
    /// # Failure testing
    ///
    /// * `test_get_price_levels_paginated_invalid_market_id()`
    fun get_price_levels_paginated(
        market_id: u64,
        n_asks_to_index_max: u64,
        n_bids_to_index_max: u64,
        starting_ask_order_id: u128,
        starting_bid_order_id: u128,
    ): (
        PriceLevels,
        u128,
        u128,
    ) acquires OrderBooks {
        verify_pagination_order_ids( // Verify order IDs.
            market_id, starting_ask_order_id, starting_bid_order_id);
        // Get address of resource account where order books are stored.
        let resource_address = resource_account::get_address();
        let order_books_map_ref = // Immutably borrow order books map.
            &borrow_global<OrderBooks>(resource_address).map;
        // Assert order books map has order book with given market ID.
        assert!(tablist::contains(order_books_map_ref, market_id),
                E_INVALID_MARKET_ID);
        // Immutably borrow order book with given market ID.
        let order_book_ref =
            tablist::borrow(order_books_map_ref, market_id);
        let (ask_levels, ask_next) = get_price_levels_for_side_paginated(
            order_book_ref, market_id, ASK, n_asks_to_index_max,
            starting_ask_order_id);
        let (bid_levels, bid_next) = get_price_levels_for_side_paginated(
            order_book_ref, market_id, BID, n_bids_to_index_max,
            starting_bid_order_id);
        (
            PriceLevels{
                market_id,
                asks: ask_levels,
                bids: bid_levels
            },
            ask_next,
            bid_next
        )
    }

    #[view]
    /// Return a `SwapperEventHandleCreationNumbers` for `market_id`, if
    /// signing `swapper` has event handles for indicated market.
    ///
    /// Restricted to private view function to prevent runtime handle
    /// contention.
    ///
    /// # Testing
    ///
    /// * `test_swap_between_coinstores_register_base_store()`
    fun get_swapper_event_handle_creation_numbers(
        swapper: address,
        market_id: u64
    ): Option<SwapperEventHandleCreationNumbers>
    acquires SwapperEventHandles {
        // Return none if swapper does not have event handles map.
        if (!exists<SwapperEventHandles>(swapper)) return option::none();
        // Return none if no handles exist for market.
        let swapper_event_handles_map_ref =
            &borrow_global<SwapperEventHandles>(swapper).map;
        let has_handles = table::contains(
            swapper_event_handles_map_ref, market_id);
        if (!has_handles) return option::none();
        let swapper_handles_ref = table::borrow(
            swapper_event_handles_map_ref, market_id);
        // Return option-packed creation numbers for market.
        option::some(SwapperEventHandleCreationNumbers{
            cancel_order_events_handle_creation_num:
                guid::creation_num(event::guid(
                    &swapper_handles_ref.cancel_order_events)),
            fill_events_handle_creation_num:
                guid::creation_num(event::guid(
                    &swapper_handles_ref.fill_events)),
            place_swap_order_events_handle_creation_num:
                guid::creation_num(event::guid(
                    &swapper_handles_ref.place_swap_order_events))
        })
    }

    #[view]
    /// Return `true` if `order_id` corresponds to open order for given
    /// `market_id`.
    ///
    /// Kept private to prevent runtime order book state contention.
    ///
    /// # Testing
    ///
    /// * `test_change_order_size_ask_custodian()`
    /// * `test_change_order_size_bid_user()`
    /// * `test_has_open_order_no_market()`
    fun has_open_order(
        market_id: u64,
        order_id: u128
    ): bool
    acquires OrderBooks {
        // Get address of resource account where order books are stored.
        let resource_address = resource_account::get_address();
        let order_books_map_ref = // Immutably borrow order books map.
            &borrow_global<OrderBooks>(resource_address).map;
        if (!tablist::contains(order_books_map_ref, market_id))
            return false; // Return false if no market with market ID.
        // Immutably borrow order book for given market ID.
        let order_book_ref = tablist::borrow(order_books_map_ref, market_id);
        // Determine side indicated by order ID.
        let side = get_posted_order_id_side(order_id);
        // Get open orders for given side.
        let orders_ref = if (side == ASK) &order_book_ref.asks
            else &order_book_ref.bids;
        // Get AVL queue access key from order ID.
        let avlq_access_key =
            get_order_id_avl_queue_access_key(order_id);
        // Check if borrowing from the AVL queue is even possible.
        let borrow_possible = avl_queue::contains_active_list_node_id(
            orders_ref, avlq_access_key);
        // Return false if can't borrow from the AVL queue.
        if (!borrow_possible) return false;
        // Immutably borrow order having list node ID.
        let order_ref = avl_queue::borrow(orders_ref, avlq_access_key);
        // Check if user has corresponding open order order ID.
        let optional_order_id = user::get_open_order_id_internal(
            order_ref.user, market_id, order_ref.custodian_id, side,
            order_ref.order_access_key);
        // If user has no corresponding order ID return false.
        if (option::is_none(&optional_order_id)) return false;
        let user_order_id = // Get user's order ID.
            option::destroy_some(optional_order_id);
        // Return if user-indicated order ID matches passed one.
        user_order_id == order_id
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
        assert!(removal_possible, E_INVALID_MARKET_ORDER_ID);
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
    fun verify_pagination_order_ids(
        market_id: u64,
        starting_ask_order_id: u128,
        starting_bid_order_id: u128,
    ) acquires OrderBooks {
        if (starting_ask_order_id != (NIL as u128)) {
            assert!(has_open_order(market_id, starting_ask_order_id),
                    E_INVALID_MARKET_ORDER_ID);
            assert!(get_posted_order_id_side(starting_ask_order_id) == ASK,
                    E_INVALID_MARKET_ORDER_ID);
        };
        if (starting_bid_order_id != (NIL as u128)) {
            assert!(has_open_order(market_id, starting_bid_order_id),
                    E_INVALID_MARKET_ORDER_ID);
            assert!(get_posted_order_id_side(starting_bid_order_id) == BID,
                    E_INVALID_MARKET_ORDER_ID);
        };
    }

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

    #[test_only]
    /// Immutably borrow market event handles for a market.
    inline fun borrow_market_event_handles_for_market_test(
        market_id: u64
    ): &MarketEventHandlesForMarket
    acquires MarketEventHandles {
        let market_event_handles_map_ref =
            &borrow_global<MarketEventHandles>(
                resource_account::get_address()).map;
        table::borrow(market_event_handles_map_ref, market_id)
    }

    #[test_only]
    /// Immutably borrow swapper event handles for a market.
    inline fun borrow_swapper_event_handles_for_market_test(
        market_id: u64,
        swapper: address
    ): &SwapperEventHandlesForMarket
    acquires SwapperEventHandles {
        let swapper_event_handles_map_ref =
            &borrow_global<SwapperEventHandles>(swapper).map;
        table::borrow(swapper_event_handles_map_ref, market_id)
    }

    #[test_only]
    /// Assuming order placed by `@user_0` on `MARKET_ID_COIN`, verify
    /// order fields.
    public fun check_order_fields_test(
        market_order_id: u128,
        side: bool,
        size: u64,
        price: u64,
        custodian_id: u64
    ) acquires OrderBooks {
        // Get order fields from book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id);
        // Assert returns.
        assert!(size_r         == size, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == custodian_id, 0);
        // Check fields user-side.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, custodian_id, side, order_access_key);
        // Assert returns.
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r            == size, 0);
    }

    #[test_only]
    /// Configure spread for `MARKET_ID_COIN` with `@user_0` placing a
    /// bid of price `max_bid_price` and an ask of price
    /// `min_ask_price`, placing no order for given side if indicated
    /// price is `NIL`. Provide user with enough collateral to cover
    /// trades and to place more later.
    public fun configure_spread_test(
        max_bid_price: u64,
        min_ask_price: u64
    ): signer
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user, _) = init_markets_users_integrator_test();
        let user_address = address_of(&user); // Get user address.
        // Deposit coins to user's market account.
        user::deposit_coins<BC>(user_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 / 2));
        user::deposit_coins<QC>(user_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 / 2));
        // Place orders accordingly.
        if (max_bid_price != NIL) {place_limit_order_user<BC, QC>(
                &user, MARKET_ID_COIN, @integrator, BID, MIN_SIZE_COIN,
                max_bid_price, NO_RESTRICTION, ABORT);};
        if (min_ask_price != NIL) {place_limit_order_user<BC, QC>(
                &user, MARKET_ID_COIN, @integrator, ASK, MIN_SIZE_COIN,
                min_ask_price, NO_RESTRICTION, ABORT);};
        user // Return user signature.
    }

    #[test_only]
    /// Return `true` if `MarketEventHandles` exists at resource
    /// account.
    public fun exists_market_event_handles(): bool {
        exists<MarketEventHandles>(resource_account::get_address())
    }

    #[test_only]
    /// Get `CancelOrderEvent`s at market level.
    public fun get_cancel_order_events_market_test(
        market_id: u64
    ): vector<CancelOrderEvent>
    acquires MarketEventHandles {
        event::emitted_events_by_handle(
            &(borrow_market_event_handles_for_market_test(market_id).
                cancel_order_events))
    }

    #[test_only]
    /// Get `CancelOrderEvent`s at swapper level.
    public fun get_cancel_order_events_swapper_test(
        market_id: u64,
        swapper: address
    ): vector<CancelOrderEvent>
    acquires SwapperEventHandles {
        event::emitted_events_by_handle(
            &(borrow_swapper_event_handles_for_market_test(market_id, swapper).
                cancel_order_events))
    }

    #[test_only]
    /// Get `FillEvent`s at swapper level.
    public fun get_fill_events_swapper_test(
        market_id: u64,
        swapper: address
    ): vector<FillEvent>
    acquires SwapperEventHandles {
        event::emitted_events_by_handle(
            &(borrow_swapper_event_handles_for_market_test(market_id, swapper).
                fill_events))
    }

    #[test_only]
    /// Get `PlaceSwapOrderEvent`s at market level.
    public fun get_place_swap_order_events_market_test(
        market_id: u64
    ): vector<PlaceSwapOrderEvent>
    acquires MarketEventHandles {
        event::emitted_events_by_handle(
            &(borrow_market_event_handles_for_market_test(market_id).
                place_swap_order_events))
    }

    #[test_only]
    /// Get `PlaceSwapOrderEvent`s at swapper level.
    public fun get_place_swap_order_events_swapper_test(
        market_id: u64,
        swapper: address
    ): vector<PlaceSwapOrderEvent>
    acquires SwapperEventHandles {
        event::emitted_events_by_handle(
            &(borrow_swapper_event_handles_for_market_test(market_id, swapper).
                place_swap_order_events))
    }

    #[test_only]
    /// Return fields of indicated `Order`.
    public fun get_order_fields_test(
        market_id: u64,
        side: bool,
        market_order_id: u128
    ): (
        u64,
        u64,
        address,
        u64,
        u64
    ) acquires OrderBooks {
        let resource_address = resource_account::get_address();
        let order_books_map_ref = // Immutably borrow order books map.
            &borrow_global<OrderBooks>(resource_address).map;
        let order_book_ref = // Immutably borrow market order book.
            tablist::borrow(order_books_map_ref, market_id);
        // Immutably borrow corresponding orders AVL queue.
        let orders_ref = if (side == ASK) &order_book_ref.asks else
            &order_book_ref.bids;
        // Get AVL queue access key from market order ID.
        let avlq_access_key = ((market_order_id & (HI_64 as u128)) as u64);
        // Immutably borrow order.
        let order_ref = avl_queue::borrow(orders_ref, avlq_access_key);
        // Return its fields.
        (order_ref.size,
         order_ref.price,
         order_ref.user,
         order_ref.custodian_id,
         order_ref.order_access_key)
    }

    #[test_only]
    /// Return order book counter.
    public fun get_order_book_counter(
        market_id: u64
    ): u64
    acquires OrderBooks {
        // Get address of resource account having order books.
        let resource_address = resource_account::get_address();
        let order_books_map_ref = // Immutably borrow order books map.
            &borrow_global<OrderBooks>(resource_address).map;
        let order_book_ref = // Immutably borrow market order book.
            tablist::borrow(order_books_map_ref, market_id);
        order_book_ref.counter
    }

    #[test_only]
    /// Initialize module for testing.
    public fun init_test() {
        let econia = registry::init_test(); // Init registry.
        init_module(&econia); // Init module.
    }

    public fun init_setup(publisher: &signer) {
        registry::init_setup(publisher); // Init registry.
        init_module(publisher)
    }

    #[test_only]
    /// Initialize test markets, users, and an integrator, returning
    /// user signers.
    public fun init_markets_users_integrator_test(): (
        signer,
        signer
    ) acquires OrderBooks {
        init_test(); // Init for testing.
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Register pure coin market.
        register_market_base_coin<BC, QC, UC>(
            LOT_SIZE_COIN, TICK_SIZE_COIN, MIN_SIZE_COIN,
            assets::mint_test(fee));
        let underwriter_capability = registry::get_underwriter_capability_test(
            UNDERWRITER_ID); // Get market underwriter capability.
        // Register generic market.
        register_market_base_generic<QC, UC>(
            string::utf8(BASE_NAME_GENERIC), LOT_SIZE_GENERIC,
            TICK_SIZE_GENERIC, MIN_SIZE_GENERIC, assets::mint_test(fee),
            &underwriter_capability);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
        // Set custodian IDs to be valid.
        registry::set_registered_custodian_test(CUSTODIAN_ID_USER_0);
        registry::set_registered_custodian_test(CUSTODIAN_ID_USER_1);
        // Initialize two users, each with delegated and self-custodied
        // market accounts for each market.
        let user_0 = account::create_account_for_test(@user_0);
        let user_1 = account::create_account_for_test(@user_1);
        user::register_market_account<BC, QC>(
            &user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        user::register_market_account<BC, QC>(
            &user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0);
        user::register_market_account_generic_base<QC>(
            &user_0, MARKET_ID_GENERIC, NO_CUSTODIAN);
        user::register_market_account_generic_base<QC>(
            &user_0, MARKET_ID_GENERIC, CUSTODIAN_ID_USER_0);
        user::register_market_account<BC, QC>(
            &user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        user::register_market_account<BC, QC>(
            &user_1, MARKET_ID_COIN, CUSTODIAN_ID_USER_1);
        user::register_market_account_generic_base<QC>(
            &user_1, MARKET_ID_GENERIC, NO_CUSTODIAN);
        user::register_market_account_generic_base<QC>(
            &user_1, MARKET_ID_GENERIC, CUSTODIAN_ID_USER_1);
        // Register integrator to base fee store tier on each market.
        let integrator = account::create_account_for_test(@integrator);
        registry::register_integrator_fee_store_base_tier<QC, UC>(
            &integrator, MARKET_ID_COIN);
        registry::register_integrator_fee_store_base_tier<QC, UC>(
            &integrator, MARKET_ID_GENERIC);
        (user_0, user_1) // Return account signers.
    }

    #[test_only]
    /// Return true if AVL list node having list node ID encoded in
    /// market order ID is active
    public fun is_list_node_order_active(
        market_id: u64,
        side: bool,
        market_order_id: u128
    ): bool
    acquires OrderBooks {
        // Get address of resource account having order books.
        let resource_address = resource_account::get_address();
        let order_books_map_ref = // Immutably borrow order books map.
            &borrow_global<OrderBooks>(resource_address).map;
        let order_book_ref = // Immutably borrow market order book.
            tablist::borrow(order_books_map_ref, market_id);
        // Immutably borrow corresponding orders AVL queue.
        let orders_ref = if (side == ASK) &order_book_ref.asks else
            &order_book_ref.bids;
        // Get AVL queue access key from market order ID.
        let avlq_access_key = ((market_order_id & (HI_64 as u128)) as u64);
        // Get list node ID from AVL queue acces key.
        let list_node_id = avl_queue::get_access_key_list_node_id_test(
            avlq_access_key);
        // Immutably borrow order value option.
        let value_option_ref = avl_queue::borrow_value_option_test(
            orders_ref, list_node_id);
        // Return if is some.
        option::is_some(value_option_ref)
    }

    #[test_only]
    /// Return if order with market order ID is local price queue tail.
    public fun is_local_tail_test(
        market_id: u64,
        side: bool,
        market_order_id: u128
    ): bool acquires OrderBooks {
        // Get address of resource account having order books.
        let resource_address = resource_account::get_address();
        let order_books_map_ref = // Immutably borrow order books map.
            &borrow_global<OrderBooks>(resource_address).map;
        let order_book_ref = // Immutably borrow market order book.
            tablist::borrow(order_books_map_ref, market_id);
        // Immutably borrow corresponding orders AVL queue.
        let orders_ref = if (side == ASK) &order_book_ref.asks else
            &order_book_ref.bids;
        // Get AVL queue access key from market order ID.
        let avlq_access_key = ((market_order_id & (HI_64 as u128)) as u64);
        // Return if corresponding order is tail of local price queue.
        avl_queue::is_local_tail(orders_ref, avlq_access_key)
    }

    #[test_only]
    /// Return order ID derived solely from order book counter for an
    /// order that did not post.
    public fun order_id_no_post(
        counter: u64
    ): u128 {
        (counter as u128) << SHIFT_COUNTER
    }

    // Test-only functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only constants >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    /// After initializing an Aptos account (2 handles) and two order
    /// books (2 handles per).
    const FIRST_EVENT_HANDLE_RESOURCE_ACCOUNT: u64 = 6;
    #[test_only]
    /// After initializing an Aptos account (2 handles), four market
    /// accounts (5 handles per), and two coin stores (2 handles per).
    const FIRST_EVENT_HANDLE_SWAPPER: u64 = 26;

    #[test_only]
    /// Custodian ID for market account with delegated custodian.
    const CUSTODIAN_ID_USER_0: u64 = 123;
    #[test_only]
    /// Custodian ID for market account with delegated custodian.
    const CUSTODIAN_ID_USER_1: u64 = 234;
    #[test_only]
    /// Integrator fee store tier for test market.
    const INTEGRATOR_TIER: u8 = 0;
    #[test_only]
    /// Market ID for pure coin test market.
    const MARKET_ID_COIN: u64 = 1;
    #[test_only]
    /// Market ID for generic test market.
    const MARKET_ID_GENERIC: u64 = 2;
    #[test_only]
    /// Underwriter ID for generic test market.
    const UNDERWRITER_ID: u64 = 345;

    #[test_only]
    /// Lot size for pure coin test market.
    const LOT_SIZE_COIN: u64 = 2;
    #[test_only]
    /// Tick size for pure coin test market.
    const TICK_SIZE_COIN: u64 = 3;
    #[test_only]
    /// Minimum size for pure coin test market.
    const MIN_SIZE_COIN: u64 = 4;
    #[test_only]
    /// Base name for generic test market.
    const BASE_NAME_GENERIC: vector<u8> = b"Generic asset";
    #[test_only]
    /// Lot size for generic test market.
    const LOT_SIZE_GENERIC: u64 = 5;
    #[test_only]
    /// Tick size for generic test market.
    const TICK_SIZE_GENERIC: u64 = 6;
    #[test_only]
    /// Minimum size for generic test market.
    const MIN_SIZE_GENERIC: u64 = 7;

    // Test-only constants <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Tests >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test]
    /// Verify state updates for cancelling three asks under authority
    /// of custodian.
    fun test_cancel_all_orders_ask_custodian()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = ASK;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = CUSTODIAN_ID_USER_0;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size                = MIN_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare base/quote posted for order.
        let base_maker  = size * LOT_SIZE_COIN;
        let quote_maker = size * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = 3 * base_maker;
        let deposit_quote = HI_64 - 3 * quote_maker;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        let custodian_capability = registry::get_custodian_capability_test(
            custodian_id); // Get custodian capability.
        // Place maker orders, storing market order IDs for lookup.
        let (market_order_id_0, _, _, _) = place_limit_order_custodian<BC, QC>(
            maker_address, market_id, integrator, side, size,
            price, restriction, self_match_behavior, &custodian_capability);
        let (market_order_id_1, _, _, _) = place_limit_order_custodian<BC, QC>(
            maker_address, market_id, integrator, side, size,
            price, restriction, self_match_behavior, &custodian_capability);
        let (market_order_id_2, _, _, _) = place_limit_order_custodian<BC, QC>(
            maker_address, market_id, integrator, side, size,
            price, restriction, self_match_behavior, &custodian_capability);
        // Assert list node orders active.
        assert!(is_list_node_order_active(
            market_id, side, market_order_id_0), 0);
        assert!(is_list_node_order_active(
            market_id, side, market_order_id_1), 0);
        assert!(is_list_node_order_active(
            market_id, side, market_order_id_2), 0);
        cancel_all_orders_custodian( // Cancel orders.
            maker_address, market_id, side, &custodian_capability);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
        // Assert list node orders inactive.
        assert!(!is_list_node_order_active(
            market_id, side, market_order_id_0), 0);
        assert!(!is_list_node_order_active(
            market_id, side, market_order_id_1), 0);
        assert!(!is_list_node_order_active(
            market_id, side, market_order_id_2), 0);
    }

    #[test]
    /// Verify state updates for cancelling three bids under authority
    /// of signing user.
    fun test_cancel_all_orders_bid_user()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size                = MIN_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare base/quote posted for order.
        let base_maker  = size * LOT_SIZE_COIN;
        let quote_maker = size * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 - 3 * base_maker;
        let deposit_quote = 3 * quote_maker;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place maker orders, storing market order IDs for lookup.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size, price, restriction,
            self_match_behavior);
        let (market_order_id_1, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size, price, restriction,
            self_match_behavior);
        let (market_order_id_2, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size, price, restriction,
            self_match_behavior);
        // Assert list node orders active.
        assert!(is_list_node_order_active(
            market_id, side, market_order_id_0), 0);
        assert!(is_list_node_order_active(
            market_id, side, market_order_id_1), 0);
        assert!(is_list_node_order_active(
            market_id, side, market_order_id_2), 0);
        // Cancel orders.
        cancel_all_orders_user(&maker, market_id, side);
        // Assert list node orders inactive.
        assert!(!is_list_node_order_active(
            market_id, side, market_order_id_0), 0);
        assert!(!is_list_node_order_active(
            market_id, side, market_order_id_1), 0);
        assert!(!is_list_node_order_active(
            market_id, side, market_order_id_2), 0);
    }

    #[test]
    /// Verify state updates for cancelling ask under authority of
    /// custodian.
    fun test_cancel_order_ask_custodian()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = ASK;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = CUSTODIAN_ID_USER_0;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size                = MIN_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare base/quote posted for order.
        let base_maker  = size * LOT_SIZE_COIN;
        let quote_maker = size * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = base_maker;
        let deposit_quote = HI_64 - quote_maker;
        // Declare expected maker asset counts after cancel.
        let base_total_end      = deposit_base;
        let base_available_end  = base_total_end;
        let base_ceiling_end    = base_total_end;
        let quote_total_end     = deposit_quote;
        let quote_available_end = quote_total_end;
        let quote_ceiling_end   = quote_total_end;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        let custodian_capability = registry::get_custodian_capability_test(
            custodian_id); // Get custodian capability.
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_custodian<BC, QC>(
            maker_address, market_id, integrator, side, size, price,
            restriction, self_match_behavior, &custodian_capability);
        // Get user-side order access key for later.
        let (_, _, _, _, order_access_key) =
            get_order_fields_test(market_id, side, market_order_id);
        // Assert event streams.
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_change_order_size_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        cancel_order_custodian( // Cancel order.
            maker_address, market_id, side, market_order_id,
            &custodian_capability);
        // Assert event streams.
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    market_order_id,
                    maker_address,
                    custodian_id,
                    CANCEL_REASON_MANUAL_CANCEL
                )
            ], 0);
        assert!(user::get_change_order_size_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
        // Assert list node order inactive.
        assert!(!is_list_node_order_active(
            market_id, side, market_order_id), 0);
        // Assert user-side order fields for cancelled order
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side,
            order_access_key);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
    }

    #[test]
    /// Verify state updates for cancelling bid under authority of
    /// signing user.
    fun test_cancel_order_bid_user()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size                = MIN_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare base/quote posted for order.
        let base_maker  = size * LOT_SIZE_COIN;
        let quote_maker = size * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 - base_maker;
        let deposit_quote = quote_maker;
        // Declare expected maker asset counts after cancel.
        let base_total_end      = deposit_base;
        let base_available_end  = base_total_end;
        let base_ceiling_end    = base_total_end;
        let quote_total_end     = deposit_quote;
        let quote_available_end = quote_total_end;
        let quote_ceiling_end   = quote_total_end;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size, price, restriction,
            self_match_behavior);
        // Get user-side order access key for later.
        let (_, _, _, _, order_access_key) =
            get_order_fields_test(market_id, side, market_order_id);
        // Assert event streams.
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_change_order_size_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        // Cancel order.
        cancel_order_user( &maker, market_id, side, market_order_id);
        // Assert event streams.
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    market_order_id,
                    maker_address,
                    custodian_id,
                    CANCEL_REASON_MANUAL_CANCEL
                )
            ], 0);
        assert!(user::get_change_order_size_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        // Assert list node order inactive.
        assert!(!is_list_node_order_active(
            market_id, side, market_order_id), 0);
        // Assert user-side order fields for cancelled order
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side,
            order_access_key);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_CUSTODIAN)]
    /// Verify failure for invalid custodian.
    fun test_cancel_order_invalid_custodian()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = ASK;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size                = MIN_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare base/quote posted for order.
        let base_maker  = size * LOT_SIZE_COIN;
        let quote_maker = size * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = base_maker;
        let deposit_quote = HI_64 - quote_maker;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        let custodian_capability = registry::get_custodian_capability_test(
            custodian_id + 1); // Get invalid custodian capability.
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size, price, restriction,
            self_match_behavior);
        cancel_order_custodian( // Attempt invalid cancel.
            maker_address, market_id, side, market_order_id,
            &custodian_capability);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ID)]
    /// Verify failure for invalid market ID.
    fun test_cancel_order_invalid_market_id()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order change parameters.
        let market_id       = HI_64;
        let side            = ASK;
        let market_order_id = (NIL as u128) + 1;
        cancel_order_user( // Attempt invalid cancel.
            &maker, market_id, side, market_order_id);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ORDER_ID)]
    /// Verify failure for invalid bogus market order ID.
    fun test_cancel_order_invalid_market_order_id_bogus()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order change parameters.
        let market_id       = MARKET_ID_COIN;
        let side            = ASK;
        let market_order_id = 0xdeadfacedeadface;
        cancel_order_user( // Attempt invalid cancel.
            &maker, market_id, side, market_order_id);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ORDER_ID)]
    /// Verify failure for invalid market order ID passed as `NIL`.
    fun test_cancel_order_invalid_market_order_id_null()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order change parameters.
        let market_id       = MARKET_ID_COIN;
        let side            = ASK;
        let market_order_id = (NIL as u128);
        cancel_order_user( // Attempt invalid cancel.
            &maker, market_id, side, market_order_id);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_USER)]
    /// Verify failure for invalid user.
    fun test_cancel_order_invalid_user()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, attacker) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = ASK;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size                = MIN_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare base/quote posted for order.
        let base_maker  = size * LOT_SIZE_COIN;
        let quote_maker = size * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = base_maker;
        let deposit_quote = HI_64 - quote_maker;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size, price, restriction,
            self_match_behavior);
        cancel_order_user( // Attempt invalid cancel.
            &attacker, market_id, side, market_order_id);
    }

    #[test]
    /// Verify cancel reasons are constant across modules.
    fun test_cancel_reasons() {
        assert!(user::get_CANCEL_REASON_MANUAL_CANCEL() ==
                CANCEL_REASON_MANUAL_CANCEL, 0);
        assert!(user::get_CANCEL_REASON_EVICTION() ==
                CANCEL_REASON_EVICTION, 0);
        assert!(user::get_CANCEL_REASON_NOT_ENOUGH_LIQUIDITY() ==
                CANCEL_REASON_NOT_ENOUGH_LIQUIDITY, 0);
        assert!(user::get_CANCEL_REASON_SELF_MATCH_MAKER() ==
                CANCEL_REASON_SELF_MATCH_MAKER, 0);
        assert!(user::get_CANCEL_REASON_SELF_MATCH_TAKER() ==
                CANCEL_REASON_SELF_MATCH_TAKER, 0);
        assert!(user::get_CANCEL_REASON_IMMEDIATE_OR_CANCEL() ==
                CANCEL_REASON_IMMEDIATE_OR_CANCEL, 0);
        assert!(user::get_CANCEL_REASON_MAX_QUOTE_TRADED() ==
                CANCEL_REASON_MAX_QUOTE_TRADED, 0);
        assert!(user::get_CANCEL_REASON_TOO_SMALL_TO_FILL_LOT() ==
                CANCEL_REASON_TOO_SMALL_TO_FILL_LOT, 0);
        assert!(user::get_CANCEL_REASON_VIOLATED_LIMIT_PRICE() ==
                CANCEL_REASON_VIOLATED_LIMIT_PRICE, 0);
    }

    #[test]
    /// Verify state updates for changing ask under authority of
    /// custodian, for size increase at tail of price level queue.
    fun test_change_order_size_ask_custodian()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = ASK;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = CUSTODIAN_ID_USER_0;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size_start          = MIN_SIZE_COIN;
        let size_end            = size_start * 2;
        let self_match_behavior = ABORT;
        // Declare base/quote posted with final order.
        let base_maker  = size_end * LOT_SIZE_COIN;
        let quote_maker = size_end * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = base_maker;
        let deposit_quote = HI_64 - quote_maker;
        // Declare expected maker asset counts after size change.
        let base_total_end      = deposit_base;
        let base_available_end  = 0;
        let base_ceiling_end    = base_total_end;
        let quote_total_end     = deposit_quote;
        let quote_available_end = quote_total_end;
        let quote_ceiling_end   = HI_64;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        let custodian_capability = registry::get_custodian_capability_test(
            custodian_id); // Get custodian capability.
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_custodian<BC, QC>(
            maker_address, market_id, integrator, side, size_start,
            price, restriction, self_match_behavior, &custodian_capability);
        // Check if order is tail of corresponding price level.
        assert!(is_local_tail_test(market_id, side, market_order_id), 0);
        // Assert event streams.
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_change_order_size_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        change_order_size_custodian( // Change order size.
            maker_address, market_id, side, market_order_id, size_end,
            &custodian_capability);
        // Assert event streams.
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_change_order_size_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_change_order_size_event_test(
                    market_id,
                    market_order_id,
                    maker_address,
                    custodian_id,
                    side,
                    size_end
                )
            ], 0);
        // Check if order is tail of corresponding price level.
        assert!(is_local_tail_test(market_id, side, market_order_id), 0);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(market_id, side, market_order_id);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_end, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == maker_address, 0);
        assert!(custodian_id_r == custodian_id, 0);
        // Assert user-side maker order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side, order_access_key);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r            == size_end, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
        // Assert order marked as open.
        assert!(has_open_order(market_id, market_order_id), 0);
        // Assert order marked as not open for market order ID that does
        // not correspond to active list node.
        assert!(!has_open_order(market_id, (HI_64 as u128)), 0);
        // Assert order marked as not open for market order ID that does
        // correspond to active list node, but has invalid insertion
        // key that triggers mismatch against user-side market order ID.
        assert!(!has_open_order(market_id, market_order_id + 1), 0);
        // Manually doctor user-side market order ID.
        user::set_market_order_id_test(
            maker_address, market_id, custodian_id, side, order_access_key,
            market_order_id + 1);
        // Assert order not marked as open.
        assert!(!has_open_order(market_id, market_order_id), 0);
        // Fix user-side market order ID.
        user::set_market_order_id_test(
            maker_address, market_id, custodian_id, side, order_access_key,
            market_order_id);
        // Assert order marked as open.
        assert!(has_open_order(market_id, market_order_id), 0);
        // Get address of resource account where order books are stored.
        let resource_address = resource_account::get_address();
        let order_books_map_ref_mut = // Mutably borrow order books map.
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        // Mutably borrow asks for given order book.
        let asks_ref_mut = &mut tablist::borrow_mut(
            order_books_map_ref_mut, market_id).asks;
        // Get AVL queue access key encoded in market order ID.
        let avl_queue_access_key =
            get_order_id_avl_queue_access_key(market_order_id);
        avl_queue::borrow_mut(asks_ref_mut, avl_queue_access_key).user =
            @econia; // Manually doctor book-side user field.
        // Assert order marked as not open due to user market order ID
        // mismatch.
        assert!(!has_open_order(market_id, market_order_id), 0);
        // Get address of resource account where order books are stored.
        resource_address = resource_account::get_address();
        order_books_map_ref_mut = // Mutably borrow order books map.
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        // Mutably borrow asks for given order book.
        asks_ref_mut = &mut tablist::borrow_mut(
            order_books_map_ref_mut, market_id).asks;
        avl_queue::borrow_mut(asks_ref_mut, avl_queue_access_key).user =
            maker_address; // Manually fix book-side user field.
        // Get order for given market order ID, unpack into fields.
        let OrderView{
            market_id: market_id_r,
            side: side_r,
            order_id: market_order_id_r,
            remaining_size: size_r,
            price: price_r,
            user: user_r,
            custodian_id: custodian_id_r,
        } = option::destroy_some(get_open_order(market_id, market_order_id));
        // Assert field returns.
        assert!(market_id_r        == market_id, 0);
        assert!(side_r             == side, 0);
        assert!(market_order_id_r  == market_order_id, 0);
        assert!(size_r             == size_end, 0);
        assert!(price_r            == price, 0);
        assert!(user_r             == maker_address, 0);
        assert!(custodian_id_r     == custodian_id, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_SIZE_CHANGE_BELOW_MIN_SIZE)]
    /// Verify failure for attempting to change size below min size.
    fun test_change_order_size_below_min_size()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size_start          = MIN_SIZE_COIN;
        let size_end            = MIN_SIZE_COIN - 1;
        let self_match_behavior = ABORT;
        // Declare base/quote posted with final order.
        let base_maker  = size_end * LOT_SIZE_COIN;
        let quote_maker = size_end * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = base_maker;
        let deposit_quote = HI_64 - quote_maker;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size_start, price,
            restriction, self_match_behavior);
        // Attempt invalid order size change.
        change_order_size_user(
            &maker, market_id, side, market_order_id, size_end);
    }

    #[test]
    /// Verify state updates for changing bid under authority of signing
    /// user, for size decrease at tail of queue.
    fun test_change_order_size_bid_user()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size_start          = MIN_SIZE_COIN * 2;
        let size_end            = MIN_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare base/quote posted with final order.
        let base_maker  = size_end * LOT_SIZE_COIN;
        let quote_maker = size_end * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 / 2;
        let deposit_quote = HI_64 / 2;
        // Declare expected maker asset counts after size change.
        let base_total_end      = deposit_base;
        let base_available_end  = base_total_end;
        let base_ceiling_end    = base_total_end + base_maker;
        let quote_total_end     = deposit_quote;
        let quote_available_end = deposit_quote - quote_maker;
        let quote_ceiling_end   = deposit_quote;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size_start, price,
            restriction, self_match_behavior);
        // Check if order is tail of corresponding price level.
        assert!(is_local_tail_test(market_id, side, market_order_id), 0);
        // Assert event streams.
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_change_order_size_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        change_order_size_user( // Change order size.
            &maker, market_id, side, market_order_id, size_end);
        // Assert event streams.
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_change_order_size_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_change_order_size_event_test(
                    market_id,
                    market_order_id,
                    maker_address,
                    custodian_id,
                    side,
                    size_end
                )
            ], 0);
        // Check if order is tail of corresponding price level.
        assert!(is_local_tail_test(market_id, side, market_order_id), 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(market_id, side, market_order_id);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_end, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == maker_address, 0);
        assert!(custodian_id_r == custodian_id, 0);
        // Assert user-side maker order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side, order_access_key);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r            == size_end, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
        // Assert order marked as open.
        assert!(has_open_order(market_id, market_order_id), 0);
        // Get order for given market order ID, unpack into fields.
        let OrderView{
            market_id: market_id_r,
            side: side_r,
            order_id: market_order_id_r,
            remaining_size: size_r,
            price: price_r,
            user: user_r,
            custodian_id: custodian_id_r,
        } = option::destroy_some(get_open_order(market_id, market_order_id));
        // Assert field returns.
        assert!(market_id_r        == market_id, 0);
        assert!(side_r             == side, 0);
        assert!(market_order_id_r  == market_order_id, 0);
        assert!(size_r             == size_end, 0);
        assert!(price_r            == price, 0);
        assert!(user_r             == maker_address, 0);
        assert!(custodian_id_r     == custodian_id, 0);
    }

    #[test]
    /// Verify state updates for changing bid under authority of signing
    /// user, for size increase not at tail of queue.
    fun test_change_order_size_bid_user_new_tail()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size_start_0        = MIN_SIZE_COIN;
        let size_end_0          = MIN_SIZE_COIN * 2;
        let size_1              = MIN_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare base/quote posted per final orders.
        let base_maker  = (size_end_0 + size_1) * LOT_SIZE_COIN;
        let quote_maker = (size_end_0 + size_1) * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 / 2;
        let deposit_quote = HI_64 / 2;
        // Declare expected maker asset counts after size change.
        let base_total_end      = deposit_base;
        let base_available_end  = base_total_end;
        let base_ceiling_end    = base_total_end + base_maker;
        let quote_total_end     = deposit_quote;
        let quote_available_end = deposit_quote - quote_maker;
        let quote_ceiling_end   = deposit_quote;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place first maker order, storing market order ID for lookup.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size_start_0, price,
            restriction, self_match_behavior);
        // Check if order is tail of corresponding price level.
        assert!(is_local_tail_test(market_id, side, market_order_id_0), 0);
        // Place second maker order, storing market order ID for lookup.
        let (market_order_id_1, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size_1, price,
            restriction, self_match_behavior);
        // Verify order tail checks at corresponding price level.
        assert!(!is_local_tail_test(market_id, side, market_order_id_0), 0);
        assert!(is_local_tail_test(market_id, side, market_order_id_1), 0);
        // Assert event streams.
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_change_order_size_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        change_order_size_user( // Change order size.
            &maker, market_id, side, market_order_id_0, size_end_0);
        // Assert event streams.
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_change_order_size_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_change_order_size_event_test(
                    market_id,
                    market_order_id_0,
                    maker_address,
                    custodian_id,
                    side,
                    size_end_0
                )
            ], 0);
        // Verify order tail checks at corresponding price level.
        assert!(is_local_tail_test(market_id, side, market_order_id_0), 0);
        assert!(!is_local_tail_test(market_id, side, market_order_id_1), 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(market_id, side, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_end_0, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == maker_address, 0);
        assert!(custodian_id_r == custodian_id, 0);
        // Assert user-side maker order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side, order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_end_0, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_SIZE_CHANGE_INSERTION_ERROR)]
    /// Verify failure for changing bid under authority of signing user,
    /// for size increase not at tail of queue, where there is an AVL
    /// queue access key mismatch. Based on
    /// `test_change_order_size_bid_user_new_tail`.
    fun test_change_order_size_insertion_error()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size_start_0        = MIN_SIZE_COIN;
        let size_end_0          = MIN_SIZE_COIN * 2;
        let size_1              = MIN_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 / 2;
        let deposit_quote = HI_64 / 2;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place first maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size_start_0, price,
            restriction, self_match_behavior);
        // Get order access key for looking up user-side.
        let (_, _, _, _, order_access_key) = get_order_fields_test(
            market_id, side, market_order_id);
        place_limit_order_user<BC, QC>( // Place second maker order.
            &maker, market_id, integrator, side, size_1, price,
            restriction, self_match_behavior);
        // Get order counter from market order ID.
        let order_counter = ((market_order_id >> SHIFT_COUNTER) as u64);
        // Get AVL queue access key from market order ID.
        let avlq_access_key = ((market_order_id & (HI_64 as u128)) as u64);
        // Flip AVL queue access key sort order bit flag.
        let avlq_access_key_doctored =
            avl_queue::flip_access_key_sort_order_bit_test(avlq_access_key);
        // Get new market order ID from AVL queue access key, counter.
        let market_order_id_doctored = (avlq_access_key_doctored as u128) |
            ((order_counter as u128) << SHIFT_COUNTER);
        // Set doctored market order ID user-side.
        user::set_market_order_id_test(
            maker_address, market_id, custodian_id, side, order_access_key,
            market_order_id_doctored);
        // Change order size by calling with the doctored market order
        // ID, which is identical except for having the sort order bit
        // flag flipped. The bit flag is metadata that goes unchecked
        // during AVL queue insert/remove operations. Hence the relevant
        // AVL queue access key check can be tripped without tripping
        // user-side market order ID checks.
        change_order_size_user(
            &maker, market_id, side, market_order_id_doctored, size_end_0);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_CUSTODIAN)]
    /// Verify failure for invalid custodian.
    fun test_change_order_size_invalid_custodian()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size_start          = MIN_SIZE_COIN;
        let size_end            = size_start * 2;
        let self_match_behavior = ABORT;
        // Declare base/quote posted with final order.
        let base_maker  = size_end * LOT_SIZE_COIN;
        let quote_maker = size_end * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 - base_maker;
        let deposit_quote = quote_maker;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size_start, price,
            restriction, self_match_behavior);
        let custodian_capability = registry::get_custodian_capability_test(
            custodian_id + 1); // Get invalid custodian capability.
        change_order_size_custodian( // Attempt invalid order change.
            maker_address, market_id, side, market_order_id, size_end,
            &custodian_capability);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ID)]
    /// Verify failure for invalid market ID.
    fun test_change_order_size_invalid_market_id()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order change parameters.
        let market_id       = HI_64;
        let side            = ASK;
        let market_order_id = (NIL as u128) + 1;
        let size_end        = MIN_SIZE_COIN;
        change_order_size_user( // Attempt invalid order change.
            &maker, market_id, side, market_order_id, size_end);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ORDER_ID)]
    /// Verify failure for invalid bogus market order ID.
    fun test_change_order_size_invalid_market_order_id_bogus()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order change parameters.
        let market_id       = MARKET_ID_COIN;
        let side            = ASK;
        let market_order_id = 0xdeadfacedeadface;
        let size_end        = MIN_SIZE_COIN;
        change_order_size_user( // Attempt invalid order change.
            &maker, market_id, side, market_order_id, size_end);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ORDER_ID)]
    /// Verify failure for invalid market order ID passed as `NIL`.
    fun test_change_order_size_invalid_market_order_id_null()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order change parameters.
        let market_id       = MARKET_ID_COIN;
        let side            = ASK;
        let market_order_id = (NIL as u128);
        let size_end        = MIN_SIZE_COIN;
        change_order_size_user( // Attempt invalid order change.
            &maker, market_id, side, market_order_id, size_end);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_USER)]
    /// Verify failure for invalid user.
    fun test_change_order_size_invalid_user()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, attacker) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size_start          = MIN_SIZE_COIN;
        let size_end            = size_start * 2;
        let self_match_behavior = ABORT;
        // Declare base/quote posted with final order.
        let base_maker  = size_end * LOT_SIZE_COIN;
        let quote_maker = size_end * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 - base_maker;
        let deposit_quote = quote_maker;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size_start, price,
            restriction, self_match_behavior);
        change_order_size_user( // Attempt invalid order change.
            &attacker, market_id, side, market_order_id, size_end);
    }

    #[test]
    /// Verify direction and side polarities.
    fun test_direction_side_polarities() {
        assert!(get_ASK() == get_SELL(), 0); // Verify ask equals sell.
        assert!(get_BID() == get_BUY(), 0); // Verify bid equals buy.
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_ABORT() {assert!(get_ABORT() == ABORT, 0)}

    #[test]
    /// Verify constant getter return.
    fun test_get_ASK() {
        assert!(get_ASK() == ASK, 0);
        assert!(get_ASK() == user::get_ASK(), 0);
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_BID() {
        assert!(get_BID() == BID, 0);
        assert!(get_BID() == user::get_BID(), 0);
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_BUY() {
        assert!(get_BUY() == BUY, 0);
        assert!(get_BUY() == incentives::get_BUY_test(), 0);
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_CANCEL_BOTH() {assert!(get_CANCEL_BOTH() == CANCEL_BOTH, 0)}

    #[test]
    /// Verify constant getter return.
    fun test_get_CANCEL_MAKER() {
        assert!(get_CANCEL_MAKER() == CANCEL_MAKER, 0)
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_CANCEL_TAKER() {
        assert!(get_CANCEL_TAKER() == CANCEL_TAKER, 0)
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_FILL_OR_ABORT() {
        assert!(get_FILL_OR_ABORT() == FILL_OR_ABORT, 0)
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_HI_PRICE() {
        assert!(get_HI_PRICE() == HI_PRICE, 0);
        assert!(get_HI_PRICE() == user::get_HI_PRICE_test(), 0)
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_IMMEDIATE_OR_CANCEL() {
        assert!(get_IMMEDIATE_OR_CANCEL() == IMMEDIATE_OR_CANCEL, 0)
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_MAX_POSSIBLE() {
        assert!(get_MAX_POSSIBLE() == MAX_POSSIBLE, 0)
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_NO_CUSTODIAN() {
        assert!(get_NO_CUSTODIAN() == NO_CUSTODIAN, 0);
        assert!(get_NO_CUSTODIAN() == user::get_NO_CUSTODIAN(), 0);
        assert!(get_NO_CUSTODIAN() == registry::get_NO_CUSTODIAN(), 0);
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_NO_RESTRICTION() {
        assert!(get_NO_RESTRICTION() == NO_RESTRICTION, 0)
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_NO_UNDERWRITER() {
        assert!(get_NO_UNDERWRITER() == NO_UNDERWRITER, 0);
        assert!(get_NO_UNDERWRITER() == registry::get_NO_UNDERWRITER(), 0);
        assert!(get_NO_UNDERWRITER() == user::get_NO_UNDERWRITER_test(), 0);
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_PERCENT() {assert!(get_PERCENT() == PERCENT, 0)}

    #[test]
    /// Verify constant getter return.
    fun test_get_POST_OR_ABORT() {
        assert!(get_POST_OR_ABORT() == POST_OR_ABORT, 0)
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_SELL() {
        assert!(get_SELL() == SELL, 0);
        assert!(get_SELL() == incentives::get_SELL_test(), 0);
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_TICKS() {assert!(get_TICKS() == TICKS, 0)}

    #[test]
    /// Verify return.
    fun test_get_market_order_id_avl_queue_access_key() {
        // Declare mock AVL queue access key, order counter.
        let avlq_access_key = 12345;
        let counter = 67890;
        // Construct mock market order ID.
        let market_order_id =
            (avlq_access_key as u128) | ((counter as u128) << SHIFT_COUNTER);
        // Assert AVL queue access key lookup.
        assert!(get_order_id_avl_queue_access_key(market_order_id) ==
                avlq_access_key, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_ORDER_DID_NOT_POST)]
    /// Verify failure for market order ID corresponding to fills only.
    fun test_get_market_order_id_price_did_not_post() {
        get_market_order_id_price((1 as u128) << SHIFT_COUNTER);
    }

    #[test]
    #[expected_failure(abort_code = E_ORDER_DID_NOT_POST)]
    /// Verify failure for market order ID corresponding to fills only.
    fun test_get_market_order_id_side_did_not_post() {
        get_posted_order_id_side((1 as u128) << SHIFT_COUNTER);
    }

    #[test]
    /// Verify return for no such order.
    fun test_get_open_order_no_such_order() acquires OrderBooks {
        init_test(); // Initialize for testing.
        // Assert empty option return.
        assert!(get_open_order(0, 0) == option::none(), 0);
    }

    #[test]
    /// Verify indexing results.
    fun test_get_open_orders() acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare common order parameters.
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Declare ask and bid parameters.
        let bid_1_size = MIN_SIZE_COIN + 1;
        let bid_0_size = bid_1_size + 1;
        let ask_0_size = bid_0_size + 1;
        let ask_1_size = ask_0_size + 1;
        let bid_1_price = 1;
        let bid_0_price = bid_1_price + 1;
        let ask_0_price = bid_0_price + 1;
        let ask_1_price = ask_0_price + 1;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(HI_64 / 2));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(HI_64 / 2));
        // Place maker orders.
        let (market_order_id_bid_1, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_1_size, bid_1_price,
            restriction, self_match_behavior);
        let (market_order_id_bid_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_0_size, bid_0_price,
            restriction, self_match_behavior);
        let (market_order_id_ask_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_0_size, ask_0_price,
            restriction, self_match_behavior);
        let (market_order_id_ask_1, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_1_size, ask_1_price,
            restriction, self_match_behavior);
        // Index orders.
        let orders = get_open_orders_all(market_id);
        // Assert order state.
        assert!(vector::length(&orders.asks) == 2, 0);
        assert!(vector::length(&orders.bids) == 2, 0);
        let order_ref = vector::borrow(&orders.asks, 1);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == ASK, 0);
        assert!(order_ref.order_id        == market_order_id_ask_1, 0);
        assert!(order_ref.price           == ask_1_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        order_ref = vector::borrow(&orders.asks, 0);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == ASK, 0);
        assert!(order_ref.order_id        == market_order_id_ask_0, 0);
        assert!(order_ref.price           == ask_0_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        order_ref = vector::borrow(&orders.bids, 0);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == BID, 0);
        assert!(order_ref.order_id        == market_order_id_bid_0, 0);
        assert!(order_ref.price           == bid_0_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        order_ref = vector::borrow(&orders.bids, 1);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == BID, 0);
        assert!(order_ref.order_id        == market_order_id_bid_1, 0);
        assert!(order_ref.price           == bid_1_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        // Place maker orders again.
        (market_order_id_bid_1, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_1_size, bid_1_price,
            restriction, self_match_behavior);
        (market_order_id_bid_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_0_size, bid_0_price,
            restriction, self_match_behavior);
        (market_order_id_ask_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_0_size, ask_0_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_1_size, ask_1_price,
            restriction, self_match_behavior);
        // Index just 1 ask and no bids.
        orders = get_open_orders(market_id, 1, 0);
        // Assert order state.
        assert!(vector::length(&orders.asks) == 1, 0);
        assert!(vector::length(&orders.bids) == 0, 0);
        order_ref = vector::borrow(&orders.asks, 0);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == ASK, 0);
        assert!(order_ref.order_id        == market_order_id_ask_0, 0);
        assert!(order_ref.price           == ask_0_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        // Index just 2 bids.
        let orders = get_open_orders(market_id, 0, 2);
        // Assert order state.
        assert!(vector::length(&orders.asks) == 0, 0);
        assert!(vector::length(&orders.bids) == 2, 0);
        order_ref = vector::borrow(&orders.bids, 0);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == BID, 0);
        assert!(order_ref.order_id        == market_order_id_bid_0, 0);
        assert!(order_ref.price           == bid_0_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        order_ref = vector::borrow(&orders.bids, 1);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == BID, 0);
        assert!(order_ref.order_id        == market_order_id_bid_1, 0);
        assert!(order_ref.price           == bid_1_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ID)]
    /// Verify failure for unregistered market.
    fun test_get_open_orders_invalid_market_id() acquires OrderBooks {
        init_test(); // Initialize for testing.
        get_open_orders_all(0); // Attempt invalid lookup.
    }

    #[test]
    /// Verify indexing results. Based on `test_get_open_orders()` and
    /// `test_get_price_levels_paginated()`.
    fun test_get_open_orders_paginated() acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare common order parameters.
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Declare ask and bid parameters.
        let ask_hi_0_size = MIN_SIZE_COIN;
        let ask_hi_1_size = ask_hi_0_size + 1;
        let ask_lo_0_size = ask_hi_1_size + 1;
        let ask_lo_1_size = ask_lo_0_size + 1;
        let bid_hi_0_size = ask_lo_1_size + 1;
        let bid_hi_1_size = bid_hi_0_size + 1;
        let bid_lo_0_size = bid_hi_1_size + 1;
        let bid_lo_1_size = bid_lo_0_size + 1;
        let bid_lo_price = 1;
        let bid_hi_price = bid_lo_price + 1;
        let ask_lo_price = bid_hi_price + 1;
        let ask_hi_price = ask_lo_price + 1;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(HI_64 / 2));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(HI_64 / 2));
        // Index orders with pagination.
        let (
            OrdersView{asks, bids},
            ask_next,
            bid_next
        ) = get_open_orders_paginated(
                market_id, 100, 0, (NIL as u128), (NIL as u128));
        // Assert asks and bids levels both empty, next as NIL.
        assert!(asks == vector[], 0);
        assert!(bids == vector[], 0);
        assert!(ask_next == (NIL as u128), 0);
        assert!(bid_next == (NIL as u128), 0);
        // Place maker orders.
        let (order_id_ask_hi_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_hi_0_size, ask_hi_price,
            restriction, self_match_behavior);
        let (order_id_ask_hi_1, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_hi_1_size, ask_hi_price,
            restriction, self_match_behavior);
        let (order_id_ask_lo_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_lo_0_size, ask_lo_price,
            restriction, self_match_behavior);
        let (order_id_ask_lo_1, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_lo_1_size, ask_lo_price,
            restriction, self_match_behavior);
        let (order_id_bid_hi_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_hi_0_size, bid_hi_price,
            restriction, self_match_behavior);
        let (order_id_bid_hi_1, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_hi_1_size, bid_hi_price,
            restriction, self_match_behavior);
        let (order_id_bid_lo_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_lo_0_size, bid_lo_price,
            restriction, self_match_behavior);
        let (order_id_bid_lo_1, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_lo_1_size, bid_lo_price,
            restriction, self_match_behavior);
        // Index all asks and bids.
        let (orders, ask_next, bid_next) = get_open_orders_paginated(
            market_id,
            HI_64,
            HI_64,
            (NIL as u128),
            (NIL as u128),
        );
        // Assert no asks or bids left to paginate.
        assert!(ask_next == (NIL as u128), 0);
        assert!(bid_next == (NIL as u128), 0);
        // Assert order state.
        assert!(vector::length(&orders.asks) == 4, 0);
        assert!(vector::length(&orders.bids) == 4, 0);
        let order_ref = vector::borrow(&orders.asks, 0);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == ASK, 0);
        assert!(order_ref.order_id        == order_id_ask_lo_0, 0);
        assert!(order_ref.remaining_size  == ask_lo_0_size, 0);
        assert!(order_ref.price           == ask_lo_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.asks, 1);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == ASK, 0);
        assert!(order_ref.order_id        == order_id_ask_lo_1, 0);
        assert!(order_ref.remaining_size  == ask_lo_1_size, 0);
        assert!(order_ref.price           == ask_lo_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.asks, 2);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == ASK, 0);
        assert!(order_ref.order_id        == order_id_ask_hi_0, 0);
        assert!(order_ref.remaining_size  == ask_hi_0_size, 0);
        assert!(order_ref.price           == ask_hi_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.asks, 3);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == ASK, 0);
        assert!(order_ref.order_id        == order_id_ask_hi_1, 0);
        assert!(order_ref.remaining_size  == ask_hi_1_size, 0);
        assert!(order_ref.price           == ask_hi_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.bids, 0);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == BID, 0);
        assert!(order_ref.order_id        == order_id_bid_hi_0, 0);
        assert!(order_ref.remaining_size  == bid_hi_0_size, 0);
        assert!(order_ref.price           == bid_hi_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.bids, 1);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == BID, 0);
        assert!(order_ref.order_id        == order_id_bid_hi_1, 0);
        assert!(order_ref.remaining_size  == bid_hi_1_size, 0);
        assert!(order_ref.price           == bid_hi_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.bids, 2);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == BID, 0);
        assert!(order_ref.order_id        == order_id_bid_lo_0, 0);
        assert!(order_ref.remaining_size  == bid_lo_0_size, 0);
        assert!(order_ref.price           == bid_lo_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.bids, 3);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == BID, 0);
        assert!(order_ref.order_id        == order_id_bid_lo_1, 0);
        assert!(order_ref.remaining_size  == bid_lo_1_size, 0);
        assert!(order_ref.price           == bid_lo_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        // Index some asks and some bids.
        let (orders, ask_next, bid_next) = get_open_orders_paginated(
            market_id,
            2,
            3,
            (NIL as u128),
            (NIL as u128),
        );
        // Assert pagination breaks.
        assert!(ask_next == order_id_ask_hi_0, 0);
        assert!(bid_next == order_id_bid_lo_1, 0);
        // Assert order state.
        assert!(vector::length(&orders.asks) == 2, 0);
        assert!(vector::length(&orders.bids) == 3, 0);
        let order_ref = vector::borrow(&orders.asks, 0);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == ASK, 0);
        assert!(order_ref.order_id        == order_id_ask_lo_0, 0);
        assert!(order_ref.remaining_size  == ask_lo_0_size, 0);
        assert!(order_ref.price           == ask_lo_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.asks, 1);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == ASK, 0);
        assert!(order_ref.order_id        == order_id_ask_lo_1, 0);
        assert!(order_ref.remaining_size  == ask_lo_1_size, 0);
        assert!(order_ref.price           == ask_lo_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.bids, 0);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == BID, 0);
        assert!(order_ref.order_id        == order_id_bid_hi_0, 0);
        assert!(order_ref.remaining_size  == bid_hi_0_size, 0);
        assert!(order_ref.price           == bid_hi_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.bids, 1);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == BID, 0);
        assert!(order_ref.order_id        == order_id_bid_hi_1, 0);
        assert!(order_ref.remaining_size  == bid_hi_1_size, 0);
        assert!(order_ref.price           == bid_hi_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.bids, 2);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == BID, 0);
        assert!(order_ref.order_id        == order_id_bid_lo_0, 0);
        assert!(order_ref.remaining_size  == bid_lo_0_size, 0);
        assert!(order_ref.price           == bid_lo_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        // Index next page.
        let (orders, ask_next, bid_next) = get_open_orders_paginated(
            market_id,
            2,
            3,
            ask_next,
            bid_next,
        );
        // Assert no asks or bids left to paginate.
        assert!(ask_next == (NIL as u128), 0);
        assert!(bid_next == (NIL as u128), 0);
        // Assert order state.
        assert!(vector::length(&orders.asks) == 2, 0);
        assert!(vector::length(&orders.bids) == 1, 0);
        let order_ref = vector::borrow(&orders.asks, 0);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == ASK, 0);
        assert!(order_ref.order_id        == order_id_ask_hi_0, 0);
        assert!(order_ref.remaining_size  == ask_hi_0_size, 0);
        assert!(order_ref.price           == ask_hi_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.asks, 1);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == ASK, 0);
        assert!(order_ref.order_id        == order_id_ask_hi_1, 0);
        assert!(order_ref.remaining_size  == ask_hi_1_size, 0);
        assert!(order_ref.price           == ask_hi_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
        let order_ref = vector::borrow(&orders.bids, 0);
        assert!(order_ref.market_id       == market_id, 0);
        assert!(order_ref.side            == BID, 0);
        assert!(order_ref.order_id        == order_id_bid_lo_1, 0);
        assert!(order_ref.remaining_size  == bid_lo_1_size, 0);
        assert!(order_ref.price           == bid_lo_price, 0);
        assert!(order_ref.user            == maker_address, 0);
        assert!(order_ref.custodian_id    == custodian_id, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ID)]
    /// Verify indexing results.
    fun test_get_open_orders_paginated_invalid_market_id()
    acquires OrderBooks {
        init_test(); // Initialize for testing.
        // Attempt invalid lookup.
        get_open_orders_paginated(0, HI_64, HI_64, 0, 0);
    }

    #[test]
    /// Verify indexing results.
    fun test_get_price_levels() acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare common order parameters.
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Declare ask and bid parameters.
        let bid_1_size = MIN_SIZE_COIN + 1;
        let bid_0_size = bid_1_size + 1;
        let ask_0_size = bid_0_size + 1;
        let ask_1_size = ask_0_size + 1;
        let bid_1_price = 1;
        let bid_0_price = bid_1_price + 1;
        let ask_0_price = bid_0_price + 1;
        let ask_1_price = ask_0_price + 1;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(HI_64 / 2));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(HI_64 / 2));
        // Place maker orders.
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_1_size, bid_1_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_0_size, bid_0_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_0_size, ask_0_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_1_size, ask_1_price,
            restriction, self_match_behavior);
        // Index price levels.
        let PriceLevels{market_id: market_id_r, asks, bids} =
            get_price_levels_all(market_id);
        // Assert market ID.
        assert!(market_id_r == market_id, 0);
        // Assert ask price levels state.
        assert!(vector::length(&asks) == 2, 0);
        assert!(*vector::borrow(&asks, 1) ==
                PriceLevel{price: ask_1_price, size: (ask_1_size as u128)}, 0);
        assert!(*vector::borrow(&asks, 0) ==
                PriceLevel{price: ask_0_price, size: (ask_0_size as u128)}, 0);
        // Assert bid price levels state.
        assert!(vector::length(&bids) == 2, 0);
        assert!(*vector::borrow(&bids, 0) ==
                PriceLevel{price: bid_0_price, size: (bid_0_size as u128)}, 0);
        assert!(*vector::borrow(&bids, 1) ==
                PriceLevel{price: bid_1_price, size: (bid_1_size as u128)}, 0);
        // Place same maker orders again.
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_1_size, bid_1_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_0_size, bid_0_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_0_size, ask_0_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_1_size, ask_1_price,
            restriction, self_match_behavior);
        // Index price levels, only requesting one ask price level.
        let PriceLevels{market_id: _, asks, bids} =
            get_price_levels(market_id, 1, 0);
        // Assert ask price levels state.
        assert!(vector::length(&asks) == 1, 0);
        assert!(*vector::borrow(&asks, 0) ==
                PriceLevel{price: ask_0_price, size: (ask_0_size as u128)}, 0);
        // Assert bid price levels state.
        assert!(vector::length(&bids) == 0, 0);
        // Index price levels, only requesting two bid price levels.
        let PriceLevels{market_id: _, asks, bids} =
            get_price_levels(market_id, 0, 2);
        // Assert ask price levels state.
        assert!(vector::length(&asks) == 0, 0);
        // Assert bid price levels state.
        assert!(vector::length(&bids) == 2, 0);
        assert!(*vector::borrow(&bids, 0) ==
                PriceLevel{price: bid_0_price, size: (bid_0_size as u128)}, 0);
        assert!(*vector::borrow(&bids, 1) ==
                PriceLevel{price: bid_1_price, size: (bid_1_size as u128)}, 0);
    }

    #[test]
    /// Verify indexing results.
    fun test_get_price_levels_paginated() acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare common order parameters.
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Declare ask and bid parameters.
        let ask_hi_0_size = MIN_SIZE_COIN;
        let ask_hi_1_size = ask_hi_0_size + 1;
        let ask_lo_0_size = ask_hi_1_size + 1;
        let ask_lo_1_size = ask_lo_0_size + 1;
        let bid_hi_0_size = ask_lo_1_size + 1;
        let bid_hi_1_size = bid_hi_0_size + 1;
        let bid_lo_0_size = bid_hi_1_size + 1;
        let bid_lo_1_size = bid_lo_0_size + 1;
        let bid_lo_price = 1;
        let bid_hi_price = bid_lo_price + 1;
        let ask_lo_price = bid_hi_price + 1;
        let ask_hi_price = ask_lo_price + 1;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(HI_64 / 2));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(HI_64 / 2));
        // Index price levels with pagination.
        let (
            PriceLevels{market_id: market_id_r, asks, bids},
            ask_next,
            bid_next
        ) = get_price_levels_paginated(
                market_id, 100, 0, (NIL as u128), (NIL as u128));
        // Assert market ID.
        assert!(market_id_r == market_id, 0);
        // Assert asks and bids levels both empty, next as NIL.
        assert!(asks == vector[], 0);
        assert!(bids == vector[], 0);
        assert!(ask_next == (NIL as u128), 0);
        assert!(bid_next == (NIL as u128), 0);
        // Place maker orders.
        let (order_id_ask_hi_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_hi_0_size, ask_hi_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_hi_1_size, ask_hi_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_lo_0_size, ask_lo_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_lo_1_size, ask_lo_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_hi_0_size, bid_hi_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_hi_1_size, bid_hi_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_lo_0_size, bid_lo_price,
            restriction, self_match_behavior);
        let (order_id_bid_lo_1, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_lo_1_size, bid_lo_price,
            restriction, self_match_behavior);
        // Index all asks and bids.
        let (
            PriceLevels{market_id: market_id_r, asks, bids},
            ask_next,
            bid_next
        ) = get_price_levels_paginated(
                market_id, HI_64, HI_64, (NIL as u128), (NIL as u128));
        // Assert no asks or bids left to paginate.
        assert!(ask_next == (NIL as u128), 0);
        assert!(bid_next == (NIL as u128), 0);
        // Assert market ID.
        assert!(market_id_r == market_id, 0);
        // Assert price levels state.
        assert!(vector::length(&asks) == 2, 0);
        assert!(vector::length(&bids) == 2, 0);
        assert!(*vector::borrow(&asks, 0) == PriceLevel{
            price: ask_lo_price,
            size: ((ask_lo_0_size + ask_lo_1_size) as u128)
        }, 0);
        assert!(*vector::borrow(&asks, 1) == PriceLevel{
            price: ask_hi_price,
            size: ((ask_hi_0_size + ask_hi_1_size) as u128)
        }, 0);
        assert!(*vector::borrow(&bids, 0) == PriceLevel{
            price: bid_hi_price,
            size: ((bid_hi_0_size + bid_hi_1_size) as u128)
        }, 0);
        assert!(*vector::borrow(&bids, 1) == PriceLevel{
            price: bid_lo_price,
            size: ((bid_lo_0_size + bid_lo_1_size) as u128)
        }, 0);
        // Index some asks and some bids.
        let (
            PriceLevels{market_id: market_id_r, asks, bids},
            ask_next,
            bid_next
        ) = get_price_levels_paginated(
                market_id, 2, 3, (NIL as u128), (NIL as u128));
        // Assert market ID.
        assert!(market_id_r == market_id, 0);
        // Assert pagination breaks.
        assert!(ask_next == order_id_ask_hi_0, 0);
        assert!(bid_next == order_id_bid_lo_1, 0);
        // Assert price levels state.
        assert!(vector::length(&asks) == 1, 0);
        assert!(vector::length(&bids) == 2, 0);
        assert!(*vector::borrow(&asks, 0) == PriceLevel{
            price: ask_lo_price,
            size: ((ask_lo_0_size + ask_lo_1_size) as u128)
        }, 0);
        assert!(*vector::borrow(&bids, 0) == PriceLevel{
            price: bid_hi_price,
            size: ((bid_hi_0_size + bid_hi_1_size) as u128)
        }, 0);
        assert!(*vector::borrow(&bids, 1) == PriceLevel{
            price: bid_lo_price,
            size: ((bid_lo_0_size) as u128)
        }, 0);
        // Index next page.
        let (
            PriceLevels{market_id: market_id_r, asks, bids},
            ask_next,
            bid_next
        ) = get_price_levels_paginated(
                market_id, 2, 1, ask_next, bid_next);
        // Assert no asks or bids left to paginate.
        assert!(ask_next == (NIL as u128), 0);
        assert!(bid_next == (NIL as u128), 0);
        // Assert market ID.
        assert!(market_id_r == market_id, 0);
        // Assert price levels state.
        assert!(vector::length(&asks) == 1, 0);
        assert!(vector::length(&bids) == 1, 0);
        assert!(*vector::borrow(&asks, 0) == PriceLevel{
            price: ask_hi_price,
            size: ((ask_hi_0_size + ask_hi_1_size) as u128)
        }, 0);
        assert!(*vector::borrow(&bids, 0) == PriceLevel{
            price: bid_lo_price,
            size: ((bid_lo_1_size) as u128)
        }, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ID)]
    /// Verify indexing results.
    fun test_get_price_levels_paginated_invalid_market_id()
    acquires OrderBooks {
        init_test(); // Initialize for testing.
        // Attempt invalid lookup.
        get_price_levels_paginated(0, HI_64, HI_64, 0, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_ORDER_PRICE_MISMATCH)]
    /// Verify failure for price mismatch results.
    fun test_get_price_levels_mismatch() acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare common order parameters.
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Declare ask and bid parameters.
        let bid_1_size = MIN_SIZE_COIN + 1;
        let bid_0_size = bid_1_size + 1;
        let ask_0_size = bid_0_size + 1;
        let ask_1_size = ask_0_size + 1;
        let bid_1_price = 1;
        let bid_0_price = bid_1_price + 1;
        let ask_0_price = bid_0_price + 1;
        let ask_1_price = ask_0_price + 1;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(HI_64 / 2));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(HI_64 / 2));
        // Place maker orders, storing order ID of final order.
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_1_size, bid_1_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, BID, bid_0_size, bid_0_price,
            restriction, self_match_behavior);
        place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_0_size, ask_0_price,
            restriction, self_match_behavior);
        let (order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, ASK, ask_1_size, ask_1_price,
            restriction, self_match_behavior);
        // Manually doctor price of final order.
        let resource_address = resource_account::get_address();
        let order_books_map_ref_mut =
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        let order_book_ref_mut =
            tablist::borrow_mut(order_books_map_ref_mut, market_id);
        let avlq_access_key = ((order_id & (HI_64 as u128)) as u64);
        let order_ref_mut = avl_queue::borrow_mut(
            &mut order_book_ref_mut.asks, avlq_access_key);
        order_ref_mut.price = ask_1_price + 1;
        // Attempt invalid indexing operation.
        get_price_levels_all(market_id);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ID)]
    /// Verify failure for unregistered market.
    fun test_get_price_levels_invalid_market_id() acquires OrderBooks {
        init_test(); // Initialize for testing.
        get_price_levels_all(0); // Attempt invalid lookup.
    }

    #[test]
    /// Verify return for no registered market.
    fun test_has_open_order_no_market() acquires OrderBooks {
        init_test(); // Initialize for testing.
        // Assert no order indicated when no markets registered.
        assert!(has_open_order(0, 0) == false, 0);
    }

    #[test(account = @econia)]
    /// Invoke deprecated function for test coverage.
    fun test_index_orders_sdk_coverage(
        account: &signer
    ) {
        index_orders_sdk(account, 0);
    }

    #[test]
    /// Verify returns, state updates for complete buy fill with no lots
    /// left to fill on matched order.
    fun test_match_complete_fill_no_lots_buy()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare shared/dependent market parameters.
        let direction_taker     = BUY;
        let side_maker          = if (direction_taker == BUY) ASK else BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let self_match_behavior = ABORT;
        // Declare additional maker order parameters.
        let custodian_id  = NO_CUSTODIAN;
        let maker_address = address_of(&maker);
        let restriction   = NO_RESTRICTION;
        // Declare price set to product of fee divisors, to eliminate
        // truncation when predicting fee amounts.
        let price = integrator_divisor * taker_divisor;
        // Declare order size posted by maker, filled by taker.
        let size_maker = MIN_SIZE_COIN;
        let size_taker = size_maker;
        // Declare base/quote posted/filled by maker/taker.
        let base_maker  = size_maker * LOT_SIZE_COIN;
        let quote_maker = size_maker * price * TICK_SIZE_COIN;
        let base_taker  = size_taker * LOT_SIZE_COIN;
        let quote_taker = size_taker * price * TICK_SIZE_COIN;
        // Declare fee and trade amounts, from taker's perspective.
        let base_trade       = base_taker;
        let integrator_share = quote_taker / integrator_divisor;
        let econia_share     = quote_taker / taker_divisor - integrator_share;
        let fee              = integrator_share + econia_share;
        let quote_trade      = if (direction_taker == BUY)
            (quote_taker + fee) else (quote_taker - fee);
        // Declare maker deposit amounts.
        let deposit_base  = base_maker;
        let deposit_quote = HI_64 - quote_maker;
        // Declare expected maker asset counts after matching.
        let base_total_end      = 0;
        let base_available_end  = 0;
        let base_ceiling_end    = 0;
        let quote_total_end     = HI_64;
        let quote_available_end = HI_64;
        let quote_ceiling_end   = HI_64;
        // Assign min/max base/quote swap input amounts for taker.
        let min_base  = 0;
        let max_base  = base_trade;
        let min_quote = 0;
        let max_quote = quote_trade * 2;
        // Declare swap coin input starting amounts.
        let base_coin_start = HI_64 - base_trade;
        let quote_coin_start = max_quote;
        // Declare swap coin end amounts.
        let base_coin_end = HI_64;
        let quote_coin_end = quote_coin_start - quote_trade;
        // Create swap coin inputs.
        let base_coins  = assets::mint_test<BC>(base_coin_start);
        let quote_coins = assets::mint_test<QC>(quote_coin_start);
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, @integrator, side_maker, size_maker, price,
            restriction, self_match_behavior);
        // Get user-side order access key for later.
        let (_, _, _, _, order_access_key) =
            get_order_fields_test(market_id, side_maker, market_order_id);
        // Invoke matching engine via coin swap.
        let (base_coins, quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_coins(market_id, integrator, direction_taker, min_base,
                       max_base, min_quote, max_quote, price, base_coins,
                       quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    restriction,
                    self_match_behavior,
                    size_maker,
                    market_order_id
                )
            ], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_fill_event_internal(
                    market_id,
                    size_taker,
                    price,
                    side_maker,
                    maker_address,
                    custodian_id,
                    market_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[
                PlaceSwapOrderEvent{
                    market_id,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction: direction_taker,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(market_id) == vector[], 0);
        // Assert returns.
        assert!(coin::value(&base_coins)  == base_coin_end, 0);
        assert!(coin::value(&quote_coins) == quote_coin_end, 0);
        assert!(base_trade_r              == base_trade, 0);
        assert!(quote_trade_r             == quote_trade, 0);
        assert!(fee_r                     == fee, 0);
        // Burn coins.
        if (base_coin_end == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_coin_end == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
        // Assert list node order inactive.
        assert!(!is_list_node_order_active(
            market_id, side_maker, market_order_id), 0);
        // Assert user-side order fields for filled maker order.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            order_access_key);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
        // Assert integrator fee share.
        assert!(incentives::get_integrator_fee_store_balance_test<QC>(
            @integrator, market_id) == integrator_share, 0);
        // Assert Econia fee share.
        assert!(incentives::get_econia_fee_store_balance_test<QC>(
            market_id) == econia_share, 0);
    }

    #[test]
    /// Verify returns, state updates for complete sell fill with no
    /// ticks left to fill on matched order.
    fun test_match_complete_fill_no_ticks_sell()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare shared/dependent market parameters.
        let direction_taker     = SELL;
        let side_maker          = if (direction_taker == BUY) ASK else BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let self_match_behavior = ABORT;
        // Declare additional maker order parameters.
        let custodian_id  = NO_CUSTODIAN;
        let maker_address = address_of(&maker);
        let restriction   = NO_RESTRICTION;
        // Declare price set to product of fee divisors, to eliminate
        // truncation when predicting fee amounts.
        let price = integrator_divisor * taker_divisor;
        // Declare order size posted by maker, filled by taker.
        let size_maker = MIN_SIZE_COIN;
        let size_taker = size_maker;
        // Declare base/quote posted/filled by maker/taker.
        let base_maker  = size_maker * LOT_SIZE_COIN;
        let quote_maker = size_maker * price * TICK_SIZE_COIN;
        let base_taker  = size_taker * LOT_SIZE_COIN;
        let quote_taker = size_taker * price * TICK_SIZE_COIN;
        // Declare fee and trade amounts, from taker's perspective.
        let base_trade       = base_taker;
        let integrator_share = quote_taker / integrator_divisor;
        let econia_share     = quote_taker / taker_divisor - integrator_share;
        let fee              = integrator_share + econia_share;
        let quote_trade      = if (direction_taker == BUY)
            (quote_taker + fee) else (quote_taker - fee);
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 - base_maker;
        let deposit_quote = quote_maker;
        // Declare expected maker asset counts after matching.
        let base_total_end      = HI_64;
        let base_available_end  = HI_64;
        let base_ceiling_end    = HI_64;
        let quote_total_end     = 0;
        let quote_available_end = 0;
        let quote_ceiling_end   = 0;
        // Assign min/max base/quote swap input amounts for taker.
        let min_base  = 0;
        let max_base  = base_trade * 2;
        let min_quote = 0;
        let max_quote = quote_trade;
        // Declare swap coin input starting amounts.
        let base_coin_start = max_base;
        let quote_coin_start = HI_64 - max_quote;
        // Declare swap coin end amounts.
        let base_coin_end = max_base - base_trade;
        let quote_coin_end = HI_64;
        // Create swap coin inputs.
        let base_coins  = assets::mint_test<BC>(base_coin_start);
        let quote_coins = assets::mint_test<QC>(quote_coin_start);
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, @integrator, side_maker, size_maker, price,
            restriction, self_match_behavior);
        // Get user-side order access key for later.
        let (_, _, _, _, order_access_key) =
            get_order_fields_test(market_id, side_maker, market_order_id);
        // Invoke matching engine via coin swap.
        let (base_coins, quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_coins(market_id, integrator, direction_taker, min_base,
                       max_base, min_quote, max_quote, price, base_coins,
                       quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    restriction,
                    self_match_behavior,
                    size_maker,
                    market_order_id
                )
            ], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_fill_event_internal(
                    market_id,
                    size_taker,
                    price,
                    side_maker,
                    maker_address,
                    custodian_id,
                    market_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            market_id) == vector[
                PlaceSwapOrderEvent{
                    market_id,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction: direction_taker,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            market_id) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    taker_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    CANCEL_REASON_NOT_ENOUGH_LIQUIDITY
                ),
            ], 0);
        // Assert returns.
        assert!(coin::value(&base_coins)  == base_coin_end, 0);
        assert!(coin::value(&quote_coins) == quote_coin_end, 0);
        assert!(base_trade_r              == base_trade, 0);
        assert!(quote_trade_r             == quote_trade, 0);
        assert!(fee_r                     == fee, 0);
        // Burn coins.
        if (base_coin_end == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_coin_end == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
        // Assert list node order inactive.
        assert!(!is_list_node_order_active(
            market_id, side_maker, market_order_id), 0);
        // Assert user-side order fields for filled maker order.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            order_access_key);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
        // Assert integrator fee share.
        assert!(incentives::get_integrator_fee_store_balance_test<QC>(
            @integrator, market_id) == integrator_share, 0);
        // Assert Econia fee share.
        assert!(incentives::get_econia_fee_store_balance_test<QC>(
            market_id) == econia_share, 0);
    }

    #[test]
    /// Verify returns for no orders to match against.
    fun test_match_empty()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Declare swap arguments.
        let market_id   = MARKET_ID_COIN;
        let integrator  = @integrator;
        let direction   = BUY;
        let min_base    = 0;
        let max_base    = LOT_SIZE_COIN;
        let min_quote   = 0;
        let max_quote   = TICK_SIZE_COIN;
        let limit_price = 1;
        let base_coins  = coin::zero<BC>();
        let quote_coins = assets::mint_test<QC>(max_quote);
        // Assert events.
        assert!(!exists_market_event_handles(), 0);
        // Invoke matching engine via coin swap.
        let (base_coins, quote_coins, base_trade, quote_trade, fee) =
            swap_coins(market_id, integrator, direction, min_base, max_base,
                       min_quote, max_quote, limit_price, base_coins,
                       quote_coins);
        let taker_order_id = order_id_no_post(1);
        // Assert events.
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[
                PlaceSwapOrderEvent{
                    market_id,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote,
                    limit_price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            market_id) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    taker_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    CANCEL_REASON_NOT_ENOUGH_LIQUIDITY
                ),
            ], 0);
        // Assert returns.
        assert!(coin::value(&base_coins)  == 0, 0);
        assert!(coin::value(&quote_coins) == max_quote, 0);
        assert!(base_trade                == 0, 0);
        assert!(quote_trade               == 0, 0);
        assert!(fee                       == 0, 0);
        // Destroy coins.
        coin::destroy_zero(base_coins);
        assets::burn(quote_coins);
    }

    #[test]
    /// Verify returns for not enough size to fill.
    fun test_match_fill_size_0()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare shared/dependent market parameters.
        let direction_taker     = SELL;
        let side_maker          = if (direction_taker == BUY) ASK else BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let self_match_behavior = ABORT;
        // Declare additional maker order parameters.
        let custodian_id  = NO_CUSTODIAN;
        let maker_address = address_of(&maker);
        let restriction   = NO_RESTRICTION;
        // Declare price set to product of fee divisors, to eliminate
        // truncation when predicting fee amounts.
        let price = integrator_divisor * taker_divisor;
        // Declare order size posted by maker.
        let size_maker = MIN_SIZE_COIN + 10;
        // Declare base and quote required to fill maker.
        let base_maker = size_maker * LOT_SIZE_COIN;
        let quote_maker = size_maker * price * TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 - base_maker;
        let deposit_quote = quote_maker;
        // Declare expected maker asset counts after matching.
        let base_total_end      = deposit_base;
        let base_available_end  = base_total_end;
        let base_ceiling_end    = HI_64;
        let quote_total_end     = deposit_quote;
        let quote_available_end = 0;
        let quote_ceiling_end   = quote_total_end;
        // Declare maker order size after matching.
        let size_maker_end = size_maker;
        // Assign min/max base/quote swap input amounts for taker.
        let min_base  = 0;
        let max_base  = LOT_SIZE_COIN - 1;
        let min_quote = 0;
        let max_quote = MAX_POSSIBLE;
        // Declare swap coin input starting amounts.
        let base_coin_start = max_base;
        let quote_coin_start = 0;
        // Declare swap coin end amounts.
        let base_coin_end = base_coin_start;
        let quote_coin_end = quote_coin_start;
        // Create swap coin inputs.
        let base_coins  = assets::mint_test<BC>(base_coin_start);
        let quote_coins = assets::mint_test<QC>(quote_coin_start);
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, @integrator, side_maker, size_maker, price,
            restriction, self_match_behavior);
        // Invoke matching engine via coin swap.
        let (base_coins, quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_coins(market_id, integrator, direction_taker, min_base,
                       max_base, min_quote, max_quote, price, base_coins,
                       quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    restriction,
                    self_match_behavior,
                    size_maker,
                    market_order_id
                )
            ], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            market_id) == vector[
                PlaceSwapOrderEvent{
                    market_id,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction: direction_taker,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            market_id) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    taker_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    CANCEL_REASON_TOO_SMALL_TO_FILL_LOT
                ),
            ], 0);
        // Assert returns.
        assert!(coin::value(&base_coins)  == base_coin_end, 0);
        assert!(coin::value(&quote_coins) == quote_coin_end, 0);
        assert!(base_trade_r              == 0, 0);
        assert!(quote_trade_r             == 0, 0);
        assert!(fee_r                     == 0, 0);
        // Burn coins.
        if (base_coin_end == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_coin_end == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(market_id, side_maker, market_order_id);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker_end, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == maker_address, 0);
        assert!(custodian_id_r == custodian_id, 0);
        // Assert user-side maker order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r            == size_maker_end, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
    }

    #[test]
    /// Verify returns, state updates for complete fill on first order,
    /// partial fill on second order during match loop. A taker sell
    /// where one maker has two bids at different prices.
    fun test_match_loop_twice()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare shared/dependent market parameters.
        let direction_taker     = SELL;
        let side_maker          = if (direction_taker == BUY) ASK else BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let self_match_behavior = ABORT;
        // Declare additional maker order parameters.
        let custodian_id  = NO_CUSTODIAN;
        let maker_address = address_of(&maker);
        let restriction   = NO_RESTRICTION;
        // Declare lower price set to product of fee divisors, to
        // eliminate truncation when predicting fee amounts.
        let price_lo = integrator_divisor * taker_divisor;
        // Declare higher price for an order closer to the spread.
        let price_hi = integrator_divisor * taker_divisor * 2;
        // Declare order size posted by maker on order with high and
        // with low price.
        let size_maker_hi = MIN_SIZE_COIN + 123;
        let size_maker_lo = MIN_SIZE_COIN + 321;
        // Declare base and quote posted by maker.
        let base_maker_hi  = size_maker_hi * LOT_SIZE_COIN;
        let base_maker_lo  = size_maker_lo * LOT_SIZE_COIN;
        let base_maker     = base_maker_hi + base_maker_lo;
        let quote_maker_hi = size_maker_hi * price_hi * TICK_SIZE_COIN;
        let quote_maker_lo = size_maker_lo * price_lo * TICK_SIZE_COIN;
        let quote_maker    = quote_maker_hi + quote_maker_lo;
        // Declare taker match amounts.
        let size_taker_hi  = size_maker_hi;
        let size_taker_lo  = size_maker_lo - MIN_SIZE_COIN;
        let size_taker     = size_taker_hi + size_taker_lo;
        let base_taker     = size_taker * LOT_SIZE_COIN;
        let quote_taker_hi = quote_maker_hi;
        let quote_taker_lo = size_taker_lo * price_lo * TICK_SIZE_COIN;
        let quote_taker    = quote_taker_hi + quote_taker_lo;
        // Declare trade and fee amounts.
        let base_trade       = base_taker;
        let integrator_share = quote_taker / integrator_divisor;
        let econia_share     = quote_taker / taker_divisor - integrator_share;
        let fee              = integrator_share + econia_share;
        let quote_trade      = if (direction_taker == BUY)
            (quote_taker + fee) else (quote_taker - fee);
        let fee_hi           = quote_taker_hi / taker_divisor;
        let fee_lo           = quote_taker_lo / taker_divisor;
        assert!(fee_hi + fee_lo == fee, 0);
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 - base_maker;
        let deposit_quote = quote_maker;
        // Declare expected maker asset counts after matching.
        let base_total_end      = deposit_base + base_taker;
        let base_available_end  = base_total_end;
        let base_ceiling_end    = HI_64;
        let quote_total_end     = deposit_quote - quote_taker;
        let quote_available_end = 0;
        let quote_ceiling_end   = quote_total_end;
        // Declare maker order size after matching, for partially filled
        // order
        let size_maker_lo_end = size_maker_lo - size_taker_lo;
        // Assign min/max base/quote swap input amounts for taker.
        let min_base  = 0;
        let max_base  = base_trade;
        let min_quote = 0;
        let max_quote = quote_trade * 2;
        // Swap price is that of second order to loop against.
        let price = price_lo;
        // Declare swap coin input starting amounts.
        let base_coin_start = max_base;
        let quote_coin_start = 0;
        // Declare swap coin end amounts.
        let base_coin_end = 0;
        let quote_coin_end = quote_trade;
        // Create swap coin inputs.
        let base_coins  = assets::mint_test<BC>(base_coin_start);
        let quote_coins = assets::mint_test<QC>(quote_coin_start);
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker orders, storing market order IDs for lookup.
        let (market_order_id_hi, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, @integrator, side_maker, size_maker_hi,
            price_hi, restriction, self_match_behavior);
        let (market_order_id_lo, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, @integrator, side_maker, size_maker_lo,
            price_lo, restriction, self_match_behavior);
        // Assert order book counter.
        assert!(get_order_book_counter(market_id) == 2, 0);
        // Get user-side high-price order access key for later.
        let (_, _, _, _, order_access_key_hi) =
            get_order_fields_test(market_id, side_maker, market_order_id_hi);
        // Invoke matching engine via coin swap.
        let (base_coins, quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_coins(market_id, integrator, direction_taker, min_base,
                       max_base, min_quote, max_quote, price, base_coins,
                       quote_coins);
        // Assert order book counter.
        assert!(get_order_book_counter(market_id) == 3, 0);
        let taker_order_id = order_id_no_post(3);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    @integrator,
                    side_maker,
                    size_maker_hi,
                    price_hi,
                    restriction,
                    self_match_behavior,
                    size_maker_hi,
                    market_order_id_hi
                ),
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    @integrator,
                    side_maker,
                    size_maker_lo,
                    price_lo,
                    restriction,
                    self_match_behavior,
                    size_maker_lo,
                    market_order_id_lo
                )
            ], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_fill_event_internal(
                    market_id,
                    size_taker_hi,
                    price_hi,
                    side_maker,
                    maker_address,
                    custodian_id,
                    market_order_id_hi,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee_hi,
                    0
                ),
                user::create_fill_event_internal(
                    market_id,
                    size_taker_lo,
                    price_lo,
                    side_maker,
                    maker_address,
                    custodian_id,
                    market_order_id_lo,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee_lo,
                    1
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            market_id) == vector[
                PlaceSwapOrderEvent{
                    market_id,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction: direction_taker,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(market_id) == vector[], 0);
        // Assert returns.
        assert!(coin::value(&base_coins)  == base_coin_end, 0);
        assert!(coin::value(&quote_coins) == quote_coin_end, 0);
        assert!(base_trade_r              == base_trade, 0);
        assert!(quote_trade_r             == quote_trade, 0);
        assert!(fee_r                     == fee, 0);
        // Burn coins.
        if (base_coin_end == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_coin_end == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
        // Assert integrator fee share.
        assert!(incentives::get_integrator_fee_store_balance_test<QC>(
            @integrator, market_id) == integrator_share, 0);
        // Assert Econia fee share.
        assert!(incentives::get_econia_fee_store_balance_test<QC>(
            market_id) == econia_share, 0);
        // Assert no order book-side and user-side for full fill.
        // Assert list node order inactive for higher price.
        assert!(!is_list_node_order_active(
            market_id, side_maker, market_order_id_hi), 0);
        // Assert user-side order fields for filled maker order.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            order_access_key_hi);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        // Get fields for maker order still on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key_lo) =
            get_order_fields_test(market_id, side_maker, market_order_id_lo);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker_lo_end, 0);
        assert!(price_r        == price_lo, 0);
        assert!(user_r         == maker_address, 0);
        assert!(custodian_id_r == custodian_id, 0);
        // Assert user-side maker order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            order_access_key_lo);
        assert!(market_order_id_r == market_order_id_lo, 0);
        assert!(size_r            == size_maker_lo_end, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_MIN_BASE_NOT_TRADED)]
    /// Verify failure for minimum base amount not traded.
    fun test_match_min_base_not_traded()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Declare swap arguments.
        let market_id   = MARKET_ID_COIN;
        let integrator  = @integrator;
        let direction   = BUY;
        let min_base    = 1;
        let max_base    = MAX_POSSIBLE;
        let min_quote   = 0;
        let max_quote   = MAX_POSSIBLE;
        let limit_price = 1;
        let base_coins  = coin::zero<BC>();
        let quote_coins = assets::mint_test<QC>(HI_64);
        // Invoke matching engine via coin swap against empty book.
        let (base_coins, quote_coins, _, _, _) = swap_coins(
            market_id, integrator, direction, min_base, max_base, min_quote,
            max_quote, limit_price, base_coins, quote_coins);
        // Burn coins.
        if (coin::value(&base_coins) == 0) coin::destroy_zero(base_coins)
            else assets::burn(base_coins);
        if (coin::value(&quote_coins) == 0) coin::destroy_zero(quote_coins)
            else assets::burn(quote_coins);
    }

    #[test]
    #[expected_failure(abort_code = E_MIN_QUOTE_NOT_TRADED)]
    /// Verify failure for minimum quote amount not traded.
    fun test_match_min_quote_not_traded()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Declare swap arguments.
        let market_id   = MARKET_ID_COIN;
        let integrator  = @integrator;
        let direction   = SELL;
        let min_base    = 0;
        let max_base    = MAX_POSSIBLE;
        let min_quote   = 1;
        let max_quote   = MAX_POSSIBLE;
        let limit_price = 1;
        let base_coins = assets::mint_test<BC>(HI_64);
        let quote_coins  = coin::zero<QC>();
        // Invoke matching engine via coin swap against empty book.
        let (base_coins, quote_coins, _, _, _) = swap_coins(
            market_id, integrator, direction, min_base, max_base, min_quote,
            max_quote, limit_price, base_coins, quote_coins);
        // Burn coins.
        if (coin::value(&base_coins) == 0) coin::destroy_zero(base_coins)
            else assets::burn(base_coins);
        if (coin::value(&quote_coins) == 0) coin::destroy_zero(quote_coins)
            else assets::burn(quote_coins);
    }

    #[test]
    /// Verify assorted operations for evicting orders of size 0 from
    /// the order book during matching, where the bids on the book are
    /// in priority from first to last:
    ///
    /// 1. Order of size 0.
    /// 2. Valid order.
    /// 3. Order of size 0.
    /// 4. Valid order.
    /// 5. Order of size 0.
    ///
    /// This requires manually doctoring the minimum order size for the
    /// market so that order size change operations can be used to put
    /// empty orders on the book.
    fun test_match_order_size_0()
    acquires
        OrderBooks
    {
        // General setup.
        let (maker, taker) = init_markets_users_integrator_test();
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        let direction_taker = SELL;
        let side_maker = BID;
        let market_id = MARKET_ID_COIN;
        let integrator = @integrator;
        let self_match_behavior = ABORT;
        let custodian_id = NO_CUSTODIAN;
        let maker_address = address_of(&maker);
        let taker_address = address_of(&taker);
        let restriction = NO_RESTRICTION;
        // Set price to product of fee divisors, to eliminate trucation
        // when predicting fee amounts.
        let price = integrator_divisor * taker_divisor;
        // Size of valid maker orders on book.
        let size_maker_valid_0 = MIN_SIZE_COIN * 2;
        let size_maker_valid_1 = MIN_SIZE_COIN * 4;
        let size_maker_valid_total = size_maker_valid_0 + size_maker_valid_1;
        // Size matched by taker and requested size.
        let size_taker = size_maker_valid_total;
        let size_taker_requested = size_taker + 1;
        // Base and quote trade amounts seen by taker.
        let base_maker = size_maker_valid_total * LOT_SIZE_COIN;
        let quote_maker = size_maker_valid_total * price * TICK_SIZE_COIN;
        // Fees.
        let integrator_share = quote_maker / integrator_divisor;
        let econia_share = quote_maker / taker_divisor - integrator_share;
        let fee = integrator_share + econia_share;
        let fee_valid_0 = ((fee as u128) * (size_maker_valid_0 as u128) /
            (size_maker_valid_total as u128) as u64);
        let fee_valid_1 = fee - fee_valid_0;
        // Quote trade amount seen by taker.
        let quote_taker = if (direction_taker == BUY)
            (quote_maker + fee) else (quote_maker - fee);
        // Deposit enough for maker and taker to avoid over/underflow.
        let deposit_amount = HI_64 / 2;
        // Expected maker asset counts after matching.
        let base_total_end_maker      = deposit_amount + base_maker;
        let base_available_end_maker  = base_total_end_maker;
        let base_ceiling_end_maker    = base_total_end_maker;
        let quote_total_end_maker     = deposit_amount - quote_maker;
        let quote_available_end_maker = quote_total_end_maker;
        let quote_ceiling_end_maker   = quote_total_end_maker;
        // Expected taker asset counts after matching.
        let base_total_end_taker      = deposit_amount - base_maker;
        let base_available_end_taker  = base_total_end_taker;
        let base_ceiling_end_taker    = base_total_end_taker;
        let quote_total_end_taker     = deposit_amount + quote_taker;
        let quote_available_end_taker = quote_total_end_taker;
        let quote_ceiling_end_taker   = quote_total_end_taker;
        // Deposit maker and taker coins.
        user::deposit_coins<BC>(
            maker_address,
            market_id,
            custodian_id,
            assets::mint_test(deposit_amount)
        );
        user::deposit_coins<BC>(
            taker_address,
            market_id,
            custodian_id,
            assets::mint_test(deposit_amount)
        );
        user::deposit_coins<QC>(
            maker_address,
            market_id,
            custodian_id,
            assets::mint_test(deposit_amount)
        );
        user::deposit_coins<QC>(
            taker_address,
            market_id,
            custodian_id,
            assets::mint_test(deposit_amount)
        );
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_change_order_size_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, taker_address, custodian_id) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, taker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, taker_address, custodian_id) == vector[], 0);
        // Place limit orders.
        let (order_id_invalid_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side_maker, MIN_SIZE_COIN,
            price, restriction, self_match_behavior);
        let (order_id_valid_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side_maker, size_maker_valid_0,
            price, restriction, self_match_behavior);
        let (order_id_invalid_1, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side_maker, MIN_SIZE_COIN,
            price, restriction, self_match_behavior);
        let (order_id_valid_1, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side_maker, size_maker_valid_1,
            price, restriction, self_match_behavior);
        let (order_id_invalid_2, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side_maker, MIN_SIZE_COIN,
            price, restriction, self_match_behavior);
        // Get maker order access keys, assert they are all open.
        let (_, _, _, _, access_key_invalid_0) =
            get_order_fields_test(market_id, side_maker, order_id_invalid_0);
        let (_, _, _, _, access_key_valid_0) =
            get_order_fields_test(market_id, side_maker, order_id_valid_0);
        let (_, _, _, _, access_key_invalid_1) =
            get_order_fields_test(market_id, side_maker, order_id_invalid_1);
        let (_, _, _, _, access_key_valid_1) =
            get_order_fields_test(market_id, side_maker, order_id_valid_1);
        let (_, _, _, _, access_key_invalid_2) =
            get_order_fields_test(market_id, side_maker, order_id_invalid_2);
        // Verify orders marked open.
        let market_account_id =
            user::get_market_account_id(market_id, custodian_id);
        assert!(user::has_order_test(
            maker_address, market_account_id, side_maker,
            access_key_invalid_0), 0);
        assert!(user::has_order_test(
            maker_address, market_account_id, side_maker,
            access_key_valid_0), 0);
        assert!(user::has_order_test(
            maker_address, market_account_id, side_maker,
            access_key_invalid_1), 0);
        assert!(user::has_order_test(
            maker_address, market_account_id, side_maker,
            access_key_valid_1), 0);
        assert!(user::has_order_test(
            maker_address, market_account_id, side_maker,
            access_key_invalid_2), 0);
        // Manually doctor min size for market so that orders can have
        // their size changed to 0.
        let resource_address = resource_account::get_address();
        let order_books_map_ref_mut =
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        tablist::borrow_mut(order_books_map_ref_mut, market_id).min_size = 0;
        change_order_size_user(
            &maker, market_id, side_maker, order_id_invalid_0, 0);
        change_order_size_user(
            &maker, market_id, side_maker, order_id_invalid_1, 0);
        change_order_size_user(
            &maker, market_id, side_maker, order_id_invalid_2, 0);
        // Change back minimum order size for market.
        order_books_map_ref_mut =
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        tablist::borrow_mut(order_books_map_ref_mut, market_id).min_size =
            MIN_SIZE_COIN;
        // Place a market order that eats all liquidity and evicts all
        // orders of size 0.
        place_market_order_user<BC, QC>(
            &taker, market_id, integrator, direction_taker,
            size_taker_requested, self_match_behavior);
        let taker_order_id = order_id_no_post(6);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    integrator,
                    side_maker,
                    MIN_SIZE_COIN,
                    price,
                    restriction,
                    self_match_behavior,
                    MIN_SIZE_COIN,
                    order_id_invalid_0,
                ),
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    integrator,
                    side_maker,
                    size_maker_valid_0,
                    price,
                    restriction,
                    self_match_behavior,
                    size_maker_valid_0,
                    order_id_valid_0,
                ),
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    integrator,
                    side_maker,
                    MIN_SIZE_COIN,
                    price,
                    restriction,
                    self_match_behavior,
                    MIN_SIZE_COIN,
                    order_id_invalid_1,
                ),
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    integrator,
                    side_maker,
                    size_maker_valid_1,
                    price,
                    restriction,
                    self_match_behavior,
                    size_maker_valid_1,
                    order_id_valid_1,
                ),
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    integrator,
                    side_maker,
                    MIN_SIZE_COIN,
                    price,
                    restriction,
                    self_match_behavior,
                    MIN_SIZE_COIN,
                    order_id_invalid_2,
                ),
            ], 0);
        assert!(user::get_change_order_size_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_change_order_size_event_test(
                    market_id,
                    order_id_invalid_0,
                    maker_address,
                    custodian_id,
                    side_maker,
                    0,
                ),
                user::create_change_order_size_event_test(
                    market_id,
                    order_id_invalid_1,
                    maker_address,
                    custodian_id,
                    side_maker,
                    0,
                ),
                user::create_change_order_size_event_test(
                    market_id,
                    order_id_invalid_2,
                    maker_address,
                    custodian_id,
                    side_maker,
                    0,
                ),
            ], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, taker_address, custodian_id) == vector[
                user::create_place_market_order_event_test(
                    market_id,
                    taker_address,
                    custodian_id,
                    integrator,
                    direction_taker,
                    size_taker_requested,
                    self_match_behavior,
                    taker_order_id,
                )
            ], 0);
        let expected_fills = vector[
            user::create_fill_event_internal(
                market_id,
                size_maker_valid_0,
                price,
                side_maker,
                maker_address,
                custodian_id,
                order_id_valid_0,
                taker_address,
                custodian_id,
                taker_order_id,
                fee_valid_0,
                0,
            ),
            user::create_fill_event_internal(
                market_id,
                size_maker_valid_1,
                price,
                side_maker,
                maker_address,
                custodian_id,
                order_id_valid_1,
                taker_address,
                custodian_id,
                taker_order_id,
                fee_valid_1,
                1,
            ),
        ];
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == expected_fills, 0);
        assert!(user::get_fill_events_test(
            market_id, taker_address, custodian_id) == expected_fills, 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    order_id_invalid_0,
                    maker_address,
                    custodian_id,
                    CANCEL_REASON_EVICTION,
                ),
                user::create_cancel_order_event_internal(
                    market_id,
                    order_id_invalid_1,
                    maker_address,
                    custodian_id,
                    CANCEL_REASON_EVICTION,
                ),
                user::create_cancel_order_event_internal(
                    market_id,
                    order_id_invalid_2,
                    maker_address,
                    custodian_id,
                    CANCEL_REASON_EVICTION,
                ),
            ], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, taker_address, custodian_id) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    taker_order_id,
                    taker_address,
                    custodian_id,
                    CANCEL_REASON_NOT_ENOUGH_LIQUIDITY,
                ),
            ], 0);
        // Maker asset/collateral counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end_maker, 0);
        assert!(base_available  == base_available_end_maker, 0);
        assert!(base_ceiling    == base_ceiling_end_maker, 0);
        assert!(quote_total     == quote_total_end_maker, 0);
        assert!(quote_available == quote_available_end_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_end_maker, 0);
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id)
            == base_total_end_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id)
            == quote_total_end_maker, 0);
        // Taker asset/collateral counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                taker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end_taker, 0);
        assert!(base_available  == base_available_end_taker, 0);
        assert!(base_ceiling    == base_ceiling_end_taker, 0);
        assert!(quote_total     == quote_total_end_taker, 0);
        assert!(quote_available == quote_available_end_taker, 0);
        assert!(quote_ceiling   == quote_ceiling_end_taker, 0);
        assert!(user::get_collateral_value_simple_test<BC>(
            taker_address, market_id, custodian_id)
            == base_total_end_taker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            taker_address, market_id, custodian_id)
            == quote_total_end_taker, 0);
        // Integrator fee share.
        assert!(incentives::get_integrator_fee_store_balance_test<QC>(
            integrator, market_id) == integrator_share, 0);
        // Econia fee share.
        assert!(incentives::get_econia_fee_store_balance_test<QC>(
            market_id) == econia_share, 0);
        // Verify list node orders are inactive.
        assert!(!is_list_node_order_active(
            market_id, side_maker, order_id_invalid_0), 0);
        assert!(!is_list_node_order_active(
            market_id, side_maker, order_id_valid_0), 0);
        assert!(!is_list_node_order_active(
            market_id, side_maker, order_id_invalid_1), 0);
        assert!(!is_list_node_order_active(
            market_id, side_maker, order_id_valid_1), 0);
        assert!(!is_list_node_order_active(
            market_id, side_maker, order_id_invalid_2), 0);
        // Assert order fields for cancelled/filled maker orders.
        let (order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            access_key_invalid_0);
        assert!(order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        let (order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            access_key_valid_0);
        assert!(order_id_r == (NIL as u128), 0);
        assert!(size_r == 1, 0);
        let (order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            access_key_invalid_1);
        assert!(order_id_r == (NIL as u128), 0);
        assert!(size_r == 2, 0);
        let (order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            access_key_valid_1);
        assert!(order_id_r == (NIL as u128), 0);
        assert!(size_r == 3, 0);
        let (order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            access_key_invalid_2);
        assert!(order_id_r == (NIL as u128), 0);
        assert!(size_r == 4, 0);
    }

    #[test]
    /// Verify returns for partial sell fill with lot-limited fill size.
    fun test_match_partial_fill_lot_limited_sell()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare shared/dependent market parameters.
        let direction_taker     = SELL;
        let side_maker          = if (direction_taker == BUY) ASK else BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let self_match_behavior = ABORT;
        // Declare additional maker order parameters.
        let custodian_id  = NO_CUSTODIAN;
        let maker_address = address_of(&maker);
        let restriction   = NO_RESTRICTION;
        // Declare price set to product of fee divisors, to eliminate
        // truncation when predicting fee amounts.
        let price = integrator_divisor * taker_divisor;
        // Declare order size posted by maker, filled by taker.
        let size_maker = MIN_SIZE_COIN + 10;
        let size_taker = 5;
        // Declare base/quote posted/filled by maker/taker.
        let base_maker  = size_maker * LOT_SIZE_COIN;
        let quote_maker = size_maker * price * TICK_SIZE_COIN;
        let base_taker  = size_taker * LOT_SIZE_COIN;
        let quote_taker = size_taker * price * TICK_SIZE_COIN;
        // Declare fee and trade amounts, from taker's perspective.
        let base_trade       = base_taker;
        let integrator_share = quote_taker / integrator_divisor;
        let econia_share     = quote_taker / taker_divisor - integrator_share;
        let fee              = integrator_share + econia_share;
        let quote_trade      = if (direction_taker == BUY)
            (quote_taker + fee) else (quote_taker - fee);
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 - base_maker;
        let deposit_quote = quote_maker;
        // Declare expected maker asset counts after matching.
        let base_total_end      = deposit_base + base_taker;
        let base_available_end  = base_total_end;
        let base_ceiling_end    = HI_64;
        let quote_total_end     = deposit_quote - quote_taker;
        let quote_available_end = 0;
        let quote_ceiling_end   = quote_total_end;
        // Declare maker order size after matching.
        let size_maker_end = size_maker - size_taker;
        // Assign min/max base/quote swap input amounts for taker.
        let min_base  = 0;
        let max_base  = base_trade;
        let min_quote = 0;
        let max_quote = quote_trade * 2;
        // Declare swap coin input starting amounts.
        let base_coin_start = max_base;
        let quote_coin_start = 0;
        // Declare swap coin end amounts.
        let base_coin_end = 0;
        let quote_coin_end = quote_trade;
        // Create swap coin inputs.
        let base_coins  = assets::mint_test<BC>(base_coin_start);
        let quote_coins = assets::mint_test<QC>(quote_coin_start);
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, @integrator, side_maker, size_maker, price,
            restriction, self_match_behavior);
        // Invoke matching engine via coin swap.
        let (base_coins, quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_coins(market_id, integrator, direction_taker, min_base,
                       max_base, min_quote, max_quote, price, base_coins,
                       quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    restriction,
                    self_match_behavior,
                    size_maker,
                    market_order_id
                )
            ], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_fill_event_internal(
                    market_id,
                    size_taker,
                    price,
                    side_maker,
                    maker_address,
                    custodian_id,
                    market_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                ),
            ], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            market_id) == vector[
                PlaceSwapOrderEvent{
                    market_id,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction: direction_taker,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(market_id) == vector[], 0);
        // Assert returns.
        assert!(coin::value(&base_coins)  == base_coin_end, 0);
        assert!(coin::value(&quote_coins) == quote_coin_end, 0);
        assert!(base_trade_r              == base_trade, 0);
        assert!(quote_trade_r             == quote_trade, 0);
        assert!(fee_r                     == fee, 0);
        // Burn coins.
        if (base_coin_end == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_coin_end == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(market_id, side_maker, market_order_id);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker_end, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == maker_address, 0);
        assert!(custodian_id_r == custodian_id, 0);
        // Assert user-side maker order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r            == size_maker_end, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
        // Assert integrator fee share.
        assert!(incentives::get_integrator_fee_store_balance_test<QC>(
            @integrator, market_id) == integrator_share, 0);
        // Assert Econia fee share.
        assert!(incentives::get_econia_fee_store_balance_test<QC>(
            market_id) == econia_share, 0);
    }

    #[test]
    /// Verify returns for partial buy fill with tick-limited fill size.
    fun test_match_partial_fill_tick_limited_buy()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare shared/dependent market parameters.
        let direction_taker     = BUY;
        let side_maker          = if (direction_taker == BUY) ASK else BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let self_match_behavior = ABORT;
        // Declare additional maker order parameters.
        let custodian_id  = NO_CUSTODIAN;
        let maker_address = address_of(&maker);
        let restriction   = NO_RESTRICTION;
        // Declare price set to product of fee divisors, to eliminate
        // truncation when predicting fee amounts.
        let price = integrator_divisor * taker_divisor;
        // Declare order size posted by maker, filled by taker.
        let size_maker = MIN_SIZE_COIN + 10;
        let size_taker = 5;
        // Declare base/quote posted/filled by maker/taker.
        let base_maker  = size_maker * LOT_SIZE_COIN;
        let quote_maker = size_maker * price * TICK_SIZE_COIN;
        let base_taker  = size_taker * LOT_SIZE_COIN;
        let quote_taker = size_taker * price * TICK_SIZE_COIN;
        // Declare fee and trade amounts, from taker's perspective.
        let base_trade       = base_taker;
        let integrator_share = quote_taker / integrator_divisor;
        let econia_share     = quote_taker / taker_divisor - integrator_share;
        let fee              = integrator_share + econia_share;
        let quote_trade      = if (direction_taker == BUY)
            (quote_taker + fee) else (quote_taker - fee);
        // Declare maker deposit amounts.
        let deposit_base  = base_maker;
        let deposit_quote = HI_64 - quote_maker;
        // Declare expected maker asset counts after matching.
        let base_total_end      = deposit_base - base_taker;
        let base_available_end  = 0;
        let base_ceiling_end    = base_total_end;
        let quote_total_end     = deposit_quote + quote_taker;
        let quote_available_end = quote_total_end;
        let quote_ceiling_end   = HI_64;
        // Declare maker order size after matching.
        let size_maker_end = size_maker - size_taker;
        // Assign min/max base/quote swap input amounts for taker.
        let min_base  = 0;
        let max_base  = (size_taker + 2) * LOT_SIZE_COIN;
        let min_quote = 0;
        let max_quote = quote_trade;
        // Declare swap coin input starting amounts.
        let base_coin_start = 0;
        let quote_coin_start = max_quote;
        // Declare swap coin end amounts.
        let base_coin_end = base_trade;
        let quote_coin_end = 0;
        // Create swap coin inputs.
        let base_coins  = assets::mint_test<BC>(base_coin_start);
        let quote_coins = assets::mint_test<QC>(quote_coin_start);
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, @integrator, side_maker, size_maker, price,
            restriction, self_match_behavior);
        // Invoke matching engine via coin swap.
        let (base_coins, quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_coins(market_id, integrator, direction_taker, min_base,
                       max_base, min_quote, max_quote, price, base_coins,
                       quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    restriction,
                    self_match_behavior,
                    size_maker,
                    market_order_id
                )
            ], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_fill_event_internal(
                    market_id,
                    size_taker,
                    price,
                    side_maker,
                    maker_address,
                    custodian_id,
                    market_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                ),
            ], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            market_id) == vector[
                PlaceSwapOrderEvent{
                    market_id,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction: direction_taker,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(market_id) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    taker_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
        ], 0);
        // Assert returns.
        assert!(coin::value(&base_coins)  == base_coin_end, 0);
        assert!(coin::value(&quote_coins) == quote_coin_end, 0);
        assert!(base_trade_r              == base_trade, 0);
        assert!(quote_trade_r             == quote_trade, 0);
        assert!(fee_r                     == fee, 0);
        // Burn coins.
        if (base_coin_end == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_coin_end == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(market_id, side_maker, market_order_id);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker_end, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == maker_address, 0);
        assert!(custodian_id_r == custodian_id, 0);
        // Assert user-side maker order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r            == size_maker_end, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
        // Assert integrator fee share.
        assert!(incentives::get_integrator_fee_store_balance_test<QC>(
            @integrator, market_id) == integrator_share, 0);
        // Assert Econia fee share.
        assert!(incentives::get_econia_fee_store_balance_test<QC>(
            market_id) == econia_share, 0);
    }

    #[test]
    /// Verify returns for limit price violation on buy.
    fun test_match_price_break_buy()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare shared/dependent market parameters.
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let direction_taker     = BUY;
        let side_maker          = if (direction_taker == BUY) ASK else BID;
        let price_taker         = 1;
        let price_maker         = price_taker + 1;
        let self_match_behavior = ABORT;
        // Declare maker order parameters.
        let maker_address = address_of(&maker);
        let custodian_id  = NO_CUSTODIAN;
        let size_maker    = MIN_SIZE_COIN;
        let base_maker    = size_maker * LOT_SIZE_COIN;
        let quote_maker   = size_maker * price_maker * TICK_SIZE_COIN;
        let restriction   = NO_RESTRICTION;
        // Declare maker deposit amounts.
        let deposit_base  = base_maker;
        let deposit_quote = HI_64 - quote_maker;
        // Declare expected maker asset counts after matching.
        let base_total_end      = deposit_base;
        let base_available_end  = 0;
        let base_ceiling_end    = base_total_end;
        let quote_total_end     = deposit_quote;
        let quote_available_end = quote_total_end;
        let quote_ceiling_end   = HI_64;
        // Declare maker order size after matching.
        let size_maker_end = size_maker;
        // Declare taker coin starting and ending amounts.
        let base_coin_start  = 0;
        let quote_coin_start = HI_64 / 2;
        let base_coin_end    = base_coin_start;
        let quote_coin_end   = quote_coin_start;
        // Declare swap arguments.
        let min_base    = 0;
        let max_base    = MAX_POSSIBLE;
        let min_quote   = 0;
        let max_quote   = 0;
        let limit_price = price_taker;
        // Create swap coin inputs.
        let base_coins  = assets::mint_test<BC>(base_coin_start);
        let quote_coins = assets::mint_test<QC>(quote_coin_start);
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, @integrator, side_maker, size_maker,
            price_maker, restriction, self_match_behavior);
        // Invoke matching engine via coin swap.
        let (base_coins, quote_coins, base_trade, quote_trade, fee) =
            swap_coins(market_id, integrator, direction_taker, min_base,
                       max_base, min_quote, max_quote, limit_price, base_coins,
                       quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    @integrator,
                    side_maker,
                    size_maker,
                    price_maker,
                    restriction,
                    self_match_behavior,
                    size_maker,
                    market_order_id
                )
            ], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            market_id) == vector[
                PlaceSwapOrderEvent{
                    market_id,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction: direction_taker,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote: quote_coin_start,
                    limit_price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(market_id) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    taker_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    CANCEL_REASON_VIOLATED_LIMIT_PRICE
                )
        ], 0);
        // Assert returns.
        assert!(coin::value(&base_coins)  == base_coin_end, 0);
        assert!(coin::value(&quote_coins) == quote_coin_end, 0);
        assert!(base_trade                == 0, 0);
        assert!(quote_trade               == 0, 0);
        assert!(fee                       == 0, 0);
        // Burn coins.
        if (base_coin_end == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_coin_end == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(market_id, side_maker, market_order_id);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker_end, 0);
        assert!(price_r        == price_maker, 0);
        assert!(user_r         == maker_address, 0);
        assert!(custodian_id_r == custodian_id, 0);
        // Assert user-side maker order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r            == size_maker_end, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
    }

    #[test]
    /// Verify returns for limit price violation on sell.
    fun test_match_price_break_sell()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare shared/dependent market parameters.
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let direction_taker     = SELL;
        let side_maker          = if (direction_taker == BUY) ASK else BID;
        let price_maker         = 1;
        let price_taker         = price_maker + 1;
        let self_match_behavior = ABORT;
        // Declare maker order parameters.
        let maker_address = address_of(&maker);
        let custodian_id  = NO_CUSTODIAN;
        let size_maker    = MIN_SIZE_COIN;
        let base_maker    = size_maker * LOT_SIZE_COIN;
        let quote_maker   = size_maker * price_maker * TICK_SIZE_COIN;
        let restriction   = NO_RESTRICTION;
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 - base_maker;
        let deposit_quote = quote_maker;
        // Declare expected maker asset counts after matching.
        let base_total_end      = deposit_base;
        let base_available_end  = base_total_end;
        let base_ceiling_end    = HI_64;
        let quote_total_end     = deposit_quote;
        let quote_available_end = 0;
        let quote_ceiling_end   = quote_total_end;
        // Declare maker order size after matching.
        let size_maker_end = size_maker;
        // Declare taker coin starting and ending amounts.
        let base_coin_start  = HI_64 / 2;
        let quote_coin_start = 0;
        let base_coin_end    = base_coin_start;
        let quote_coin_end   = quote_coin_start;
        // Declare swap arguments.
        let min_base    = 0;
        let max_base    = 0;
        let min_quote   = 0;
        let max_quote   = MAX_POSSIBLE;
        let limit_price = price_taker;
        // Create swap coin inputs.
        let base_coins  = assets::mint_test<BC>(base_coin_start);
        let quote_coins = assets::mint_test<QC>(quote_coin_start);
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, @integrator, side_maker, size_maker,
            price_maker, restriction, self_match_behavior);
        // Invoke matching engine via coin swap.
        let (base_coins, quote_coins, base_trade, quote_trade, fee) =
            swap_coins(market_id, integrator, direction_taker, min_base,
                       max_base, min_quote, max_quote, limit_price, base_coins,
                       quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, maker_address, custodian_id) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    maker_address,
                    custodian_id,
                    @integrator,
                    side_maker,
                    size_maker,
                    price_maker,
                    restriction,
                    self_match_behavior,
                    size_maker,
                    market_order_id
                )
            ], 0);
        assert!(user::get_fill_events_test(
            market_id, maker_address, custodian_id) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, maker_address, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            market_id) == vector[
                PlaceSwapOrderEvent{
                    market_id,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction: direction_taker,
                    min_base,
                    max_base: base_coin_start,
                    min_quote,
                    max_quote: HI_64 - quote_coin_start,
                    limit_price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(market_id) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    taker_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    CANCEL_REASON_VIOLATED_LIMIT_PRICE
                )
        ], 0);
        // Assert returns.
        assert!(coin::value(&base_coins)  == base_coin_end, 0);
        assert!(coin::value(&quote_coins) == quote_coin_end, 0);
        assert!(base_trade                == 0, 0);
        assert!(quote_trade               == 0, 0);
        assert!(fee                       == 0, 0);
        // Burn coins.
        if (base_coin_end == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_coin_end == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(market_id, side_maker, market_order_id);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker_end, 0);
        assert!(price_r        == price_maker, 0);
        assert!(user_r         == maker_address, 0);
        assert!(custodian_id_r == custodian_id, 0);
        // Assert user-side maker order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, market_id, custodian_id, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r            == size_maker_end, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, market_id, custodian_id);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, market_id, custodian_id) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, market_id, custodian_id) == quote_total_end, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_HEAD_KEY_PRICE_MISMATCH)]
    /// Verify failure for price mismatch between order and AVL queue
    /// head key. Test setup based on `test_match_fill_size_0()`
    fun test_match_price_mismatch()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare shared/dependent market parameters.
        let direction_taker     = SELL;
        let side_maker          = if (direction_taker == BUY) ASK else BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let self_match_behavior = ABORT;
        // Declare additional maker order parameters.
        let custodian_id  = NO_CUSTODIAN;
        let maker_address = address_of(&maker);
        let restriction   = NO_RESTRICTION;
        let price = 456;
        // Declare order size posted by maker, filled by taker.
        let size_maker = MIN_SIZE_COIN + 10;
        // Declare base and quote required to fill maker.
        let base_maker = size_maker * LOT_SIZE_COIN;
        let quote_maker = size_maker * price* TICK_SIZE_COIN;
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 - base_maker;
        let deposit_quote = quote_maker;
        // Assign min/max base/quote swap input amounts for taker.
        let min_base  = 0;
        let max_base  = LOT_SIZE_COIN - 1;
        let min_quote = 0;
        let max_quote = MAX_POSSIBLE;
        // Declare swap coin input starting amounts.
        let base_coin_start = max_base;
        let quote_coin_start = 0;
        // Create swap coin inputs.
        let base_coins  = assets::mint_test<BC>(base_coin_start);
        let quote_coins = assets::mint_test<QC>(quote_coin_start);
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place maker order.
        place_limit_order_user<BC, QC>(
            &maker, market_id, @integrator, side_maker, size_maker, price,
            restriction, self_match_behavior);
        // Get address of resource account for borrowing order book.
        let resource_address = resource_account::get_address();
        let order_books_map_ref_mut = // Mutably borrow order books map.
            &mut borrow_global_mut<OrderBooks>(resource_address).map;
        let order_book_ref_mut = // Mutably borrow market order book.
            tablist::borrow_mut(order_books_map_ref_mut, market_id);
        // Mutably borrow corresponding orders AVL queue.
        let orders_ref_mut = if (side_maker == ASK)
            &mut order_book_ref_mut.asks else &mut order_book_ref_mut.bids;
        // Mutably borrow order at head of AVL queue.
        let order_ref_mut = avl_queue::borrow_head_mut(orders_ref_mut);
        // Manually modify price.
        order_ref_mut.price = price + 1;
        // Invoke matching engine via coin swap, triggering abort.
        let (base_coins, quote_coins, _, _, _) = swap_coins(
            market_id, integrator, direction_taker, min_base, max_base,
            min_quote, max_quote, price, base_coins, quote_coins);
        // Burn coins.
        assets::burn(base_coins);
        assets::burn(quote_coins);
    }

    #[test]
    #[expected_failure(abort_code = E_PRICE_TOO_HIGH)]
    /// Verify failure for price too high.
    fun test_match_price_too_high()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Declare swap arguments.
        let market_id   = MARKET_ID_COIN;
        let integrator  = @integrator;
        let direction   = SELL;
        let min_base    = 0;
        let max_base    = MAX_POSSIBLE;
        let min_quote   = 0;
        let max_quote   = MAX_POSSIBLE;
        let limit_price = HI_PRICE + 1;
        let base_coins = assets::mint_test<BC>(HI_64);
        let quote_coins  = coin::zero<QC>();
        // Invoke matching engine via coin swap against empty book.
        let (base_coins, quote_coins, _, _, _) = swap_coins(
            market_id, integrator, direction, min_base, max_base, min_quote,
            max_quote, limit_price, base_coins, quote_coins);
        // Burn coins.
        if (coin::value(&base_coins) == 0) coin::destroy_zero(base_coins)
            else assets::burn(base_coins);
        if (coin::value(&quote_coins) == 0) coin::destroy_zero(quote_coins)
            else assets::burn(quote_coins);
    }

    #[test]
    #[expected_failure(abort_code = E_SELF_MATCH)]
    /// Verify failure for self match with abort behavior.
    fun test_match_self_match_abort()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user, _) = init_markets_users_integrator_test();
        // Declare common market parameters.
        let market_id    = MARKET_ID_COIN;
        let integrator   = @integrator;
        let user_address = address_of(&user);
        let custodian_id = NO_CUSTODIAN;
        // Declare maker order parameters.
        let side_maker  = ASK;
        let size_maker  = MIN_SIZE_COIN;
        let price_maker = 100;
        let restriction = NO_RESTRICTION;
        // Declare taker order parameters.
        let direction_taker     = if (side_maker == ASK) BUY else SELL;
        let self_match_behavior = ABORT;
        let size                = size_maker;
        // Declare deposit amounts.
        let deposit_base  = HI_64 / 2;
        let deposit_quote = HI_64 / 2;
        // Deposit coins.
        user::deposit_coins<BC>(user_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(user_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place a maker order.
        place_limit_order_user<BC, QC>(
            &user, market_id, integrator, side_maker, size_maker, price_maker,
            restriction, self_match_behavior);
        // Place an invalid taker order.
        place_market_order_user<BC, QC>(
            &user, market_id, integrator, direction_taker, size,
            self_match_behavior);
    }

    #[test]
    /// Have user place two maker asks:
    ///
    /// * One at a low price without a delegated custodian.
    /// * One at a high price with a delegated custodian.
    ///
    /// Then have signing user place a taker buy, self matching against
    /// the order at the lower price. Here, matching halts after the
    /// self match.
    fun test_match_self_match_cancel_both()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user, _) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare common market parameters.
        let market_id      = MARKET_ID_COIN;
        let integrator     = @integrator;
        let user_address   = address_of(&user);
        // Declare maker order parameters.
        let side_maker                = ASK;
        let restriction               = NO_RESTRICTION;
        let size_maker                = MIN_SIZE_COIN;
        let self_match_behavior_maker = ABORT;
        // Declare lower price set to product of fee divisors, to
        // eliminate truncation when predicting fee amounts.
        let price_lo = integrator_divisor * taker_divisor;
        // Declare higher price for an order further from the spread.
        let price_hi = integrator_divisor * taker_divisor * 2;
        // Declare taker order parameters.
        let direction_taker           = if (side_maker == ASK) BUY else SELL;
        let size_taker                = size_maker;
        let self_match_behavior_taker = CANCEL_BOTH;
        // Declare deposit amounts.
        let deposit_base  = HI_64 / 2;
        let deposit_quote = HI_64 / 2;
        // Deposit coins.
        user::deposit_coins<BC>(user_address, market_id, NO_CUSTODIAN,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(user_address, market_id, NO_CUSTODIAN,
                                assets::mint_test(deposit_quote));
        user::deposit_coins<BC>(user_address, market_id, CUSTODIAN_ID_USER_0,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(user_address, market_id, CUSTODIAN_ID_USER_0,
                                assets::mint_test(deposit_quote));
        let custodian_capability = // Get custodian capability.
            registry::get_custodian_capability_test(CUSTODIAN_ID_USER_0);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        // Place maker orders, storing market order IDs.
        let (market_order_id_lo, _, _, _) = place_limit_order_user<BC, QC>(
            &user, market_id, integrator, side_maker, size_maker, price_lo,
            restriction, self_match_behavior_maker);
        let (market_order_id_hi, _, _, _) =
                place_limit_order_custodian<BC, QC>(
            user_address, market_id, integrator, side_maker, size_maker,
            price_hi, restriction, self_match_behavior_maker,
            &custodian_capability);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
        // Get user-side order access keys for later.
        let (_, _, _, _, order_access_key_lo) =
            get_order_fields_test(market_id, side_maker, market_order_id_lo);
        let (_, _, _, _, order_access_key_hi) =
            get_order_fields_test(market_id, side_maker, market_order_id_hi);
        // Assert list node orders active.
        assert!(is_list_node_order_active(
            market_id, side_maker, market_order_id_lo), 0);
        assert!(is_list_node_order_active(
            market_id, side_maker, market_order_id_hi), 0);
        // Place taker order with self match.
        place_market_order_user<BC, QC>(
            &user, market_id, integrator, direction_taker, size_taker,
            self_match_behavior_taker);
        let taker_order_id = order_id_no_post(3);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    user_address,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price_lo,
                    restriction,
                    self_match_behavior_maker,
                    size_maker,
                    market_order_id_lo
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    user_address,
                    CUSTODIAN_ID_USER_0,
                    @integrator,
                    side_maker,
                    size_maker,
                    price_hi,
                    restriction,
                    self_match_behavior_maker,
                    size_maker,
                    market_order_id_hi
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[
                user::create_place_market_order_event_test(
                    market_id,
                    user_address,
                    NO_CUSTODIAN,
                    @integrator,
                    direction_taker,
                    size_taker,
                    self_match_behavior_taker,
                    taker_order_id
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    market_order_id_lo,
                    user_address,
                    NO_CUSTODIAN,
                    CANCEL_REASON_SELF_MATCH_MAKER
                ),
                user::create_cancel_order_event_internal(
                    market_id,
                    taker_order_id,
                    user_address,
                    NO_CUSTODIAN,
                    CANCEL_REASON_SELF_MATCH_TAKER
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        // Assert list node order inactive for low price, active for
        // high price.
        assert!(!is_list_node_order_active(
            market_id, side_maker, market_order_id_lo), 0);
        assert!(is_list_node_order_active(
            market_id, side_maker, market_order_id_hi), 0);
        // Assert user-side order fields for cancelled/active orders.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            user_address, market_id, NO_CUSTODIAN, side_maker,
            order_access_key_lo);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            user_address, market_id, CUSTODIAN_ID_USER_0, side_maker,
            order_access_key_hi);
        assert!(market_order_id_r == market_order_id_hi, 0);
        assert!(size_r == size_maker, 0);
        // Assert users's asset counts for signing market account.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                user_address, market_id, NO_CUSTODIAN);
        assert!(base_total      == deposit_base, 0);
        assert!(base_available  == deposit_base, 0);
        assert!(base_ceiling    == deposit_base, 0);
        assert!(quote_total     == deposit_quote, 0);
        assert!(quote_available == deposit_quote, 0);
        assert!(quote_ceiling   == deposit_quote, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            user_address, market_id, NO_CUSTODIAN) == deposit_base, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            user_address, market_id, NO_CUSTODIAN) == deposit_quote, 0);
    }

    #[test]
    /// Have user place two maker asks:
    ///
    /// * One at a low price without a delegated custodian.
    /// * One at a high price with a delegated custodian.
    ///
    /// Then have signing user place a taker buy, self matching against
    /// the order at the lower price. Here, matching continues against
    /// the order at the higher price.
    fun test_match_self_match_cancel_maker()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user, _) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare common market parameters.
        let market_id      = MARKET_ID_COIN;
        let integrator     = @integrator;
        let user_address   = address_of(&user);
        // Declare maker order parameters.
        let side_maker                = ASK;
        let restriction               = NO_RESTRICTION;
        let size_maker                = MIN_SIZE_COIN;
        let self_match_behavior_maker = ABORT;
        // Declare lower price set to product of fee divisors, to
        // eliminate truncation when predicting fee amounts.
        let price_lo = integrator_divisor * taker_divisor;
        // Declare higher price for an order further from the spread.
        let price_hi = integrator_divisor * taker_divisor * 2;
        // Declare taker order parameters.
        let direction_taker           = if (side_maker == ASK) BUY else SELL;
        let size_taker                = size_maker;
        let self_match_behavior_taker = CANCEL_MAKER;
        // Declare taker match amounts.
        let size_taker_match = size_maker;
        let base_taker       = size_taker_match * LOT_SIZE_COIN;
        let quote_taker      = size_taker_match * price_hi * TICK_SIZE_COIN;
        // Declare trade and fee amounts.
        let base_trade       = base_taker;
        let integrator_share = quote_taker / integrator_divisor;
        let econia_share     = quote_taker / taker_divisor - integrator_share;
        let fee              = integrator_share + econia_share;
        let quote_trade      = if (direction_taker == BUY)
            (quote_taker + fee) else (quote_taker - fee);
        // Declare deposit amounts.
        let deposit_base  = HI_64 / 2;
        let deposit_quote = HI_64 / 2;
        // Declare expected maker asset counts after matching, for
        // signing user.
        let base_total_end      = deposit_base + base_trade;
        let base_available_end  = base_total_end;
        let base_ceiling_end    = base_total_end;
        let quote_total_end     = deposit_quote - quote_trade;
        let quote_available_end = quote_total_end;
        let quote_ceiling_end   = quote_total_end;
        // Deposit coins.
        user::deposit_coins<BC>(user_address, market_id, NO_CUSTODIAN,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(user_address, market_id, NO_CUSTODIAN,
                                assets::mint_test(deposit_quote));
        user::deposit_coins<BC>(user_address, market_id, CUSTODIAN_ID_USER_0,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(user_address, market_id, CUSTODIAN_ID_USER_0,
                                assets::mint_test(deposit_quote));
        let custodian_capability = // Get custodian capability.
            registry::get_custodian_capability_test(CUSTODIAN_ID_USER_0);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        // Place maker orders, storing market order IDs.
        let (market_order_id_lo, _, _, _) = place_limit_order_user<BC, QC>(
            &user, market_id, integrator, side_maker, size_maker, price_lo,
            restriction, self_match_behavior_maker);
        let (market_order_id_hi, _, _, _) =
                place_limit_order_custodian<BC, QC>(
            user_address, market_id, integrator, side_maker, size_maker,
            price_hi, restriction, self_match_behavior_maker,
            &custodian_capability);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
        // Get user-side order access keys for later.
        let (_, _, _, _, order_access_key_lo) =
            get_order_fields_test(market_id, side_maker, market_order_id_lo);
        let (_, _, _, _, order_access_key_hi) =
            get_order_fields_test(market_id, side_maker, market_order_id_hi);
        // Assert list node orders active.
        assert!(is_list_node_order_active(
            market_id, side_maker, market_order_id_lo), 0);
        assert!(is_list_node_order_active(
            market_id, side_maker, market_order_id_hi), 0);
        // Place taker order with self match.
        let (base_trade_r, quote_trade_r, fee_r) =
                place_market_order_user<BC, QC>(
            &user, market_id, integrator, direction_taker, size_taker,
            self_match_behavior_taker);
        let taker_order_id = order_id_no_post(3);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    user_address,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price_lo,
                    restriction,
                    self_match_behavior_maker,
                    size_maker,
                    market_order_id_lo
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    user_address,
                    CUSTODIAN_ID_USER_0,
                    @integrator,
                    side_maker,
                    size_maker,
                    price_hi,
                    restriction,
                    self_match_behavior_maker,
                    size_maker,
                    market_order_id_hi
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[
                user::create_place_market_order_event_test(
                    market_id,
                    user_address,
                    NO_CUSTODIAN,
                    @integrator,
                    direction_taker,
                    size_taker,
                    self_match_behavior_taker,
                    taker_order_id
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        let fill_event = user::create_fill_event_internal(
            market_id,
            size_taker,
            price_hi,
            side_maker,
            user_address,
            CUSTODIAN_ID_USER_0,
            market_order_id_hi,
            user_address,
            NO_CUSTODIAN,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0)
            == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    market_order_id_lo,
                    user_address,
                    NO_CUSTODIAN,
                    CANCEL_REASON_SELF_MATCH_MAKER
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_trade, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Assert list node orders inactive.
        assert!(!is_list_node_order_active(
            market_id, side_maker, market_order_id_lo), 0);
        assert!(!is_list_node_order_active(
            market_id, side_maker, market_order_id_hi), 0);
        // Assert user-side order fields for cancelled/filled orders.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            user_address, market_id, NO_CUSTODIAN, side_maker,
            order_access_key_lo);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            user_address, market_id, CUSTODIAN_ID_USER_0, side_maker,
            order_access_key_hi);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        // Assert users's asset counts for signing market account.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                user_address, market_id, NO_CUSTODIAN);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            user_address, market_id, NO_CUSTODIAN) == base_total_end, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            user_address, market_id, NO_CUSTODIAN) == quote_total_end, 0);
    }

    #[test]
    /// Have user place two maker asks:
    ///
    /// * One at a low price without a delegated custodian.
    /// * One at a high price with a delegated custodian.
    ///
    /// Then have signing user place a taker buy, self matching against
    /// the order at the lower price. Here, matching halts after the
    /// self match.
    fun test_match_self_match_cancel_taker()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user, _) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare common market parameters.
        let market_id      = MARKET_ID_COIN;
        let integrator     = @integrator;
        let user_address   = address_of(&user);
        // Declare maker order parameters.
        let side_maker                = ASK;
        let restriction               = NO_RESTRICTION;
        let size_maker                = MIN_SIZE_COIN;
        let self_match_behavior_maker = ABORT;
        // Declare lower price set to product of fee divisors, to
        // eliminate truncation when predicting fee amounts.
        let price_lo = integrator_divisor * taker_divisor;
        // Declare higher price for an order further from the spread.
        let price_hi = integrator_divisor * taker_divisor * 2;
        // Declare taker order parameters.
        let direction_taker           = if (side_maker == ASK) BUY else SELL;
        let size_taker                = size_maker;
        let self_match_behavior_taker = CANCEL_TAKER;
        // Declare deposit amounts.
        let deposit_base  = HI_64 / 2;
        let deposit_quote = HI_64 / 2;
        // Declare taker match amounts against low price order.
        let base_taker  = size_maker * LOT_SIZE_COIN;
        let quote_taker = size_maker * price_lo * TICK_SIZE_COIN;
        // Declare expected maker asset counts after matching, for
        // signing user.
        let base_total_end      = deposit_base;
        let base_available_end  = base_total_end - base_taker;
        let base_ceiling_end    = base_total_end;
        let quote_total_end     = deposit_quote;
        let quote_available_end = quote_total_end;
        let quote_ceiling_end   = quote_total_end + quote_taker;
        // Deposit coins.
        user::deposit_coins<BC>(user_address, market_id, NO_CUSTODIAN,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(user_address, market_id, NO_CUSTODIAN,
                                assets::mint_test(deposit_quote));
        user::deposit_coins<BC>(user_address, market_id, CUSTODIAN_ID_USER_0,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(user_address, market_id, CUSTODIAN_ID_USER_0,
                                assets::mint_test(deposit_quote));
        let custodian_capability = // Get custodian capability.
            registry::get_custodian_capability_test(CUSTODIAN_ID_USER_0);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        // Place maker orders, storing market order IDs.
        let (market_order_id_lo, _, _, _) = place_limit_order_user<BC, QC>(
            &user, market_id, integrator, side_maker, size_maker, price_lo,
            restriction, self_match_behavior_maker);
        let (market_order_id_hi, _, _, _) =
                place_limit_order_custodian<BC, QC>(
            user_address, market_id, integrator, side_maker, size_maker,
            price_hi, restriction, self_match_behavior_maker,
            &custodian_capability);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
        // Get user-side order access keys for later.
        let (_, _, _, _, order_access_key_lo) =
            get_order_fields_test(market_id, side_maker, market_order_id_lo);
        let (_, _, _, _, order_access_key_hi) =
            get_order_fields_test(market_id, side_maker, market_order_id_hi);
        // Assert list node orders active.
        assert!(is_list_node_order_active(
            market_id, side_maker, market_order_id_lo), 0);
        assert!(is_list_node_order_active(
            market_id, side_maker, market_order_id_hi), 0);
        // Place taker order with self match.
        place_market_order_user<BC, QC>(
            &user, market_id, integrator, direction_taker, size_taker,
            self_match_behavior_taker);
        let taker_order_id = order_id_no_post(3);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    user_address,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price_lo,
                    restriction,
                    self_match_behavior_maker,
                    size_maker,
                    market_order_id_lo
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[
                user::create_place_limit_order_event_test(
                    market_id,
                    user_address,
                    CUSTODIAN_ID_USER_0,
                    @integrator,
                    side_maker,
                    size_maker,
                    price_hi,
                    restriction,
                    self_match_behavior_maker,
                    size_maker,
                    market_order_id_hi
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[
                user::create_place_market_order_event_test(
                    market_id,
                    user_address,
                    NO_CUSTODIAN,
                    @integrator,
                    direction_taker,
                    size_taker,
                    self_match_behavior_taker,
                    taker_order_id
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, user_address, NO_CUSTODIAN) == vector[
                user::create_cancel_order_event_internal(
                    market_id,
                    taker_order_id,
                    user_address,
                    NO_CUSTODIAN,
                    CANCEL_REASON_SELF_MATCH_TAKER
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            market_id, user_address, CUSTODIAN_ID_USER_0) == vector[], 0);
        // Assert list node orders active.
        assert!(is_list_node_order_active(
            market_id, side_maker, market_order_id_lo), 0);
        assert!(is_list_node_order_active(
            market_id, side_maker, market_order_id_hi), 0);
        // Assert user-side order fields for active orders.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            user_address, market_id, NO_CUSTODIAN, side_maker,
            order_access_key_lo);
        // No market order ID.
        assert!(market_order_id_r == market_order_id_lo, 0);
        assert!(size_r == size_maker, 0);
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            user_address, market_id, CUSTODIAN_ID_USER_0, side_maker,
            order_access_key_hi);
        assert!(market_order_id_r == market_order_id_hi, 0);
        assert!(size_r == size_maker, 0);
        // Assert users's asset counts for signing market account.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                user_address, market_id, NO_CUSTODIAN);
        assert!(base_total      == base_total_end, 0);
        assert!(base_available  == base_available_end, 0);
        assert!(base_ceiling    == base_ceiling_end, 0);
        assert!(quote_total     == quote_total_end, 0);
        assert!(quote_available == quote_available_end, 0);
        assert!(quote_ceiling   == quote_ceiling_end, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            user_address, market_id, NO_CUSTODIAN) == base_total, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            user_address, market_id, NO_CUSTODIAN) == quote_total, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_SELF_MATCH_BEHAVIOR)]
    /// Verify failure for self match with invalid abort behavior.
    fun test_match_self_match_invalid()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user, _) = init_markets_users_integrator_test();
        // Declare common market parameters.
        let market_id    = MARKET_ID_COIN;
        let integrator   = @integrator;
        let user_address = address_of(&user);
        let custodian_id = NO_CUSTODIAN;
        // Declare maker order parameters.
        let side_maker  = ASK;
        let size_maker  = MIN_SIZE_COIN;
        let price_maker = 100;
        let restriction = NO_RESTRICTION;
        // Declare taker order parameters.
        let direction_taker     = if (side_maker == ASK) BUY else SELL;
        let self_match_behavior = 0xff;
        let size_taker          = size_maker;
        // Declare deposit amounts.
        let deposit_base  = HI_64 / 2;
        let deposit_quote = HI_64 / 2;
        // Deposit coins.
        user::deposit_coins<BC>(user_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(user_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place a maker order.
        place_limit_order_user<BC, QC>(
            &user, market_id, integrator, side_maker, size_maker, price_maker,
            restriction, self_match_behavior);
        // Place an invalid taker order.
        place_market_order_user<BC, QC>(
            &user, market_id, integrator, direction_taker, size_taker,
            self_match_behavior);
    }

    #[test]
    #[expected_failure(abort_code = E_SIZE_BASE_OVERFLOW)]
    /// Verify failure for base overflow.
    fun test_place_limit_order_base_overflow()
    acquires OrderBooks {
        // Declare order parameters.
        let side = ASK;
        let size = HI_64 / LOT_SIZE_COIN + 1;
        let price = HI_PRICE;
        let restriction = NO_RESTRICTION;
        let critical_height = CRITICAL_HEIGHT;
        let self_match_behavior = ABORT;
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        place_limit_order<BC, QC>( // Attempt invalid invocation.
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, @integrator, side, size,
            price, restriction, self_match_behavior, critical_height);
    }

    #[test]
    /// Verify state updates, returns, for placing ask that fills
    /// completely and exactly across the spread, under authority of
    /// signing user.
    fun test_place_limit_order_crosses_ask_exact()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters from taker's perspective, with
        // price set to product of divisors to prevent truncation
        // effects on estimates.
        let side                = ASK; // Taker sell.
        let size                = MIN_SIZE_COIN;
        let base                = size * LOT_SIZE_COIN;
        let price               = integrator_divisor * taker_divisor;
        let quote               = size * price * TICK_SIZE_COIN;
        let integrator_share    = quote / integrator_divisor;
        let econia_share        = quote / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote - fee;
        let restriction         = FILL_OR_ABORT;
        let self_match_behavior = ABORT;
        // Deposit to user's accounts asset amounts that fill them just
        // up to max or down to min after the trade, for user 0 holding
        // maker order.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote));
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_trade));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size, price,
            POST_OR_ABORT, self_match_behavior);
        // Get user-side order access key for later.
        let (_, _, _, _, order_access_key_0) = get_order_fields_test(
            MARKET_ID_COIN, !side, market_order_id_0);
        assert!(is_list_node_order_active( // Assert order is active.
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Place taker order.
        let (market_order_id_1, base_trade_r, quote_trade_r, fee_r) =
            place_limit_order_user<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, side, size, price,
                restriction, self_match_behavior);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    !side,
                    size,
                    price,
                    POST_OR_ABORT,
                    self_match_behavior,
                    size,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    0,
                    market_order_id_1
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size,
            price,
            !side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            market_order_id_1,
            fee_r,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Assert returns.
        assert!(market_order_id_1 == order_id_no_post(2), 0);
        assert!(base_trade_r      == base, 0);
        assert!(quote_trade_r     == quote_trade, 0);
        assert!(fee_r             == fee, 0);
        // Assert order is inactive on the order book.
        assert!(!is_list_node_order_active(
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Assert user-side order fields for filled maker order.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, !side, order_access_key_0);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == HI_64, 0);
        assert!(base_available  == HI_64, 0);
        assert!(base_ceiling    == HI_64, 0);
        assert!(quote_total     == 0, 0);
        assert!(quote_available == 0, 0);
        assert!(quote_ceiling   == 0, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == 0, 0);
        // Assert takers's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == 0, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == 0, 0);
        assert!(quote_total     == HI_64, 0);
        assert!(quote_available == HI_64, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == 0, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
        // Assert integrator fee share.
        assert!(incentives::get_integrator_fee_store_balance_test<QC>(
            @integrator, MARKET_ID_COIN) == integrator_share, 0);
        // Assert Econia fee share.
        assert!(incentives::get_econia_fee_store_balance_test<QC>(
            MARKET_ID_COIN) == econia_share, 0);
    }

    #[test]
    /// Verify state updates, returns for placing ask that fills
    /// partially across the spread, under authority of signing user.
    /// Based on `test_place_limit_order_crosses_ask_exact()`.
    fun test_place_limit_order_crosses_ask_partial()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side                = ASK; // Taker sell.
        let size_match          = MIN_SIZE_COIN + 123;
        let size_post           = MIN_SIZE_COIN + 456;
        let size                = size_match + size_post;
        let base_match          = size_match * LOT_SIZE_COIN;
        let base_post           = size_post * LOT_SIZE_COIN;
        let base                = base_match + base_post;
        let price               = integrator_divisor * taker_divisor;
        let quote_match         = size_match * price * TICK_SIZE_COIN;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let integrator_share    = quote_match / integrator_divisor;
        let econia_share        =
            quote_match / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote_match - fee;
        let quote_total         = quote_trade + quote_post;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Deposit to first maker's account enough to impinge on min
        // and max amounts after fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base_match));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_match));
        // Deposit to second maker's account similarly, for meeting
        // range checks.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_total));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place first maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size_match, price,
            restriction, self_match_behavior);
        assert!(is_list_node_order_active( // Assert order is active.
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Place partial maker, partial taker order.
        let (market_order_id_1, base_trade_r, quote_trade_r, fee_r) =
            place_limit_order_user<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, side, size, price,
                restriction, self_match_behavior);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    !side,
                    size_match,
                    price,
                    restriction,
                    self_match_behavior,
                    size_match,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    size_post,
                    market_order_id_1
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_match,
            price,
            !side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            market_order_id_1,
            fee_r,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Assert order book counter.
        assert!(get_order_book_counter(MARKET_ID_COIN) == 2, 0);
        // Assert returns
        assert!(base_trade_r      == base_match, 0);
        assert!(quote_trade_r     == quote_trade, 0);
        assert!(fee_r             == fee, 0);
        // Assert filled order inactive.
        assert!(!is_list_node_order_active(
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Get fields for new order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_1);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_post, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_1, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert first maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == HI_64, 0);
        assert!(base_available  == HI_64, 0);
        assert!(base_ceiling    == HI_64, 0);
        assert!(quote_total     == 0, 0);
        assert!(quote_available == 0, 0);
        assert!(quote_ceiling   == 0, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == 0, 0);
        // Assert asset counts of partial maker/taker.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_post, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == base_post, 0);
        assert!(quote_total     == HI_64 - quote_post, 0);
        assert!(quote_available == HI_64 - quote_post, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == base_post, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64 - quote_post, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key);
        assert!(market_order_id_r == market_order_id_1, 0);
        assert!(size_r == size_post, 0);
    }

    #[test]
    /// Verify state updates, returns for placing immediate-or-cancel
    /// ask that fills partially across the spread, under authority of
    /// signing user. Based on
    /// `test_place_limit_order_crosses_ask_partial()`.
    fun test_place_limit_order_crosses_ask_partial_cancel()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side                = ASK; // Taker sell.
        let size_match          = MIN_SIZE_COIN + 123;
        let size_post           = MIN_SIZE_COIN + 456;
        let size                = size_match + size_post;
        let base_match          = size_match * LOT_SIZE_COIN;
        let base_post           = size_post * LOT_SIZE_COIN;
        let base                = base_match + base_post;
        let price               = integrator_divisor * taker_divisor;
        let quote_match         = size_match * price * TICK_SIZE_COIN;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let integrator_share    = quote_match / integrator_divisor;
        let econia_share        =
            quote_match / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote_match - fee;
        let quote_total         = quote_trade + quote_post;
        let restriction         = IMMEDIATE_OR_CANCEL;
        let self_match_behavior = ABORT;
        // Deposit to first maker's account enough to impinge on min
        // and max amounts after fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base_match));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_match));
        // Deposit to second maker's account similarly.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_total));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place first maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size_match, price,
            POST_OR_ABORT, self_match_behavior);
        // Place immediate-or-cancel order.
        let (market_order_id_1, base_trade_r, quote_trade_r, fee_r) =
            place_limit_order_user<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, side, size, price,
                restriction, self_match_behavior);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    !side,
                    size_match,
                    price,
                    POST_OR_ABORT,
                    self_match_behavior,
                    size_match,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    size_post,
                    market_order_id_1
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_match,
            price,
            !side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            market_order_id_1,
            fee_r,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    market_order_id_1,
                    @user_1,
                    NO_CUSTODIAN,
                    CANCEL_REASON_IMMEDIATE_OR_CANCEL
                )
            ], 0);
        // Assert returns
        assert!(market_order_id_1 == order_id_no_post(2), 0);
        assert!(base_trade_r      == base_match, 0);
        assert!(quote_trade_r     == quote_trade, 0);
        assert!(fee_r             == fee, 0);
        // Assert filled order inactive.
        assert!(!is_list_node_order_active(
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Assert asset counts of taker.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_post, 0);
        assert!(base_available  == base_post, 0);
        assert!(base_ceiling    == base_post, 0);
        assert!(quote_total     == HI_64 - quote_post, 0);
        assert!(quote_available == HI_64 - quote_post, 0);
        assert!(quote_ceiling   == HI_64 - quote_post, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == base_post, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64 - quote_post, 0);
    }

    #[test]
    /// Verify state updates, returns, for placing taker ask that fills
    /// completely across the spread with a partial maker match, under
    /// authority of signing user. Based on
    /// `test_place_limit_order_crosses_ask_exact()`.
    fun test_place_limit_order_crosses_ask_partial_maker()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters from taker's perspective, with
        // price set to product of divisors to prevent truncation
        // effects on estimates.
        let side                = ASK; // Taker sell.
        let size                = MIN_SIZE_COIN;
        let size_maker          = size + 1;
        let base                = size * LOT_SIZE_COIN;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let price               = integrator_divisor * taker_divisor;
        let quote               = size * price * TICK_SIZE_COIN;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let integrator_share    = quote / integrator_divisor;
        let econia_share        = quote / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote - fee;
        let restriction         = FILL_OR_ABORT;
        let self_match_behavior = ABORT;
        // Deposit to user's accounts asset amounts that fill them just
        // up to max or down to min after the trade, for user 0 holding
        // maker order.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_maker));
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_trade));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size_maker, price,
            POST_OR_ABORT, self_match_behavior);
        // Get user-side order access key for later.
        let (_, _, _, _, order_access_key_0) = get_order_fields_test(
            MARKET_ID_COIN, !side, market_order_id_0);
        assert!(is_list_node_order_active( // Assert order is active.
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Place taker order.
        let (market_order_id_1, base_trade_r, quote_trade_r, fee_r) =
            place_limit_order_user<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, side, size, price,
                restriction, self_match_behavior);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    !side,
                    size_maker,
                    price,
                    POST_OR_ABORT,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    0,
                    market_order_id_1
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size,
            price,
            !side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            market_order_id_1,
            fee_r,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Assert returns.
        assert!(market_order_id_1 == order_id_no_post(2), 0);
        assert!(base_trade_r      == base, 0);
        assert!(quote_trade_r     == quote_trade, 0);
        assert!(fee_r             == fee, 0);
        // Assert order is active on the order book.
        assert!(is_list_node_order_active(
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Assert user-side order fields for partially-filled maker
        // order.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, !side, order_access_key_0);
        // No market order ID.
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r == size_maker - size, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == HI_64 - base_maker + base, 0);
        assert!(base_available  == HI_64 - base_maker + base, 0);
        assert!(base_ceiling    == HI_64, 0);
        assert!(quote_total     == quote_maker - quote, 0);
        assert!(quote_available == 0, 0);
        assert!(quote_ceiling   == quote_maker - quote, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) ==
            HI_64 - base_maker + base, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_maker - quote, 0);
        // Assert takers's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == 0, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == 0, 0);
        assert!(quote_total     == HI_64, 0);
        assert!(quote_available == HI_64, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == 0, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
        // Assert integrator fee share.
        assert!(incentives::get_integrator_fee_store_balance_test<QC>(
            @integrator, MARKET_ID_COIN) == integrator_share, 0);
        // Assert Econia fee share.
        assert!(incentives::get_econia_fee_store_balance_test<QC>(
            MARKET_ID_COIN) == econia_share, 0);
    }

    #[test]
    /// Verify state updates, returns for placing ask that self matches
    /// with taker cancellation. Based on
    /// `test_place_limit_order_crosses_ask_partial()`.
    fun test_place_limit_order_crosses_ask_self_match_cancel()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side_maker          = BID;
        let side_taker          = ASK;
        let size                = MIN_SIZE_COIN;
        let price               = 123;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = CANCEL_BOTH;
        // Declare deposit amounts.
        let deposit_base  = HI_64 / 2;
        let deposit_quote = HI_64 / 2;
        // Deposit to user's account.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(deposit_quote));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        // Place maker order.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &user, MARKET_ID_COIN, @integrator, side_maker, size, price,
            restriction, self_match_behavior);
        // Get user-side order access key for later.
        let (_, _, _, _, order_access_key) = get_order_fields_test(
            MARKET_ID_COIN, side_maker, market_order_id);
        assert!(is_list_node_order_active( // Assert order is active.
            MARKET_ID_COIN, side_maker, market_order_id), 0);
        // Place taker order.
        let (market_order_id_r, base_trade_r, quote_trade_r, fee_r) =
            place_limit_order_user<BC, QC>(
                &user, MARKET_ID_COIN, @integrator, side_taker, size, price,
                restriction, self_match_behavior);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    size,
                    market_order_id
                ),
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_taker,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    size,
                    market_order_id_r
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    market_order_id,
                    @user_0,
                    NO_CUSTODIAN,
                    CANCEL_REASON_SELF_MATCH_MAKER
                ),
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    market_order_id_r,
                    @user_0,
                    NO_CUSTODIAN,
                    CANCEL_REASON_SELF_MATCH_TAKER
                )
            ], 0);
        // Assert returns
        assert!(market_order_id_r == order_id_no_post(2), 0);
        assert!(base_trade_r      == 0, 0);
        assert!(quote_trade_r     == 0, 0);
        assert!(fee_r             == 0, 0);
        // Assert maker order inactive.
        assert!(!is_list_node_order_active(
            MARKET_ID_COIN, side_maker, market_order_id), 0);
        // Assert user's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == deposit_base, 0);
        assert!(base_available  == deposit_base, 0);
        assert!(base_ceiling    == deposit_base, 0);
        assert!(quote_total     == deposit_quote, 0);
        assert!(quote_available == deposit_quote, 0);
        assert!(quote_ceiling   == deposit_quote, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == deposit_base, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == deposit_quote, 0);
        // Assert user-side order fields for cancelled maker order.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
    }

    #[test]
    /// Verify state updates, returns, for placing bid that fills
    /// completely and exactly across the spread, under authority of
    /// signing user. Mirror of
    /// `test_place_limit_order_crosses_ask_exact()`.
    fun test_place_limit_order_crosses_bid_exact()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters from taker's perspective, with
        // price set to product of divisors to prevent truncation
        // effects on estimates.
        let side                = BID; // Taker buy.
        let size                = MIN_SIZE_COIN;
        let base                = size * LOT_SIZE_COIN;
        let price               = integrator_divisor * taker_divisor;
        let quote               = size * price * TICK_SIZE_COIN;
        let integrator_share    = quote / integrator_divisor;
        let econia_share        = quote / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote + fee;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Deposit to user's accounts asset amounts that fill them just
        // up to max or down to min after the trade, for user 0 holding
        // maker order.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote));
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_trade));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size, price,
            restriction, self_match_behavior);
        // Get user-side order access key for later.
        let (_, _, _, _, order_access_key_0) = get_order_fields_test(
            MARKET_ID_COIN, !side, market_order_id_0);
        assert!(is_list_node_order_active( // Assert order is active.
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Place taker order.
        let (market_order_id_1, base_trade_r, quote_trade_r, fee_r) =
            place_limit_order_user<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, side, size, price,
                restriction, self_match_behavior);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    !side,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    size,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    0,
                    market_order_id_1
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size,
            price,
            !side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            market_order_id_1,
            fee_r,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Assert returns.
        assert!(market_order_id_1 == order_id_no_post(2), 0);
        assert!(base_trade_r      == base, 0);
        assert!(quote_trade_r     == quote_trade, 0);
        assert!(fee_r             == fee, 0);
        // Assert order is inactive on the order book.
        assert!(!is_list_node_order_active(
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Assert user-side order fields for filled maker order.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, !side, order_access_key_0);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == 0, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == 0, 0);
        assert!(quote_total     == HI_64, 0);
        assert!(quote_available == HI_64, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == 0, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
        // Assert takers's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == HI_64, 0);
        assert!(base_available  == HI_64, 0);
        assert!(base_ceiling    == HI_64, 0);
        assert!(quote_total     == 0, 0);
        assert!(quote_available == 0, 0);
        assert!(quote_ceiling   == 0, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == 0, 0);
        // Assert integrator fee share.
        assert!(incentives::get_integrator_fee_store_balance_test<QC>(
            @integrator, MARKET_ID_COIN) == integrator_share, 0);
        // Assert Econia fee share.
        assert!(incentives::get_econia_fee_store_balance_test<QC>(
            MARKET_ID_COIN) == econia_share, 0);
    }

    #[test]
    /// Verify state updates, returns for placing bid that fills
    /// partially across the spread, under authority of signing user.
    /// Mirror of `test_place_limit_order_crosses_ask_partial()`.
    fun test_place_limit_order_crosses_bid_partial()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side                = BID; // Taker buy.
        let size_match          = MIN_SIZE_COIN + 123;
        let size_post           = MIN_SIZE_COIN;
        let size                = size_match + size_post;
        let base_match          = size_match * LOT_SIZE_COIN;
        let base_post           = size_post * LOT_SIZE_COIN;
        let base                = base_match + base_post;
        let price               = integrator_divisor * taker_divisor;
        let quote_match         = size_match * price * TICK_SIZE_COIN;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let quote               = quote_match + quote_post;
        let quote_max           = quote + quote / taker_divisor;
        let integrator_share    = quote_match / integrator_divisor;
        let econia_share        =
            quote_match / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote_match + fee;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Deposit to first maker's account enough to impinge on min
        // and max amounts after fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_match));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_match));
        // Deposit to second maker's account minimum amount to pass
        // range checking for taker fill.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_max));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place first maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size_match, price,
            restriction, self_match_behavior);
        assert!(is_list_node_order_active( // Assert order is active.
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Place partial maker, partial taker order.
        let (market_order_id_1, base_trade_r, quote_trade_r, fee_r) =
            place_limit_order_user<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, side, size, price,
                restriction, self_match_behavior);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    !side,
                    size_match,
                    price,
                    restriction,
                    self_match_behavior,
                    size_match,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    size_post,
                    market_order_id_1
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_match,
            price,
            !side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            market_order_id_1,
            fee_r,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Assert returns
        assert!(base_trade_r      == base_match, 0);
        assert!(quote_trade_r     == quote_trade, 0);
        assert!(fee_r             == fee, 0);
        // Assert filled order inactive.
        assert!(!is_list_node_order_active(
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Get fields for new order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_1);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_post, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_1, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert first maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == 0, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == 0, 0);
        assert!(quote_total     == HI_64, 0);
        assert!(quote_available == HI_64, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == 0, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
        // Assert asset counts of partial maker/taker.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == HI_64 - base_post, 0);
        assert!(base_available  == HI_64 - base_post, 0);
        assert!(base_ceiling    == HI_64, 0);
        assert!(quote_total     == quote_max - quote_trade, 0);
        assert!(quote_available == quote_max - quote_trade - quote_post, 0);
        assert!(quote_ceiling   == quote_max - quote_trade, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64 - base_post, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN)
            == quote_max - quote_trade, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key);
        assert!(market_order_id_r == market_order_id_1, 0);
        assert!(size_r == size_post, 0);
    }

    #[test]
    /// Verify state updates, returns, for placing taker bid that fills
    /// completely across the spread with a partial maker match, under
    /// authority of signing user. Based on
    /// `test_place_limit_order_crosses_bid_exact()`.
    fun test_place_limit_order_crosses_bid_partial_maker()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters from taker's perspective, with
        // price set to product of divisors to prevent truncation
        // effects on estimates.
        let side                = BID; // Taker buy.
        let size                = MIN_SIZE_COIN;
        let size_maker          = size + 1;
        let base                = size * LOT_SIZE_COIN;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let price               = integrator_divisor * taker_divisor;
        let quote               = size * price * TICK_SIZE_COIN;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let integrator_share    = quote / integrator_divisor;
        let econia_share        = quote / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote + fee;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Deposit to user's accounts asset amounts that fill them just
        // up to max or down to min after the trade, for user 0 holding
        // maker order.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_maker));
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_trade));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size_maker, price,
            restriction, self_match_behavior);
        // Get user-side order access key for later.
        let (_, _, _, _, order_access_key_0) = get_order_fields_test(
            MARKET_ID_COIN, !side, market_order_id_0);
        assert!(is_list_node_order_active( // Assert order is active.
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Place taker order.
        let (market_order_id_1, base_trade_r, quote_trade_r, fee_r) =
            place_limit_order_user<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, side, size, price,
                restriction, self_match_behavior);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    !side,
                    size_maker,
                    price,
                    restriction,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    0,
                    market_order_id_1
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size,
            price,
            !side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            market_order_id_1,
            fee_r,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Assert returns.
        assert!(market_order_id_1 == order_id_no_post(2), 0);
        assert!(base_trade_r      == base, 0);
        assert!(quote_trade_r     == quote_trade, 0);
        assert!(fee_r             == fee, 0);
        // Assert order is active on the order book.
        assert!(is_list_node_order_active(
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Assert user-side order fields for partially-filled maker
        // order.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, !side, order_access_key_0);
        // No market order ID.
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r == size_maker - size, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_maker - base, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == base_maker - base, 0);
        assert!(quote_total     == HI_64 - quote_maker + quote, 0);
        assert!(quote_available == HI_64 - quote_maker + quote, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_maker - base, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) ==
            HI_64 - quote_maker + quote, 0);
        // Assert takers's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == HI_64, 0);
        assert!(base_available  == HI_64, 0);
        assert!(base_ceiling    == HI_64, 0);
        assert!(quote_total     == 0, 0);
        assert!(quote_available == 0, 0);
        assert!(quote_ceiling   == 0, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == 0, 0);
        // Assert integrator fee share.
        assert!(incentives::get_integrator_fee_store_balance_test<QC>(
            @integrator, MARKET_ID_COIN) == integrator_share, 0);
        // Assert Econia fee share.
        assert!(incentives::get_econia_fee_store_balance_test<QC>(
            MARKET_ID_COIN) == econia_share, 0);
    }

    #[test]
    /// Verify state updates, returns for placing bid that fills
    /// partially across the spread, then posts amount less than minimum
    /// order size for market. Based on
    /// `test_place_limit_order_crosses_bid_partial()`.
    fun test_place_limit_order_crosses_bid_partial_post_under_min()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side                = BID; // Taker buy.
        let size_match          = MIN_SIZE_COIN + 123;
        let size_post           = MIN_SIZE_COIN - 1;
        let size                = size_match + size_post;
        let base_match          = size_match * LOT_SIZE_COIN;
        let base_post           = size_post * LOT_SIZE_COIN;
        let base                = base_match + base_post;
        let price               = integrator_divisor * taker_divisor;
        let quote_match         = size_match * price * TICK_SIZE_COIN;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let quote               = quote_match + quote_post;
        let quote_max           = quote + quote / taker_divisor;
        let integrator_share    = quote_match / integrator_divisor;
        let econia_share        =
            quote_match / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote_match + fee;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Deposit to first maker's account enough to impinge on min
        // and max amounts after fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_match));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_match));
        // Deposit to second maker's account minimum amount to pass
        // range checking for taker fill.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_max));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place first maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size_match, price,
            restriction, self_match_behavior);
        assert!(is_list_node_order_active( // Assert order is active.
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Place partial maker, partial taker order.
        let (market_order_id_1, base_trade_r, quote_trade_r, fee_r) =
            place_limit_order_user<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, side, size, price,
                restriction, self_match_behavior);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    !side,
                    size_match,
                    price,
                    restriction,
                    self_match_behavior,
                    size_match,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    size_post,
                    market_order_id_1
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_match,
            price,
            !side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            market_order_id_1,
            fee_r,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Assert returns
        assert!(did_order_post(market_order_id_1), 0);
        assert!(base_trade_r      == base_match, 0);
        assert!(quote_trade_r     == quote_trade, 0);
        assert!(fee_r             == fee, 0);
        // Assert filled order inactive.
        assert!(!is_list_node_order_active(
            MARKET_ID_COIN, !side, market_order_id_0), 0);
        // Get fields for new order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_1);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_post, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_1, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert first maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == 0, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == 0, 0);
        assert!(quote_total     == HI_64, 0);
        assert!(quote_available == HI_64, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == 0, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
        // Assert asset counts of partial maker/taker.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == HI_64 - base_post, 0);
        assert!(base_available  == HI_64 - base_post, 0);
        assert!(base_ceiling    == HI_64, 0);
        assert!(quote_total     == quote_max - quote_trade, 0);
        assert!(quote_available == quote_max - quote_trade - quote_post, 0);
        assert!(quote_ceiling   == quote_max - quote_trade, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64 - base_post, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN)
            == quote_max - quote_trade, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key);
        assert!(market_order_id_r == market_order_id_1, 0);
        assert!(size_r == size_post, 0);
    }

    #[test]
    /// Verify state updates, returns, for placing limit order that
    /// evicts another user's order.
    fun test_place_limit_order_evict()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = ASK;
        let size_0              = MIN_SIZE_COIN;
        let size_1              = size_0 + 1;
        let size_2              = size_1 + 1;
        let price_0             = 123;
        let price_1             = price_0 - 1;
        let price_2             = price_1 - 1;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Declare min base and max quote to deposit.
        let base_deposit  = HI_64 / 2;
        let quote_deposit = HI_64 / 2;
        // Declare critical height.
        let critical_height = 0;
        // Deposit base and quote coins to each user's account.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit));
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place a single order by user 0.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side, size_0, price_0,
            restriction, self_match_behavior);
        // Place a single order by user 1, with better price-time
        // priority, taking tree height to 1.
        let (market_order_id_1, _, _, _) = place_limit_order_user<BC, QC>(
            &user_1, MARKET_ID_COIN, @integrator, side, size_1, price_1,
            restriction, self_match_behavior);
        // Assert order fields for first placed order.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key_0) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_0, 0);
        assert!(price_r        == price_0, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key_0);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r == size_0, 0);
        // Assert order fields for second placed order.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key_1) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_1);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_1, 0);
        assert!(price_r        == price_1, 0);
        assert!(user_r         == @user_1, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key_1);
        assert!(market_order_id_r == market_order_id_1, 0);
        assert!(size_r == size_1, 0);
        // Place another order by user 1, with better price-time
        // priority, evicting user 0's order for low critical height.
        let (market_order_id_2, _, _, _) = place_limit_order<BC, QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN, @integrator, side, size_2,
            price_2, restriction, self_match_behavior, critical_height);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size_0,
                    price_0,
                    restriction,
                    self_match_behavior,
                    size_0,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size_1,
                    price_1,
                    restriction,
                    self_match_behavior,
                    size_1,
                    market_order_id_1
                ),
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size_2,
                    price_2,
                    restriction,
                    self_match_behavior,
                    size_2,
                    market_order_id_2
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    market_order_id_0,
                    @user_0,
                    NO_CUSTODIAN,
                    CANCEL_REASON_EVICTION
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Assert order fields for third placed order.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key_2) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_2);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_2, 0);
        assert!(price_r        == price_2, 0);
        assert!(user_r         == @user_1, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key_2);
        assert!(market_order_id_r == market_order_id_2, 0);
        assert!(size_r == size_2, 0);
        // Assert order fields returned the same when attempting to
        // look up using the evicted order ID, since the same AVL queue
        // list node ID is re-used.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key_r) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_0);
        assert!(size_r             == size_2, 0);
        assert!(price_r            == price_2, 0);
        assert!(user_r             == @user_1, 0);
        assert!(custodian_id_r     == NO_CUSTODIAN, 0);
        assert!(order_access_key_r == order_access_key_2, 0);
        // Assert user-side order fields for inactive evicted order.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key_0);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        // Assert evictee's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_deposit , 0);
        assert!(base_available  == base_deposit , 0);
        assert!(base_ceiling    == base_deposit , 0);
        assert!(quote_total     == quote_deposit, 0);
        assert!(quote_available == quote_deposit, 0);
        assert!(quote_ceiling   == quote_deposit, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_deposit, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_deposit, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_FILL_OR_ABORT_NOT_CROSS_SPREAD)]
    /// Verify failure for not crossing spread when fill-or-abort.
    fun test_place_limit_order_fill_or_abort_not_cross()
    acquires OrderBooks {
        // Declare order parameters.
        let side = ASK;
        let size = MIN_SIZE_COIN;
        let price = HI_PRICE;
        let restriction = FILL_OR_ABORT;
        let critical_height = CRITICAL_HEIGHT;
        let self_match_behavior = ABORT;
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Deposit base and quote coins to maker's account.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0,
                                assets::mint_test(HI_64));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0,
                                assets::mint_test(HI_64));
        place_limit_order<BC, QC>( // Attempt invalid invocation.
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, @integrator, side, size,
            price, restriction, self_match_behavior, critical_height);
    }

    #[test]
    #[expected_failure(abort_code = E_MIN_BASE_NOT_TRADED)]
    /// Verify failure for not filling completely across spread when
    /// fill-or-abort.
    fun test_place_limit_order_fill_or_abort_partial()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = ASK; // Taker sell.
        let size                = MIN_SIZE_COIN;
        let price               = 789;
        let restriction         = FILL_OR_ABORT;
        let self_match_behavior = ABORT;
        // Deposit sufficient coins as collateral.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(0));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64));
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(0));
        // Place maker order.
        place_limit_order_user_entry<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size, price,
            POST_OR_ABORT, self_match_behavior);
        // Place limit order that can not fill.
        place_limit_order_user<BC, QC>(
            &user_1, MARKET_ID_COIN, @integrator, side, size + 1, price,
            restriction, self_match_behavior);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_BASE)]
    /// Verify failure for invalid base type argument.
    fun test_place_limit_order_invalid_base()
    acquires OrderBooks {
        // Declare order parameters.
        let side = ASK;
        let size = MIN_SIZE_COIN;
        let price = HI_PRICE;
        let restriction = NO_RESTRICTION;
        let critical_height = CRITICAL_HEIGHT;
        let self_match_behavior = ABORT;
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Deposit base and quote coins to user's account.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0,
                                assets::mint_test(HI_64));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0,
                                assets::mint_test(HI_64));
        place_limit_order<QC, QC>( // Attempt invalid invocation.
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, @integrator, side, size,
            price, restriction, self_match_behavior, critical_height);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_QUOTE)]
    /// Verify failure for invalid quote type argument.
    fun test_place_limit_order_invalid_quote()
    acquires OrderBooks {
        // Declare order parameters.
        let side = ASK;
        let size = MIN_SIZE_COIN;
        let price = HI_PRICE;
        let restriction = NO_RESTRICTION;
        let critical_height = CRITICAL_HEIGHT;
        let self_match_behavior = ABORT;
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Deposit base and quote coins to user's account.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0,
                                assets::mint_test(HI_64));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0,
                                assets::mint_test(HI_64));
        place_limit_order<BC, BC>( // Attempt invalid invocation.
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, @integrator, side, size,
            price, restriction, self_match_behavior, critical_height);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_RESTRICTION)]
    /// Verify failure for invalid restriction.
    fun test_place_limit_order_invalid_restriction()
    acquires OrderBooks {
        // Declare order parameters.
        let side = ASK;
        let size = 123;
        let price = 123;
        let restriction = N_RESTRICTIONS + 1;
        let critical_height = CRITICAL_HEIGHT;
        let self_match_behavior = ABORT;
        place_limit_order<BC, QC>( // Attempt invalid invocation.
            @user, MARKET_ID_COIN, NO_CUSTODIAN, @integrator, side, size,
            price, restriction, self_match_behavior, critical_height);
    }

    #[test]
    /// Verify state updates, returns, for placing ask that does not
    /// cross the spread, under authority of signing user.
    fun test_place_limit_order_no_cross_ask_user()
    acquires OrderBooks {
        // Declare order parameters.
        let side                = ASK;
        let size                = MIN_SIZE_COIN;
        let price               = 123;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Declare change in base and quote seen by maker.
        let base_delta    = size * LOT_SIZE_COIN;
        let quote_delta   = size * price * TICK_SIZE_COIN;
        // Declare min base and max quote to deposit.
        let base_deposit  = base_delta;
        let quote_deposit = HI_64 - quote_delta;
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Deposit base and quote coins to user's account.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit));
        // Assert order book counter.
        assert!(get_order_book_counter(MARKET_ID_COIN) == 0, 0);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        let (market_order_id, base_trade, quote_trade, fees) =
            place_limit_order_user<BC, QC>( // Place limit order.
                &user_0, MARKET_ID_COIN, @integrator, side, size, price,
                restriction, self_match_behavior);
        // Assert trade amount returns.
        assert!(base_trade == 0, 0);
        assert!(quote_trade == 0, 0);
        assert!(fees == 0, 0);
        // Assert counter encoded in order ID.
        assert!(get_market_order_id_counter(market_order_id) == 1, 0);
        // Assert market order ID marked as posting.
        assert!(did_order_post(market_order_id), 0);
        // Assert price encoded in order ID.
        assert!(get_market_order_id_price(market_order_id) == price, 0);
        // Assert side encoded in order ID.
        assert!(get_posted_order_id_side(market_order_id) == side, 0);
        // Assert order book counter.
        assert!(get_order_book_counter(MARKET_ID_COIN) == 1, 0);
        // Get order fields.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_deposit , 0);
        assert!(base_available  == 0            , 0);
        assert!(base_ceiling    == base_deposit , 0);
        assert!(quote_total     == quote_deposit, 0);
        assert!(quote_available == quote_deposit, 0);
        assert!(quote_ceiling   == HI_64        , 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_deposit, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_deposit, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r == size, 0);
        // Place another order, asserting counter in market order ID.
        let (market_order_id_1, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size, price - 1,
            restriction, self_match_behavior);
        assert!(get_market_order_id_counter(market_order_id_1) == 2, 0);
        // Assert order book counter.
        assert!(get_order_book_counter(MARKET_ID_COIN) == 2, 0);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    size,
                    market_order_id
                ),
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    !side,
                    size,
                    price - 1,
                    restriction,
                    self_match_behavior,
                    size,
                    market_order_id_1
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
    }

    #[test]
    /// Verify state updates, returns for placing immediate-or-cancel
    /// ask that does not fill at all across the spread, under authority
    /// of signing user
    fun test_place_limit_order_no_cross_ask_user_ioc()
    acquires OrderBooks {
        // Declare order parameters.
        let side                = ASK;
        let size                = MIN_SIZE_COIN;
        let price               = 123;
        let restriction_0       = NO_RESTRICTION;
        let restriction_1       = IMMEDIATE_OR_CANCEL;
        let self_match_behavior = ABORT;
        // Declare change in base and quote seen by maker.
        let base_delta    = size * LOT_SIZE_COIN;
        let quote_delta   = size * price * TICK_SIZE_COIN;
        // Declare min base and max quote to deposit.
        let base_deposit  = base_delta;
        let quote_deposit = HI_64 - quote_delta;
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Deposit base and quote coins to user's account.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit));
        // Assert order book counter.
        assert!(get_order_book_counter(MARKET_ID_COIN) == 0, 0);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        let (market_order_id, base_trade, quote_trade, fees) =
            place_limit_order_user<BC, QC>( // Place limit order.
                &user_0, MARKET_ID_COIN, @integrator, side, size, price,
                restriction_0, self_match_behavior);
        // Assert trade amount returns.
        assert!(base_trade == 0, 0);
        assert!(quote_trade == 0, 0);
        assert!(fees == 0, 0);
        // Assert counter encoded in order ID.
        assert!(get_market_order_id_counter(market_order_id) == 1, 0);
        // Assert market order ID marked as posting.
        assert!(did_order_post(market_order_id), 0);
        // Assert price encoded in order ID.
        assert!(get_market_order_id_price(market_order_id) == price, 0);
        // Assert side encoded in order ID.
        assert!(get_posted_order_id_side(market_order_id) == side, 0);
        // Assert order book counter.
        assert!(get_order_book_counter(MARKET_ID_COIN) == 1, 0);
        // Get order fields.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r == size, 0);
        // Place another order, asserting counter in market order ID.
        let (market_order_id_1, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size, price - 1,
            restriction_1, self_match_behavior);
        assert!(get_market_order_id_counter(market_order_id_1) == 2, 0);
        // Assert order did not post.
        assert!(!did_order_post(market_order_id_1), 0);
        // Assert user's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_deposit , 0);
        assert!(base_available  == 0            , 0);
        assert!(base_ceiling    == base_deposit , 0);
        assert!(quote_total     == quote_deposit, 0);
        assert!(quote_available == quote_deposit, 0);
        assert!(quote_ceiling   == HI_64        , 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_deposit, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_deposit, 0);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size,
                    price,
                    restriction_0,
                    self_match_behavior,
                    size,
                    market_order_id
                ),
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    !side,
                    size,
                    price - 1,
                    restriction_1,
                    self_match_behavior,
                    size,
                    market_order_id_1
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    market_order_id_1,
                    @user_0,
                    NO_CUSTODIAN,
                    CANCEL_REASON_IMMEDIATE_OR_CANCEL
                )
            ], 0);
    }

    #[test]
    /// Verify state updates, returns, for placing bid that does not
    /// cross the spread, under authority of custodian.
    fun test_place_limit_order_no_cross_bid_custodian()
    acquires OrderBooks {
        // Declare order parameters.
        let side                = BID;
        let size                = MIN_SIZE_COIN;
        let price               = 123;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Declare change in base and quote seen by maker.
        let base_delta    = size * LOT_SIZE_COIN;
        let quote_delta   = size * price * TICK_SIZE_COIN;
        // Declare max base and min quote to deposit.
        let base_deposit  = HI_64 - base_delta;
        let quote_deposit = quote_delta;
        // Initialize markets, users, and an integrator.
        let (_, _) = init_markets_users_integrator_test();
        // Deposit base and quote coins to user's account.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0,
                                assets::mint_test(base_deposit));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0,
                                assets::mint_test(quote_deposit));
        let custodian_capability = registry::get_custodian_capability_test(
            CUSTODIAN_ID_USER_0); // Get custodian capability.
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, CUSTODIAN_ID_USER_0) == vector[], 0);
        let (market_order_id, base_trade, quote_trade, fees) =
            place_limit_order_custodian<BC, QC>( // Place limit order.
                @user_0, MARKET_ID_COIN, @integrator, side, size, price,
                restriction, self_match_behavior, &custodian_capability);
        // Assert counter encoded in order ID.
        assert!(get_market_order_id_counter(market_order_id) == 1, 0);
        // Assert market order ID marked as posting.
        assert!(did_order_post(market_order_id), 0);
        // Assert price encoded in order ID.
        assert!(get_market_order_id_price(market_order_id) == price, 0);
        // Assert side encoded in order ID.
        assert!(get_posted_order_id_side(market_order_id) == side, 0);
        // Assert trade amount returns.
        assert!(base_trade == 0, 0);
        assert!(quote_trade == 0, 0);
        assert!(fees == 0, 0);
        // Get order fields.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == CUSTODIAN_ID_USER_0, 0);
        // Assert user's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0);
        assert!(base_total      == base_deposit , 0);
        assert!(base_available  == base_deposit , 0);
        assert!(base_ceiling    == HI_64        , 0);
        assert!(quote_total     == quote_deposit, 0);
        assert!(quote_available == 0            , 0);
        assert!(quote_ceiling   == quote_deposit, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0) == base_deposit, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0) == quote_deposit, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0, side,
            order_access_key);
        assert!(market_order_id_r == market_order_id, 0);
        assert!(size_r == size, 0);
        // Place another order, asserting counter in market order ID.
        let (market_order_id_1, _, _, _) = place_limit_order_custodian<BC, QC>(
            @user_0, MARKET_ID_COIN, @integrator, !side, size, price + 1,
            restriction, self_match_behavior, &custodian_capability);
        assert!(get_market_order_id_counter(market_order_id_1) == 2, 0);
        // Assert order book counter.
        assert!(get_order_book_counter(MARKET_ID_COIN) == 2, 0);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, CUSTODIAN_ID_USER_0) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    CUSTODIAN_ID_USER_0,
                    @integrator,
                    side,
                    size,
                    price,
                    restriction,
                    self_match_behavior,
                    size,
                    market_order_id
                ),
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    CUSTODIAN_ID_USER_0,
                    @integrator,
                    !side,
                    size,
                    price + 1,
                    restriction,
                    self_match_behavior,
                    size,
                    market_order_id_1
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, CUSTODIAN_ID_USER_0) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, CUSTODIAN_ID_USER_0) == vector[], 0);
    }

    #[test]
    #[expected_failure(abort_code = E_PRICE_0)]
    /// Verify failure for invalid price.
    fun test_place_limit_order_no_price()
    acquires OrderBooks {
        // Declare order parameters.
        let side = ASK;
        let size = 123;
        let price = 0;
        let restriction = NO_RESTRICTION;
        let critical_height = CRITICAL_HEIGHT;
        let self_match_behavior = ABORT;
        place_limit_order<BC, QC>( // Attempt invalid invocation.
            @user, MARKET_ID_COIN, NO_CUSTODIAN, @integrator, side, size,
            price, restriction, self_match_behavior, critical_height);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_BASE)]
    /// Verify failure for invalid base type argument.
    fun test_place_limit_order_passive_advance_invalid_base()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let market_id             = MARKET_ID_COIN;
        let integrator            = @integrator;
        let side                  = ASK;
        let size                  = MIN_SIZE_COIN;
        let advance_style         = PERCENT;
        let target_advance_amount = 50;
        // Attempt invalid invocation.
        place_limit_order_passive_advance_user<QC, QC>(
            &user, market_id, integrator, side, size, advance_style,
            target_advance_amount);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ID)]
    /// Verify failure for invalid market ID.
    fun test_place_limit_order_passive_advance_invalid_market_id()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let market_id             = HI_64;
        let integrator            = @integrator;
        let side                  = ASK;
        let size                  = MIN_SIZE_COIN;
        let advance_style         = PERCENT;
        let target_advance_amount = 50;
        // Attempt invalid invocation.
        place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style,
            target_advance_amount);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_PERCENT)]
    /// Verify failure for invalid percent
    fun test_place_limit_order_passive_advance_invalid_percent()
    acquires OrderBooks {
        // Configure spread.
        let max_bid_price = 100;
        let min_ask_price = 201;
        let user = configure_spread_test(max_bid_price, min_ask_price);
        // Declare order parameters.
        let market_id             = MARKET_ID_COIN;
        let integrator            = @integrator;
        let side                  = ASK;
        let size                  = MIN_SIZE_COIN;
        let advance_style         = PERCENT;
        let target_advance_amount = 101;
        // Attempt invalid invocation.
        place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style,
            target_advance_amount);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_QUOTE)]
    /// Verify failure for invalid quote type argument.
    fun test_place_limit_order_passive_advance_invalid_quote()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let market_id             = MARKET_ID_COIN;
        let integrator            = @integrator;
        let side                  = ASK;
        let size                  = MIN_SIZE_COIN;
        let advance_style         = PERCENT;
        let target_advance_amount = 50;
        // Attempt invalid invocation.
        place_limit_order_passive_advance_user<BC, BC>(
            &user, market_id, integrator, side, size, advance_style,
            target_advance_amount);
    }

    #[test]
    /// Verify return for no cross price when placing an ask.
    fun test_place_limit_order_passive_advance_no_cross_price_ask()
    acquires OrderBooks {
        // Configure spread.
        let max_bid_price = NIL;
        let min_ask_price = 201;
        let user = configure_spread_test(max_bid_price, min_ask_price);
        // Declare order parameters.
        let market_id             = MARKET_ID_COIN;
        let integrator            = @integrator;
        let side                  = ASK;
        let size                  = MIN_SIZE_COIN;
        let advance_style         = PERCENT;
        let target_advance_amount = 1;
        // Place passive advance ask with percent advance style.
        let market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style,
            target_advance_amount);
        // Assert no order placed.
        assert!(market_order_id == (NIL as u128), 0);
    }

    #[test]
    /// Verify return for no cross price when placing a bid.
    fun test_place_limit_order_passive_advance_no_cross_price_bid()
    acquires OrderBooks {
        // Configure spread.
        let max_bid_price = 100;
        let min_ask_price = NIL;
        let user = configure_spread_test(max_bid_price, min_ask_price);
        // Declare order parameters.
        let market_id             = MARKET_ID_COIN;
        let integrator            = @integrator;
        let side                  = BID;
        let size                  = MIN_SIZE_COIN;
        let advance_style         = TICKS;
        let target_advance_amount = 1;
        // Place passive advance ask with percent advance style.
        let market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style,
            target_advance_amount);
        // Assert no order placed.
        assert!(market_order_id == (NIL as u128), 0);
    }

    #[test]
    /// Verify returns, state updates for full advance is start price.
    fun test_place_limit_order_passive_advance_no_full_advance()
    acquires OrderBooks {
        // Configure spread.
        let max_bid_price = 100;
        let min_ask_price = 101;
        let user = configure_spread_test(max_bid_price, min_ask_price);
        // Declare order parameters.
        let market_id             = MARKET_ID_COIN;
        let integrator            = @integrator;
        let size                  = MIN_SIZE_COIN + 1;
        let advance_style         = PERCENT;
        let target_advance_amount = 100;
        // Place passive advance ask.
        let market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, ASK, size, advance_style,
            target_advance_amount);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, ASK, size, min_ask_price, NO_CUSTODIAN);
        // Place passive advance bid.
        market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, BID, size, advance_style,
            target_advance_amount);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, BID, size, max_bid_price, NO_CUSTODIAN);
    }

    #[test]
    /// Verify returns for no start price.
    fun test_place_limit_order_passive_advance_no_start_price()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let market_id             = MARKET_ID_COIN;
        let integrator            = @integrator;
        let side                  = ASK;
        let size                  = MIN_SIZE_COIN;
        let advance_style         = PERCENT;
        let target_advance_amount = 50;
        // Place passive advance limit order.
        let market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style,
            target_advance_amount);
        // Assert no order placed.
        assert!(market_order_id == (NIL as u128), 0);
        // Place passive advance limit order on other side.
        market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, !side, size, advance_style,
            target_advance_amount);
        // Assert no order placed.
        assert!(market_order_id == (NIL as u128), 0);
    }

    #[test]
    /// Verify returns, state updates for no target advance amount.
    fun test_place_limit_order_passive_advance_no_target_advance()
    acquires OrderBooks {
        // Configure spread.
        let max_bid_price = 100;
        let min_ask_price = 201;
        let user = configure_spread_test(max_bid_price, min_ask_price);
        // Declare order parameters.
        let market_id             = MARKET_ID_COIN;
        let integrator            = @integrator;
        let size                  = MIN_SIZE_COIN;
        let target_advance_amount = 0;
        // Place passive advance ask with percent advance style.
        let market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, ASK, size, PERCENT,
            target_advance_amount);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, ASK, size, min_ask_price, NO_CUSTODIAN);
        // Place passive advance bid with tick advance style.
        market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, BID, size, TICKS,
            target_advance_amount);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, BID, size, max_bid_price, NO_CUSTODIAN);
    }

    #[test]
    /// Verify returns, state updates for percent-specified asks.
    fun test_place_limit_order_passive_advance_percent_ask()
    acquires OrderBooks {
        // Configure spread.
        let max_bid_price = 99;
        let min_ask_price = 500;
        let user = configure_spread_test(max_bid_price, min_ask_price);
        // Declare order parameters.
        let market_id     = MARKET_ID_COIN;
        let integrator    = @integrator;
        let side          = ASK;
        let size          = MIN_SIZE_COIN + 123;
        let advance_style = PERCENT;
        // Place passive advance ask for halfway through spread.
        let market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style, 50);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 300, NO_CUSTODIAN);
        // Place another ask for halfway through spread.
        market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style, 50);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 200, NO_CUSTODIAN);
        // Place another ask for full advance
        market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style, 100);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 100, NO_CUSTODIAN);
    }

    #[test]
    /// Verify returns, state updates for percent-specified bids.
    fun test_place_limit_order_passive_advance_percent_bid()
    acquires OrderBooks {
        // Configure spread.
        let max_bid_price = 100;
        let min_ask_price = 501;
        let user = configure_spread_test(max_bid_price, min_ask_price);
        // Declare order parameters.
        let market_id     = MARKET_ID_COIN;
        let integrator    = @integrator;
        let side          = BID;
        let size          = MIN_SIZE_COIN + 123;
        let advance_style = PERCENT;
        // Place passive advance bid for one quarter through spread.
        let market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style, 25);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 200, NO_CUSTODIAN);
        // Place another bid for three quarters through spread.
        market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style, 75);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 425, NO_CUSTODIAN);
        // Place another bid for full advance.
        market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style, 100);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 500, NO_CUSTODIAN);
    }

    #[test]
    /// Verify returns, state updates for ticks-specified asks.
    fun test_place_limit_order_passive_advance_ticks_ask()
    acquires OrderBooks {
        // Configure spread.
        let max_bid_price = 100;
        let min_ask_price = 500;
        let user = configure_spread_test(max_bid_price, min_ask_price);
        // Declare order parameters.
        let market_id     = MARKET_ID_COIN;
        let integrator    = @integrator;
        let side          = ASK;
        let size          = MIN_SIZE_COIN + 123;
        let advance_style = TICKS;
        // Place passive advance ask for arbitrary ticks.
        let market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style, 100);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 400, NO_CUSTODIAN);
        // Place another ask exceeding ticks left to advance.
        market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style, 300);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 101, NO_CUSTODIAN);
        // Cancel the order.
        cancel_order_user(&user, market_id, side, market_order_id);
        // Place another ask exactly matching ticks left to advance.
        market_order_id = place_limit_order_passive_advance_user<BC, QC>(
            &user, market_id, integrator, side, size, advance_style, 299);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 101, NO_CUSTODIAN);
        // Get order access key from book.
        let (_, _, _, _, order_access_key) = get_order_fields_test(
            market_id, side, market_order_id);
        // Cancel the order, resulting in order access key at top of
        // inactive orders stack for user's market account.
        cancel_order_user(&user, market_id, side, market_order_id);
        // Place a 1 tick advance ask, using public entry wrapper.
        place_limit_order_passive_advance_user_entry<BC, QC>(
            &user, market_id, integrator, side, size, advance_style, 1);
        // Get market order ID for ask just placed, based on order
        // access key just popped off inactive order stack.
        (market_order_id, _) = user::get_order_fields_simple_test(
            @user_0, market_id, NO_CUSTODIAN, side, order_access_key);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 399, NO_CUSTODIAN);
    }

    #[test]
    /// Verify returns, state updates for ticks-specified bid, for
    /// delegated custodian.
    fun test_place_limit_order_passive_advance_ticks_bid()
    acquires OrderBooks {
        // Configure spread.
        let max_bid_price = 100;
        let min_ask_price = 500;
        let user = configure_spread_test(max_bid_price, min_ask_price);
        // Declare order parameters.
        let user_address  = address_of(&user);
        let custodian_id  = CUSTODIAN_ID_USER_0;
        let market_id     = MARKET_ID_COIN;
        let integrator    = @integrator;
        let side          = BID;
        let size          = MIN_SIZE_COIN + 123;
        let advance_style = TICKS;
        // Deposit coins to user's market account.
        user::deposit_coins<BC>(user_address, market_id, custodian_id,
                                assets::mint_test(HI_64 / 2));
        user::deposit_coins<QC>(user_address, market_id, custodian_id,
                                assets::mint_test(HI_64 / 2));
        // Get custodian capability.
        let custodian_capability = registry::get_custodian_capability_test(
            custodian_id);
        // Place passive advance ask for arbitrary ticks.
        let market_order_id =
                place_limit_order_passive_advance_custodian<BC, QC>(
            user_address, market_id, integrator, side, size, advance_style,
            100, &custodian_capability);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 200, custodian_id);
        // Place another ask exceeding ticks left to advance.
        market_order_id = place_limit_order_passive_advance_custodian<BC, QC>(
            user_address, market_id, integrator, side, size, advance_style,
            300, &custodian_capability);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 499, custodian_id);
        // Cancel the order.
        cancel_order_custodian(address_of(&user), market_id, side,
                               market_order_id, &custodian_capability);
        // Place another bid exactly matching ticks left to advance.
        market_order_id = place_limit_order_passive_advance_custodian<BC, QC>(
            user_address, market_id, integrator, side, size, advance_style,
            299, &custodian_capability);
        check_order_fields_test( // Check fields for placed order.
            market_order_id, side, size, 499, custodian_id);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
    }

    #[test]
    #[expected_failure(abort_code = E_POST_OR_ABORT_CROSSES_SPREAD)]
    /// Verify failure for not crossing spread as post-or-abort.
    fun test_place_limit_order_post_or_abort_crosses()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = ASK; // Taker sell.
        let size                = MIN_SIZE_COIN;
        let price               = 789;
        let restriction         = POST_OR_ABORT;
        let self_match_behavior = ABORT;
        // Deposit sufficient coins as collateral.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(0));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64));
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(0));
        // Place maker order.
        place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size, price,
            POST_OR_ABORT, self_match_behavior);
        // Place limit order that can not fill.
        place_limit_order_user<BC, QC>(
            &user_1, MARKET_ID_COIN, @integrator, side, size, price,
            restriction, self_match_behavior);
    }

    #[test]
    #[expected_failure(abort_code = E_PRICE_TOO_HIGH)]
    /// Verify failure for invalid price.
    fun test_place_limit_order_price_hi()
    acquires OrderBooks {
        // Declare order parameters.
        let side = ASK;
        let size = 123;
        let price = HI_PRICE + 1;
        let restriction = NO_RESTRICTION;
        let critical_height = CRITICAL_HEIGHT;
        let self_match_behavior = ABORT;
        place_limit_order<BC, QC>( // Attempt invalid invocation.
            @user, MARKET_ID_COIN, NO_CUSTODIAN, @integrator, side, size,
            price, restriction, self_match_behavior, critical_height);
    }

    #[test]
    #[expected_failure(abort_code = E_PRICE_TIME_PRIORITY_TOO_LOW)]
    /// Verify failure for unable to insert to AVL queue. Modeled off
    /// `test_place_limit_order_evict()`.
    fun test_place_limit_order_price_time_priority_low()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = ASK;
        let size_0              = MIN_SIZE_COIN;
        let size_1              = size_0 + 1;
        let size_2              = size_1 + 1;
        let price_0             = 123;
        let price_1             = price_0 - 1;
        let price_2             = price_0;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Declare min base and max quote to deposit.
        let base_deposit  = HI_64 / 2;
        let quote_deposit = HI_64 / 2;
        // Declare critical height.
        let critical_height = 0;
        // Deposit base and quote coins to each user's account.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit));
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit));
        // Place a single order by user 0.
        place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side, size_0, price_0,
            restriction, self_match_behavior);
        // Place a single order by user 1, with better price-time
        // priority, taking tree height to 1.
        place_limit_order_user<BC, QC>(
            &user_1, MARKET_ID_COIN, @integrator, side, size_1, price_1,
            restriction, self_match_behavior);
        // Place another order by user 1, with worse price-time than
        // AVL queue tail.
        place_limit_order<BC, QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN, @integrator, side, size_2,
            price_2, restriction, self_match_behavior, critical_height);
    }

    #[test]
    #[expected_failure(abort_code = E_SIZE_PRICE_QUOTE_OVERFLOW)]
    /// Verify failure for quote overflow.
    fun test_place_limit_order_quote_overflow()
    acquires OrderBooks {
        // Declare order parameters.
        let side = ASK;
        let size = HI_64 / LOT_SIZE_COIN;
        let price = HI_64 / size;
        let restriction = NO_RESTRICTION;
        let critical_height = CRITICAL_HEIGHT;
        let self_match_behavior = ABORT;
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        place_limit_order<BC, QC>( // Attempt invalid invocation.
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, @integrator, side, size,
            price, restriction, self_match_behavior, critical_height);
    }

    #[test]
    /// Manually remove market event handles before emission calls.
    /// Based on `test_place_limit_order_crosses_ask_partial()`.
    fun test_place_limit_order_remove_event_handles()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side                = ASK; // Taker sell.
        let size_match          = MIN_SIZE_COIN + 123;
        let size_post           = MIN_SIZE_COIN + 456;
        let size                = size_match + size_post;
        let base_match          = size_match * LOT_SIZE_COIN;
        let base_post           = size_post * LOT_SIZE_COIN;
        let base                = base_match + base_post;
        let price               = integrator_divisor * taker_divisor;
        let quote_match         = size_match * price * TICK_SIZE_COIN;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let integrator_share    = quote_match / integrator_divisor;
        let econia_share        =
            quote_match / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote_match - fee;
        let quote_total         = quote_trade + quote_post;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Deposit to first maker's account enough to impinge on min
        // and max amounts after fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base_match));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_match));
        // Deposit to second maker's account similarly, for meeting
        // range checks.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_total));
        // Remove all event handles for user.
        user::remove_market_event_handles_test(@user_0);
        // Place first maker order.
        place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, !side, size_match, price,
            restriction, self_match_behavior);
        // Remove only event handles for market account for user.
        user::remove_market_event_handles_for_market_account_test(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        // Place partial maker, partial taker order.
        place_limit_order_user<BC, QC>(
            &user_1, MARKET_ID_COIN, @integrator, side, size, price,
            restriction, self_match_behavior);
    }

    #[test]
    #[expected_failure(abort_code = E_SIZE_TOO_SMALL)]
    /// Verify failure for invalid size.
    fun test_place_limit_order_size_lo()
    acquires OrderBooks {
        // Declare order parameters.
        let side = ASK;
        let size = MIN_SIZE_COIN - 1;
        let price = HI_PRICE;
        let restriction = NO_RESTRICTION;
        let critical_height = CRITICAL_HEIGHT;
        let self_match_behavior = ABORT;
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Deposit base and quote coins to user's account.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0,
                                assets::mint_test(HI_64));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, CUSTODIAN_ID_USER_0,
                                assets::mint_test(HI_64));
        place_limit_order<BC, QC>( // Attempt invalid invocation.
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, @integrator, side, size,
            price, restriction, self_match_behavior, critical_height);
    }

    #[test]
    /// Verify state updates, returns for placing ask that still crosses
    /// the spread after matching as a taker, under authority of signing
    /// user. Based on `test_place_limit_order_crosses_ask_partial()`.
    fun test_place_limit_order_still_crosses_ask()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, taker) = init_markets_users_integrator_test();
        let (maker_address, taker_address) = // Get user addresses.
            (address_of(&maker), address_of(&taker));
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side_taker           = ASK;
        let direction_taker      = if (side_taker == ASK) SELL else BUY;
        let side_maker           = !side_taker;
        let size_maker           = MIN_SIZE_COIN + 123;
        let size_taker           = size_maker - 10;
        let size_taker_requested = size_maker + 1;
        let size_post            = size_taker_requested - size_taker;
        let base_maker           = size_maker * LOT_SIZE_COIN;
        let base_taker           = size_taker * LOT_SIZE_COIN;
        let base_taker_requested = size_taker_requested * LOT_SIZE_COIN;
        let price                = integrator_divisor * taker_divisor;
        let quote_maker          = size_maker * price * TICK_SIZE_COIN;
        let quote_match          = size_taker * price * TICK_SIZE_COIN;
        let integrator_share     = quote_match / integrator_divisor;
        let econia_share         =
            quote_match / taker_divisor - integrator_share;
        let fee                  = integrator_share + econia_share;
        let quote_trade          = if (direction_taker == BUY)
            (quote_match + fee) else (quote_match - fee);
        let restriction          = NO_RESTRICTION;
        let self_match_behavior  = ABORT;
        // Deposit to maker's account enough to impinge on min and max
        // amounts after fill.
        let base_deposit_maker  = HI_64 - base_maker;
        let quote_deposit_maker = quote_maker;
        user::deposit_coins<BC>(maker_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(maker_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Deposit to second maker's account such that matching engine
        // halts before matching requested size.
        let base_deposit_taker  = base_taker_requested;
        let quote_deposit_taker = HI_64 - quote_trade;
        user::deposit_coins<BC>(taker_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_taker));
        user::deposit_coins<QC>(taker_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place first maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, MARKET_ID_COIN, @integrator, side_maker, size_maker, price,
            restriction, self_match_behavior);
        // Place limit order that still crosses spread after matching.
        let (market_order_id_1, base_trade_r, quote_trade_r, fee_r) =
            place_limit_order_user<BC, QC>(
                &taker, MARKET_ID_COIN, @integrator, side_taker,
                size_taker_requested, price, restriction, self_match_behavior);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    restriction,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    side_taker,
                    size_taker_requested,
                    price,
                    restriction,
                    self_match_behavior,
                    size_post,
                    market_order_id_1
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side_maker,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            market_order_id_1,
            fee_r,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    market_order_id_1,
                    @user_1,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        // Assert returns
        assert!(market_order_id_1 == order_id_no_post(2), 0);
        assert!(base_trade_r      == base_taker, 0);
        assert!(quote_trade_r     == quote_trade, 0);
        assert!(fee_r             == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side_maker,
                                  market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == maker_address, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_deposit_maker + base_taker, 0);
        assert!(base_available  == base_deposit_maker + base_taker, 0);
        assert!(base_ceiling    == HI_64, 0);
        assert!(quote_total     == quote_deposit_maker - quote_match, 0);
        assert!(quote_available == 0, 0);
        assert!(quote_ceiling   == quote_deposit_maker - quote_match, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, MARKET_ID_COIN, NO_CUSTODIAN) ==
            base_deposit_maker + base_taker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, MARKET_ID_COIN, NO_CUSTODIAN) ==
            quote_deposit_maker - quote_match, 0);
        // Assert asset counts of taker.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                taker_address, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_deposit_taker - base_taker, 0);
        assert!(base_available  == base_deposit_taker - base_taker, 0);
        assert!(base_ceiling    == base_deposit_taker - base_taker, 0);
        assert!(quote_total     == HI_64, 0);
        assert!(quote_available == HI_64, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            taker_address, MARKET_ID_COIN, NO_CUSTODIAN)
            == base_deposit_taker - base_taker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            taker_address, MARKET_ID_COIN, NO_CUSTODIAN)
            == HI_64, 0);
    }

    #[test]
    /// Verify state updates, returns for placing bid that still crosses
    /// the spread after matching as a taker, under authority of signing
    /// user. Based on `test_place_limit_order_still_crosses_ask()`.
    fun test_place_limit_order_still_crosses_bid()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, taker) = init_markets_users_integrator_test();
        let (maker_address, taker_address) = // Get user addresses.
            (address_of(&maker), address_of(&taker));
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side_taker           = BID;
        let direction_taker      = if (side_taker == ASK) SELL else BUY;
        let side_maker           = !side_taker;
        let size_maker           = MIN_SIZE_COIN + 123;
        let size_taker           = size_maker - 10;
        let size_taker_requested = size_maker + 1;
        let size_post            = size_taker_requested - size_taker;
        let base_maker           = size_maker * LOT_SIZE_COIN;
        let base_taker           = size_taker * LOT_SIZE_COIN;
        let price                = integrator_divisor * taker_divisor;
        let quote_maker          = size_maker * price * TICK_SIZE_COIN;
        let quote_match          = size_taker * price * TICK_SIZE_COIN;
        let integrator_share     = quote_match / integrator_divisor;
        let econia_share         =
            quote_match / taker_divisor - integrator_share;
        let fee                  = integrator_share + econia_share;
        let quote_trade          = if (direction_taker == BUY)
            (quote_match + fee) else (quote_match - fee);
        let restriction          = NO_RESTRICTION;
        let self_match_behavior  = ABORT;
        // Deposit to maker's account enough to impinge on min and max
        // amounts after fill.
        let base_deposit_maker  = base_maker;
        let quote_deposit_maker = HI_64 - quote_maker;
        user::deposit_coins<BC>(maker_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(maker_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Deposit to second maker's account such that matching engine
        // halts before matching requested size.
        let base_deposit_taker  = 0;
        let quote_deposit_taker = quote_trade;
        user::deposit_coins<BC>(taker_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_taker));
        user::deposit_coins<QC>(taker_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place first maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, MARKET_ID_COIN, @integrator, side_maker, size_maker, price,
            restriction, self_match_behavior);
        // Place limit order that still crosses spread after matching.
        let (market_order_id_1, base_trade_r, quote_trade_r, fee_r) =
            place_limit_order_user<BC, QC>(
                &taker, MARKET_ID_COIN, @integrator, side_taker,
                size_taker_requested, price, restriction, self_match_behavior);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    restriction,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    side_taker,
                    size_taker_requested,
                    price,
                    restriction,
                    self_match_behavior,
                    size_post,
                    market_order_id_1
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side_maker,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            market_order_id_1,
            fee_r,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    market_order_id_1,
                    @user_1,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        // Assert returns
        assert!(market_order_id_1 == order_id_no_post(2), 0);
        assert!(base_trade_r      == base_taker, 0);
        assert!(quote_trade_r     == quote_trade, 0);
        assert!(fee_r             == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side_maker,
                                  market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == maker_address, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_deposit_maker - base_taker, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == base_deposit_maker - base_taker, 0);
        assert!(quote_total     == quote_deposit_maker + quote_match, 0);
        assert!(quote_available == quote_deposit_maker + quote_match, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, MARKET_ID_COIN, NO_CUSTODIAN) ==
            base_deposit_maker - base_taker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, MARKET_ID_COIN, NO_CUSTODIAN) ==
            quote_deposit_maker + quote_match, 0);
        // Assert asset counts of taker.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                taker_address, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_taker, 0);
        assert!(base_available  == base_taker, 0);
        assert!(base_ceiling    == base_taker, 0);
        assert!(quote_total     == 0, 0);
        assert!(quote_available == 0, 0);
        assert!(quote_ceiling   == 0, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            taker_address, MARKET_ID_COIN, NO_CUSTODIAN) == base_taker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            taker_address, MARKET_ID_COIN, NO_CUSTODIAN) == 0, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_SIZE_PRICE_TICKS_OVERFLOW)]
    /// Verify failure for ticks overflow.
    fun test_place_limit_order_ticks_overflow()
    acquires OrderBooks {
        // Declare order parameters.
        let side = ASK;
        let size = HI_64 / LOT_SIZE_COIN;
        let price = HI_64 / size + 1;
        let restriction = NO_RESTRICTION;
        let critical_height = CRITICAL_HEIGHT;
        let self_match_behavior = ABORT;
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        place_limit_order<BC, QC>( // Attempt invalid invocation.
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, @integrator, side, size,
            price, restriction, self_match_behavior, critical_height);
    }

    #[test]
    /// Verify state updates for public entry wrapper invocation. Based
    /// on `test_place_limit_order_no_cross_ask_user()`.
    fun test_place_limit_order_user_entry()
    acquires OrderBooks {
        // Declare order parameters.
        let side                = ASK;
        let size                = MIN_SIZE_COIN;
        let price               = 123;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Declare change in base and quote seen by maker.
        let base_delta    = size * LOT_SIZE_COIN;
        let quote_delta   = size * price * TICK_SIZE_COIN;
        // Declare min base and max quote to deposit.
        let base_deposit  = base_delta;
        let quote_deposit = HI_64 - quote_delta;
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Deposit base and quote coins to user's account.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit));
        // Get order access key for order about to be placed.
        let order_access_key = user::get_next_order_access_key_internal(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side);
        place_limit_order_user_entry<BC, QC>( // Place limit order.
            &user_0, MARKET_ID_COIN, @integrator, side, size, price,
            restriction, self_match_behavior);
        // Assert user's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_deposit , 0);
        assert!(base_available  == 0            , 0);
        assert!(base_ceiling    == base_deposit , 0);
        assert!(quote_total     == quote_deposit, 0);
        assert!(quote_available == quote_deposit, 0);
        assert!(quote_ceiling   == HI_64        , 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_deposit, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_deposit, 0);
        // Check user-side order fields.
        let (market_order_id, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key);
        assert!(size_r == size, 0);
        // Get order fields.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key_r) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r             == size, 0);
        assert!(price_r            == price, 0);
        assert!(user_r             == @user_0, 0);
        assert!(custodian_id_r     == NO_CUSTODIAN, 0);
        assert!(order_access_key_r == order_access_key, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_BASE)]
    /// Verify failure for invalid base type argument.
    fun test_place_market_order_invalid_base()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Declare order arguments.
        let user_address = @user_0;
        let market_id = MARKET_ID_COIN;
        let custodian_id = NO_CUSTODIAN;
        let integrator = @integrator;
        let direction = BUY;
        let size = MAX_POSSIBLE;
        let self_match_behavior = ABORT;
        // Attempt invalid invocation.
        place_market_order<QC, QC>(
            user_address, market_id, custodian_id, integrator, direction,
            size, self_match_behavior);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_QUOTE)]
    /// Verify failure for invalid quote type argument.
    fun test_place_market_order_invalid_quote()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Declare order arguments.
        let user_address = @user_0;
        let market_id = MARKET_ID_COIN;
        let custodian_id = NO_CUSTODIAN;
        let integrator = @integrator;
        let direction = BUY;
        let size = MAX_POSSIBLE;
        let self_match_behavior = ABORT;
        // Attempt invalid invocation.
        place_market_order<BC, BC>(
            user_address, market_id, custodian_id, integrator, direction,
            size, self_match_behavior);
    }

    #[test]
    /// Verify state updates, returns for market buy when user specifies
    /// base trade amount that is less than max possible, under
    /// authority of signing user.
    fun test_place_market_order_max_base_below_buy_user()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side                = ASK; // Maker sell.
        let size_post           = 10; // Maker order size.
        let size_match          = MIN_SIZE_COIN; // Taker order size.
        let base_match          = size_match * LOT_SIZE_COIN;
        let base_post           = size_post * LOT_SIZE_COIN;
        let price               = integrator_divisor * taker_divisor;
        let quote_match         = size_match * price * TICK_SIZE_COIN;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let integrator_share    = quote_match / integrator_divisor;
        let econia_share        =
            quote_match / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote_match + fee;
        let self_match_behavior = ABORT;
        let base_deposit_taker  = HI_64 / 2;
        let quote_deposit_taker = HI_64 / 2;
        // Deposit to maker's account enough to impinge on min and max
        // amounts after complete fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_post));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_post));
        // Deposit to taker's account such that size, not max possible
        // match amount, is limiting factor for matching.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_taker));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side, size_post, price,
            NO_RESTRICTION, self_match_behavior);
        // Place taker order.
        let (base_trade_r, quote_trade_r, fee_r) = place_market_order_user<
            BC, QC>(&user_1, MARKET_ID_COIN, @integrator, BUY, size_match,
            self_match_behavior);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size_post,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_post,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_market_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    BUY,
                    size_match,
                    self_match_behavior,
                    taker_order_id
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_match,
            price,
            side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_match, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_post - size_match, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_post - size_match, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_post - base_match, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == base_post - base_match, 0);
        assert!(quote_total     == HI_64 - quote_post + quote_match, 0);
        assert!(quote_available == HI_64 - quote_post + quote_match, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN)
            == base_post - base_match, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN)
            == HI_64 - quote_post + quote_match, 0);
        // Assert taker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_deposit_taker + base_match, 0);
        assert!(base_available  == base_deposit_taker + base_match, 0);
        assert!(base_ceiling    == base_deposit_taker + base_match, 0);
        assert!(quote_total     == quote_deposit_taker - quote_trade, 0);
        assert!(quote_available == quote_deposit_taker - quote_trade, 0);
        assert!(quote_ceiling   == quote_deposit_taker - quote_trade, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN)
            == base_deposit_taker + base_match, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN)
            == quote_deposit_taker - quote_trade, 0);
    }

    #[test]
    /// Verify state updates, returns for market buy when user specifies
    /// max possible base trade amount, under authority of signing
    /// user.
    fun test_place_market_order_max_base_buy_user()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side                = ASK; // Maker sell.
        let size_match          = MIN_SIZE_COIN;
        let size_post           = size_match + 1; // Maker order size.
        let base_match          = size_match * LOT_SIZE_COIN;
        let base_post           = size_post * LOT_SIZE_COIN;
        let price               = integrator_divisor * taker_divisor;
        let quote_match         = size_match * price * TICK_SIZE_COIN;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let integrator_share    = quote_match / integrator_divisor;
        let econia_share        =
            quote_match / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote_match + fee;
        let quote_deposit       = quote_trade * TICK_SIZE_COIN;
        let self_match_behavior = ABORT;
        let size_taker          = size_match;
        // Deposit to maker's account enough to impinge on min and max
        // amounts after fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_post));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_post));
        // Deposit to takers's account similarly, for base match amount
        // as limiting factor in matching engine.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base_match));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side, size_post, price,
            NO_RESTRICTION, self_match_behavior);
        // Place taker order.
        let (base_trade_r, quote_trade_r, fee_r) = place_market_order_user<
            BC, QC>(&user_1, MARKET_ID_COIN, @integrator, BUY, size_taker,
            self_match_behavior);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size_post,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_post,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_market_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    BUY,
                    size_taker,
                    self_match_behavior,
                    taker_order_id
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_match, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_post - size_match, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_post - size_match, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_post - base_match, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == base_post - base_match, 0);
        assert!(quote_total     == HI_64 - quote_post + quote_match, 0);
        assert!(quote_available == HI_64 - quote_post + quote_match, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN)
            == base_post - base_match, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN)
            == HI_64 - quote_post + quote_match, 0);
        // Assert taker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == HI_64, 0);
        assert!(base_available  == HI_64, 0);
        assert!(base_ceiling    == HI_64, 0);
        assert!(quote_total     == quote_deposit - quote_trade, 0);
        assert!(quote_available == quote_deposit - quote_trade, 0);
        assert!(quote_ceiling   == quote_deposit - quote_trade, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN)
            == quote_deposit - quote_trade, 0);
    }

    #[test]
    /// Verify state updates, returns for market sell when max possible
    /// base trade amount specified, under authority of custodian.
    fun test_place_market_order_max_base_sell_custodian()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side                = BID; // Maker buy.
        let size_match          = MIN_SIZE_COIN;
        let size_post           = size_match + 1; // Maker order size.
        let base_match          = size_match * LOT_SIZE_COIN;
        let base_post           = size_post * LOT_SIZE_COIN;
        let price               = integrator_divisor * taker_divisor;
        let quote_match         = size_match * price * TICK_SIZE_COIN;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let integrator_share    = quote_match / integrator_divisor;
        let econia_share        =
            quote_match / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote_match - fee;
        let max_quote           = quote_trade * TICK_SIZE_COIN;
        let quote_deposit       = HI_64 - max_quote;
        let self_match_behavior = ABORT;
        let size_taker          = size_match;
        // Deposit to maker's account enough to impinge on min and max
        // amounts after fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base_post));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_post));
        // Deposit to takers's account similarly, for base match amount
        // as limiting factor in matching engine.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, CUSTODIAN_ID_USER_1,
                                assets::mint_test(base_match));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, CUSTODIAN_ID_USER_1,
                                assets::mint_test(quote_deposit));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, CUSTODIAN_ID_USER_1) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, CUSTODIAN_ID_USER_1) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, CUSTODIAN_ID_USER_1) == vector[], 0);
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side, size_post, price,
            NO_RESTRICTION, self_match_behavior); // Place maker order.
        let custodian_capability = registry::get_custodian_capability_test(
            CUSTODIAN_ID_USER_1); // Get custodian capability.
        let (base_trade_r, quote_trade_r, fee_r) =
            place_market_order_custodian<BC, QC>( // Place taker order.
                @user_1, MARKET_ID_COIN, @integrator, SELL, size_taker,
                self_match_behavior, &custodian_capability);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size_post,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_post,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, CUSTODIAN_ID_USER_1) == vector[
                user::create_place_market_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    CUSTODIAN_ID_USER_1,
                    @integrator,
                    SELL,
                    size_taker,
                    self_match_behavior,
                    taker_order_id
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            CUSTODIAN_ID_USER_1,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, CUSTODIAN_ID_USER_1) ==
            vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, CUSTODIAN_ID_USER_1) == vector[], 0);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
        // Assert returns.
        assert!(base_trade_r  == base_match, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_post - size_match, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_post - size_match, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == HI_64 - base_post + base_match, 0);
        assert!(base_available  == HI_64 - base_post + base_match, 0);
        assert!(base_ceiling    == HI_64, 0);
        assert!(quote_total     == quote_post - quote_match, 0);
        assert!(quote_available == 0, 0);
        assert!(quote_ceiling   == quote_post - quote_match, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN)
            == HI_64 - base_post + base_match, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN)
            == quote_post - quote_match, 0);
        // Assert taker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, CUSTODIAN_ID_USER_1);
        assert!(base_total      == 0, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == 0, 0);
        assert!(quote_total     == quote_deposit + quote_trade, 0);
        assert!(quote_available == quote_deposit + quote_trade, 0);
        assert!(quote_ceiling   == quote_deposit + quote_trade, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, CUSTODIAN_ID_USER_1) == 0, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, CUSTODIAN_ID_USER_1)
            == quote_deposit + quote_trade, 0);
    }

    #[test]
    /// Verify state updates, returns for market buy when max possible
    /// quote trade amount specified, under authority of custodian.
    fun test_place_market_order_max_quote_buy_custodian()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side                = ASK; // Maker sell.
        let size_match          = MIN_SIZE_COIN;
        let size_post           = size_match + 1; // Maker order size.
        let base_match          = size_match * LOT_SIZE_COIN;
        let base_post           = size_post * LOT_SIZE_COIN;
        let price               = integrator_divisor * taker_divisor;
        let quote_match         = size_match * price * TICK_SIZE_COIN;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let integrator_share    = quote_match / integrator_divisor;
        let econia_share        =
            quote_match / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote_match + fee;
        let size_taker          = size_match;
        let base_deposit        = HI_64 - (size_taker * LOT_SIZE_COIN);
        let self_match_behavior = ABORT;
        // Deposit to maker's account enough to impinge on min and max
        // amounts after fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_post));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_post));
        // Deposit to takers's account similarly, for quote match amount
        // as limiting factor in matching engine.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, CUSTODIAN_ID_USER_1,
                                assets::mint_test(base_deposit));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, CUSTODIAN_ID_USER_1,
                                assets::mint_test(quote_trade));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, CUSTODIAN_ID_USER_1) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, CUSTODIAN_ID_USER_1) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, CUSTODIAN_ID_USER_1) == vector[], 0);
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side, size_post, price,
            NO_RESTRICTION, self_match_behavior); // Place maker order.
        let custodian_capability = registry::get_custodian_capability_test(
            CUSTODIAN_ID_USER_1); // Get custodian capability.
        let (base_trade_r, quote_trade_r, fee_r) =
            place_market_order_custodian<BC, QC>( // Place taker order.
                @user_1, MARKET_ID_COIN, @integrator, BUY, size_taker,
                self_match_behavior, &custodian_capability);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size_post,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_post,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, CUSTODIAN_ID_USER_1) == vector[
                user::create_place_market_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    CUSTODIAN_ID_USER_1,
                    @integrator,
                    BUY,
                    size_taker,
                    self_match_behavior,
                    taker_order_id
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            CUSTODIAN_ID_USER_1,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, CUSTODIAN_ID_USER_1)
            == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, CUSTODIAN_ID_USER_1) == vector[], 0);
        // Drop custodian capability.
        registry::drop_custodian_capability_test(custodian_capability);
        // Assert returns.
        assert!(base_trade_r  == base_match, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_post - size_match, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_post - size_match, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_post - base_match, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == base_post - base_match, 0);
        assert!(quote_total     == HI_64 - quote_post + quote_match, 0);
        assert!(quote_available == HI_64 - quote_post + quote_match, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN)
            == base_post - base_match, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN)
            == HI_64 - quote_post + quote_match, 0);
        // Assert taker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, CUSTODIAN_ID_USER_1);
        assert!(base_total      == base_deposit + base_match, 0);
        assert!(base_available  == base_deposit + base_match, 0);
        assert!(base_ceiling    == base_deposit + base_match, 0);
        assert!(quote_total     == 0, 0);
        assert!(quote_available == 0, 0);
        assert!(quote_ceiling   == 0, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, CUSTODIAN_ID_USER_1)
            == base_deposit + base_match, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, CUSTODIAN_ID_USER_1) == 0, 0);
    }

    #[test]
    /// Verify state updates, returns for market sell when max possible
    /// quote trade amount specified, under authority of signing user.
    fun test_place_market_order_max_quote_sell_user()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side                = BID; // Maker buy.
        let size_match          = MIN_SIZE_COIN;
        let size_post           = size_match + 1; // Maker order size.
        let base_match          = size_match * LOT_SIZE_COIN;
        let base_post           = size_post * LOT_SIZE_COIN;
        let price               = integrator_divisor * taker_divisor;
        let quote_match         = size_match * price * TICK_SIZE_COIN;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let integrator_share    = quote_match / integrator_divisor;
        let econia_share        =
            quote_match / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote_match - fee;
        let size_taker          = size_match;
        let base_deposit        = HI_64 - (size_taker * LOT_SIZE_COIN);
        let self_match_behavior = ABORT;
        // Deposit to maker's account enough to impinge on min and max
        // amounts after fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base_post));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_post));
        // Deposit to taker's account similarly, for quote match amount
        // as limiting factor in matching engine.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_trade));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side, size_post, price,
            NO_RESTRICTION, self_match_behavior); // Place maker order.
        let (base_trade_r, quote_trade_r, fee_r) =
            place_market_order_user<BC, QC>( // Place taker order.
                &user_1, MARKET_ID_COIN, @integrator, SELL, size_taker,
                self_match_behavior);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side,
                    size_post,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_post,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_market_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    SELL,
                    size_taker,
                    self_match_behavior,
                    taker_order_id
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_match, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_post - size_match, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_post - size_match, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == HI_64 - base_post + base_match, 0);
        assert!(base_available  == HI_64 - base_post + base_match, 0);
        assert!(base_ceiling    == HI_64, 0);
        assert!(quote_total     == quote_post - quote_match, 0);
        assert!(quote_available == 0, 0);
        assert!(quote_ceiling   == quote_post - quote_match, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN)
            == HI_64 - base_post + base_match, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN)
            == quote_post - quote_match, 0);
        // Assert taker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_deposit - base_match, 0);
        assert!(base_available  == base_deposit - base_match, 0);
        assert!(base_ceiling    == base_deposit - base_match, 0);
        assert!(quote_total     == HI_64, 0);
        assert!(quote_available == HI_64, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN)
            == base_deposit - base_match, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
    }

    #[test]
    /// Verify state updates, returns for market buy when matching
    /// engine halts due to max quote traded. Based on
    /// Based on `test_place_limit_order_still_crosses_bid()`.
    fun test_place_market_order_max_quote_traded()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, taker) = init_markets_users_integrator_test();
        let (maker_address, taker_address) = // Get user addresses.
            (address_of(&maker), address_of(&taker));
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let direction_taker      = BUY;
        let side_maker           = if (direction_taker == BUY) ASK else SELL;
        let size_maker           = MIN_SIZE_COIN + 123;
        let size_taker           = size_maker - 10;
        let size_taker_requested = size_maker + 1;
        let base_maker           = size_maker * LOT_SIZE_COIN;
        let base_taker           = size_taker * LOT_SIZE_COIN;
        let price                = integrator_divisor * taker_divisor;
        let quote_maker          = size_maker * price * TICK_SIZE_COIN;
        let quote_match          = size_taker * price * TICK_SIZE_COIN;
        let integrator_share     = quote_match / integrator_divisor;
        let econia_share         =
            quote_match / taker_divisor - integrator_share;
        let fee                  = integrator_share + econia_share;
        let quote_trade          = if (direction_taker == BUY)
            (quote_match + fee) else (quote_match - fee);
        let restriction          = NO_RESTRICTION;
        let self_match_behavior  = ABORT;
        // Deposit to maker's account enough to impinge on min and max
        // amounts after fill.
        let base_deposit_maker  = base_maker;
        let quote_deposit_maker = HI_64 - quote_maker;
        user::deposit_coins<BC>(maker_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(maker_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Deposit to second maker's account such that matching engine
        // halts before matching requested size.
        let base_deposit_taker  = 0;
        let quote_deposit_taker = quote_trade;
        user::deposit_coins<BC>(taker_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_taker));
        user::deposit_coins<QC>(taker_address, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, MARKET_ID_COIN, @integrator, side_maker, size_maker, price,
            restriction, self_match_behavior);
        let (base_trade_r, quote_trade_r, fee_r) =
            place_market_order_user<BC, QC>( // Place taker order.
                &taker, MARKET_ID_COIN, @integrator, direction_taker,
                size_taker_requested, self_match_behavior);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    restriction,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_market_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    direction_taker,
                    size_taker_requested,
                    self_match_behavior,
                    taker_order_id
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side_maker,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee_r,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    taker_order_id,
                    @user_1,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        // Assert returns
        assert!(base_trade_r      == base_taker, 0);
        assert!(quote_trade_r     == quote_trade, 0);
        assert!(fee_r             == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side_maker,
                                  market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == maker_address, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            maker_address, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                maker_address, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_deposit_maker - base_taker, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == base_deposit_maker - base_taker, 0);
        assert!(quote_total     == quote_deposit_maker + quote_match, 0);
        assert!(quote_available == quote_deposit_maker + quote_match, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            maker_address, MARKET_ID_COIN, NO_CUSTODIAN) ==
            base_deposit_maker - base_taker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            maker_address, MARKET_ID_COIN, NO_CUSTODIAN) ==
            quote_deposit_maker + quote_match, 0);
        // Assert asset counts of taker.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                taker_address, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_taker, 0);
        assert!(base_available  == base_taker, 0);
        assert!(base_ceiling    == base_taker, 0);
        assert!(quote_total     == 0, 0);
        assert!(quote_available == 0, 0);
        assert!(quote_ceiling   == 0, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            taker_address, MARKET_ID_COIN, NO_CUSTODIAN) == base_taker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            taker_address, MARKET_ID_COIN, NO_CUSTODIAN) == 0, 0);
    }

    #[test]
    /// Verify state updates, returns for market sell when not enough
    /// liquidity to fill order. Based on
    /// `test_place_limit_order_crosses_bid_partial()`.
    fun test_place_market_order_not_enough_liquidity()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let direction_taker     = BUY;
        let side_maker          = if (direction_taker == BUY) ASK else BID;
        let size_match          = MIN_SIZE_COIN + 123;
        let size_post           = MIN_SIZE_COIN;
        let size                = size_match + size_post;
        let base_match          = size_match * LOT_SIZE_COIN;
        let base_post           = size_post * LOT_SIZE_COIN;
        let base                = base_match + base_post;
        let price               = integrator_divisor * taker_divisor;
        let quote_match         = size_match * price * TICK_SIZE_COIN;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let quote               = quote_match + quote_post;
        let quote_max           = quote + quote / taker_divisor;
        let integrator_share    = quote_match / integrator_divisor;
        let econia_share        =
            quote_match / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote_match + fee;
        let restriction         = NO_RESTRICTION;
        let self_match_behavior = ABORT;
        // Deposit to maker's account enough to impinge on min and max
        // amounts after fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_match));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_match));
        // Deposit to taker's account minimum amount to pass range
        // checking for taker fill.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_max));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[], 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_match,
            price, restriction, self_match_behavior);
        assert!(is_list_node_order_active( // Assert order is active.
            MARKET_ID_COIN, side_maker, market_order_id_0), 0);
        let (base_trade_r, quote_trade_r, fee_r) =
            place_market_order_user<BC, QC>( // Place taker order.
                &user_1, MARKET_ID_COIN, @integrator, direction_taker,
                size, self_match_behavior);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_match,
                    price,
                    restriction,
                    self_match_behavior,
                    size_match,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_place_market_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_place_market_order_event_test(
                    MARKET_ID_COIN,
                    @user_1,
                    NO_CUSTODIAN,
                    @integrator,
                    direction_taker,
                    size,
                    self_match_behavior,
                    taker_order_id
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_match,
            price,
            side_maker,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee_r,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_1, NO_CUSTODIAN) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    taker_order_id,
                    @user_1,
                    NO_CUSTODIAN,
                    CANCEL_REASON_NOT_ENOUGH_LIQUIDITY
                )
            ], 0);
        // Assert returns
        assert!(base_trade_r      == base_match, 0);
        assert!(quote_trade_r     == quote_trade, 0);
        assert!(fee_r             == fee, 0);
        // Assert filled order inactive.
        assert!(!is_list_node_order_active(
            MARKET_ID_COIN, side_maker, market_order_id_0), 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == 0, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == 0, 0);
        assert!(quote_total     == HI_64, 0);
        assert!(quote_available == HI_64, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == 0, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
        // Assert asset counts of taker.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == HI_64 - base_post, 0);
        assert!(base_available  == HI_64 - base_post, 0);
        assert!(base_ceiling    == HI_64 - base_post, 0);
        assert!(quote_total     == quote_max - quote_trade, 0);
        assert!(quote_available == quote_max - quote_trade, 0);
        assert!(quote_ceiling   == quote_max - quote_trade, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64 - base_post, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN)
            == quote_max - quote_trade, 0);
    }

    #[test]
    /// Manually remove market event handles before emission calls.
    fun test_place_market_order_remove_event_handles()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side                = ASK; // Maker sell.
        let size_post           = 3 * MIN_SIZE_COIN;
        let size_match          = MIN_SIZE_COIN; // Taker order size.
        let base_post           = size_post * LOT_SIZE_COIN;
        let price               = integrator_divisor * taker_divisor;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let self_match_behavior = ABORT;
        let base_deposit_taker  = HI_64 / 2;
        let quote_deposit_taker = HI_64 / 2;
        // Deposit to maker's account enough to impinge on min and max
        // amounts after complete fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_post));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_post));
        // Deposit to taker's account such that size, not max possible
        // match amount, is limiting factor for matching.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_taker));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_taker));
        // Place maker order.
        place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side, size_post, price,
            NO_RESTRICTION, self_match_behavior);
        // Remove only event handles for market account for maker.
        user::remove_market_event_handles_for_market_account_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        // Remove only event handles for market account for taker.
        user::remove_market_event_handles_for_market_account_test(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        // Place taker order.
        place_market_order_user<BC, QC>(
            &user_1, MARKET_ID_COIN, @integrator, BUY, size_match,
            self_match_behavior);
        // Remove all event handles for maker.
        user::remove_market_event_handles_test(@user_0);
        // Remove all event handles for taker.
        user::remove_market_event_handles_test(@user_1);
        // Place another taker order.
        place_market_order_user<BC, QC>(
            &user_1, MARKET_ID_COIN, @integrator, BUY, size_match,
            self_match_behavior);
    }

    #[test]
    #[expected_failure(abort_code = E_SIZE_BASE_OVERFLOW)]
    /// Verify failure for invalid size argument.
    fun test_place_market_order_size_base_overflow()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Declare order arguments.
        let user_address = @user_0;
        let market_id = MARKET_ID_COIN;
        let custodian_id = NO_CUSTODIAN;
        let integrator = @integrator;
        let direction = BUY;
        let size = HI_64 / LOT_SIZE_COIN + 1;
        let self_match_behavior = ABORT;
        // Attempt invalid invocation.
        place_market_order<BC, QC>(
            user_address, market_id, custodian_id, integrator, direction,
            size, self_match_behavior);
    }

    #[test]
    #[expected_failure(abort_code = E_SIZE_TOO_SMALL)]
    /// Verify failure for invalid size argument.
    fun test_place_market_order_size_too_small()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Declare order arguments.
        let user_address = @user_0;
        let market_id = MARKET_ID_COIN;
        let custodian_id = NO_CUSTODIAN;
        let integrator = @integrator;
        let direction = BUY;
        let size = MIN_SIZE_COIN - 1;
        let self_match_behavior = ABORT;
        // Attempt invalid invocation.
        place_market_order<BC, QC>(
            user_address, market_id, custodian_id, integrator, direction,
            size, self_match_behavior);
    }

    #[test]
    /// Verify state updates for public entry wrapper invocation. Based
    /// on `test_place_market_order_max_base_buy_user()`.
    fun test_place_market_order_user_entry()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get fee divisors.
        let (taker_divisor, integrator_divisor) =
            (incentives::get_taker_fee_divisor(),
             incentives::get_fee_share_divisor(INTEGRATOR_TIER));
        // Declare order parameters with price set to product of
        // divisors, to prevent truncation effects on estimates.
        let side                = ASK; // Maker sell.
        let size_match          = MIN_SIZE_COIN;
        let size_post           = size_match + 1; // Maker order size.
        let base_match          = size_match * LOT_SIZE_COIN;
        let base_post           = size_post * LOT_SIZE_COIN;
        let price               = integrator_divisor * taker_divisor;
        let quote_match         = size_match * price * TICK_SIZE_COIN;
        let quote_post          = size_post * price * TICK_SIZE_COIN;
        let integrator_share    = quote_match / integrator_divisor;
        let econia_share        =
            quote_match / taker_divisor - integrator_share;
        let fee                 = integrator_share + econia_share;
        let quote_trade         = quote_match + fee;
        let quote_deposit       = quote_trade * TICK_SIZE_COIN;
        let self_match_behavior = ABORT;
        let size_taker          = size_match;
        // Deposit to maker's account enough to impinge on min and max
        // amounts after fill.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_post));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - quote_post));
        // Deposit to taker's account similarly, for base match amount
        // as limiting factor in matching engine.
        user::deposit_coins<BC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(HI_64 - base_match));
        user::deposit_coins<QC>(@user_1, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit));
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side, size_post, price,
            NO_RESTRICTION, self_match_behavior); // Place maker order.
        place_market_order_user_entry<BC, QC>( // Place taker order.
            &user_1, MARKET_ID_COIN, @integrator, BUY, size_taker,
            self_match_behavior);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(MARKET_ID_COIN, side, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_post - size_match, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side, order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_post - size_match, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_post - base_match, 0);
        assert!(base_available  == 0, 0);
        assert!(base_ceiling    == base_post - base_match, 0);
        assert!(quote_total     == HI_64 - quote_post + quote_match, 0);
        assert!(quote_available == HI_64 - quote_post + quote_match, 0);
        assert!(quote_ceiling   == HI_64, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN)
            == base_post - base_match, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN)
            == HI_64 - quote_post + quote_match, 0);
        // Assert taker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_1, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == HI_64, 0);
        assert!(base_available  == HI_64, 0);
        assert!(base_ceiling    == HI_64, 0);
        assert!(quote_total     == quote_deposit - quote_trade, 0);
        assert!(quote_available == quote_deposit - quote_trade, 0);
        assert!(quote_ceiling   == quote_deposit - quote_trade, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN) == HI_64, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_1, MARKET_ID_COIN, NO_CUSTODIAN)
            == quote_deposit - quote_trade, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_OVERFLOW_ASSET_IN)]
    /// Verify failure for overflowing asset in for a buy.
    fun test_range_check_trade_asset_in_buy() {
        // Declare inputs.
        let direction = BUY;
        let min_base = 0;
        let max_base = 1;
        let min_quote = 0;
        let max_quote = 1;
        let base_available = 0;
        let base_ceiling = HI_64;
        let quote_available = 0;
        let quote_ceiling = HI_64;
        // Attempt invalid invocation.
        range_check_trade(
            direction, min_base, max_base, min_quote, max_quote,
            base_available, base_ceiling, quote_available, quote_ceiling);
    }

    #[test]
    #[expected_failure(abort_code = E_OVERFLOW_ASSET_IN)]
    /// Verify failure for overflowing asset in for a sell.
    fun test_range_check_trade_asset_in_sell() {
        // Declare inputs.
        let direction = SELL;
        let min_base = 0;
        let max_base = 1;
        let min_quote = 0;
        let max_quote = 1;
        let base_available = 0;
        let base_ceiling = HI_64;
        let quote_available = 0;
        let quote_ceiling = HI_64;
        // Attempt invalid invocation.
        range_check_trade(
            direction, min_base, max_base, min_quote, max_quote,
            base_available, base_ceiling, quote_available, quote_ceiling);
    }

    #[test]
    #[expected_failure(abort_code = E_NOT_ENOUGH_ASSET_OUT)]
    /// Verify failure for underflowing asset out for a buy.
    fun test_range_check_trade_asset_out_buy() {
        // Declare inputs.
        let direction = BUY;
        let min_base = 0;
        let max_base = 1;
        let min_quote = 0;
        let max_quote = 1;
        let base_available = 0;
        let base_ceiling = 1;
        let quote_available = 0;
        let quote_ceiling = 1;
        // Attempt invalid invocation.
        range_check_trade(
            direction, min_base, max_base, min_quote, max_quote,
            base_available, base_ceiling, quote_available, quote_ceiling);
    }

    #[test]
    #[expected_failure(abort_code = E_NOT_ENOUGH_ASSET_OUT)]
    /// Verify failure for underflowing asset out for a sell.
    fun test_range_check_trade_asset_out_sell() {
        // Declare inputs.
        let direction = SELL;
        let min_base = 0;
        let max_base = 1;
        let min_quote = 0;
        let max_quote = 1;
        let base_available = 0;
        let base_ceiling = 1;
        let quote_available = 0;
        let quote_ceiling = 1;
        // Attempt invalid invocation.
        range_check_trade(
            direction, min_base, max_base, min_quote, max_quote,
            base_available, base_ceiling, quote_available, quote_ceiling);
    }

    #[test]
    #[expected_failure(abort_code = E_MAX_BASE_0)]
    /// Verify failure for max base specified as 0.
    fun test_range_check_trade_base_0() {
        // Declare inputs.
        let direction = SELL;
        let min_base = 0;
        let max_base = 0;
        let min_quote = 0;
        let max_quote = 0;
        let base_available = 0;
        let base_ceiling = 0;
        let quote_available = 0;
        let quote_ceiling = 0;
        // Attempt invalid invocation.
        range_check_trade(
            direction, min_base, max_base, min_quote, max_quote,
            base_available, base_ceiling, quote_available, quote_ceiling);
    }

    #[test]
    #[expected_failure(abort_code = E_MIN_BASE_EXCEEDS_MAX)]
    /// Verify failure for min base exceeds max
    fun test_range_check_trade_min_base_exceeds_max() {
        // Declare inputs.
        let direction = SELL;
        let min_base = 2;
        let max_base = 1;
        let min_quote = 0;
        let max_quote = 1;
        let base_available = 0;
        let base_ceiling = 0;
        let quote_available = 0;
        let quote_ceiling = 0;
        // Attempt invalid invocation.
        range_check_trade(
            direction, min_base, max_base, min_quote, max_quote,
            base_available, base_ceiling, quote_available, quote_ceiling);
    }

    #[test]
    #[expected_failure(abort_code = E_MIN_QUOTE_EXCEEDS_MAX)]
    /// Verify failure for min quote exceeds max
    fun test_range_check_trade_min_quote_exceeds_max() {
        // Declare inputs.
        let direction = SELL;
        let min_base = 0;
        let max_base = 1;
        let min_quote = 2;
        let max_quote = 1;
        let base_available = 0;
        let base_ceiling = 0;
        let quote_available = 0;
        let quote_ceiling = 0;
        // Attempt invalid invocation.
        range_check_trade(
            direction, min_base, max_base, min_quote, max_quote,
            base_available, base_ceiling, quote_available, quote_ceiling);
    }

    #[test]
    #[expected_failure(abort_code = E_MAX_QUOTE_0)]
    /// Verify failure for max quote specified as 0.
    fun test_range_check_trade_quote_0() {
        // Declare inputs.
        let direction = SELL;
        let min_base = 0;
        let max_base = 1;
        let min_quote = 0;
        let max_quote = 0;
        let base_available = 0;
        let base_ceiling = 0;
        let quote_available = 0;
        let quote_ceiling = 0;
        // Attempt invalid invocation.
        range_check_trade(
            direction, min_base, max_base, min_quote, max_quote,
            base_available, base_ceiling, quote_available, quote_ceiling);
    }

    #[test]
    /// Assert state updates and returns for:
    ///
    /// 1. Registering pure coin market from coin store.
    /// 2. Registering generic market.
    /// 3. Registering pure coin market, not from coin store.
    fun test_register_markets()
    acquires OrderBooks {
        init_test(); // Init for testing.
        // Get market registration fee, denominated in utility coins.
        let fee = incentives::get_market_registration_fee();
        // Create user account.
        let user = account::create_account_for_test(@user);
        coin::register<UC>(&user); // Register user coin store.
        // Deposit utility coins required to cover fee.
        coin::deposit<UC>(@user, assets::mint_test(fee));
        // Register pure coin market from coinstore.
        register_market_base_coin_from_coinstore<BC, QC, UC>(
            &user, LOT_SIZE_COIN, TICK_SIZE_COIN, MIN_SIZE_COIN);
        // Get market info returns from registry.
        let (base_name_generic_r, lot_size_r, tick_size_r, min_size_r,
             underwriter_id_r) = registry::get_market_info_for_market_account(
                MARKET_ID_COIN, type_info::type_of<BC>(),
                type_info::type_of<QC>());
        // Assert registry market info returns.
        assert!(base_name_generic_r == string::utf8(b""), 0);
        assert!(lot_size_r          == LOT_SIZE_COIN, 0);
        assert!(tick_size_r         == TICK_SIZE_COIN, 0);
        assert!(min_size_r          == MIN_SIZE_COIN, 0);
        assert!(underwriter_id_r    == NO_UNDERWRITER, 0);
        // Assert fee store with corresponding market ID is empty.
        assert!(incentives::get_econia_fee_store_balance_test<QC>(
            MARKET_ID_COIN) == 0, 0);
        let order_books_map_ref = // Immutably borrow order books map.
            &borrow_global<OrderBooks>(resource_account::get_address()).map;
        let order_book_ref = // Immutably borrow order book.
            tablist::borrow(order_books_map_ref, MARKET_ID_COIN);
        // Assert order book state.
        assert!(order_book_ref.base_type == type_info::type_of<BC>(), 0);
        assert!(order_book_ref.base_name_generic == string::utf8(b""), 0);
        assert!(order_book_ref.quote_type == type_info::type_of<QC>(), 0);
        assert!(order_book_ref.lot_size == LOT_SIZE_COIN, 0);
        assert!(order_book_ref.tick_size == TICK_SIZE_COIN, 0);
        assert!(order_book_ref.min_size == MIN_SIZE_COIN, 0);
        assert!(order_book_ref.underwriter_id == NO_UNDERWRITER, 0);
        assert!(avl_queue::is_empty(&order_book_ref.asks), 0);
        assert!(avl_queue::is_ascending(&order_book_ref.asks), 0);
        assert!(avl_queue::is_empty(&order_book_ref.bids), 0);
        assert!(!avl_queue::is_ascending(&order_book_ref.bids), 0);
        assert!(order_book_ref.counter == 0, 0);
        let underwriter_capability = registry::get_underwriter_capability_test(
            UNDERWRITER_ID); // Get market underwriter capability.
        // Register generic market, storing market ID.
        let market_id = register_market_base_generic<QC, UC>(
            string::utf8(BASE_NAME_GENERIC), LOT_SIZE_GENERIC,
            TICK_SIZE_GENERIC, MIN_SIZE_GENERIC, assets::mint_test<UC>(fee),
            &underwriter_capability);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
        // Assert market ID.
        assert!(market_id == MARKET_ID_GENERIC, 0);
        // Get market info returns from registry.
        (base_name_generic_r, lot_size_r, tick_size_r, min_size_r,
         underwriter_id_r) = registry::get_market_info_for_market_account(
            MARKET_ID_GENERIC, type_info::type_of<GenericAsset>(),
            type_info::type_of<QC>());
        // Assert registry market info returns.
        assert!(base_name_generic_r == string::utf8(BASE_NAME_GENERIC), 0);
        assert!(lot_size_r          == LOT_SIZE_GENERIC, 0);
        assert!(tick_size_r         == TICK_SIZE_GENERIC, 0);
        assert!(min_size_r          == MIN_SIZE_GENERIC, 0);
        assert!(underwriter_id_r    == UNDERWRITER_ID, 0);
        // Assert fee store with corresponding market ID is empty.
        assert!(incentives::get_econia_fee_store_balance_test<QC>(
            MARKET_ID_GENERIC) == 0, 0);
        order_books_map_ref = // Immutably borrow order books map.
            &borrow_global<OrderBooks>(resource_account::get_address()).map;
        order_book_ref = // Immutably borrow order book.
            tablist::borrow(order_books_map_ref, MARKET_ID_GENERIC);
        // Assert order book state.
        assert!(order_book_ref.base_type ==
                type_info::type_of<GenericAsset>(), 0);
        assert!(order_book_ref.base_name_generic ==
                string::utf8(BASE_NAME_GENERIC), 0);
        assert!(order_book_ref.quote_type == type_info::type_of<QC>(), 0);
        assert!(order_book_ref.lot_size == LOT_SIZE_GENERIC, 0);
        assert!(order_book_ref.tick_size == TICK_SIZE_GENERIC, 0);
        assert!(order_book_ref.min_size == MIN_SIZE_GENERIC, 0);
        assert!(order_book_ref.underwriter_id == UNDERWRITER_ID, 0);
        assert!(avl_queue::is_empty(&order_book_ref.asks), 0);
        assert!(avl_queue::is_ascending(&order_book_ref.asks), 0);
        assert!(avl_queue::is_empty(&order_book_ref.bids), 0);
        assert!(!avl_queue::is_ascending(&order_book_ref.bids), 0);
        assert!(order_book_ref.counter == 0, 0);
        // Assert market ID return for registering pure coin market not
        // from coin store.
        assert!(register_market_base_coin<QC, BC, UC>(
            1, 1, 1, assets::mint_test<UC>(fee)) == 3, 0);
    }

    #[test]
    /// Verify returns, state updates for specifying max possible base
    /// during a buy.
    fun test_swap_between_coinstores_max_possible_base_buy()
    acquires
        MarketEventHandles,
        OrderBooks,
        SwapperEventHandles
    {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = BUY;
        let side_maker          = ASK; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = MAX_POSSIBLE;
        let min_quote           = 0;
        let max_quote           = quote_trade * TICK_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = base_maker;
        let quote_deposit_maker = HI_64 - quote_maker;
        let base_deposit_taker  = HI_64 - base_taker;
        let quote_deposit_taker = max_quote;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker - base_taker;
        let base_available_maker  = 0;
        let base_ceiling_maker    = base_total_maker;
        let quote_total_maker     = quote_deposit_maker + quote_match;
        let quote_available_maker = quote_total_maker;
        let quote_ceiling_maker   = HI_64;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = HI_64;
        let quote_total_taker = quote_deposit_taker - quote_trade;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Deposit taker coins.
        coin::register<BC>(&user_1);
        coin::register<QC>(&user_1);
        coin::deposit<BC>(@user_1, assets::mint_test(base_deposit_taker));
        coin::deposit<QC>(@user_1, assets::mint_test(quote_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_1), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        let (base_trade_r, quote_trade_r, fee_r) = // Place taker order.
            swap_between_coinstores<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, direction, min_base,
                max_base, min_quote, max_quote, price);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side_maker,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: @user_1,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base: base_taker,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[], 0);
        assert!(get_fill_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[fill_event], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::balance<BC>(@user_1) == base_total_taker, 0);
        assert!(coin::balance<QC>(@user_1) == quote_total_taker, 0);
    }

    #[test]
    /// Verify returns, state updates for specifying max possible base
    /// during a sell.
    fun test_swap_between_coinstores_max_possible_base_sell()
    acquires
        MarketEventHandles,
        OrderBooks,
        SwapperEventHandles
    {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = SELL;
        let side_maker          = BID; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = MAX_POSSIBLE;
        let min_quote           = 0;
        let max_quote           = quote_trade * TICK_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = HI_64 - base_maker;
        let quote_deposit_maker = quote_maker;
        let base_deposit_taker  = base_taker;
        let quote_deposit_taker = HI_64 - max_quote;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker + base_taker;
        let base_available_maker  = base_total_maker;
        let base_ceiling_maker    = HI_64;
        let quote_total_maker     = quote_maker - quote_match;
        let quote_available_maker = 0;
        let quote_ceiling_maker   = quote_total_maker;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = 0;
        let quote_total_taker = quote_deposit_taker + quote_trade;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Deposit taker coins.
        coin::register<BC>(&user_1);
        coin::register<QC>(&user_1);
        coin::deposit<BC>(@user_1, assets::mint_test(base_deposit_taker));
        coin::deposit<QC>(@user_1, assets::mint_test(quote_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_1), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        let (base_trade_r, quote_trade_r, fee_r) = // Place taker order.
            swap_between_coinstores<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, direction, min_base,
                max_base, min_quote, max_quote, price);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side_maker,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: @user_1,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base: base_taker,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[], 0);
        assert!(get_fill_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[fill_event], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::balance<BC>(@user_1) == base_total_taker, 0);
        assert!(coin::balance<QC>(@user_1) == quote_total_taker, 0);
    }

    #[test]
    /// Verify returns, state updates for specifying max possible quote
    /// during a buy.
    fun test_swap_between_coinstores_max_possible_quote_buy()
    acquires
        MarketEventHandles,
        OrderBooks,
        SwapperEventHandles
    {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = BUY;
        let side_maker          = ASK; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = base_taker * LOT_SIZE_COIN;
        let min_quote           = 0;
        let max_quote           = MAX_POSSIBLE;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = base_maker;
        let quote_deposit_maker = HI_64 - quote_maker;
        let base_deposit_taker  = HI_64 - max_base;
        let quote_deposit_taker = quote_trade;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_maker - base_taker;
        let base_available_maker  = 0;
        let base_ceiling_maker    = base_total_maker;
        let quote_total_maker     = quote_deposit_maker + quote_match;
        let quote_available_maker = quote_total_maker;
        let quote_ceiling_maker   = HI_64;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = base_deposit_taker + base_taker;
        let quote_total_taker = 0;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Deposit taker coins.
        coin::register<BC>(&user_1);
        coin::register<QC>(&user_1);
        coin::deposit<BC>(@user_1, assets::mint_test(base_deposit_taker));
        coin::deposit<QC>(@user_1, assets::mint_test(quote_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_1), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        let (base_trade_r, quote_trade_r, fee_r) = // Place taker order.
            swap_between_coinstores<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, direction, min_base,
                max_base, min_quote, max_quote, price);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side_maker,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: @user_1,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote: quote_trade,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    taker_order_id,
                    @user_1,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        assert!(get_fill_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[fill_event], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::balance<BC>(@user_1) == base_total_taker, 0);
        assert!(coin::balance<QC>(@user_1) == quote_total_taker, 0);
    }

    #[test]
    /// Verify returns, state updates for specifying max possible quote
    /// during a sell.
    fun test_swap_between_coinstores_max_possible_quote_sell()
    acquires
        MarketEventHandles,
        OrderBooks,
        SwapperEventHandles
    {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = SELL;
        let side_maker          = BID; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = base_taker * LOT_SIZE_COIN;
        let min_quote           = 0;
        let max_quote           = MAX_POSSIBLE;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = HI_64 - base_maker;
        let quote_deposit_maker = quote_maker;
        let base_deposit_taker  = max_base;
        let quote_deposit_taker = HI_64 - quote_trade;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker + base_taker;
        let base_available_maker  = base_total_maker;
        let base_ceiling_maker    = HI_64;
        let quote_total_maker     = quote_deposit_maker - quote_match;
        let quote_available_maker = 0;
        let quote_ceiling_maker   = quote_total_maker;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = base_deposit_taker - base_taker;
        let quote_total_taker = HI_64;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Deposit taker coins.
        coin::register<BC>(&user_1);
        coin::register<QC>(&user_1);
        coin::deposit<BC>(@user_1, assets::mint_test(base_deposit_taker));
        coin::deposit<QC>(@user_1, assets::mint_test(quote_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_1), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        let (base_trade_r, quote_trade_r, fee_r) = // Place taker order.
            swap_between_coinstores<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, direction, min_base,
                max_base, min_quote, max_quote, price);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side_maker,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: @user_1,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote: quote_trade,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    taker_order_id,
                    @user_1,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        assert!(get_fill_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[fill_event], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::balance<BC>(@user_1) == base_total_taker, 0);
        assert!(coin::balance<QC>(@user_1) == quote_total_taker, 0);
    }

    #[test]
    /// Verify returns, state updates for specifying sell when matching
    /// ends early due to quote coin constraints.
    fun test_swap_between_coinstores_max_quote_traded()
    acquires
        MarketEventHandles,
        OrderBooks,
        SwapperEventHandles
    {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = SELL;
        let side_maker          = BID; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN * 2;
        let size_taker          = MIN_SIZE_COIN;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = (size_taker + MIN_SIZE_COIN) * LOT_SIZE_COIN;
        let min_quote           = 0;
        let max_quote           = quote_trade;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = HI_64 - base_maker;
        let quote_deposit_maker = quote_maker;
        let base_deposit_taker  = max_base;
        let quote_deposit_taker = HI_64 - quote_trade;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker + base_taker;
        let base_available_maker  = base_total_maker;
        let base_ceiling_maker    = HI_64;
        let quote_total_maker     = quote_deposit_maker - quote_match;
        let quote_available_maker = 0;
        let quote_ceiling_maker   = quote_total_maker;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = base_deposit_taker - base_taker;
        let quote_total_taker = HI_64;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Deposit taker coins.
        coin::register<BC>(&user_1);
        coin::register<QC>(&user_1);
        coin::deposit<BC>(@user_1, assets::mint_test(base_deposit_taker));
        coin::deposit<QC>(@user_1, assets::mint_test(quote_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_1), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        // Create swapper event handles for market.
        move_to(&user_1, SwapperEventHandles{map: table::new()});
        let swapper_event_handles_map_ref_mut =
            &mut borrow_global_mut<SwapperEventHandles>(@user_1).map;
        let handles = SwapperEventHandlesForMarket{
            cancel_order_events: account::new_event_handle(&user_1),
            fill_events: account::new_event_handle(&user_1),
            place_swap_order_events: account::new_event_handle(&user_1)
        };
        table::add(
            swapper_event_handles_map_ref_mut, MARKET_ID_COIN, handles);
        let (base_trade_r, quote_trade_r, fee_r) = // Place taker order.
            swap_between_coinstores<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, direction, min_base,
                max_base, min_quote, max_quote, price);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side_maker,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: @user_1,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote: quote_trade,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    taker_order_id,
                    @user_1,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        assert!(get_fill_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[fill_event], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::balance<BC>(@user_1) == base_total_taker, 0);
        assert!(coin::balance<QC>(@user_1) == quote_total_taker, 0);
    }

    #[test]
    /// Verify returns, state updates for specifying sell when there is
    /// not enough liquidity.
    fun test_swap_between_coinstores_not_enough_liquidity()
    acquires
        MarketEventHandles,
        OrderBooks,
        SwapperEventHandles
    {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = SELL;
        let side_maker          = BID; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = size_maker;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = (size_taker + MIN_SIZE_COIN) * LOT_SIZE_COIN;
        let min_quote           = 0;
        let max_quote           = MAX_POSSIBLE;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = HI_64 - base_maker;
        let quote_deposit_maker = quote_maker;
        let base_deposit_taker  = max_base;
        let quote_deposit_taker = HI_64 - 2 * quote_trade;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker + base_taker;
        let base_available_maker  = base_total_maker;
        let base_ceiling_maker    = HI_64;
        let quote_total_maker     = quote_deposit_maker - quote_match;
        let quote_available_maker = 0;
        let quote_ceiling_maker   = quote_total_maker;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = base_deposit_taker - base_taker;
        let quote_total_taker = HI_64 - quote_trade;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Deposit taker coins.
        coin::register<BC>(&user_1);
        coin::register<QC>(&user_1);
        coin::deposit<BC>(@user_1, assets::mint_test(base_deposit_taker));
        coin::deposit<QC>(@user_1, assets::mint_test(quote_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_1), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        // Get user-side order access key for later.
        let (_, _, _, _, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        let (base_trade_r, quote_trade_r, fee_r) = // Place taker order.
            swap_between_coinstores<BC, QC>(
                &user_1, MARKET_ID_COIN, @integrator, direction, min_base,
                max_base, min_quote, max_quote, price);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side_maker,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: @user_1,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote: HI_64 - quote_deposit_taker,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    taker_order_id,
                    @user_1,
                    NO_CUSTODIAN,
                    CANCEL_REASON_NOT_ENOUGH_LIQUIDITY
                )
            ], 0);
        assert!(get_fill_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[fill_event], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Assert list node order inactive.
        assert!(!is_list_node_order_active(
            MARKET_ID_COIN, side_maker, market_order_id_0), 0);
        // Assert user-side order fields for filled order
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        // No market order ID.
        assert!(market_order_id_r == (NIL as u128), 0);
        assert!(size_r == NIL, 0); // Bottom of inactive stack.
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::balance<BC>(@user_1) == base_total_taker, 0);
        assert!(coin::balance<QC>(@user_1) == quote_total_taker, 0);
    }

    #[test]
    /// Verify returns, state updates for registering base coin store.
    fun test_swap_between_coinstores_register_base_store()
    acquires
        MarketEventHandles,
        OrderBooks,
        SwapperEventHandles
    {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Verify no event handle info.
        assert!(get_market_event_handle_creation_info(MARKET_ID_COIN) ==
                option::none(), 0);
        assert!(get_swapper_event_handle_creation_numbers(
                    @user_1, MARKET_ID_COIN) == option::none(), 0);
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = BUY;
        let side_maker          = ASK; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = base_taker;
        let min_quote           = 0;
        let max_quote           = MAX_POSSIBLE;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = base_maker;
        let quote_deposit_maker = HI_64 - quote_maker;
        let quote_deposit_taker = quote_trade * TICK_SIZE_COIN;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker - base_taker;
        let base_available_maker  = 0;
        let base_ceiling_maker    = base_total_maker;
        let quote_total_maker     = quote_deposit_maker + quote_match;
        let quote_available_maker = quote_total_maker;
        let quote_ceiling_maker   = HI_64;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = base_taker;
        let quote_total_taker = quote_deposit_taker - quote_trade;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Deposit taker coins.
        coin::register<QC>(&user_1);
        coin::deposit<QC>(@user_1, assets::mint_test(quote_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_1), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        swap_between_coinstores_entry<BC, QC>( // Place taker order.
            &user_1, MARKET_ID_COIN, @integrator, direction, min_base,
            max_base, min_quote, max_quote, price);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side_maker,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: @user_1,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote: quote_deposit_taker,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[], 0);
        assert!(get_fill_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[fill_event], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::balance<BC>(@user_1) == base_total_taker, 0);
        assert!(coin::balance<QC>(@user_1) == quote_total_taker, 0);
        let market_handle_0 = FIRST_EVENT_HANDLE_RESOURCE_ACCOUNT;
        let market_handle_1 = market_handle_0 + 1;
        // Verify event handle info.
        assert!(get_market_event_handle_creation_info(MARKET_ID_COIN) ==
                option::some(MarketEventHandleCreationInfo{
                    resource_account_address: resource_account::get_address(),
                    cancel_order_events_handle_creation_num: market_handle_0,
                    place_swap_order_events_handle_creation_num:
                        market_handle_1
                }), 0);
        let swapper_handle_0 = FIRST_EVENT_HANDLE_SWAPPER;
        let swapper_handle_1 = swapper_handle_0 + 1;
        let swapper_handle_2 = swapper_handle_1 + 1;
        assert!(get_swapper_event_handle_creation_numbers(
            @user_1, MARKET_ID_COIN) ==
                option::some(SwapperEventHandleCreationNumbers{
                    cancel_order_events_handle_creation_num: swapper_handle_0,
                    fill_events_handle_creation_num: swapper_handle_1,
                    place_swap_order_events_handle_creation_num:
                        swapper_handle_2,
                }), 0);
        assert!(get_market_event_handle_creation_info(MARKET_ID_GENERIC) ==
                option::none(), 0);
        assert!(get_swapper_event_handle_creation_numbers(
                    @user_1, MARKET_ID_GENERIC) == option::none(), 0);
    }

    #[test]
    /// Verify returns, state updates for registering quote coin store.
    fun test_swap_between_coinstores_register_quote_store()
    acquires
        MarketEventHandles,
        OrderBooks,
        SwapperEventHandles
    {
        // Initialize markets, users, and an integrator.
        let (user_0, user_1) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = SELL;
        let side_maker          = BID; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = MAX_POSSIBLE;
        let min_quote           = 0;
        let max_quote           = quote_trade;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = HI_64 - base_maker;
        let quote_deposit_maker = quote_maker;
        let base_deposit_taker  = base_taker * LOT_SIZE_COIN;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker + base_taker;
        let base_available_maker  = base_total_maker;
        let base_ceiling_maker    = HI_64;
        let quote_total_maker     = quote_deposit_maker - quote_match;
        let quote_available_maker = 0;
        let quote_ceiling_maker   = quote_total_maker;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = base_deposit_taker - base_taker;
        let quote_total_taker = quote_trade;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Deposit taker coins.
        coin::register<BC>(&user_1);
        coin::deposit<BC>(@user_1, assets::mint_test(base_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_1), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        // Create swapper event handles map, but not handles for market.
        move_to(&user_1, SwapperEventHandles{map: table::new()});
        swap_between_coinstores_entry<BC, QC>( // Place taker order.
            &user_1, MARKET_ID_COIN, @integrator, direction, min_base,
            max_base, min_quote, max_quote, price);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        let fill_event = user::create_fill_event_internal(
            MARKET_ID_COIN,
            size_taker,
            price,
            side_maker,
            @user_0,
            NO_CUSTODIAN,
            market_order_id_0,
            @user_1,
            NO_CUSTODIAN,
            taker_order_id,
            fee,
            0
        );
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[fill_event], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: @user_1,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base: base_deposit_taker,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    taker_order_id,
                    @user_1,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        assert!(get_fill_events_swapper_test(
            MARKET_ID_COIN, @user_1) == vector[fill_event], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::balance<BC>(@user_1) == base_total_taker, 0);
        assert!(coin::balance<QC>(@user_1) == quote_total_taker, 0);
    }

    #[test]
    /// Verify returns, state updates for self match taker cancel.
    fun test_swap_between_coinstores_self_match_taker_cancel()
    acquires
        MarketEventHandles,
        OrderBooks,
        SwapperEventHandles
    {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = BUY;
        let side_maker          = ASK; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = MAX_POSSIBLE;
        let min_quote           = 0;
        let max_quote           = quote_trade * TICK_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = base_maker;
        let quote_deposit_maker = HI_64 - quote_maker;
        let base_deposit_taker  = HI_64 - base_taker;
        let quote_deposit_taker = max_quote;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker;
        let base_available_maker  = 0;
        let base_ceiling_maker    = base_total_maker;
        let quote_total_maker     = quote_deposit_maker;
        let quote_available_maker = quote_total_maker;
        let quote_ceiling_maker   = HI_64;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Deposit taker coins.
        coin::register<BC>(&user_0);
        coin::register<QC>(&user_0);
        coin::deposit<BC>(@user_0, assets::mint_test(base_deposit_taker));
        coin::deposit<QC>(@user_0, assets::mint_test(quote_deposit_taker));
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_0), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        let (base_trade_r, quote_trade_r, fee_r) = // Place taker order.
            swap_between_coinstores<BC, QC>(
                &user_0, MARKET_ID_COIN, @integrator, direction, min_base,
                max_base, min_quote, max_quote, price);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_swapper_test(
            MARKET_ID_COIN, @user_0) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: @user_0,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base: base_taker,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_swapper_test(
            MARKET_ID_COIN, @user_0) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    taker_order_id,
                    @user_0,
                    NO_CUSTODIAN,
                    CANCEL_REASON_SELF_MATCH_TAKER
                )
            ], 0);
        assert!(get_fill_events_swapper_test(
            MARKET_ID_COIN, @user_0) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == 0, 0);
        assert!(quote_trade_r == 0, 0);
        assert!(fee_r         == 0, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::balance<BC>(@user_0) == base_deposit_taker, 0);
        assert!(coin::balance<QC>(@user_0) == quote_deposit_taker, 0);
    }

    #[test]
    /// Verify returns, state updates for swap buy for max possible
    /// base amount specified, with base amount as limiting factor.
    fun test_swap_coins_buy_max_base_limiting()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = BUY;
        let side_maker          = ASK; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = MAX_POSSIBLE;
        let min_quote           = 0;
        let max_quote           = 0;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = base_maker;
        let quote_deposit_maker = HI_64 - quote_maker;
        let base_deposit_taker  = HI_64 - base_taker;
        let quote_deposit_taker = quote_trade * TICK_SIZE_COIN;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker - base_taker;
        let base_available_maker  = 0;
        let base_ceiling_maker    = base_total_maker;
        let quote_total_maker     = quote_deposit_maker + quote_match;
        let quote_available_maker = quote_total_maker;
        let quote_ceiling_maker   = HI_64;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = HI_64;
        let quote_total_taker = quote_deposit_taker - quote_trade;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Create taker coins.
        let base_coins  = assets::mint_test<BC>(base_deposit_taker);
        let quote_coins = assets::mint_test<QC>(quote_deposit_taker);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_0), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        let (base_coins, quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_coins<BC, QC>( // Place taker order.
                MARKET_ID_COIN, @integrator, direction, min_base, max_base,
                min_quote, max_quote, price, base_coins, quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_fill_event_internal(
                    MARKET_ID_COIN,
                    size_taker,
                    price,
                    side_maker,
                    @user_0,
                    NO_CUSTODIAN,
                    market_order_id_0,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base: HI_64 - base_deposit_taker,
                    min_quote,
                    max_quote: quote_deposit_taker,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::value<BC>(&base_coins)  == base_total_taker, 0);
        assert!(coin::value<QC>(&quote_coins) == quote_total_taker, 0);
        // Burn coins.
        if (base_total_taker == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_total_taker == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
    }

    #[test]
    /// Verify returns, state updates for swap buy for max possible
    /// base amount not specified, with base amount as limiting factor.
    fun test_swap_coins_buy_no_max_base_limiting()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = BUY;
        let side_maker          = ASK; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = base_taker;
        let min_quote           = 0;
        let max_quote           = 0;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = base_maker;
        let quote_deposit_maker = HI_64 - quote_maker;
        let base_deposit_taker  = HI_64 - max_base;
        let quote_deposit_taker = quote_trade * TICK_SIZE_COIN;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker - base_taker;
        let base_available_maker  = 0;
        let base_ceiling_maker    = base_total_maker;
        let quote_total_maker     = quote_deposit_maker + quote_match;
        let quote_available_maker = quote_total_maker;
        let quote_ceiling_maker   = HI_64;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = HI_64;
        let quote_total_taker = quote_deposit_taker - quote_trade;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Create taker coins.
        let base_coins  = assets::mint_test<BC>(base_deposit_taker);
        let quote_coins = assets::mint_test<QC>(quote_deposit_taker);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_0), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        let (base_coins, quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_coins<BC, QC>( // Place taker order.
                MARKET_ID_COIN, @integrator, direction, min_base, max_base,
                min_quote, max_quote, price, base_coins, quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_fill_event_internal(
                    MARKET_ID_COIN,
                    size_taker,
                    price,
                    side_maker,
                    @user_0,
                    NO_CUSTODIAN,
                    market_order_id_0,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base: base_taker,
                    min_quote,
                    max_quote: quote_deposit_taker,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::value<BC>(&base_coins)  == base_total_taker, 0);
        assert!(coin::value<QC>(&quote_coins) == quote_total_taker, 0);
        // Burn coins.
        if (base_total_taker == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_total_taker == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
    }

    #[test]
    /// Verify returns, state updates for swap buy for max possible
    /// base amount not specified, with quote amount as limiting factor.
    fun test_swap_coins_buy_no_max_quote_limiting()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = BUY;
        let side_maker          = ASK; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = base_taker * LOT_SIZE_COIN;
        let min_quote           = 0;
        let max_quote           = 0;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = base_maker;
        let quote_deposit_maker = HI_64 - quote_maker;
        let base_deposit_taker  = HI_64 - max_base;
        let quote_deposit_taker = quote_trade;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker - base_taker;
        let base_available_maker  = 0;
        let base_ceiling_maker    = base_total_maker;
        let quote_total_maker     = quote_deposit_maker + quote_match;
        let quote_available_maker = quote_total_maker;
        let quote_ceiling_maker   = HI_64;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = base_deposit_taker + base_taker;
        let quote_total_taker = 0;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Create taker coins.
        let base_coins  = assets::mint_test<BC>(base_deposit_taker);
        let quote_coins = assets::mint_test<QC>(quote_deposit_taker);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_0), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        let (base_coins, quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_coins<BC, QC>( // Place taker order.
                MARKET_ID_COIN, @integrator, direction, min_base, max_base,
                min_quote, max_quote, price, base_coins, quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_fill_event_internal(
                    MARKET_ID_COIN,
                    size_taker,
                    price,
                    side_maker,
                    @user_0,
                    NO_CUSTODIAN,
                    market_order_id_0,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote: quote_deposit_taker,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    taker_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::value<BC>(&base_coins)  == base_total_taker, 0);
        assert!(coin::value<QC>(&quote_coins) == quote_total_taker, 0);
        // Burn coins.
        if (base_total_taker == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_total_taker == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
    }

    #[test]
    /// Verify returns, state updates for swap sell for max possible
    /// quote amount specified, with quote amount as limiting factor.
    fun test_swap_coins_sell_max_quote_limiting()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = SELL;
        let side_maker          = BID; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = 0;
        let min_quote           = 0;
        let max_quote           = MAX_POSSIBLE;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = HI_64 - base_maker;
        let quote_deposit_maker = quote_maker;
        let base_deposit_taker  = base_taker * LOT_SIZE_COIN;
        let quote_deposit_taker = HI_64 - quote_trade;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker + base_taker;
        let base_available_maker  = base_total_maker;
        let base_ceiling_maker    = HI_64;
        let quote_total_maker     = quote_deposit_maker - quote_match;
        let quote_available_maker = 0;
        let quote_ceiling_maker   = quote_total_maker;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = base_deposit_taker - base_taker;
        let quote_total_taker = HI_64;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Create taker coins.
        let base_coins  = assets::mint_test<BC>(base_deposit_taker);
        let quote_coins = assets::mint_test<QC>(quote_deposit_taker);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_0), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        let (base_coins, quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_coins<BC, QC>( // Place taker order.
                MARKET_ID_COIN, @integrator, direction, min_base, max_base,
                min_quote, max_quote, price, base_coins, quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_fill_event_internal(
                    MARKET_ID_COIN,
                    size_taker,
                    price,
                    side_maker,
                    @user_0,
                    NO_CUSTODIAN,
                    market_order_id_0,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base: base_deposit_taker,
                    min_quote,
                    max_quote: HI_64 - quote_deposit_taker,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    taker_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::value<BC>(&base_coins)  == base_total_taker, 0);
        assert!(coin::value<QC>(&quote_coins) == quote_total_taker, 0);
        // Burn coins.
        if (base_total_taker == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_total_taker == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
    }

    #[test]
    /// Verify returns, state updates for swap sell for no max possible
    /// quote amount specified, with base amount as limiting factor.
    fun test_swap_coins_sell_no_max_base_limiting()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = SELL;
        let side_maker          = BID; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = 0;
        let min_quote           = 0;
        let max_quote           = quote_trade * TICK_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = HI_64 - base_maker;
        let quote_deposit_maker = quote_maker;
        let base_deposit_taker  = base_taker;
        let quote_deposit_taker = HI_64 - max_quote;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker + base_taker;
        let base_available_maker  = base_total_maker;
        let base_ceiling_maker    = HI_64;
        let quote_total_maker     = quote_deposit_maker - quote_match;
        let quote_available_maker = 0;
        let quote_ceiling_maker   = quote_total_maker;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = 0;
        let quote_total_taker = quote_deposit_taker + quote_trade;
        // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Create taker coins.
        let base_coins  = assets::mint_test<BC>(base_deposit_taker);
        let quote_coins = assets::mint_test<QC>(quote_deposit_taker);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_0), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        let (base_coins, quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_coins<BC, QC>( // Place taker order.
                MARKET_ID_COIN, @integrator, direction, min_base, max_base,
                min_quote, max_quote, price, base_coins, quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_fill_event_internal(
                    MARKET_ID_COIN,
                    size_taker,
                    price,
                    side_maker,
                    @user_0,
                    NO_CUSTODIAN,
                    market_order_id_0,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base: base_deposit_taker,
                    min_quote,
                    max_quote: max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[], 0);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::value<BC>(&base_coins)  == base_total_taker, 0);
        assert!(coin::value<QC>(&quote_coins) == quote_total_taker, 0);
        // Burn coins.
        if (base_total_taker == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_total_taker == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
    }

    #[test]
    /// Verify returns, state updates for swap sell for no max possible
    /// quote amount specified, with quote amount as limiting factor.
    fun test_swap_coins_sell_no_max_quote_limiting()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = SELL;
        let side_maker          = BID; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_COIN;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_COIN;
        let base_taker          = size_taker * LOT_SIZE_COIN;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_COIN;
        let quote_match         = size_taker * price * TICK_SIZE_COIN;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = 0;
        let min_quote           = 0;
        let max_quote           = quote_trade;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = HI_64 - base_maker;
        let quote_deposit_maker = quote_maker;
        let base_deposit_taker  = base_taker * LOT_SIZE_COIN;
        let quote_deposit_taker = HI_64 - max_quote;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker + base_taker;
        let base_available_maker  = base_total_maker;
        let base_ceiling_maker    = HI_64;
        let quote_total_maker     = quote_deposit_maker - quote_match;
        let quote_available_maker = 0;
        let quote_ceiling_maker   = quote_total_maker;
        // Declare expected asset amounts after the match, for taker.
        let base_total_taker  = base_deposit_taker - base_taker;
        let quote_total_taker = HI_64; // Deposit maker coins.
        user::deposit_coins<BC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(base_deposit_maker));
        user::deposit_coins<QC>(@user_0, MARKET_ID_COIN, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Create taker coins.
        let base_coins  = assets::mint_test<BC>(base_deposit_taker);
        let quote_coins = assets::mint_test<QC>(quote_deposit_taker);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists<SwapperEventHandles>(@user_0), 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<BC, QC>(
            &user_0, MARKET_ID_COIN, @integrator, side_maker, size_maker,
            price, NO_RESTRICTION, self_match_behavior);
        let (base_coins, quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_coins<BC, QC>( // Place taker order.
                MARKET_ID_COIN, @integrator, direction, min_base, max_base,
                min_quote, max_quote, price, base_coins, quote_coins);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_COIN,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[
                user::create_fill_event_internal(
                    MARKET_ID_COIN,
                    size_taker,
                    price,
                    side_maker,
                    @user_0,
                    NO_CUSTODIAN,
                    market_order_id_0,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_COIN, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_COIN) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_COIN,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base: base_deposit_taker,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_COIN) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_COIN,
                    taker_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_COIN, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_COIN, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amounts.
        assert!(user::get_collateral_value_simple_test<BC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == base_total_maker, 0);
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_COIN, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::value<BC>(&base_coins)  == base_total_taker, 0);
        assert!(coin::value<QC>(&quote_coins) == quote_total_taker, 0);
        // Burn coins.
        if (base_total_taker == 0) coin::destroy_zero(base_coins) else
            assets::burn(base_coins);
        if (quote_total_taker == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
    }

    #[test]
    /// Verify returns, state updates for swap buy with base amount as
    /// limiting factor.
    fun test_swap_generic_buy_base_limiting()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = BUY;
        let side_maker          = ASK; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_GENERIC;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_GENERIC;
        let base_taker          = size_taker * LOT_SIZE_GENERIC;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_GENERIC;
        let quote_match         = size_taker * price * TICK_SIZE_GENERIC;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = base_taker;
        let min_quote           = 0;
        let max_quote           = 0;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = base_maker;
        let quote_deposit_maker = HI_64 - quote_maker;
        let quote_deposit_taker = quote_trade * TICK_SIZE_GENERIC;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker - base_taker;
        let base_available_maker  = 0;
        let base_ceiling_maker    = base_total_maker;
        let quote_total_maker     = quote_deposit_maker + quote_match;
        let quote_available_maker = quote_total_maker;
        let quote_ceiling_maker   = HI_64;
        // Declare expected asset amounts after the match, for taker.
        let quote_total_taker = quote_deposit_taker - quote_trade;
        let underwriter_capability = registry::get_underwriter_capability_test(
            UNDERWRITER_ID); // Get underwriter capability.
        // Deposit maker assets.
        user::deposit_generic_asset(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN, base_deposit_maker,
            &underwriter_capability);
        user::deposit_coins<QC>(@user_0, MARKET_ID_GENERIC, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Create taker coins.
        let quote_coins = assets::mint_test<QC>(quote_deposit_taker);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<
            GenericAsset, QC>(&user_0, MARKET_ID_GENERIC, @integrator,
            side_maker, size_maker, price, NO_RESTRICTION,
            self_match_behavior);
        let (quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_generic<QC>( // Place taker order.
                MARKET_ID_GENERIC, @integrator, direction, min_base, max_base,
                min_quote, max_quote, price, quote_coins,
                &underwriter_capability);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_GENERIC,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[
                user::create_fill_event_internal(
                    MARKET_ID_GENERIC,
                    size_taker,
                    price,
                    side_maker,
                    @user_0,
                    NO_CUSTODIAN,
                    market_order_id_0,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_GENERIC) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_GENERIC,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote: quote_deposit_taker,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_GENERIC) == vector[], 0);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_GENERIC, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amount.
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::value<QC>(&quote_coins) == quote_total_taker, 0);
        // Burn coins.
        if (quote_total_taker == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
    }

    #[test]
    /// Verify returns, state updates for swap buy with quote amount as
    /// limiting factor.
    fun test_swap_generic_buy_quote_limiting()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = BUY;
        let side_maker          = ASK; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_GENERIC;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_GENERIC;
        let base_taker          = size_taker * LOT_SIZE_GENERIC;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_GENERIC;
        let quote_match         = size_taker * price * TICK_SIZE_GENERIC;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = base_taker * LOT_SIZE_GENERIC;
        let min_quote           = 0;
        let max_quote           = 0;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = base_maker;
        let quote_deposit_maker = HI_64 - quote_maker;
        let quote_deposit_taker = quote_trade;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker - base_taker;
        let base_available_maker  = 0;
        let base_ceiling_maker    = base_total_maker;
        let quote_total_maker     = quote_deposit_maker + quote_match;
        let quote_available_maker = quote_total_maker;
        let quote_ceiling_maker   = HI_64;
        // Declare expected asset amounts after the match, for taker.
        let quote_total_taker = 0;
        let underwriter_capability = registry::get_underwriter_capability_test(
            UNDERWRITER_ID); // Get underwriter capability.
        // Deposit maker assets.
        user::deposit_generic_asset(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN, base_deposit_maker,
            &underwriter_capability);
        user::deposit_coins<QC>(@user_0, MARKET_ID_GENERIC, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Create taker coins.
        let quote_coins = assets::mint_test<QC>(quote_deposit_taker);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<
            GenericAsset, QC>(&user_0, MARKET_ID_GENERIC, @integrator,
            side_maker, size_maker, price, NO_RESTRICTION,
            self_match_behavior);
        let (quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_generic<QC>( // Place taker order.
                MARKET_ID_GENERIC, @integrator, direction, min_base, max_base,
                min_quote, max_quote, price, quote_coins,
                &underwriter_capability);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_GENERIC,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[
                user::create_fill_event_internal(
                    MARKET_ID_GENERIC,
                    size_taker,
                    price,
                    side_maker,
                    @user_0,
                    NO_CUSTODIAN,
                    market_order_id_0,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_GENERIC) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_GENERIC,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote: quote_deposit_taker,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_GENERIC) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_GENERIC,
                    taker_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_GENERIC, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amount.
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::value<QC>(&quote_coins) == quote_total_taker, 0);
        // Burn coins.
        if (quote_total_taker == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
    }

    #[test]
    /// Verify returns, state updates for swap sell with max possible
    /// quote flag specified, with quote amount as limiting factor.
    fun test_swap_generic_sell_max_quote_limiting()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = SELL;
        let side_maker          = BID; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_GENERIC;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_GENERIC;
        let base_taker          = size_taker * LOT_SIZE_GENERIC;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_GENERIC;
        let quote_match         = size_taker * price * TICK_SIZE_GENERIC;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = base_taker * LOT_SIZE_GENERIC;
        let min_quote           = 0;
        let max_quote           = MAX_POSSIBLE;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = HI_64 - base_maker;
        let quote_deposit_maker = quote_maker;
        let quote_deposit_taker = HI_64 - quote_trade;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker + base_taker;
        let base_available_maker  = base_total_maker;
        let base_ceiling_maker    = HI_64;
        let quote_total_maker     = quote_deposit_maker - quote_match;
        let quote_available_maker = 0;
        let quote_ceiling_maker   = quote_total_maker;
        // Declare expected asset amounts after the match, for taker.
        let quote_total_taker = HI_64;
        let underwriter_capability = registry::get_underwriter_capability_test(
            UNDERWRITER_ID); // Get underwriter capability.
        // Deposit maker assets.
        user::deposit_generic_asset(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN, base_deposit_maker,
            &underwriter_capability);
        user::deposit_coins<QC>(@user_0, MARKET_ID_GENERIC, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Create taker coins.
        let quote_coins = assets::mint_test<QC>(quote_deposit_taker);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<
            GenericAsset, QC>(&user_0, MARKET_ID_GENERIC, @integrator,
            side_maker, size_maker, price, NO_RESTRICTION,
            self_match_behavior);
        let (quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_generic<QC>( // Place taker order.
                MARKET_ID_GENERIC, @integrator, direction, min_base, max_base,
                min_quote, max_quote, price, quote_coins,
                &underwriter_capability);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_GENERIC,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[
                user::create_fill_event_internal(
                    MARKET_ID_GENERIC,
                    size_taker,
                    price,
                    side_maker,
                    @user_0,
                    NO_CUSTODIAN,
                    market_order_id_0,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_GENERIC) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_GENERIC,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote: HI_64 - quote_deposit_taker,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_GENERIC) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_GENERIC,
                    taker_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_GENERIC, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amount.
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::value<QC>(&quote_coins) == quote_total_taker, 0);
        // Burn coins.
        if (quote_total_taker == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
    }

    #[test]
    /// Verify returns, state updates for swap sell without max possible
    /// quote flag specified, with base amount as limiting factor.
    fun test_swap_generic_sell_no_max_base_limiting()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = SELL;
        let side_maker          = BID; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_GENERIC;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_GENERIC;
        let base_taker          = size_taker * LOT_SIZE_GENERIC;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_GENERIC;
        let quote_match         = size_taker * price * TICK_SIZE_GENERIC;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = base_taker;
        let min_quote           = 0;
        let max_quote           = quote_trade * TICK_SIZE_GENERIC;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = HI_64 - base_maker;
        let quote_deposit_maker = quote_maker;
        let quote_deposit_taker = HI_64 - max_quote;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker + base_taker;
        let base_available_maker  = base_total_maker;
        let base_ceiling_maker    = HI_64;
        let quote_total_maker     = quote_deposit_maker - quote_match;
        let quote_available_maker = 0;
        let quote_ceiling_maker   = quote_total_maker;
        // Declare expected asset amounts after the match, for taker.
        let quote_total_taker = quote_deposit_taker + quote_trade;
        let underwriter_capability = registry::get_underwriter_capability_test(
            UNDERWRITER_ID); // Get underwriter capability.
        // Deposit maker assets.
        user::deposit_generic_asset(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN, base_deposit_maker,
            &underwriter_capability);
        user::deposit_coins<QC>(@user_0, MARKET_ID_GENERIC, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Create taker coins.
        let quote_coins = assets::mint_test<QC>(quote_deposit_taker);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<
            GenericAsset, QC>(&user_0, MARKET_ID_GENERIC, @integrator,
            side_maker, size_maker, price, NO_RESTRICTION,
            self_match_behavior);
        let (quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_generic<QC>( // Place taker order.
                MARKET_ID_GENERIC, @integrator, direction, min_base, max_base,
                min_quote, max_quote, price, quote_coins,
                &underwriter_capability);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_GENERIC,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[
                user::create_fill_event_internal(
                    MARKET_ID_GENERIC,
                    size_taker,
                    price,
                    side_maker,
                    @user_0,
                    NO_CUSTODIAN,
                    market_order_id_0,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_GENERIC) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_GENERIC,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_GENERIC) == vector[], 0);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_GENERIC, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amount.
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::value<QC>(&quote_coins) == quote_total_taker, 0);
        // Burn coins.
        if (quote_total_taker == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
    }

    #[test]
    /// Verify returns, state updates for swap sell without max possible
    /// quote flag specified, with quote amount as limiting factor.
    fun test_swap_generic_sell_no_max_quote_limiting()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        let (user_0, _) = init_markets_users_integrator_test();
        // Get taker fee divisor.
        let taker_divisor = incentives::get_taker_fee_divisor();
        // Declare order setup parameters, with price set to taker fee
        // divisor, to prevent truncation effects on estimates.
        let direction           = SELL;
        let side_maker          = BID; // If buy then ask, else bid.
        let size_maker          = MIN_SIZE_GENERIC;
        let size_taker          = 1;
        let base_maker          = size_maker * LOT_SIZE_GENERIC;
        let base_taker          = size_taker * LOT_SIZE_GENERIC;
        let price               = taker_divisor;
        let quote_maker         = size_maker * price * TICK_SIZE_GENERIC;
        let quote_match         = size_taker * price * TICK_SIZE_GENERIC;
        let fee                 = quote_match / taker_divisor;
        let quote_trade         = if (direction == BUY) quote_match + fee else
                                                        quote_match - fee;
        let min_base            = 0;
        let max_base            = base_taker * LOT_SIZE_GENERIC;
        let min_quote           = 0;
        let max_quote           = quote_trade;
        let self_match_behavior = ABORT;
        // Declare deposit amounts so as to impinge on available/ceiling
        // boundaries.
        let base_deposit_maker  = HI_64 - base_maker;
        let quote_deposit_maker = quote_maker;
        let quote_deposit_taker = HI_64 - max_quote;
        // Declare expected asset amounts after the match, for maker.
        let base_total_maker      = base_deposit_maker + base_taker;
        let base_available_maker  = base_total_maker;
        let base_ceiling_maker    = HI_64;
        let quote_total_maker     = quote_deposit_maker - quote_match;
        let quote_available_maker = 0;
        let quote_ceiling_maker   = quote_total_maker;
        // Declare expected asset amounts after the match, for taker.
        let quote_total_taker = HI_64;
        let underwriter_capability = registry::get_underwriter_capability_test(
            UNDERWRITER_ID); // Get underwriter capability.
        // Deposit maker assets.
        user::deposit_generic_asset(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN, base_deposit_maker,
            &underwriter_capability);
        user::deposit_coins<QC>(@user_0, MARKET_ID_GENERIC, NO_CUSTODIAN,
                                assets::mint_test(quote_deposit_maker));
        // Create taker coins.
        let quote_coins = assets::mint_test<QC>(quote_deposit_taker);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(!exists_market_event_handles(), 0);
        // Place maker order.
        let (market_order_id_0, _, _, _) = place_limit_order_user<
            GenericAsset, QC>(&user_0, MARKET_ID_GENERIC, @integrator,
            side_maker, size_maker, price, NO_RESTRICTION,
            self_match_behavior);
        let (quote_coins, base_trade_r, quote_trade_r, fee_r) =
            swap_generic<QC>( // Place taker order.
                MARKET_ID_GENERIC, @integrator, direction, min_base, max_base,
                min_quote, max_quote, price, quote_coins,
                &underwriter_capability);
        let taker_order_id = order_id_no_post(2);
        // Assert events.
        assert!(user::get_place_limit_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[
                user::create_place_limit_order_event_test(
                    MARKET_ID_GENERIC,
                    @user_0,
                    NO_CUSTODIAN,
                    @integrator,
                    side_maker,
                    size_maker,
                    price,
                    NO_RESTRICTION,
                    self_match_behavior,
                    size_maker,
                    market_order_id_0
                )
            ], 0);
        assert!(user::get_fill_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[
                user::create_fill_event_internal(
                    MARKET_ID_GENERIC,
                    size_taker,
                    price,
                    side_maker,
                    @user_0,
                    NO_CUSTODIAN,
                    market_order_id_0,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    taker_order_id,
                    fee,
                    0
                )
            ], 0);
        assert!(user::get_cancel_order_events_test(
            MARKET_ID_GENERIC, @user_0, NO_CUSTODIAN) == vector[], 0);
        assert!(get_place_swap_order_events_market_test(
            MARKET_ID_GENERIC) == vector[
                PlaceSwapOrderEvent{
                    market_id: MARKET_ID_GENERIC,
                    signing_account: NO_TAKER_ADDRESS,
                    integrator: @integrator,
                    direction,
                    min_base,
                    max_base,
                    min_quote,
                    max_quote,
                    limit_price: price,
                    order_id: taker_order_id
                }
            ], 0);
        assert!(get_cancel_order_events_market_test(
            MARKET_ID_GENERIC) == vector[
                user::create_cancel_order_event_internal(
                    MARKET_ID_GENERIC,
                    taker_order_id,
                    NO_TAKER_ADDRESS,
                    NO_CUSTODIAN,
                    CANCEL_REASON_MAX_QUOTE_TRADED
                )
            ], 0);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
        // Assert returns.
        assert!(base_trade_r  == base_taker, 0);
        assert!(quote_trade_r == quote_trade, 0);
        assert!(fee_r         == fee, 0);
        // Get fields for maker order on book.
        let (size_r, price_r, user_r, custodian_id_r, order_access_key) =
            get_order_fields_test(
                MARKET_ID_GENERIC, side_maker, market_order_id_0);
        // Assert field returns except access key, used for user lookup.
        assert!(size_r         == size_maker - size_taker, 0);
        assert!(price_r        == price, 0);
        assert!(user_r         == @user_0, 0);
        assert!(custodian_id_r == NO_CUSTODIAN, 0);
        // Assert user-side order fields.
        let (market_order_id_r, size_r) = user::get_order_fields_simple_test(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN, side_maker,
            order_access_key);
        assert!(market_order_id_r == market_order_id_0, 0);
        assert!(size_r            == size_maker - size_taker, 0);
        // Assert maker's asset counts.
        let (base_total , base_available , base_ceiling,
             quote_total, quote_available, quote_ceiling) =
            user::get_asset_counts_internal(
                @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN);
        assert!(base_total      == base_total_maker, 0);
        assert!(base_available  == base_available_maker, 0);
        assert!(base_ceiling    == base_ceiling_maker, 0);
        assert!(quote_total     == quote_total_maker, 0);
        assert!(quote_available == quote_available_maker, 0);
        assert!(quote_ceiling   == quote_ceiling_maker, 0);
        // Assert collateral amount.
        assert!(user::get_collateral_value_simple_test<QC>(
            @user_0, MARKET_ID_GENERIC, NO_CUSTODIAN) == quote_total_maker, 0);
        // Assert taker's asset counts.
        assert!(coin::value<QC>(&quote_coins) == quote_total_taker, 0);
        // Burn coins.
        if (quote_total_taker == 0) coin::destroy_zero(quote_coins) else
            assets::burn(quote_coins);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_BASE)]
    /// Verify failure for invalid base type.
    fun test_swap_invalid_base()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Define swap parameters.
        let market_id = MARKET_ID_COIN;
        let integrator = @integrator;
        let direction = BUY;
        let min_base = 0;
        let max_base = LOT_SIZE_COIN;
        let min_quote = 0;
        let max_quote = TICK_SIZE_COIN;
        let limit_price = 1;
        let base_coins = coin::zero();
        let quote_coins = assets::mint_test(max_quote);
        // Attempt invalid invocation.
        let (base_coins, quote_coins, _, _, _) = swap_coins<QC, QC>(
            market_id, integrator, direction, min_base, max_base, min_quote,
            max_quote, limit_price, base_coins, quote_coins);
        // Burn coins.
        assets::burn(base_coins);
        assets::burn(quote_coins);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ID)]
    /// Verify failure for invalid market ID.
    fun test_swap_invalid_market_id()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Define swap parameters.
        let market_id = HI_64;
        let integrator = @integrator;
        let direction = BUY;
        let min_base = 0;
        let max_base = LOT_SIZE_COIN;
        let min_quote = 0;
        let max_quote = TICK_SIZE_COIN;
        let limit_price = 1;
        let base_coins = coin::zero();
        let quote_coins = assets::mint_test(max_quote);
        // Attempt invalid invocation.
        let (base_coins, quote_coins, _, _, _) = swap_coins<BC, QC>(
            market_id, integrator, direction, min_base, max_base, min_quote,
            max_quote, limit_price, base_coins, quote_coins);
        // Burn coins.
        assets::burn(base_coins);
        assets::burn(quote_coins);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_QUOTE)]
    /// Verify failure for invalid quote type.
    fun test_swap_invalid_quote()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Define swap parameters.
        let market_id = MARKET_ID_COIN;
        let integrator = @integrator;
        let direction = BUY;
        let min_base = 0;
        let max_base = LOT_SIZE_COIN;
        let min_quote = 0;
        let max_quote = TICK_SIZE_COIN;
        let limit_price = 1;
        let base_coins = coin::zero();
        let quote_coins = assets::mint_test(max_quote);
        // Attempt invalid invocation.
        let (base_coins, quote_coins, _, _, _) = swap_coins<BC, BC>(
            market_id, integrator, direction, min_base, max_base, min_quote,
            max_quote, limit_price, base_coins, quote_coins);
        // Burn coins.
        assets::burn(base_coins);
        assets::burn(quote_coins);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_UNDERWRITER)]
    /// Verify failure for invalid underwriter.
    fun test_swap_invalid_underwriter()
    acquires
        MarketEventHandles,
        OrderBooks
    {
        // Initialize markets, users, and an integrator.
        init_markets_users_integrator_test();
        // Define swap parameters.
        let market_id = MARKET_ID_GENERIC;
        let integrator = @integrator;
        let direction = BUY;
        let min_base = 0;
        let max_base = LOT_SIZE_GENERIC;
        let min_quote = 0;
        let max_quote = TICK_SIZE_GENERIC;
        let limit_price = 1;
        let quote_coins = assets::mint_test(max_quote);
        let underwriter_capability = registry::get_underwriter_capability_test(
                HI_64); // Get invalid market underwriter capability.
        // Attempt invalid invocation.
        let (quote_coins, _, _, _) = swap_generic<QC>(
            market_id, integrator, direction, min_base, max_base, min_quote,
            max_quote, limit_price, quote_coins, &underwriter_capability);
        // Burn coins.
        assets::burn(quote_coins);
        // Drop underwriter capability.
        registry::drop_underwriter_capability_test(underwriter_capability);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ORDER_ID)]
    /// Verify failure for no such ask for market.
    fun test_verify_pagination_order_ids_ask_does_not_exist()
    acquires OrderBooks {
        init_markets_users_integrator_test(); // Initialize for testing.
        // Attempt invalid lookup.
        verify_pagination_order_ids(MARKET_ID_COIN, 1, (NIL as u128));
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ORDER_ID)]
    /// Verify failure for ask order ID that is actually a bid.
    fun test_verify_pagination_order_ids_ask_wrong_side()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = BID;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size                = MIN_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 / 2;
        let deposit_quote = HI_64 / 2;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size, price, restriction,
            self_match_behavior);
        verify_pagination_order_ids(
            MARKET_ID_COIN, market_order_id, (NIL as u128));
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ORDER_ID)]
    /// Verify failure for no such ask for market.
    fun test_verify_pagination_order_ids_bid_does_not_exist()
    acquires OrderBooks {
        init_markets_users_integrator_test(); // Initialize for testing.
        verify_pagination_order_ids(MARKET_ID_COIN, (NIL as u128), 1);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ORDER_ID)]
    /// Verify failure for bid order ID that is actually an ask.
    fun test_verify_pagination_order_ids_bid_wrong_side()
    acquires OrderBooks {
        // Initialize markets, users, and an integrator.
        let (maker, _) = init_markets_users_integrator_test();
        // Declare order parameters.
        let side                = ASK;
        let market_id           = MARKET_ID_COIN;
        let integrator          = @integrator;
        let custodian_id        = NO_CUSTODIAN;
        let maker_address       = address_of(&maker);
        let restriction         = NO_RESTRICTION;
        let price               = 10;
        let size                = MIN_SIZE_COIN;
        let self_match_behavior = ABORT;
        // Declare maker deposit amounts.
        let deposit_base  = HI_64 / 2;
        let deposit_quote = HI_64 / 2;
        // Deposit maker coins.
        user::deposit_coins<BC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_base));
        user::deposit_coins<QC>(maker_address, market_id, custodian_id,
                                assets::mint_test(deposit_quote));
        // Place maker order, storing market order ID for lookup.
        let (market_order_id, _, _, _) = place_limit_order_user<BC, QC>(
            &maker, market_id, integrator, side, size, price, restriction,
            self_match_behavior);
        verify_pagination_order_ids(
            MARKET_ID_COIN, (NIL as u128), market_order_id);
    }

    // Tests <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

}
