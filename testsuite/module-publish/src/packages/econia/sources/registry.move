/// Market and capability registration operations.
///
/// Econia relies on a global market registry, which supports
/// permissionless registration of markets, as well as capabilities.
/// Custodian capabilities are required to approve order operations and
/// withdrawals, while underwriter capabilities are required to approve
/// generic asset amounts.
///
/// The registry is paired with a recognized market list that tabulates
/// a recognized market for select trading pairs. The recognized market
/// list can only be managed by the Econia account, and provides a set
/// of public APIs that allow lookup of an official market based on a
/// trading pair.
///
/// Custodian capabilities and underwriter capabilities are 1-indexed,
/// with an ID of 0 reserved as a flag for null. For consistency, market
/// IDs are 1-indexed too.
///
/// # General overview sections
///
/// [View functions](#view-functions)
///
/// * [Constant getters](#constant-getters)
/// * [Market lookup](#market-lookup)
///
/// [Public function index](#public-function-index)
///
/// * [Capability management](#capability-management)
/// * [Integrator fee store setup](#integrator-fee-store-setup)
/// * [Recognized market lookup](#recognized-market-lookup)
/// * [Recognized market management](#recognized-market-management)
///
/// [Dependency charts](#dependency-charts)
///
/// * [Capability registration](#capability-registration)
/// * [Fee store registration](#fee-store-registration)
/// * [Market getters](#market-getters)
/// * [Recognized market setters](#recognized-market-setters)
/// * [Internal market registration](#internal-market-registration)
///
/// [Complete DocGen index](#complete-docgen-index)
///
/// # View functions
///
/// ## Constant getters
///
/// * `get_MAX_CHARACTERS_GENERIC()`
/// * `get_MIN_CHARACTERS_GENERIC()`
/// * `get_NO_CUSTODIAN()`
/// * `get_NO_UNDERWRITER()`
///
/// ## Market lookup
///
/// * `get_market_counts()`
/// * `get_market_info()`
/// * `get_market_id_base_coin()`
/// * `get_market_id_base_generic()`
/// * `get_recognized_market_id_base_coin()`
/// * `get_recognized_market_id_base_generic()`
/// * `has_recognized_market_base_coin_by_type()`
/// * `has_recognized_market_base_generic_by_type()`
///
/// # Public function index
///
/// ## Capability management
///
/// * `get_custodian_id()`
/// * `get_underwriter_id()`
/// * `register_custodian_capability()`
/// * `register_underwriter_capability()`
///
/// ## Integrator fee store setup
///
/// * `register_integrator_fee_store()`
/// * `register_integrator_fee_store_base_tier()`
/// * `register_integrator_fee_store_from_coinstore()`
///
/// ## Recognized market lookup
///
/// * `get_recognized_market_info_base_coin()`
/// * `get_recognized_market_info_base_coin_by_type()`
/// * `get_recognized_market_info_base_generic()`
/// * `get_recognized_market_info_base_generic_by_type()`
/// * `has_recognized_market_base_coin()`
/// * `has_recognized_market_base_generic()`
///
/// ## Recognized market management
///
/// * `remove_recognized_market()`
/// * `remove_recognized_markets()`
/// * `set_recognized_market()`
/// * `set_recognized_markets()`
///
/// (These are public entry functions.)
///
/// # Dependency charts
///
/// The below dependency charts use `mermaid.js` syntax, which can be
/// automatically rendered into a diagram (depending on the browser)
/// when viewing the documentation file generated from source code. If
/// a browser renders the diagrams with coloring that makes it difficult
/// to read, try a different browser.
///
/// ## Capability registration
///
/// ```mermaid
///
/// flowchart LR
///
/// register_custodian_capability -->
///     incentives::deposit_custodian_registration_utility_coins
/// register_underwriter_capability -->
///     incentives::deposit_underwriter_registration_utility_coins
///
/// ```
///
/// ## Fee store registration
///
/// ```mermaid
///
/// flowchart LR
///
/// register_integrator_fee_store_base_tier -->
///     register_integrator_fee_store
/// register_integrator_fee_store_from_coinstore -->
///     register_integrator_fee_store
///
/// ```
///
/// ## Market getters
///
/// ```mermaid
///
/// flowchart LR
///
/// get_recognized_market_info_base_coin -->
///     get_recognized_market_info
/// get_recognized_market_info_base_coin_by_type -->
///     get_recognized_market_info_base_coin
/// get_recognized_market_info_base_generic -->
///     get_recognized_market_info
/// get_recognized_market_info_base_generic_by_type  -->
///     get_recognized_market_info_base_generic
///
/// has_recognized_market_base_coin --> has_recognized_market
/// has_recognized_market_base_coin_by_type -->
///     has_recognized_market_base_coin
/// has_recognized_market_base_generic --> has_recognized_market
/// has_recognized_market_base_generic_by_type -->
///     has_recognized_market_base_generic
///
/// get_recognized_market_id_base_coin -->
///     get_recognized_market_info_base_coin_by_type
///
/// get_recognized_market_id_base_generic -->
///     get_recognized_market_info_base_generic_by_type
///
/// get_market_info --> has_recognized_market
/// get_market_info --> get_recognized_market_info
///
/// get_market_id_base_coin --> get_market_id
/// get_market_id_base_generic --> get_market_id
///
/// ```
///
/// ## Recognized market setters
///
/// ```mermaid
///
/// flowchart LR
///
/// remove_recognized_markets --> remove_recognized_market
/// set_recognized_markets --> set_recognized_market
///
/// ```
///
/// ## Internal market registration
///
/// ```mermaid
///
/// flowchart LR
///
/// register_market_base_coin_internal --> register_market_internal
/// register_market_base_generic_internal --> register_market_internal
///
/// register_market_internal -->
///     incentives::deposit_market_registration_utility_coins
///
/// ```
///
/// # Complete DocGen index
///
/// The below index is automatically generated from source code:
module econia::registry {

    // Uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::table::{Self, Table};
    use aptos_framework::type_info::{Self, TypeInfo};
    use econia::incentives;
    use econia::tablist::{Self, Tablist};
    use std::option::{Self, Option};
    use std::signer::address_of;
    use std::string::{Self, String};
    use std::vector;

    // Uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Friends >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    friend econia::user;
    friend econia::market;

    // Friends <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    use econia::assets::{Self, BC, QC, UC};

    // Test-only uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Structs >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Human-readable asset type descriptor for view functions.
    struct AssetTypeView has copy, drop {
        /// Address of package containing type definition.
        package_address: address,
        /// Module name where type is defined.
        module_name: String,
        /// Struct type name, either a phantom `CoinType` or
        /// `GenericAsset`
        type_name: String
    }

    /// Custodian capability required to approve order operations and
    /// withdrawals. Administered to third-party registrants who may
    /// store it as they wish.
    struct CustodianCapability has store {
        /// Serial ID, 1-indexed, generated upon registration as a
        /// custodian.
        custodian_id: u64
    }

    /// Type flag for generic asset. Must be passed as base asset type
    /// argument for generic market operations. Has key ability to
    /// restrict unexpected malicious attack vectors.
    struct GenericAsset has key {}

    /// View function return specifying number of markets and recognized
    /// markets that have been registered.
    struct MarketCounts has copy, drop {
        /// Number of markets.
        n_markets: u64,
        /// Number of recognized markets.
        n_recognized_markets: u64
    }

    /// Information about a market.
    struct MarketInfo has copy, drop, store {
        /// Base asset type info. When base asset is an
        /// `aptos_framework::coin::Coin`, corresponds to the phantom
        /// `CoinType` (`address:module::MyCoin` rather than
        /// `aptos_framework::coin::Coin<address:module::MyCoin>`).
        /// Otherwise should be `GenericAsset`.
        base_type: TypeInfo,
        /// Custom base asset name for a generic market, provided by the
        /// underwriter who registers the market. Empty if a pure coin
        /// market.
        base_name_generic: String,
        /// Quote asset coin type info. Corresponds to a phantom
        /// `CoinType` (`address:module::MyCoin` rather than
        /// `aptos_framework::coin::Coin<address:module::MyCoin>`).
        quote_type: TypeInfo,
        /// Number of base units exchanged per lot (when base asset is
        /// a coin, corresponds to `aptos_framework::coin::Coin.value`).
        lot_size: u64,
        /// Number of quote coin units exchanged per tick (corresponds
        /// to `aptos_framework::coin::Coin.value`).
        tick_size: u64,
        /// Minimum number of lots per order.
        min_size: u64,
        /// `NO_UNDERWRITER` if a pure coin market, otherwise ID of
        /// underwriter capability required to verify generic asset
        /// amounts. A market-wide ID that only applies to markets
        /// having a generic base asset.
        underwriter_id: u64
    }

    /// Human-readable market info return for view functions.
    struct MarketInfoView has copy, drop {
        /// 1-indexed Market ID.
        market_id: u64,
        /// Marked `true` if market is recognized.
        is_recognized: bool,
        /// `MarketInfo.base_type` as an `AssetTypeView`.
        base_type: AssetTypeView,
        /// `MarketInfo.base_name_generic`.
        base_name_generic: String,
        /// `MarketInfo.quote_type` as an `AssetTypeView`.
        quote_type: AssetTypeView,
        /// `MarketInfo.lot_size`.
        lot_size: u64,
        /// `MarketInfo.tick_size`.
        tick_size: u64,
        /// `MarketInfo.min_size`.
        min_size: u64,
        /// `MarketInfo.underwriter_id`.
        underwriter_id: u64
    }

    /// Emitted when a market is registered.
    struct MarketRegistrationEvent has drop, store {
        /// Market ID of the market just registered.
        market_id: u64,
        /// Base asset type info.
        base_type: TypeInfo,
        /// Base asset generic name, if any.
        base_name_generic: String,
        /// Quote asset type info.
        quote_type: TypeInfo,
        /// Number of base units exchanged per lot.
        lot_size: u64,
        /// Number of quote units exchanged per tick.
        tick_size: u64,
        /// Minimum number of lots per order.
        min_size: u64,
        /// `NO_UNDERWRITER` if a pure coin market, otherwise ID of
        /// underwriter capability required to verify generic asset
        /// amounts.
        underwriter_id: u64,
    }

    /// Emitted when a recognized market is added, removed, or updated.
    struct RecognizedMarketEvent has drop, store {
        /// The associated trading pair.
        trading_pair: TradingPair,
        /// The recognized market info for the given trading pair after
        /// an addition or update. None if a removal.
        recognized_market_info: Option<RecognizedMarketInfo>
    }

    /// Recognized market info for a given trading pair.
    struct RecognizedMarketInfo has copy, drop, store {
        /// Market ID of recognized market, 0-indexed.
        market_id: u64,
        /// Number of base units exchanged per lot.
        lot_size: u64,
        /// Number of quote units exchanged per tick.
        tick_size: u64,
        /// Minimum number of lots per order.
        min_size: u64,
        /// `NO_UNDERWRITER` if a pure coin market, otherwise ID of
        /// underwriter capability required to verify generic asset
        /// amounts.
        underwriter_id: u64
    }

    /// Recognized markets for specific trading pairs.
    struct RecognizedMarkets has key {
        /// Map from trading pair info to market information for the
        /// recognized market, if any, for given trading pair. Enables
        /// off-chain iterated indexing by market ID.
        map: Tablist<TradingPair, RecognizedMarketInfo>,
        /// Event handle for recognized market events.
        recognized_market_events: EventHandle<RecognizedMarketEvent>
    }

    /// Global registration information.
    struct Registry has key {
        /// Map from 1-indexed market ID to corresponding market info,
        /// enabling off-chain iterated indexing by market ID.
        market_id_to_info: Tablist<u64, MarketInfo>,
        /// Map from market info to corresponding 1-indexed market ID,
        /// enabling market duplicate checks.
        market_info_to_id: Table<MarketInfo, u64>,
        /// The number of registered custodians.
        n_custodians: u64,
        /// The number of registered underwriters.
        n_underwriters: u64,
        /// Event handle for market registration events.
        market_registration_events: EventHandle<MarketRegistrationEvent>
    }

    /// A combination of a base asset and a quote asset.
    struct TradingPair has copy, drop, store {
        /// Base asset type info.
        base_type: TypeInfo,
        /// Base asset generic name, if any.
        base_name_generic: String,
        /// Quote asset type info.
        quote_type: TypeInfo
    }

    /// Underwriter capability required to verify generic asset
    /// amounts. Administered to third-party registrants who may store
    /// it as they wish.
    struct UnderwriterCapability has store {
        /// Serial ID, 1-indexed, generated upon registration as an
        /// underwriter.
        underwriter_id: u64
    }

    // Structs <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Error codes >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Lot size specified as 0.
    const E_LOT_SIZE_0: u64 = 0;
    /// Tick size specified as 0.
    const E_TICK_SIZE_0: u64 = 1;
    /// Minimum order size specified as 0.
    const E_MIN_SIZE_0: u64 = 2;
    /// Quote asset type has not been initialized as a coin.
    const E_QUOTE_NOT_COIN: u64 = 3;
    /// Base and quote asset descriptors are identical.
    const E_BASE_QUOTE_SAME: u64 = 4;
    /// Market is already registered.
    const E_MARKET_REGISTERED: u64 = 5;
    /// Base coin type has not been initialized for a pure coin market.
    const E_BASE_NOT_COIN: u64 = 6;
    /// Generic base asset descriptor has too few characters.
    const E_GENERIC_TOO_FEW_CHARACTERS: u64 = 7;
    /// Generic base asset descriptor has too many characters.
    const E_GENERIC_TOO_MANY_CHARACTERS: u64 = 8;
    /// Caller is not Econia, but should be.
    const E_NOT_ECONIA: u64 = 9;
    /// Trading pair does not have recognized market.
    const E_NO_RECOGNIZED_MARKET: u64 = 10;
    /// Market ID is not recognized for corresponding trading pair.
    const E_WRONG_RECOGNIZED_MARKET: u64 = 11;
    /// Market ID is invalid.
    const E_INVALID_MARKET_ID: u64 = 12;
    /// Base asset type is invalid.
    const E_INVALID_BASE: u64 = 13;
    /// Quote asset type is invalid.
    const E_INVALID_QUOTE: u64 = 14;

    // Error codes <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Constants >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Maximum number of characters permitted in a generic asset name,
    /// equal to the maximum number of characters permitted in a comment
    /// line per PEP 8.
    const MAX_CHARACTERS_GENERIC: u64 = 72;
    /// Minimum number of characters permitted in a generic asset name,
    /// equal to the number of spaces in an indentation level per PEP 8.
    const MIN_CHARACTERS_GENERIC: u64 = 4;
    /// Custodian ID flag for no custodian.
    const NO_CUSTODIAN: u64 = 0;
    /// Underwriter ID flag for no underwriter.
    const NO_UNDERWRITER: u64 = 0;

    // Constants <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // View functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[view]
    /// Public constant getter for `MAX_CHARACTERS_GENERIC`.
    ///
    /// # Testing
    ///
    /// * `test_get_MAX_CHARACTERS_GENERIC()`
    public fun get_MAX_CHARACTERS_GENERIC(): u64 {MAX_CHARACTERS_GENERIC}

    #[view]
    /// Public constant getter for `MIN_CHARACTERS_GENERIC`.
    ///
    /// # Testing
    ///
    /// * `test_get_MIN_CHARACTERS_GENERIC()`
    public fun get_MIN_CHARACTERS_GENERIC(): u64 {MIN_CHARACTERS_GENERIC}

    #[view]
    /// Public constant getter for `NO_CUSTODIAN`.
    ///
    /// # Testing
    ///
    /// * `test_get_NO_CUSTODIAN()`
    public fun get_NO_CUSTODIAN(): u64 {NO_CUSTODIAN}

    #[view]
    /// Public constant getter for `NO_UNDERWRITER`.
    ///
    /// # Testing
    ///
    /// * `test_get_NO_UNDERWRITER()`
    public fun get_NO_UNDERWRITER(): u64 {NO_UNDERWRITER}

    #[view]
    /// Return a `MarketCounts` for current registry state.
    ///
    /// Restricted to private view function to prevent runtime
    /// transaction collisions against the registry.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    fun get_market_counts():
    MarketCounts
    acquires
        RecognizedMarkets,
        Registry
    {
        let markets_map_ref = // Immutably borrow markets map.
            &borrow_global<Registry>(@econia).market_id_to_info;
        // Get number of markets.
        let n_markets = tablist::length(markets_map_ref);
        // Immutably borrow recognized markets map.
        let recognized_markets_map_ref =
            &borrow_global<RecognizedMarkets>(@econia).map;
        // Get number of recognized markets.
        let n_recognized_markets = tablist::length(recognized_markets_map_ref);
        // Return market counts.
        MarketCounts{n_markets, n_recognized_markets}
    }

    #[view]
    /// Return optional market ID corresponding to given market
    /// parameters when the base asset is a coin type.
    ///
    /// Restricted to private view function to prevent runtime
    /// transaction collisions against the registry.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    fun get_market_id_base_coin<BaseType, QuoteType>(
        lot_size: u64,
        tick_size: u64,
        min_size: u64,
    ): Option<u64>
    acquires Registry {
        get_market_id(MarketInfo{
            base_type: type_info::type_of<BaseType>(),
            base_name_generic: string::utf8(b""),
            quote_type: type_info::type_of<QuoteType>(),
            lot_size,
            tick_size,
            min_size,
            underwriter_id: NO_UNDERWRITER
        })
    }

    #[view]
    /// Return optional market ID corresponding to given market
    /// parameters when the base asset is generic.
    ///
    /// Restricted to private view function to prevent runtime
    /// transaction collisions against the registry.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    fun get_market_id_base_generic<QuoteType>(
        base_name_generic: String,
        lot_size: u64,
        tick_size: u64,
        min_size: u64,
        underwriter_id: u64
    ): Option<u64>
    acquires Registry {
        get_market_id(MarketInfo{
            base_type: type_info::type_of<GenericAsset>(),
            base_name_generic,
            quote_type: type_info::type_of<QuoteType>(),
            lot_size,
            tick_size,
            min_size,
            underwriter_id
        })
    }

    #[view]
    /// Return a `MarketInfoView` for given `market_id`.
    ///
    /// Restricted to private view function to prevent runtime
    /// transaction collisions against the registry.
    ///
    /// # Testing
    ///
    /// * `test_get_market_info_invalid_market_id()`
    /// * `test_set_remove_check_recognized_markets()`
    fun get_market_info(
        market_id: u64
    ): MarketInfoView
    acquires
        RecognizedMarkets,
        Registry
    {
        let markets_map_ref = // Immutably borrow markets map.
            &borrow_global<Registry>(@econia).market_id_to_info;
        assert!( // Assert market ID corresponds to registered market.
            tablist::contains(markets_map_ref, market_id),
            E_INVALID_MARKET_ID);
        // Immutably borrow market info for market ID.
        let market_info_ref = tablist::borrow(markets_map_ref, market_id);
        // Get base type for market.
        let base_type = market_info_ref.base_type;
        // Get generic base name for market.
        let base_name_generic = market_info_ref.base_name_generic;
        // Get quote type for market.
        let quote_type = market_info_ref.quote_type;
        let trading_pair = // Get trading pair for market.
            TradingPair{base_type, base_name_generic, quote_type};
        // Check if market is recognized. If a recognized market exists
        // for given trading pair:
        let is_recognized = if (has_recognized_market(trading_pair)) {
            // Get recognized market ID for given trading pair.
            let (recognized_market_id_for_trading_pair, _, _, _, _) =
                get_recognized_market_info(trading_pair);
            // Indicated market ID is recognized if it is the same as
            // the recognized market ID for the given trading pair.
            market_id == recognized_market_id_for_trading_pair
        } else { // If no recognized market for given trading pair:
            false // Market is necessarily not recognized.
        };
        // Pack and return a human-readable market info view.
        MarketInfoView{
            market_id,
            is_recognized,
            base_type: to_asset_type_view(&base_type),
            base_name_generic,
            quote_type: to_asset_type_view(&quote_type),
            lot_size: market_info_ref.lot_size,
            tick_size: market_info_ref.tick_size,
            min_size: market_info_ref.min_size,
            underwriter_id: market_info_ref.underwriter_id
        }
    }

    #[view]
    /// Return recognized market ID for a pure coin trading pair.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    public fun get_recognized_market_id_base_coin<
        BaseCoinType,
        QuoteCoinType
    >(): u64
    acquires RecognizedMarkets {
        // Check market info for coin types, dropping all but market ID.
        let (market_id, _, _, _, _) =
            get_recognized_market_info_base_coin_by_type<
                BaseCoinType, QuoteCoinType>();
        market_id // Return resultant market ID
    }

    #[view]
    /// Return recognized market ID for trading pair with generic base
    /// asset.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    public fun get_recognized_market_id_base_generic<
        QuoteCoinType
    >(
        base_name_generic: String
    ): u64
    acquires RecognizedMarkets {
        // Check market info for coin types, dropping all but market ID.
        let (market_id, _, _, _, _) =
            get_recognized_market_info_base_generic_by_type<QuoteCoinType>(
                base_name_generic);
        market_id // Return resultant market ID
    }

    #[view]
    /// Wrapper for `has_recognized_market_base_coin()` with type
    /// parameters.
    ///
    /// # Type parameters
    ///
    /// * `BaseCoinType`: Base asset phantom coin type.
    /// * `QuoteCoinType`: Quote asset phantom coin type.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    public fun has_recognized_market_base_coin_by_type<
        BaseCoinType,
        QuoteCoinType
    >(): bool
    acquires RecognizedMarkets {
        has_recognized_market_base_coin(
            type_info::type_of<BaseCoinType>(),
            type_info::type_of<QuoteCoinType>())
    }

    #[view]
    /// Wrapper for `has_recognized_market_base_generic()` with quote
    /// type parameter.
    ///
    /// # Type parameters
    ///
    /// * `QuoteCoinType`: Quote asset phantom coin type.
    ///
    /// # Parameters
    ///
    /// * `base_name_generic`: Generic base asset name.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    public fun has_recognized_market_base_generic_by_type<
        QuoteCoinType
    >(
        base_name_generic: String
    ): bool
    acquires RecognizedMarkets {
        has_recognized_market_base_generic(
            base_name_generic,
            type_info::type_of<QuoteCoinType>())
    }

    // View functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Return serial ID of given `CustodianCapability`.
    ///
    /// # Testing
    ///
    /// * `test_register_capabilities()`
    public fun get_custodian_id(
        custodian_capability_ref: &CustodianCapability
    ): u64 {
        custodian_capability_ref.custodian_id
    }

    /// Wrapper for `get_recognized_market_info()` for coin base asset.
    ///
    /// # Parameters
    ///
    /// * `base_type`: Base asset phantom coin type info.
    /// * `quote_type`: Quote asset phantom coin type info.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    public fun get_recognized_market_info_base_coin(
        base_type: TypeInfo,
        quote_type: TypeInfo
    ): (
        u64,
        u64,
        u64,
        u64,
        u64
    ) acquires RecognizedMarkets {
        // Get empty generic base asset name.
        let base_name_generic = string::utf8(b"");
        let trading_pair = // Pack trading pair.
            TradingPair{base_type, base_name_generic, quote_type};
        // Get recognized market info.
        get_recognized_market_info(trading_pair)
    }

    /// Wrapper for `get_recognized_market_info_base_coin()` with
    /// type parameters.
    ///
    /// # Type parameters
    ///
    /// * `BaseCoinType`: Base asset phantom coin type.
    /// * `QuoteCoinType`: Quote asset phantom coin type.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    public fun get_recognized_market_info_base_coin_by_type<
        BaseCoinType,
        QuoteCoinType
    >(): (
        u64,
        u64,
        u64,
        u64,
        u64
    ) acquires RecognizedMarkets {
        get_recognized_market_info_base_coin(
            type_info::type_of<BaseCoinType>(),
            type_info::type_of<QuoteCoinType>())
    }

    /// Wrapper for `get_recognized_market_info()` for generic base
    /// asset.
    ///
    /// # Parameters
    ///
    /// * `base_name_generic`: Generic base asset name.
    /// * `quote_type`: Quote asset phantom coin type info.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    public fun get_recognized_market_info_base_generic(
        base_name_generic: String,
        quote_type: TypeInfo
    ): (
        u64,
        u64,
        u64,
        u64,
        u64
    ) acquires RecognizedMarkets {
        // Get generic base asset type info.
        let base_type = type_info::type_of<GenericAsset>();
        let trading_pair = // Pack trading pair.
            TradingPair{base_type, base_name_generic, quote_type};
        // Get recognized market info.
        get_recognized_market_info(trading_pair)
    }

    /// Wrapper for `get_recognized_market_info_base_generic()` with
    /// quote type parameter.
    ///
    /// # Type parameters
    ///
    /// * `QuoteCoinType`: Quote asset phantom coin type.
    ///
    /// # Parameters
    ///
    /// * `base_name_generic`: Generic base asset name.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    public fun get_recognized_market_info_base_generic_by_type<
        QuoteCoinType
    >(
        base_name_generic: String,
    ): (
        u64,
        u64,
        u64,
        u64,
        u64
    ) acquires RecognizedMarkets {
        get_recognized_market_info_base_generic(
            base_name_generic,
            type_info::type_of<QuoteCoinType>())
    }

    /// Return serial ID of given `UnderwriterCapability`.
    ///
    /// # Testing
    ///
    /// * `test_register_capabilities()`
    public fun get_underwriter_id(
        underwriter_capability_ref: &UnderwriterCapability
    ): u64 {
        underwriter_capability_ref.underwriter_id
    }

    /// Wrapper for `has_recognized_market()` for coin base asset.
    ///
    /// # Parameters
    ///
    /// * `base_type`: Base asset phantom coin type info.
    /// * `quote_type`: Quote asset phantom coin type info.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    public fun has_recognized_market_base_coin(
        base_type: TypeInfo,
        quote_type: TypeInfo
    ): bool
    acquires RecognizedMarkets {
        // Get empty generic base asset name.
        let base_name_generic = string::utf8(b"");
        let trading_pair = // Pack trading pair.
            TradingPair{base_type, base_name_generic, quote_type};
        // Check if trading pair has recognized market.
        has_recognized_market(trading_pair)
    }

    /// Wrapper for `has_recognized_market()` for generic base asset.
    ///
    /// # Parameters
    ///
    /// * `base_name_generic`: Generic base asset name.
    /// * `quote_type`: Quote asset phantom coin type info.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    public fun has_recognized_market_base_generic(
        base_name_generic: String,
        quote_type: TypeInfo
    ): bool
    acquires RecognizedMarkets {
        // Get generic base asset type info.
        let base_type = type_info::type_of<GenericAsset>();
        let trading_pair = // Pack trading pair.
            TradingPair{base_type, base_name_generic, quote_type};
        // Check if trading pair has recognized market.
        has_recognized_market(trading_pair)
    }

    /// Return a unique `CustodianCapability`.
    ///
    /// Increment the number of registered custodians, then issue a
    /// capability with the corresponding serial ID. Requires utility
    /// coins to cover the custodian registration fee.
    ///
    /// # Testing
    ///
    /// * `test_register_capabilities()`
    public fun register_custodian_capability<UtilityCoinType>(
        utility_coins: Coin<UtilityCoinType>
    ): CustodianCapability
    acquires Registry {
        // Borrow mutable reference to registry.
        let registry_ref_mut = borrow_global_mut<Registry>(@econia);
        // Set custodian serial ID to the new number of custodians.
        let custodian_id = registry_ref_mut.n_custodians + 1;
        // Update the registry for the new count.
        registry_ref_mut.n_custodians = custodian_id;
        incentives:: // Deposit provided utility coins.
            deposit_custodian_registration_utility_coins(utility_coins);
        // Pack and return corresponding capability.
        CustodianCapability{custodian_id}
    }

    /// Register integrator fee store to given tier on given market.
    ///
    /// # Type parameters
    ///
    /// * `QuoteCoinType`: The quote coin type for market.
    /// * `UtilityCoinType`: The utility coin type.
    ///
    /// # Parameters
    ///
    /// * `integrator`: Integrator account.
    /// * `market_id`: Market ID for corresponding market.
    /// * `tier`: `incentives::IntegratorFeeStore` tier to activate to.
    /// * `utility_coins`: Utility coins paid to activate to given tier.
    ///
    /// # Aborts
    ///
    /// * `E_INVALID_MARKET_ID`: No such registered market ID.
    /// * `E_INVALID_QUOTE`: Invalid quote coin type for market.
    ///
    /// # Testing
    ///
    /// * `test_register_integrator_fee_store_invalid_market_id()`
    /// * `test_register_integrator_fee_store_invalid_quote()`
    /// * `test_register_integrator_fee_stores()`
    public fun register_integrator_fee_store<
        QuoteCoinType,
        UtilityCoinType
    >(
        integrator: &signer,
        market_id: u64,
        tier: u8,
        utility_coins: Coin<UtilityCoinType>
    ) acquires Registry {
        let market_map_ref = // Immutably borrow markets map.
            &borrow_global<Registry>(@econia).market_id_to_info;
        // Assert market ID is registered.
        assert!(tablist::contains(market_map_ref, market_id),
                E_INVALID_MARKET_ID);
        // Immutably borrow market info.
        let market_info_ref = tablist::borrow(market_map_ref, market_id);
        // Assert quote type.
        assert!(market_info_ref.quote_type ==
                type_info::type_of<QuoteCoinType>(), E_INVALID_QUOTE);
        // Register an integrator fee store at integrator's account.
        incentives::register_integrator_fee_store<
            QuoteCoinType, UtilityCoinType>(integrator, market_id, tier,
            utility_coins);
    }

    /// Return a unique `UnderwriterCapability`.
    ///
    /// Increment the number of registered underwriters, then issue a
    /// capability with the corresponding serial ID. Requires utility
    /// coins to cover the underwriter registration fee.
    ///
    /// # Testing
    ///
    /// * `test_register_capabilities()`
    public fun register_underwriter_capability<UtilityCoinType>(
        utility_coins: Coin<UtilityCoinType>
    ): UnderwriterCapability
    acquires Registry {
        // Borrow mutable reference to registry.
        let registry_ref_mut = borrow_global_mut<Registry>(@econia);
        // Set underwriter serial ID to the new number of underwriters.
        let underwriter_id = registry_ref_mut.n_underwriters + 1;
        // Update the registry for the new count.
        registry_ref_mut.n_underwriters = underwriter_id;
        incentives:: // Deposit provided utility coins.
            deposit_underwriter_registration_utility_coins(utility_coins);
        // Pack and return corresponding capability.
        UnderwriterCapability{underwriter_id}
    }

    // Public functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public entry functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Wrapped call to `register_integrator_fee_store()` for activating
    /// to base tier, which does not require utility coins.
    ///
    /// # Testing
    ///
    /// * `test_register_integrator_fee_stores()`
    public entry fun register_integrator_fee_store_base_tier<
        QuoteCoinType,
        UtilityCoinType
    >(
        integrator: &signer,
        market_id: u64,
    ) acquires Registry {
        register_integrator_fee_store<QuoteCoinType, UtilityCoinType>(
            integrator, market_id, 0, coin::zero<UtilityCoinType>());
    }

    /// Wrapped call to `register_integrator_fee_store()` for paying
    /// utility coins from an `aptos_framework::coin::CoinStore`.
    ///
    /// # Testing
    ///
    /// * `test_register_integrator_fee_stores()`
    public entry fun register_integrator_fee_store_from_coinstore<
        QuoteCoinType,
        UtilityCoinType
    >(
        integrator: &signer,
        market_id: u64,
        tier: u8
    ) acquires Registry {
        register_integrator_fee_store<QuoteCoinType, UtilityCoinType>(
            integrator, market_id, tier, coin::withdraw<UtilityCoinType>(
                integrator, incentives::get_tier_activation_fee(tier)));
    }

    /// Remove market having given ID from recognized markets list.
    ///
    /// # Parameters
    ///
    /// * `account`: Econia account.
    /// * `market_id`: Market ID to recognize.
    ///
    /// # Emits
    ///
    /// * `RecognizedMarketEvent`: Info about recognized market for
    ///   given trading pair.
    ///
    /// # Aborts
    ///
    /// * `E_NOT_ECONIA`: `account` is not Econia.
    /// * `E_NO_RECOGNIZED_MARKET`: Market having given ID is not a
    ///   recognized market.
    /// * `E_WRONG_RECOGNIZED_MARKET`: Market ID is not recognized for
    ///   corresponding trading pair.
    ///
    /// # Assumptions
    ///
    /// * `market_id` corresponds to a registered market.
    ///
    /// # Testing
    ///
    /// * `test_remove_recognized_market_no_recognized()`
    /// * `test_remove_recognized_market_not_econia()`
    /// * `test_remove_recognized_market_wrong_market()`
    /// * `test_set_remove_check_recognized_markets()`
    public entry fun remove_recognized_market(
        account: &signer,
        market_id: u64
    ) acquires
        RecognizedMarkets,
        Registry
    {
        // Assert account is Econia.
        assert!(address_of(account) == @econia, E_NOT_ECONIA);
        let markets_map_ref = // Immutably borrow markets map.
            &borrow_global<Registry>(@econia).market_id_to_info;
        // Immutably borrow info for market having given ID.
        let market_info_ref = tablist::borrow(markets_map_ref, market_id);
        let trading_pair = // Pack trading pair from market info.
            TradingPair{base_type: market_info_ref.base_type,
                        base_name_generic: market_info_ref.base_name_generic,
                        quote_type: market_info_ref.quote_type};
        // Mutably borrow recognized markets resource.
        let recognized_markets_ref_mut =
            borrow_global_mut<RecognizedMarkets>(@econia);
        // Mutably borrow recognized markets map.
        let recognized_map_ref_mut = &mut recognized_markets_ref_mut.map;
        assert!( // Assert trading pair has a recognized market.
            tablist::contains(recognized_map_ref_mut, trading_pair),
            E_NO_RECOGNIZED_MARKET);
        // Get recognized market ID for corresponding trading pair.
        let recognized_market_id_for_trading_pair =
            tablist::borrow(recognized_map_ref_mut, trading_pair).market_id;
        // Assert passed market ID matches that of recognized market ID
        // for given trading pair.
        assert!(recognized_market_id_for_trading_pair == market_id,
                E_WRONG_RECOGNIZED_MARKET);
        // Remove entry for given trading pair.
        tablist::remove(recognized_map_ref_mut, trading_pair);
        // Mutably borrow recognized markets events handle.
        let event_handle_ref_mut =
            &mut recognized_markets_ref_mut.recognized_market_events;
        // Emit a recognized market event.
        event::emit_event(event_handle_ref_mut, RecognizedMarketEvent{
            trading_pair, recognized_market_info: option::none()});
    }

    /// Wrapper for `remove_recognized_market()` with market IDs vector.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    public entry fun remove_recognized_markets(
        account: &signer,
        market_ids: vector<u64>
    ) acquires
        RecognizedMarkets,
        Registry
    {
        // Get number of markets to remove.
        let n_markets = vector::length(&market_ids);
        let i = 0; // Declare loop counter.
        while (i < n_markets) { // Loop over all markets in vector:
            // Get market ID to remove.
            let market_id = *vector::borrow(&market_ids, i);
            // Remove as recognized market.
            remove_recognized_market(account, market_id);
            i = i + 1; // Increment loop counter.
        }
    }

    /// Set market having given ID as recognized market.
    ///
    /// # Parameters
    ///
    /// * `account`: Econia account.
    /// * `market_id`: Market ID to recognize.
    ///
    /// # Emits
    ///
    /// * `RecognizedMarketEvent`: Info about recognized market for
    ///   given trading pair.
    ///
    /// # Aborts
    ///
    /// * `E_NOT_ECONIA`: `account` is not Econia.
    ///
    /// # Assumptions
    ///
    /// * `market_id` corresponds to a registered market.
    ///
    /// # Testing
    ///
    /// * `test_set_recognized_market_not_econia()`
    /// * `test_set_recognized_market_update()`
    /// * `test_set_remove_check_recognized_markets()`
    public entry fun set_recognized_market(
        account: &signer,
        market_id: u64
    ) acquires
        RecognizedMarkets,
        Registry
    {
        // Assert account is Econia.
        assert!(address_of(account) == @econia, E_NOT_ECONIA);
        let markets_map_ref = // Immutably borrow markets map.
            &borrow_global<Registry>(@econia).market_id_to_info;
        // Immutably borrow info for market having given ID.
        let market_info_ref = tablist::borrow(markets_map_ref, market_id);
        // Get recognized market info parameters.
        let (base_type, base_name_generic, quote_type, lot_size, tick_size,
             min_size, underwriter_id) =
            (market_info_ref.base_type, market_info_ref.base_name_generic,
             market_info_ref.quote_type, market_info_ref.lot_size,
             market_info_ref.tick_size, market_info_ref.min_size,
             market_info_ref.underwriter_id);
        let trading_pair = // Pack trading pair.
            TradingPair{base_type, base_name_generic, quote_type};
        // Pack recognized market info.
        let recognized_market_info = RecognizedMarketInfo{
            market_id, lot_size, tick_size, min_size, underwriter_id};
        // Mutably borrow recognized markets resource.
        let recognized_markets_ref_mut =
            borrow_global_mut<RecognizedMarkets>(@econia);
        // Mutably borrow recognized markets map.
        let recognized_map_ref_mut = &mut recognized_markets_ref_mut.map;
        let new = // New if trading pair not already recognized.
            !tablist::contains(recognized_map_ref_mut, trading_pair);
        // If new trading pair, add an entry to map.
        if (new) tablist::add(
            recognized_map_ref_mut, trading_pair, recognized_market_info)
        // Otherwise update existing entry.
        else *tablist::borrow_mut(recognized_map_ref_mut, trading_pair) =
                recognized_market_info;
        // Pack market info in an option.
        let optional_market_info = option::some(recognized_market_info);
        // Mutably borrow recognized markets events handle.
        let event_handle_ref_mut =
            &mut recognized_markets_ref_mut.recognized_market_events;
        // Emit a recognized market event.
        event::emit_event(event_handle_ref_mut, RecognizedMarketEvent{
            trading_pair, recognized_market_info: optional_market_info});
    }

    /// Wrapper for `set_recognized_market()` with market IDs vector.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    public entry fun set_recognized_markets(
        account: &signer,
        market_ids: vector<u64>
    ) acquires
        RecognizedMarkets,
        Registry
    {
        // Get number of markets to set.
        let n_markets = vector::length(&market_ids);
        let i = 0; // Declare loop counter.
        while (i < n_markets) { // Loop over all markets in vector:
            // Get market ID to set.
            let market_id = *vector::borrow(&market_ids, i);
            // Set as recognized market.
            set_recognized_market(account, market_id);
            i = i + 1; // Increment loop counter.
        }
    }

    // Public entry functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public friend functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Check types, return market info for market account registration.
    ///
    /// Restricted to friends to prevent excessive public queries
    /// against the registry.
    ///
    /// # Parameters
    ///
    /// * `market_id`: Market ID to check.
    /// * `base_type`: Base type to check.
    /// * `quote_type`: Quote type to check.
    ///
    /// # Returns
    ///
    /// * `String`: `MarketInfo.base_name_generic`.
    /// * `u64`: `MarketInfo.lot_size`.
    /// * `u64`: `MarketInfo.tick_size`.
    /// * `u64`: `MarketInfo.min_size`.
    /// * `u64`: `MarketInfo.underwriter_id`.
    ///
    /// # Aborts
    ///
    /// * `E_INVALID_MARKET_ID`: Market ID is invalid.
    /// * `E_INVALID_BASE`: Base asset type is invalid.
    /// * `E_INVALID_QUOTE`: Quote asset type is invalid.
    ///
    /// # Testing
    ///
    /// * `test_get_market_info_for_market_account()`
    /// * `test_get_market_info_for_market_account_invalid_base()`
    /// * `test_get_market_info_for_market_account_invalid_market_id()`
    /// * `test_get_market_info_for_market_account_invalid_quote()`
    public(friend) fun get_market_info_for_market_account(
        market_id: u64,
        base_type: TypeInfo,
        quote_type: TypeInfo
    ): (
        String,
        u64,
        u64,
        u64,
        u64
    ) acquires Registry {
        let markets_map_ref = // Immutably borrow markets map.
            &borrow_global<Registry>(@econia).market_id_to_info;
        assert!( // Assert market ID corresponds to registered market.
            tablist::contains(markets_map_ref, market_id),
            E_INVALID_MARKET_ID);
        // Immutably borrow market info for market ID.
        let market_info_ref = tablist::borrow(markets_map_ref, market_id);
        // Assert valid base asset type info.
        assert!(base_type == market_info_ref.base_type, E_INVALID_BASE);
        // Assert valid quote asset type info.
        assert!(quote_type == market_info_ref.quote_type, E_INVALID_QUOTE);
        (market_info_ref.base_name_generic, // Return market info.
         market_info_ref.lot_size,
         market_info_ref.tick_size,
         market_info_ref.min_size,
         market_info_ref.underwriter_id)
    }

    /// Return `true` if `custodian_id` has been registered.
    ///
    /// Restricted to friends to prevent excessive public queries
    /// against the registry.
    ///
    /// # Testing
    ///
    /// * `test_register_capabilities()`
    public(friend) fun is_registered_custodian_id(
        custodian_id: u64
    ): bool
    acquires Registry {
        // Get number of registered custodians.
        let n_custodians = borrow_global<Registry>(@econia).n_custodians;
        // Return if custodian ID is less than or equal to number of
        // registered custodians and, if is not flag for no custodian.
        (custodian_id <= n_custodians) && (custodian_id != NO_CUSTODIAN)
    }

    /// Wrapped market registration call for a base coin type.
    ///
    /// See inner function `register_market_internal()`.
    ///
    /// # Aborts
    ///
    /// * `E_BASE_NOT_COIN`: Base coin type is not initialized.
    ///
    /// # Testing
    ///
    /// * `test_register_market_base_not_coin()`
    /// * `test_register_market_base_coin_internal()`
    public(friend) fun register_market_base_coin_internal<
        BaseCoinType,
        QuoteCoinType,
        UtilityCoinType
    >(
        lot_size: u64,
        tick_size: u64,
        min_size: u64,
        utility_coins: Coin<UtilityCoinType>
    ): u64
    acquires Registry {
        // Assert base coin type is initialized.
        assert!(coin::is_coin_initialized<BaseCoinType>(), E_BASE_NOT_COIN);
        // Add to the registry a corresponding entry, returning new
        // market ID.
        register_market_internal<QuoteCoinType, UtilityCoinType>(
            type_info::type_of<BaseCoinType>(), string::utf8(b""), lot_size,
            tick_size, min_size, NO_UNDERWRITER, utility_coins)
    }

    /// Wrapped market registration call for a generic base type,
    /// requiring immutable reference to corresponding
    /// `UnderwriterCapability` for the market, and `base_type`
    /// descriptor.
    ///
    /// See inner function `register_market_internal()`.
    ///
    /// # Aborts
    ///
    /// * `E_GENERIC_TOO_FEW_CHARACTERS`: Asset descriptor is too short.
    /// * `E_GENERIC_TOO_MANY_CHARACTERS`: Asset descriptor is too long.
    ///
    /// # Testing
    ///
    /// * `test_register_market_base_generic_internal()`
    /// * `test_register_market_generic_name_too_few()`
    /// * `test_register_market_generic_name_too_many()`
    public(friend) fun register_market_base_generic_internal<
        QuoteCoinType,
        UtilityCoinType
    >(
        base_name_generic: String,
        lot_size: u64,
        tick_size: u64,
        min_size: u64,
        underwriter_capability_ref: &UnderwriterCapability,
        utility_coins: Coin<UtilityCoinType>
    ): u64
    acquires Registry {
        // Get generic asset name length.
        let name_length = string::length(&base_name_generic);
        assert!( // Assert generic base asset string is not too short.
            name_length >= MIN_CHARACTERS_GENERIC,
            E_GENERIC_TOO_FEW_CHARACTERS);
        assert!( // Assert generic base asset string is not too long.
            name_length <= MAX_CHARACTERS_GENERIC,
            E_GENERIC_TOO_MANY_CHARACTERS);
        // Get underwriter ID.
        let underwriter_id = underwriter_capability_ref.underwriter_id;
        // Add to the registry a corresponding entry, returning new
        // market ID.
        register_market_internal<QuoteCoinType, UtilityCoinType>(
            type_info::type_of<GenericAsset>(), base_name_generic, lot_size,
            tick_size, min_size, underwriter_id, utility_coins)
    }

    // Public friend functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Private functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Return optional market ID corresponding to given `MarketInfo`.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    fun get_market_id(
        market_info: MarketInfo
    ): Option<u64>
    acquires Registry {
        let market_id_map_ref = // Immutably borrow market ID map.
            &borrow_global<Registry>(@econia).market_info_to_id;
        // Return optional market ID if one exists, else empty option.
        if (table::contains(market_id_map_ref, market_info))
            option::some(*table::borrow(market_id_map_ref, market_info)) else
            option::none()
    }

    /// Return recognized market info for given trading pair.
    ///
    /// # Parameters
    ///
    /// * `trading_pair`: Trading pair to look up.
    ///
    /// # Returns
    ///
    /// * `u64`: `RecognizedMarketInfo.market_id`
    /// * `u64`: `RecognizedMarketInfo.lot_size`
    /// * `u64`: `RecognizedMarketInfo.tick_size`
    /// * `u64`: `RecognizedMarketInfo.min_size`
    /// * `u64`: `RecognizedMarketInfo.underwriter_id`
    ///
    /// # Aborts
    ///
    /// * `E_NO_RECOGNIZED_MARKET`: Trading pair has no recognized
    ///   market.
    ///
    /// # Testing
    ///
    /// * `test_get_recognized_market_info_no_market()`
    /// * `test_set_remove_check_recognized_markets()`
    fun get_recognized_market_info(
        trading_pair: TradingPair
    ): (
        u64,
        u64,
        u64,
        u64,
        u64
    ) acquires RecognizedMarkets {
        // Mutably borrow recognized markets map.
        let recognized_map_ref =
            &borrow_global<RecognizedMarkets>(@econia).map;
        // Assert is actually recognized.
        assert!(tablist::contains(recognized_map_ref, trading_pair),
                E_NO_RECOGNIZED_MARKET);
        // Immutably borrow corresponding recognized market info.
        let recognized_market_info_ref =
            *tablist::borrow(recognized_map_ref, trading_pair);
        // Return recognized market info.
        (recognized_market_info_ref.market_id,
         recognized_market_info_ref.lot_size,
         recognized_market_info_ref.tick_size,
         recognized_market_info_ref.min_size,
         recognized_market_info_ref.underwriter_id)
    }

    /// Return `true` if given `TradingPair` has recognized market.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    fun has_recognized_market(
        trading_pair: TradingPair
    ): bool
    acquires RecognizedMarkets {
        // Mutably borrow recognized markets map.
        let recognized_map_ref =
            &borrow_global<RecognizedMarkets>(@econia).map;
        // Return if map contains entry for given trading pair.
        tablist::contains(recognized_map_ref, trading_pair)
    }

    /// Initialize the Econia registry and recognized markets list upon
    /// module publication.
    fun init_module(
        econia: &signer
    ) {
        // Initialize registry.
        move_to(econia, Registry{
            market_id_to_info: tablist::new(),
            market_info_to_id: table::new(),
            n_custodians: 0,
            n_underwriters: 0,
            market_registration_events:
                account::new_event_handle<MarketRegistrationEvent>(econia)});
        // Initialize recognized markets list.
        move_to(econia, RecognizedMarkets{
            map: tablist::new(),
            recognized_market_events:
                account::new_event_handle<RecognizedMarketEvent>(econia)});
    }

    /// Register a market in the global registry.
    ///
    /// # Type parameters
    ///
    /// * `QuoteCoinType`: The quote coin type for the market.
    /// * `UtilityCoinType`: The utility coin type.
    ///
    /// # Parameters
    ///
    /// * `base_type`: The base coin type info for a pure coin market,
    ///   otherwise that of `GenericAsset`.
    /// * `base_name_generic`: Base asset generic name, if any.
    /// * `lot_size`: Lot size for the market.
    /// * `tick_size`: Tick size for the market.
    /// * `min_size`: Minimum lots per order for market.
    /// * `underwriter_id`: `NO_UNDERWRITER` if a pure coin market,
    ///   otherwise ID of market underwriter.
    /// * `utility_coins`: Utility coins paid to register a market.
    ///
    /// # Emits
    ///
    /// * `MarketRegistrationEvent`: Parameters of market just
    ///   registered.
    ///
    /// # Aborts
    ///
    /// * `E_LOT_SIZE_0`: Lot size is 0.
    /// * `E_TICK_SIZE_0`: Tick size is 0.
    /// * `E_MIN_SIZE_0`: Minimum size is 0.
    /// * `E_QUOTE_NOT_COIN`: Quote coin type not initialized as coin.
    /// * `E_BASE_QUOTE_SAME`: Base and quote type are the same.
    /// * `E_MARKET_REGISTERED`: Markets map already contains an entry
    ///   for specified market info.
    ///
    /// # Assumptions
    ///
    /// * `underwriter_id` has been properly passed by either
    ///   `register_market_base_coin_internal()` or
    ///   `register_market_base_generic_internal()`.
    ///
    /// # Testing
    ///
    /// * `test_register_market_base_coin_internal()`
    /// * `test_register_market_base_generic_internal()`
    /// * `test_register_market_lot_size_0()`
    /// * `test_register_market_min_size_0()`
    /// * `test_register_market_quote_not_coin()`
    /// * `test_register_market_registered()`
    /// * `test_register_market_same_type()`
    /// * `test_register_market_tick_size_0()`
    fun register_market_internal<
        QuoteCoinType,
        UtilityCoinType
    >(
        base_type: TypeInfo,
        base_name_generic: String,
        lot_size: u64,
        tick_size: u64,
        min_size: u64,
        underwriter_id: u64,
        utility_coins: Coin<UtilityCoinType>
    ): u64
    acquires Registry {
        // Assert lot size is nonzero.
        assert!(lot_size > 0, E_LOT_SIZE_0);
        // Assert tick size is nonzero.
        assert!(tick_size > 0, E_TICK_SIZE_0);
        // Assert minimum size is nonzero.
        assert!(min_size > 0, E_MIN_SIZE_0);
        // Assert quote coin type is initialized.
        assert!(coin::is_coin_initialized<QuoteCoinType>(), E_QUOTE_NOT_COIN);
        // Get quote coin type.
        let quote_type = type_info::type_of<QuoteCoinType>();
        // Assert base and quote type names are not the same.
        assert!(base_type != quote_type, E_BASE_QUOTE_SAME);
        let market_info = MarketInfo{ // Pack market info.
            base_type, base_name_generic, quote_type, lot_size, tick_size,
            min_size, underwriter_id};
        // Mutably borrow registry.
        let registry_ref_mut = borrow_global_mut<Registry>(@econia);
        // Mutably borrow map from market info to market ID.
        let info_to_id_ref_mut = &mut registry_ref_mut.market_info_to_id;
        assert!( // Assert market not registered.
            !table::contains(info_to_id_ref_mut, market_info),
            E_MARKET_REGISTERED);
        // Mutably borrow map from market ID to market info.
        let id_to_info_ref_mut = &mut registry_ref_mut.market_id_to_info;
        // Get 1-indexed market ID.
        let market_id = tablist::length(id_to_info_ref_mut) + 1;
        // Register a market entry in map from market info to market ID.
        table::add(info_to_id_ref_mut, market_info, market_id);
        // Register a market entry in map from market ID to market info.
        tablist::add(id_to_info_ref_mut, market_id, market_info);
        // Mutably borrow market registration events handle.
        let event_handle_ref_mut =
            &mut registry_ref_mut.market_registration_events;
        // Emit a market registration event.
        event::emit_event(event_handle_ref_mut, MarketRegistrationEvent{
            market_id, base_type, base_name_generic, quote_type, lot_size,
            tick_size, min_size, underwriter_id});
        incentives::deposit_market_registration_utility_coins<UtilityCoinType>(
                utility_coins); // Deposit utility coins.
        market_id // Return market ID.
    }

    /// Convert a `TypeInfo` to an `AssetTypeView`.
    ///
    /// # Testing
    ///
    /// * `test_set_remove_check_recognized_markets()`
    fun to_asset_type_view(
        type_info_ref: &TypeInfo
    ): AssetTypeView {
        AssetTypeView{
            package_address: type_info::account_address(type_info_ref),
            module_name: string::utf8(type_info::module_name(type_info_ref)),
            type_name: string::utf8(type_info::struct_name(type_info_ref)),
        }
    }

    // Private functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only constants >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    /// For `register_markets_test()`.
    const LOT_SIZE_PURE_COIN: u64 = 1;
    #[test_only]
    /// For `register_markets_test()`.
    const TICK_SIZE_PURE_COIN: u64 = 2;
    #[test_only]
    /// For `register_markets_test()`.
    const MIN_SIZE_PURE_COIN: u64 = 3;
    #[test_only]
    /// For `register_markets_test()`.
    const BASE_NAME_GENERIC_GENERIC: vector<u8> = b"Generic asset";
    #[test_only]
    /// For `register_markets_test()`.
    const LOT_SIZE_GENERIC: u64 = 4;
    #[test_only]
    /// For `register_markets_test()`.
    const TICK_SIZE_GENERIC: u64 = 5;
    #[test_only]
    /// For `register_markets_test()`.
    const MIN_SIZE_GENERIC: u64 = 6;
    #[test_only]
    /// For `register_markets_test()`.
    const UNDERWRITER_ID_GENERIC: u64 = 7;

    // Test-only constants <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    /// Drop the given `CustodianCapability`.
    public fun drop_custodian_capability_test(
        custodian_capability: CustodianCapability
    ) {
        // Unpack provided capability.
        let CustodianCapability{custodian_id: _} = custodian_capability;
    }

    #[test_only]
    /// Drop the given `UnderwriterCapability`.
    public fun drop_underwriter_capability_test(
        underwriter_capability: UnderwriterCapability
    ) {
        // Unpack provided capability.
        let UnderwriterCapability{underwriter_id: _} = underwriter_capability;
    }

    #[test_only]
    /// Return a `CustodianCapabilty` having given ID, setting it as
    /// a valid ID in the registry.
    public fun get_custodian_capability_test(
        custodian_id: u64
    ): CustodianCapability
    acquires Registry {
        // If proposed custodian ID is less than number registered:
        if (custodian_id < borrow_global<Registry>(@econia).n_custodians)
            // Update registry to have provided ID as number registered.
            borrow_global_mut<Registry>(@econia).n_custodians =
                custodian_id;
        // Return corresponding custodian capability.
        CustodianCapability{custodian_id}
    }

    #[test_only]
    /// Return an `UnderwriterCapabilty` having given ID, setting it as
    /// a valid ID in the registry.
    public fun get_underwriter_capability_test(
        underwriter_id: u64
    ): UnderwriterCapability
    acquires Registry {
        // If proposed underwriter ID is less than number registered:
        if (underwriter_id < borrow_global<Registry>(@econia).n_underwriters)
            // Update registry to have provided ID as number registered.
            borrow_global_mut<Registry>(@econia).n_underwriters =
                underwriter_id;
        // Return corresponding underwriter capability.
        UnderwriterCapability{underwriter_id}
    }

    #[test_only]
    /// Initialize registry for testing, returning Econia signer.
    public fun init_test():
    signer {
        // Create Aptos-style account for Econia, storing signer.
        let econia = account::create_account_for_test(@econia);
        init_module(&econia); // Init registry.
        incentives::init_test(); // Init incentives.
        econia // Return signer.
    }

    #[test_only]
    /// Register pure coin and generic markets, returning market info.
    public fun register_markets_test(): (
        u64,
        String,
        u64,
        u64,
        u64,
        u64,
        u64,
        String,
        u64,
        u64,
        u64,
        u64
    ) acquires Registry {
        init_test(); // Initialize for testing.
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Declare market parameters.
        let base_name_generic_pure_coin = string::utf8(b"");
        let lot_size_pure_coin          = LOT_SIZE_PURE_COIN;
        let tick_size_pure_coin         = TICK_SIZE_PURE_COIN;
        let min_size_pure_coin          = MIN_SIZE_PURE_COIN;
        let underwriter_id_pure_coin    = NO_UNDERWRITER;
        let base_name_generic_generic
            = string::utf8(BASE_NAME_GENERIC_GENERIC);
        let lot_size_generic            = LOT_SIZE_GENERIC;
        let tick_size_generic           = TICK_SIZE_GENERIC;
        let min_size_generic            = MIN_SIZE_GENERIC;
        let underwriter_id_generic      = UNDERWRITER_ID_GENERIC;
        let underwriter_capability = // Get underwriter capability.
            get_underwriter_capability_test(underwriter_id_generic);
        // Register markets.
        let market_id_pure_coin = register_market_base_coin_internal<
            BC, QC, UC>(lot_size_pure_coin, tick_size_pure_coin,
            min_size_pure_coin, assets::mint_test(fee));
        let market_id_generic = register_market_base_generic_internal<QC, UC>(
            base_name_generic_generic, lot_size_generic, tick_size_generic,
            min_size_generic, &underwriter_capability, assets::mint_test(fee));
        // Drop underwriter capability.
        drop_underwriter_capability_test(underwriter_capability);
        // Return market info.
        (market_id_pure_coin,
         base_name_generic_pure_coin,
         lot_size_pure_coin,
         tick_size_pure_coin,
         min_size_pure_coin,
         underwriter_id_pure_coin,
         market_id_generic,
         base_name_generic_generic,
         lot_size_generic,
         tick_size_generic,
         min_size_generic,
         underwriter_id_generic)
    }

    #[test_only]
    /// Update registry to indicate custodian ID is valid.
    public fun set_registered_custodian_test(
        custodian_id: u64
    ) acquires Registry {
        let n_custodians_ref_mut =  // Mutably borrow custodian count.
            &mut borrow_global_mut<Registry>(@econia).n_custodians;
        // If custodian ID is greater than number of registered
        // custodians, update count to ID.
        if (custodian_id > *n_custodians_ref_mut)
            *n_custodians_ref_mut = custodian_id;
    }

    // Test-only functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Tests >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test]
    /// Verify returns.
    fun test_get_market_info_for_market_account()
    acquires Registry {
        init_test(); // Initialize for testing.
        let underwriter_id = 100; // Declare underwriter ID.
        let underwriter_capability = // Get underwriter capability.
            get_underwriter_capability_test(underwriter_id);
        // Declare arguments.
        let lot_size = 123;
        let tick_size = 456;
        let min_size = 789;
        let base_name_generic = string::utf8(b"Generic asset");
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Register market, storing market ID.
        let market_id = register_market_base_generic_internal<QC, UC>(
            base_name_generic, lot_size, tick_size, min_size,
            &underwriter_capability, assets::mint_test(fee));
        // Drop underwriter capability.
        drop_underwriter_capability_test(underwriter_capability);
        let (base_type, quote_type) = // Get asset types.
            (type_info::type_of<GenericAsset>(), type_info::type_of<QC>());
        // Get market info returns.
        let (base_name_generic_r, lot_size_r, tick_size_r, min_size_r,
             underwriter_id_r) = get_market_info_for_market_account(
                market_id, base_type, quote_type);
        // Assert returns.
        assert!(base_name_generic_r == base_name_generic, 0);
        assert!(lot_size_r == lot_size, 0);
        assert!(tick_size_r == tick_size, 0);
        assert!(min_size_r == min_size, 0);
        assert!(underwriter_id_r == underwriter_id, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_BASE)]
    /// Verify failure for invalid base asset.
    fun test_get_market_info_for_market_account_invalid_base()
    acquires Registry {
        init_test(); // Initialize for testing.
        let underwriter_id = 100; // Declare underwriter ID.
        let underwriter_capability = // Get underwriter capability.
            get_underwriter_capability_test(underwriter_id);
        // Declare arguments.
        let lot_size = 123;
        let tick_size = 456;
        let min_size = 789;
        let base_name_generic = string::utf8(b"Generic asset");
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Register market, storing market ID.
        let market_id = register_market_base_generic_internal<QC, UC>(
            base_name_generic, lot_size, tick_size, min_size,
            &underwriter_capability, assets::mint_test(fee));
        // Drop underwriter capability.
        drop_underwriter_capability_test(underwriter_capability);
        let (base_type, quote_type) = // Get asset types (invalid base).
            (type_info::type_of<BC>(), type_info::type_of<BC>());
        // Attempt invalid invocation.
        get_market_info_for_market_account(market_id, base_type, quote_type);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ID)]
    /// Verify failure for invalid market ID.
    fun test_get_market_info_for_market_account_invalid_market_id()
    acquires Registry {
        init_test(); // Initialize for testing.
        let (base_type, quote_type) = // Get asset types.
            (type_info::type_of<BC>(), type_info::type_of<QC>());
        // Attempt invalid invocation.
        get_market_info_for_market_account(123, base_type, quote_type);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_QUOTE)]
    /// Verify failure for invalid quote asset.
    fun test_get_market_info_for_market_account_invalid_quote()
    acquires Registry {
        init_test(); // Initialize for testing.
        let underwriter_id = 100; // Declare underwriter ID.
        let underwriter_capability = // Get underwriter capability.
            get_underwriter_capability_test(underwriter_id);
        // Declare arguments.
        let lot_size = 123;
        let tick_size = 456;
        let min_size = 789;
        let base_name_generic = string::utf8(b"Generic asset");
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Register market, storing market ID.
        let market_id = register_market_base_generic_internal<QC, UC>(
            base_name_generic, lot_size, tick_size, min_size,
            &underwriter_capability, assets::mint_test(fee));
        // Drop underwriter capability.
        drop_underwriter_capability_test(underwriter_capability);
        let (base_type, quote_type) = // Get asset types (wrong quote).
            (type_info::type_of<GenericAsset>(), type_info::type_of<BC>());
        // Attempt invalid invocation.
        get_market_info_for_market_account(market_id, base_type, quote_type);
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_MAX_CHARACTERS_GENERIC() {
        assert!(get_MAX_CHARACTERS_GENERIC() == MAX_CHARACTERS_GENERIC, 0)
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_MIN_CHARACTERS_GENERIC() {
        assert!(get_MIN_CHARACTERS_GENERIC() == MIN_CHARACTERS_GENERIC, 0)
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_NO_CUSTODIAN() {
        assert!(get_NO_CUSTODIAN() == NO_CUSTODIAN, 0);
    }

    #[test]
    /// Verify constant getter return.
    fun test_get_NO_UNDERWRITER() {
        assert!(get_NO_UNDERWRITER() == NO_UNDERWRITER, 0)
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_MARKET_ID)]
    /// Verify failure for no such market ID.
    fun test_get_market_info_invalid_market_id()
    acquires
        RecognizedMarkets,
        Registry
    {
        init_test(); // Initialize for testing.
        get_market_info(1); // Attempt invalid invocation.
    }

    #[test]
    #[expected_failure(abort_code = E_NO_RECOGNIZED_MARKET)]
    /// Verify failure for no recognized market.
    fun test_get_recognized_market_info_no_market()
    acquires RecognizedMarkets {
        init_test(); // Initialize for testing.
        // Attempt invalid invocation.
        get_recognized_market_info_base_coin_by_type<BC, QC>();
    }

    #[test]
    /// Verify custodian then underwriter capability registration.
    fun test_register_capabilities()
    acquires Registry {
        init_test(); // Initialize for testing.
        // Get custodian registration fee.
        let custodian_registration_fee =
            incentives::get_custodian_registration_fee();
        // Assert custodian ID 1 marked as not registered.
        assert!(!is_registered_custodian_id(1), 0);
        // Get custodian capability.
        let custodian_capability = register_custodian_capability(
            assets::mint_test<UC>(custodian_registration_fee));
        // Assert it has ID 1.
        assert!(get_custodian_id(&custodian_capability) == 1, 0);
        // Assert custodian ID 1 marked as registered.
        assert!(is_registered_custodian_id(1), 0);
        // Assert custodian ID 2 marked as not registered.
        assert!(!is_registered_custodian_id(2), 0);
        // Drop custodian capability.
        drop_custodian_capability_test(custodian_capability);
        // Get another custodian capability.
        custodian_capability = register_custodian_capability(
            assets::mint_test<UC>(custodian_registration_fee));
        // Assert it has ID 2.
        assert!(get_custodian_id(&custodian_capability) == 2, 0);
        // Assert custodian ID 2 marked as registered.
        assert!(is_registered_custodian_id(2), 0);
        // Drop custodian capability.
        drop_custodian_capability_test(custodian_capability);
        // Get another custodian capability.
        custodian_capability = register_custodian_capability(
            assets::mint_test<UC>(custodian_registration_fee));
        // Assert it has ID 3.
        assert!(get_custodian_id(&custodian_capability) == 3, 0);
        // Drop custodian capability.
        drop_custodian_capability_test(custodian_capability);
        // Get underwriter registration fee.
        let underwriter_registration_fee =
            incentives::get_underwriter_registration_fee();
        // Get underwriter capability.
        let underwriter_capability = register_underwriter_capability(
            assets::mint_test<UC>(underwriter_registration_fee));
        // Assert it has ID 1.
        assert!(get_underwriter_id(&underwriter_capability) == 1, 0);
        // Drop underwriter capability.
        drop_underwriter_capability_test(underwriter_capability);
        // Get another underwriter capability.
        underwriter_capability = register_underwriter_capability(
            assets::mint_test<UC>(underwriter_registration_fee));
        // Assert it has ID 2.
        assert!(get_underwriter_id(&underwriter_capability) == 2, 0);
        // Drop underwriter capability.
        drop_underwriter_capability_test(underwriter_capability);
        // Get another underwriter capability.
        underwriter_capability = register_underwriter_capability(
            assets::mint_test<UC>(underwriter_registration_fee));
        // Assert it has ID 3.
        assert!(get_underwriter_id(&underwriter_capability) == 3, 0);
        // Drop underwriter capability.
        drop_underwriter_capability_test(underwriter_capability);
        // Assert no custodian flag not marked as registered.
        assert!(!is_registered_custodian_id(NO_CUSTODIAN), 0);
    }

    #[test(user = @user)]
    #[expected_failure(abort_code = E_INVALID_MARKET_ID)]
    /// Verify failure for no such market ID.
    fun test_register_integrator_fee_store_invalid_market_id(
        user: &signer
    ) acquires Registry {
        init_test(); // Initialize for testing.
        // Attempt invalid invocation.
        register_integrator_fee_store<QC, UC>(user, 1, 1, coin::zero());
    }

    #[test(user = @user)]
    #[expected_failure(abort_code = E_INVALID_QUOTE)]
    /// Verify failure for invalid quote coin.
    fun test_register_integrator_fee_store_invalid_quote(
        user: &signer
    ) acquires Registry {
        register_markets_test(); // Register test markets.
        // Attempt invalid invocation.
        register_integrator_fee_store<UC, UC>(user, 1, 1, coin::zero());
    }

    #[test]
    /// Verify successful registration.
    fun test_register_integrator_fee_stores()
    acquires Registry {
        register_markets_test(); // Register test markets.
        // Create integrator accounts.
        let integrator_0 = account::create_account_for_test(@user_0);
        let integrator_1 = account::create_account_for_test(@user_1);
        // Register utility coin stores.
        coin::register<UC>(&integrator_0);
        coin::register<UC>(&integrator_1);
        // Deposit utility coins.
        coin::deposit<UC>(@user_0, assets::mint_test(10000000000000000));
        coin::deposit<UC>(@user_0, assets::mint_test(10000000000000000));
        // Register first integrator to base tier on first market.
        register_integrator_fee_store_base_tier<QC, UC>(&integrator_0, 1);
        // Assert activation tier.
        assert!(incentives::get_integrator_fee_store_tier_test<QC>(@user_0, 1)
                == 0, 0);
        // Register first integrator to tier 1 on second market.
        register_integrator_fee_store_from_coinstore<QC, UC>(
            &integrator_0, 2, 1);
        // Assert activation tier.
        assert!(incentives::get_integrator_fee_store_tier_test<QC>(@user_0, 2)
                == 1, 0);
        // Register second integrator to base 0 on second market with
        // redundant call to coinstore version function.
        register_integrator_fee_store_from_coinstore<QC, UC>(
            &integrator_1, 2, 0);
        // Assert activation tier.
        assert!(incentives::get_integrator_fee_store_tier_test<QC>(@user_1, 2)
                == 0, 0);
    }

    #[test]
    /// Verify state updates, return, for pure coin market.
    fun test_register_market_base_coin_internal()
    acquires Registry {
        init_test(); // Initialize for testing.
        // Declare arguments.
        let lot_size = 123;
        let tick_size = 456;
        let min_size = 789;
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Register market, storing market ID.
        let market_id = register_market_base_coin_internal<BC, QC, UC>(
            lot_size - 1, tick_size, min_size, assets::mint_test(fee));
        assert!(market_id == 1, 0); // Assert market ID.
        // Register another market, storing market ID.
        market_id = register_market_base_coin_internal<BC, QC, UC>(
            lot_size, tick_size, min_size, assets::mint_test(fee));
        assert!(market_id == 2, 0); // Assert market ID.
        let markets_tablist_ref = // Immutably borrow markets tablist.
            &borrow_global<Registry>(@econia).market_id_to_info;
        // Immutably borrow market info.
        let market_info_ref = tablist::borrow(markets_tablist_ref, market_id);
        // Assert fields.
        assert!(market_info_ref.base_type == type_info::type_of<BC>(), 0);
        assert!(string::is_empty(&market_info_ref.base_name_generic), 0);
        assert!(market_info_ref.quote_type == type_info::type_of<QC>(), 0);
        assert!(market_info_ref.lot_size == lot_size, 0);
        assert!(market_info_ref.tick_size == tick_size, 0);
        assert!(market_info_ref.min_size == min_size, 0);
        assert!(market_info_ref.underwriter_id == NO_UNDERWRITER, 0);
        let market_info_map_ref = // Immutably borrow market info map.
            &borrow_global<Registry>(@econia).market_info_to_id;
        assert!( // Assert lookup on market info.
            *table::borrow(market_info_map_ref, *market_info_ref) == market_id,
            0);
    }

    #[test]
    /// Verify state updates, return, for generic asset market.
    fun test_register_market_base_generic_internal()
    acquires Registry {
        init_test(); // Initialize for testing.
        let underwriter_id = 100; // Declare underwriter ID.
        let underwriter_capability = // Get underwriter capability.
            get_underwriter_capability_test(underwriter_id);
        // Declare arguments.
        let lot_size = 123;
        let tick_size = 456;
        let min_size = 789;
        let base_name_generic = string::utf8(b"Generic asset");
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Register market, storing market ID.
        let market_id = register_market_base_generic_internal<QC, UC>(
            base_name_generic, lot_size - 1, tick_size, min_size,
            &underwriter_capability, assets::mint_test(fee));
        assert!(market_id == 1, 0); // Assert market ID.
        // Register another market, storing market ID.
        let market_id = register_market_base_generic_internal<QC, UC>(
            base_name_generic, lot_size, tick_size, min_size,
            &underwriter_capability, assets::mint_test(fee));
        assert!(market_id == 2, 0); // Assert market ID.
        let markets_tablist_ref = // Immutably borrow markets tablist.
            &borrow_global<Registry>(@econia).market_id_to_info;
        // Immutably borrow market info.
        let market_info_ref = tablist::borrow(markets_tablist_ref, market_id);
        assert!( // Assert fields.
            market_info_ref.base_type == type_info::type_of<GenericAsset>(),
            0);
        assert!(market_info_ref.base_name_generic == base_name_generic, 0);
        assert!(market_info_ref.quote_type == type_info::type_of<QC>(), 0);
        assert!(market_info_ref.lot_size == lot_size, 0);
        assert!(market_info_ref.tick_size == tick_size, 0);
        assert!(market_info_ref.min_size == min_size, 0);
        assert!(market_info_ref.underwriter_id == underwriter_id, 0);
        let market_info_map_ref = // Immutably borrow market info map.
            &borrow_global<Registry>(@econia).market_info_to_id;
        assert!( // Assert lookup on market info.
            *table::borrow(market_info_map_ref, *market_info_ref) == market_id,
            0);
        // Drop underwriter capability.
        drop_underwriter_capability_test(underwriter_capability);
    }

    #[test]
    #[expected_failure(abort_code = E_BASE_NOT_COIN)]
    /// Verify failure for non-coin type.
    fun test_register_market_base_not_coin()
    acquires Registry {
        // Declare arguments.
        let lot_size = 0;
        let tick_size = 0;
        let min_size = 0;
        // Attempt invalid invocation.
        register_market_base_coin_internal<BC, QC, UC>(
            lot_size, tick_size, min_size, coin::zero());
    }

    #[test]
    #[expected_failure(abort_code = E_GENERIC_TOO_FEW_CHARACTERS)]
    /// Verify failure for too few characters in generic asset name.
    fun test_register_market_generic_name_too_few()
    acquires Registry {
        init_test(); // Initialize for testing.
        // Get underwriter capability.
        let underwriter_capability = get_underwriter_capability_test(1);
        // Declare arguments.
        let base_name_generic = string::utf8(b"ABC");
        let lot_size = 0;
        let tick_size = 0;
        let min_size = 0;
        // Attempt invalid invocation.
        register_market_base_generic_internal<QC, UC>(
            base_name_generic, lot_size, tick_size, min_size,
            &underwriter_capability, coin::zero());
        // Drop underwriter capability.
        drop_underwriter_capability_test(underwriter_capability);
    }

    #[test]
    #[expected_failure(abort_code = E_GENERIC_TOO_MANY_CHARACTERS)]
    /// Verify failure for too many characters in generic asset name.
    fun test_register_market_generic_name_too_many()
    acquires Registry {
        init_test(); // Initialize for testing.
        // Get underwriter capability.
        let underwriter_capability = get_underwriter_capability_test(1);
        // Declare arguments.
        let base_name_generic = // Get 36-character string.
            string::utf8(b"123456789012345678901234567890123456");
        string::append(&mut base_name_generic, // Append 37 characters.
            string::utf8(b"1111111111111111111111111111111111111"));
        let lot_size = 0;
        let tick_size = 0;
        let min_size = 0;
        // Attempt invalid invocation.
        register_market_base_generic_internal<QC, UC>(
            base_name_generic, lot_size, tick_size, min_size,
            &underwriter_capability, coin::zero());
        // Drop underwriter capability.
        drop_underwriter_capability_test(underwriter_capability);
    }

    #[test]
    #[expected_failure(abort_code = E_LOT_SIZE_0)]
    /// Verify failure for lot size 0.
    fun test_register_market_lot_size_0()
    acquires Registry {
        init_test(); // Initialize for testing.
        // Declare arguments.
        let lot_size = 0;
        let tick_size = 0;
        let min_size = 0;
        // Attempt invalid invocation.
        register_market_base_coin_internal<BC, QC, UC>(
            lot_size, tick_size, min_size, assets::mint_test(1));
    }

    #[test]
    #[expected_failure(abort_code = E_MIN_SIZE_0)]
    /// Verify failure for minimum size 0.
    fun test_register_market_min_size_0()
    acquires Registry {
        init_test(); // Initialize for testing.
        // Declare arguments.
        let lot_size = 1;
        let tick_size = 1;
        let min_size = 0;
        // Attempt invalid invocation.
        register_market_base_coin_internal<BC, QC, UC>(
            lot_size, tick_size, min_size, coin::zero());
    }

    #[test]
    #[expected_failure(abort_code = E_QUOTE_NOT_COIN)]
    /// Verify failure for quote asset not coin.
    fun test_register_market_quote_not_coin()
    acquires Registry {
        init_test(); // Initialize for testing.
        // Declare arguments.
        let lot_size = 1;
        let tick_size = 1;
        let min_size = 1;
        // Attempt invalid invocation.
        register_market_base_coin_internal<QC, GenericAsset, UC>(
            lot_size, tick_size, min_size, coin::zero());
    }

    #[test]
    #[expected_failure(abort_code = E_MARKET_REGISTERED)]
    /// Verify failure for market already registered.
    fun test_register_market_registered()
    acquires Registry {
        init_test(); // Initialize for testing.
        // Declare arguments.
        let lot_size = 1;
        let tick_size = 1;
        let min_size = 1;
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Register valid market.
        register_market_base_coin_internal<BC, QC, UC>(
            lot_size, tick_size, min_size, assets::mint_test(fee));
        // Attempt invalid re-registration.
        register_market_base_coin_internal<BC, QC, UC>(
            lot_size, tick_size, min_size, coin::zero());
    }

    #[test]
    #[expected_failure(abort_code = E_BASE_QUOTE_SAME)]
    /// Verify failure for base and quote same coin type.
    fun test_register_market_same_type()
    acquires Registry {
        init_test(); // Initialize for testing.
        // Declare arguments.
        let lot_size = 1;
        let tick_size = 1;
        let min_size = 1;
        // Attempt invalid invocation.
        register_market_base_coin_internal<QC, QC, UC>(
            lot_size, tick_size, min_size, coin::zero());
    }

    #[test]
    #[expected_failure(abort_code = E_TICK_SIZE_0)]
    /// Verify failure for tick size 0.
    fun test_register_market_tick_size_0()
    acquires Registry {
        init_test(); // Initialize for testing.
        // Declare arguments.
        let lot_size = 1;
        let tick_size = 0;
        let min_size = 0;
        // Attempt invalid invocation.
        register_market_base_coin_internal<BC, QC, UC>(
            lot_size, tick_size, min_size, coin::zero());
    }

    #[test(account = @econia)]
    #[expected_failure(abort_code = E_NO_RECOGNIZED_MARKET)]
    /// Verify failure for market no recognized market.
    fun test_remove_recognized_market_no_recognized(
        account: &signer
    ) acquires
        RecognizedMarkets,
        Registry
    {
        init_test(); // Initialize for testing.
        // Declare arguments.
        let lot_size = 123;
        let tick_size = 456;
        let min_size = 789;
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Register market, storing ID.
        let market_id = register_market_base_coin_internal<BC, QC, UC>(
            lot_size, tick_size, min_size, assets::mint_test(fee));
        // Attempt invalid invocation.
        remove_recognized_market(account, market_id);
    }

    #[test(account = @user)]
    #[expected_failure(abort_code = E_NOT_ECONIA)]
    /// Verify failure for account is not Econia.
    fun test_remove_recognized_market_not_econia(
        account: &signer
    ) acquires
        RecognizedMarkets,
        Registry
    {
        // Attempt invalid invocation.
        remove_recognized_market(account, 0);
    }

    #[test(account = @econia)]
    #[expected_failure(abort_code = E_WRONG_RECOGNIZED_MARKET)]
    /// Verify failure for wrong recognized market.
    fun test_remove_recognized_market_wrong_market(
        account: &signer
    ) acquires
        RecognizedMarkets,
        Registry
    {
        init_test(); // Initialize for testing.
        // Declare arguments.
        let lot_size = 123;
        let tick_size = 456;
        let min_size = 789;
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Register market, storing ID.
        let market_id = register_market_base_coin_internal<BC, QC, UC>(
            lot_size, tick_size, min_size, assets::mint_test(fee));
        set_recognized_market(account, market_id); // Set as recognized.
        // Register different market with same trading pair, storing ID.
        let market_id_2 = register_market_base_coin_internal<BC, QC, UC>(
            lot_size - 1, tick_size, min_size, assets::mint_test(fee));
        // Attempt invalid invocation.
        remove_recognized_market(account, market_id_2);
    }

    #[test(account = @user)]
    #[expected_failure(abort_code = E_NOT_ECONIA)]
    /// Verify failure for account is not Econia.
    fun test_set_recognized_market_not_econia(
        account: &signer
    ) acquires
        RecognizedMarkets,
        Registry
    {
        // Attempt invalid invocation.
        set_recognized_market(account, 0);
    }

    #[test(account = @econia)]
    /// Verify state updates for updating recognized market info for
    /// given trading pair.
    fun test_set_recognized_market_update(
        account: &signer
    ) acquires
        RecognizedMarkets,
        Registry
    {
        init_test(); // Initialize for testing.
        // Declare arguments.
        let lot_size_1 = 123;
        let tick_size_1 = 456;
        let min_size_1 = 789;
        let lot_size_2 = lot_size_1 - 1;
        let tick_size_2 = tick_size_1 - 1;
        let min_size_2 = min_size_1 - 1;
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Register markets, storing IDs.
        let market_id_1 = register_market_base_coin_internal<BC, QC, UC>(
            lot_size_1, tick_size_1, min_size_1, assets::mint_test(fee));
        let market_id_2 = register_market_base_coin_internal<BC, QC, UC>(
            lot_size_2, tick_size_2, min_size_2, assets::mint_test(fee));
        // Set first market as recognized.
        set_recognized_market(account, market_id_1);
        // Assert lookup.
        assert!(has_recognized_market_base_coin_by_type<BC, QC>(), 0);
        // Assert pure coin asset market info.
        let (market_id, lot_size, tick_size, min_size, underwriter_id) =
            get_recognized_market_info_base_coin_by_type<BC, QC>();
        assert!(market_id == market_id_1, 0);
        assert!(lot_size == lot_size_1, 0);
        assert!(tick_size == tick_size_1, 0);
        assert!(min_size == min_size_1, 0);
        assert!(underwriter_id == NO_UNDERWRITER, 0);
        // Set second market as recognized.
        set_recognized_market(account, market_id_2);
        // Assert update.
        (market_id, lot_size, tick_size, min_size, underwriter_id) =
            get_recognized_market_info_base_coin_by_type<BC, QC>();
        assert!(market_id == market_id_2, 0);
        assert!(lot_size == lot_size_2, 0);
        assert!(tick_size == tick_size_2, 0);
        assert!(min_size == min_size_2, 0);
        assert!(underwriter_id == NO_UNDERWRITER, 0);
    }

    #[test(econia = @econia)]
    /// Verify returns, state updates for setting and removing
    /// registered markets, lookup operations.
    fun test_set_remove_check_recognized_markets(
        econia: &signer
    ) acquires
        RecognizedMarkets,
        Registry
    {
        init_test(); // Initialize for testing.
        // Assert market counts.
        assert!(get_market_counts() ==
                MarketCounts{n_markets: 0, n_recognized_markets: 0}, 0);
        // Get generic market underwriter capability.
        let underwriter_id_generic = 123;
        let underwriter_capability = // Get underwriter capability.
            get_underwriter_capability_test(underwriter_id_generic);
        // Declare market parameters.
        let base_name_generic = string::utf8(b"Generic asset");
        let lot_size_1 = 234;
        let tick_size_1 = 345;
        let min_size_1 = 456;
        let lot_size_2 = lot_size_1 - 1;
        let tick_size_2 = tick_size_1 - 1;
        let min_size_2 = min_size_1 - 1;
        let lot_size_3 = lot_size_2 - 1;
        let tick_size_3 = tick_size_2 - 1;
        let min_size_3 = min_size_2 - 1;
        let base_type_view = AssetTypeView{
            package_address: @econia,
            module_name: string::utf8(b"assets"),
            type_name: string::utf8(b"BC"),
        };
        let base_type_view_generic = AssetTypeView{
            package_address: @econia,
            module_name: string::utf8(b"registry"),
            type_name: string::utf8(b"GenericAsset"),
        };
        let quote_type_view = AssetTypeView{
            package_address: @econia,
            module_name: string::utf8(b"assets"),
            type_name: string::utf8(b"QC"),
        };
        // Get market registration fee.
        let fee = incentives::get_market_registration_fee();
        // Assert existence checks.
        assert!(
            !has_recognized_market_base_generic_by_type<QC>(base_name_generic),
            0);
        assert!(!has_recognized_market_base_coin_by_type<BC, QC>(), 0);
        assert!(get_market_id_base_generic<QC>(
                    base_name_generic, lot_size_1, tick_size_1, min_size_1,
                    underwriter_id_generic)
                == option::none(), 0);
        assert!(get_market_id_base_coin<BC, QC>(
                    lot_size_2, tick_size_2, min_size_2) == option::none(), 0);
        // Assert events.
        let market_registration_events = event::emitted_events_by_handle(
            &borrow_global<Registry>(@econia).market_registration_events);
        assert!(market_registration_events == vector[], 0);
        // Register markets.
        register_market_base_generic_internal<QC, UC>(
            base_name_generic, lot_size_1, tick_size_1, min_size_1,
            &underwriter_capability, assets::mint_test(fee));
        register_market_base_coin_internal<BC, QC, UC>(
            lot_size_2, tick_size_2, min_size_2, assets::mint_test(fee));
        // Assert events.
        market_registration_events = event::emitted_events_by_handle(
            &borrow_global<Registry>(@econia).market_registration_events);
        assert!(market_registration_events == vector[
            MarketRegistrationEvent{
                market_id: 1,
                base_type: type_info::type_of<GenericAsset>(),
                base_name_generic,
                quote_type: type_info::type_of<QC>(),
                lot_size: lot_size_1,
                tick_size: tick_size_1,
                min_size: min_size_1,
                underwriter_id: underwriter_id_generic
            },
            MarketRegistrationEvent{
                market_id: 2,
                base_type: type_info::type_of<BC>(),
                base_name_generic: string::utf8(b""),
                quote_type: type_info::type_of<QC>(),
                lot_size: lot_size_2,
                tick_size: tick_size_2,
                min_size: min_size_2,
                underwriter_id: NO_UNDERWRITER
            },
        ], 0);
        // Verify market info.
        assert!(get_market_info(1) == MarketInfoView{
            market_id: 1,
            is_recognized: false,
            base_type: base_type_view_generic,
            base_name_generic,
            quote_type: quote_type_view,
            lot_size: lot_size_1,
            tick_size: tick_size_1,
            min_size: min_size_1,
            underwriter_id: underwriter_id_generic
        }, 0);
        assert!(get_market_info(2) == MarketInfoView{
            market_id: 2,
            is_recognized: false,
            base_type: base_type_view,
            base_name_generic: string::utf8(b""),
            quote_type: quote_type_view,
            lot_size: lot_size_2,
            tick_size: tick_size_2,
            min_size: min_size_2,
            underwriter_id: NO_UNDERWRITER
        }, 0);
        // Drop underwriter capability.
        drop_underwriter_capability_test(underwriter_capability);
        // Assert events.
        let recognized_market_events = event::emitted_events_by_handle(
            &borrow_global<RecognizedMarkets>(@econia).
            recognized_market_events);
        assert!(recognized_market_events == vector[], 0);
        // Set both as recognized markets.
        set_recognized_markets(econia, vector[1, 2]);
        // Assert events.
        recognized_market_events = event::emitted_events_by_handle(
            &borrow_global<RecognizedMarkets>(@econia).
            recognized_market_events);
        assert!(recognized_market_events == vector[
            RecognizedMarketEvent{
                trading_pair: TradingPair{
                    base_type: type_info::type_of<GenericAsset>(),
                    base_name_generic,
                    quote_type: type_info::type_of<QC>()
                },
                recognized_market_info: option::some(RecognizedMarketInfo{
                    market_id: 1,
                    lot_size: lot_size_1,
                    tick_size: tick_size_1,
                    min_size: min_size_1,
                    underwriter_id: underwriter_id_generic
                })
            },
            RecognizedMarketEvent{
                trading_pair: TradingPair{
                    base_type: type_info::type_of<BC>(),
                    base_name_generic: string::utf8(b""),
                    quote_type: type_info::type_of<QC>()
                },
                recognized_market_info: option::some(RecognizedMarketInfo{
                    market_id: 2,
                    lot_size: lot_size_2,
                    tick_size: tick_size_2,
                    min_size: min_size_2,
                    underwriter_id: NO_UNDERWRITER
                })
            },
        ], 0);
        // Assert existence checks.
        assert!(
            has_recognized_market_base_generic_by_type<QC>(base_name_generic),
            0);
        assert!(has_recognized_market_base_coin_by_type<BC, QC>(), 0);
        assert!(get_market_id_base_generic<QC>(
                    base_name_generic, lot_size_1, tick_size_1, min_size_1,
                    underwriter_id_generic)
                == option::some(1), 0);
        assert!(get_market_id_base_coin<BC, QC>(
                    lot_size_2, tick_size_2, min_size_2)
                == option::some(2), 0);
        // Assert generic asset market info.
        let (market_id, lot_size, tick_size, min_size, underwriter_id) =
            get_recognized_market_info_base_generic_by_type<QC>(
                base_name_generic);
        assert!(market_id == 1, 0);
        assert!(lot_size == lot_size_1, 0);
        assert!(tick_size == tick_size_1, 0);
        assert!(min_size == min_size_1, 0);
        assert!(underwriter_id == underwriter_id_generic, 0);
        assert!(get_recognized_market_id_base_generic<QC>(base_name_generic)
                == 1, 0);
        // Assert pure coin asset market info.
        let (market_id, lot_size, tick_size, min_size, underwriter_id) =
            get_recognized_market_info_base_coin_by_type<BC, QC>();
        assert!(market_id == 2, 0);
        assert!(lot_size == lot_size_2, 0);
        assert!(tick_size == tick_size_2, 0);
        assert!(min_size == min_size_2, 0);
        assert!(underwriter_id == NO_UNDERWRITER, 0);
        assert!(get_recognized_market_id_base_coin<BC, QC>() == 2, 0);
        // Assert market counts.
        assert!(get_market_counts() ==
                MarketCounts{n_markets: 2, n_recognized_markets: 2}, 0);
        // Verify market info.
        assert!(get_market_info(1) == MarketInfoView{
            market_id: 1,
            is_recognized: true,
            base_type: base_type_view_generic,
            base_name_generic,
            quote_type: quote_type_view,
            lot_size: lot_size_1,
            tick_size: tick_size_1,
            min_size: min_size_1,
            underwriter_id: underwriter_id_generic
        }, 0);
        assert!(get_market_info(2) == MarketInfoView{
            market_id: 2,
            is_recognized: true,
            base_type: base_type_view,
            base_name_generic: string::utf8(b""),
            quote_type: quote_type_view,
            lot_size: lot_size_2,
            tick_size: tick_size_2,
            min_size: min_size_2,
            underwriter_id: NO_UNDERWRITER
        }, 0);
        // Remove both recognized markets.
        remove_recognized_markets(econia, vector[1, 2]);
        // Assert events.
        recognized_market_events = event::emitted_events_by_handle(
            &borrow_global<RecognizedMarkets>(@econia).
            recognized_market_events);
        assert!(vector::length(&recognized_market_events) == 4, 0);
        assert!(vector::pop_back(&mut recognized_market_events) ==
            RecognizedMarketEvent{
                trading_pair: TradingPair{
                    base_type: type_info::type_of<BC>(),
                    base_name_generic: string::utf8(b""),
                    quote_type: type_info::type_of<QC>()
                },
            recognized_market_info: option::none()
            }, 0);
        assert!(vector::pop_back(&mut recognized_market_events) ==
            RecognizedMarketEvent{
                trading_pair: TradingPair{
                    base_type: type_info::type_of<GenericAsset>(),
                    base_name_generic,
                    quote_type: type_info::type_of<QC>()
                },
            recognized_market_info: option::none()
            }, 0);
        // Assert existence checks.
        assert!(
            !has_recognized_market_base_generic_by_type<QC>(base_name_generic),
            0);
        assert!(!has_recognized_market_base_coin_by_type<BC, QC>(), 0);
        // Assert market counts.
        assert!(get_market_counts() ==
                MarketCounts{n_markets: 2, n_recognized_markets: 0}, 0);
        // Register a third market having the same trading pair as the
        // second market, and set it as recognized.
        register_market_base_coin_internal<BC, QC, UC>(
            lot_size_3, tick_size_3, min_size_3, assets::mint_test(fee));
        set_recognized_markets(econia, vector[3]);
        // Verify that second market, which has same trading pair, is
        // not marked as recognized.
        assert!(get_market_info(2) == MarketInfoView{
            market_id: 2,
            is_recognized: false,
            base_type: base_type_view,
            base_name_generic: string::utf8(b""),
            quote_type: quote_type_view,
            lot_size: lot_size_2,
            tick_size: tick_size_2,
            min_size: min_size_2,
            underwriter_id: NO_UNDERWRITER
        }, 0);
        // Assert events.
        market_registration_events = event::emitted_events_by_handle(
            &borrow_global<Registry>(@econia).market_registration_events);
        assert!(vector::length(&market_registration_events) == 3, 0);
        assert!(vector::pop_back(&mut market_registration_events) ==
            MarketRegistrationEvent{
                market_id: 3,
                base_type: type_info::type_of<BC>(),
                base_name_generic: string::utf8(b""),
                quote_type: type_info::type_of<QC>(),
                lot_size: lot_size_3,
                tick_size: tick_size_3,
                min_size: min_size_3,
                underwriter_id: NO_UNDERWRITER
            }, 0);
        recognized_market_events = event::emitted_events_by_handle(
            &borrow_global<RecognizedMarkets>(@econia).
            recognized_market_events);
        assert!(vector::length(&recognized_market_events) == 5, 0);
        assert!(vector::pop_back(&mut recognized_market_events) ==
            RecognizedMarketEvent{
                trading_pair: TradingPair{
                    base_type: type_info::type_of<BC>(),
                    base_name_generic: string::utf8(b""),
                    quote_type: type_info::type_of<QC>()
                },
                recognized_market_info: option::some(RecognizedMarketInfo{
                    market_id: 3,
                    lot_size: lot_size_3,
                    tick_size: tick_size_3,
                    min_size: min_size_3,
                    underwriter_id: NO_UNDERWRITER
                })
            }, 0);
        // Set the second market as the recognized market, thus removing
        // the third market as recognized, and assert event.
        set_recognized_markets(econia, vector[2]);
        recognized_market_events = event::emitted_events_by_handle(
            &borrow_global<RecognizedMarkets>(@econia).
            recognized_market_events);
        assert!(vector::length(&recognized_market_events) == 6, 0);
        assert!(vector::pop_back(&mut recognized_market_events) ==
            RecognizedMarketEvent{
                trading_pair: TradingPair{
                    base_type: type_info::type_of<BC>(),
                    base_name_generic: string::utf8(b""),
                    quote_type: type_info::type_of<QC>()
                },
                recognized_market_info: option::some(RecognizedMarketInfo{
                    market_id: 2,
                    lot_size: lot_size_2,
                    tick_size: tick_size_2,
                    min_size: min_size_2,
                    underwriter_id: NO_UNDERWRITER
                })
            }, 0);
    }

    // Tests <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

}