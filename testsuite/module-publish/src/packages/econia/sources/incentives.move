/// Incentive-associated parameters and data structures.
///
/// Contains hard-coded "genesis parameters" that are are set
/// upon module publication per `init_module()`, and which can be
/// updated later per `set_incentive_parameters()`.
///
/// # General overview sections
///
/// [Incentive model](#incentive-model)
///
/// [Functions](#functions)
///
/// * [View functions](#view-functions)
/// * [Public getters](#public-getters)
/// * [Other public functions](#other-public-functions)
/// * [Public entry functions](#public-entry-functions)
/// * [Public friend functions](#public-friend-functions)
///
/// [Dependency charts](#dependency-charts)
///
/// * [Incentive parameters setters](#incentive-parameter-setters)
/// * [Econia fee account operations](#econia-fee-account-operations)
/// * [Registrant operations](#registrant-operations)
/// * [Integrator operations](#integrator-operations)
/// * [Match operations](#match-operations)
///
/// [Complete DocGen index](#complete-docgen-index)
///
/// # Incentive model
///
/// As a permissionless system, Econia mitigates denial-of-service (DoS)
/// attacks by charging utility coins for assorted operations. Econia
/// also charges taker fees, denominated in the quote coin for a given
/// market, which are distributed between integrators and Econia. The
/// share of taker fees distributed between an integrator and Econia,
/// for a given market, is determined by the "tier" to which the
/// integrator has "activated" their fee store: when the matching engine
/// fills a taker order, the integrator who facilitated the transaction
/// has a portion of taker fees deposited to their fee store, and Econia
/// gets the rest, with the split thereof determined by the integrator's
/// fee store tier for the given market. Econia does not charge maker
/// fees.
///
/// Hence Econia involves 5 major incentive parameters, defined at
/// `IncentiveParameters`:
///
/// 1. The utility coin type.
/// 2. The fee, denominated in the utility coin, to register a market.
/// 3. The fee, denominated in the utility coin, to register as an
///    underwriter for a generic market.
/// 4. The fee, denominated in the utility coin, to register as
///    custodian.
/// 5. The taker fee divisor, denoting the portion of quote coins for a
///    particular trade, paid by the taker, to be split between the
///    integrator who facilitated the trade, and Econia.
///
/// `IncentiveParameters` also includes a vector of
/// `IntegratorFeeStoreTierParameters`, which define 3 parameters per
/// tier:
///
/// 1. The taker fee divisor, denoting the portion of quote coins for a
///    particular trade, paid by the taker, to be collected by an
///    integrator whose fee store is activated to the given tier.
/// 2. The cumulative fee, denominated in the utility coin, to activate
///    to the given tier.
/// 3. The fee, denominated in the utility coin, to withdraw quote coins
///    collected as fees, from an integrator's fee store.
///
/// Upon module publication, the Econia "genesis parameters" are
/// set according to hard-coded values via `init_module()`. Later, the
/// parameters can be updated via `set_incentive_parameters()`, so long
/// as the number of tiers is not reduced and other minor restrictions
/// are met. For an implementation-exact description of restrictions and
/// corresponding abort codes, see:
///
/// * `set_incentive_parameters()`
/// * `set_incentive_parameters_range_check_inputs()`
/// * `set_incentive_parameters_parse_tiers_vector()`
///
/// # Functions
///
/// ## View functions
///
/// * `get_cost_to_upgrade_integrator_fee_store_view()`
/// * `get_custodian_registration_fee()`
/// * `get_fee_share_divisor()`
/// * `get_integrator_withdrawal_fee_view()`
/// * `get_market_registration_fee()`
/// * `get_n_fee_store_tiers()`
/// * `get_taker_fee_divisor()`
/// * `get_tier_activation_fee()`
/// * `get_tier_withdrawal_fee()`
/// * `get_underwriter_registration_fee()`
/// * `is_utility_coin_type()`
///
/// ## Public getters
///
/// * `get_cost_to_upgrade_integrator_fee_store()`
/// * `get_integrator_withdrawal_fee()`
/// * `verify_utility_coin_type()`
///
/// ## Other public functions
///
/// * `upgrade_integrator_fee_store()`
/// * `withdraw_econia_fees()`
/// * `withdraw_econia_fees_all()`
/// * `withdraw_integrator_fees()`
/// * `withdraw_utility_coins()`
/// * `withdraw_utility_coins_all()`
///
/// ## Public entry functions
///
/// * `update_incentives()`
/// * `upgrade_integrator_fee_store_via_coinstore()`
/// * `withdraw_econia_fees_all_to_coin_store()`
/// * `withdraw_econia_fees_to_coin_store()`
/// * `withdraw_integrator_fees_via_coinstores()`
/// * `withdraw_utility_coins_all_to_coin_store()`
/// * `withdraw_utility_coins_to_coin_store()`
///
/// ## Public friend functions
///
/// * `assess_taker_fees()`
/// * `calculate_max_quote_match()`
/// * `deposit_custodian_registration_utility_coins()`
/// * `deposit_market_registration_utility_coins()`
/// * `deposit_underwriter_registration_utility_coins()`
/// * `register_econia_fee_store_entry()`
/// * `register_integrator_fee_store()`
///
/// # Dependency charts
///
/// The below dependency charts use `mermaid.js` syntax, which can be
/// automatically rendered into a diagram (depending on the browser)
/// when viewing the documentation file generated from source code. If
/// a browser renders the diagrams with coloring that makes it difficult
/// to read, try a different browser.
///
/// ## Incentive parameter setters
///
/// ```mermaid
///
/// flowchart LR
///
/// update_incentives --> set_incentive_parameters
/// init_module --> set_incentive_parameters
/// set_incentive_parameters -->
///     set_incentive_parameters_parse_tiers_vector
/// set_incentive_parameters --> resource_account::get_signer
/// set_incentive_parameters -->
///     set_incentive_parameters_range_check_inputs
/// set_incentive_parameters --> init_utility_coin_store
/// set_incentive_parameters --> get_n_fee_store_tiers
///
/// ```
///
/// ## Econia fee account operations
///
/// ```mermaid
///
/// flowchart LR
///
/// deposit_utility_coins --> resource_account::get_address
/// deposit_utility_coins --> range_check_coin_merge
/// deposit_utility_coins_verified --> verify_utility_coin_type
/// deposit_utility_coins_verified --> deposit_utility_coins
/// withdraw_utility_coins --> withdraw_utility_coins_internal
/// withdraw_utility_coins_all --> withdraw_utility_coins_internal
/// withdraw_utility_coins_all_to_coin_store -->
///     withdraw_utility_coins_to_coin_store_internal
/// withdraw_utility_coins_to_coin_store -->
///     withdraw_utility_coins_to_coin_store_internal
/// withdraw_utility_coins_to_coin_store_internal -->
///     withdraw_utility_coins_internal
/// withdraw_utility_coins_internal --> resource_account::get_address
/// withdraw_econia_fees --> withdraw_econia_fees_internal
/// withdraw_econia_fees_all --> withdraw_econia_fees_internal
/// withdraw_econia_fees_internal --> resource_account::get_address
/// withdraw_econia_fees_all_to_coin_store -->
///     withdraw_econia_fees_to_coin_store_internal
/// withdraw_econia_fees_to_coin_store -->
///     withdraw_econia_fees_to_coin_store_internal
/// withdraw_econia_fees_to_coin_store_internal -->
///     withdraw_econia_fees_internal
/// register_econia_fee_store_entry --> resource_account::get_signer
///
/// ```
///
/// ## Registrant operations
///
/// ```mermaid
///
/// flowchart LR
///
/// deposit_custodian_registration_utility_coins -->
///     get_custodian_registration_fee
/// deposit_custodian_registration_utility_coins -->
///     deposit_utility_coins_verified
/// deposit_underwriter_registration_utility_coins -->
///     get_underwriter_registration_fee
/// deposit_underwriter_registration_utility_coins --->
///     deposit_utility_coins_verified
/// deposit_market_registration_utility_coins -->
///     deposit_utility_coins_verified
/// deposit_market_registration_utility_coins -->
///     get_market_registration_fee
///
/// ```
///
/// ## Integrator operations
///
/// ```mermaid
///
/// flowchart LR
///
/// withdraw_integrator_fees_via_coinstores -->
///     get_integrator_withdrawal_fee
/// get_integrator_withdrawal_fee --> get_tier_withdrawal_fee
/// withdraw_integrator_fees_via_coinstores --> withdraw_integrator_fees
/// withdraw_integrator_fees --> get_tier_withdrawal_fee
/// withdraw_integrator_fees --> deposit_utility_coins_verified
/// register_integrator_fee_store ---> deposit_utility_coins_verified
/// register_integrator_fee_store --> get_tier_activation_fee
/// upgrade_integrator_fee_store_via_coinstore -->
///     upgrade_integrator_fee_store
/// upgrade_integrator_fee_store_via_coinstore -->
///     get_cost_to_upgrade_integrator_fee_store
/// upgrade_integrator_fee_store --> deposit_utility_coins_verified
/// upgrade_integrator_fee_store -->
///     get_cost_to_upgrade_integrator_fee_store
/// get_cost_to_upgrade_integrator_fee_store -->
///     get_cost_to_upgrade_integrator_fee_store_view
/// get_integrator_withdrawal_fee --> get_integrator_withdrawal_fee_view
///
/// ```
///
/// ## Match operations
///
/// ```mermaid
///
/// flowchart LR
///
/// assess_taker_fees --> get_fee_share_divisor
/// assess_taker_fees --> get_taker_fee_divisor
/// assess_taker_fees --> resource_account::get_address
/// assess_taker_fees --> range_check_coin_merge
///
/// ```
///
/// # Complete DocGen index
///
/// The below index is automatically generated from source code:
module econia::incentives {

    // Uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    use aptos_framework::coin::{Self, Coin};
    use aptos_std::type_info::{Self, TypeInfo};
    use econia::resource_account;
    use econia::tablist::{Self, Tablist};
    use std::signer::address_of;
    use std::vector;

    // Uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Friends >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    friend econia::registry;
    friend econia::market;

    // Friends <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only uses >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    use aptos_framework::account;

    use econia::assets::UC;

    // Test-only uses <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Structs >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Portion of taker fees not claimed by an integrator, which are
    /// reserved for Econia.
    struct EconiaFeeStore<phantom QuoteCoinType> has key {
        /// Map from market ID to fees collected for given market,
        /// enabling duplicate checks and iterable indexing.
        map: Tablist<u64, Coin<QuoteCoinType>>
    }

    /// Incentive parameters for assorted operations.
    struct IncentiveParameters has drop, key {
        /// Utility coin type info. Corresponds to the phantom
        /// `CoinType` (`address:module::MyCoin` rather than
        /// `aptos_framework::coin::Coin<address:module::MyCoin>`) of
        /// the coin required for utility purposes. Set to `APT` at
        /// mainnet launch, later the Econia coin.
        utility_coin_type_info: TypeInfo,
        /// `Coin.value` required to register a market.
        market_registration_fee: u64,
        /// `Coin.value` required to register as an underwriter.
        underwriter_registration_fee: u64,
        /// `Coin.value` required to register as a custodian.
        custodian_registration_fee: u64,
        /// Nominal amount divisor for quote coin fee charged to takers.
        /// For example, if a transaction involves a quote coin fill of
        /// 1000000 units and the taker fee divisor is 2000, takers pay
        /// 1/2000th (0.05%) of the nominal amount (500 quote coin
        /// units) in fees. Instituted as a divisor for optimized
        /// calculations.
        taker_fee_divisor: u64,
        /// 0-indexed list from tier number to corresponding parameters.
        integrator_fee_store_tiers: vector<IntegratorFeeStoreTierParameters>
    }

    /// Fee store for a given integrator, on a given market.
    struct IntegratorFeeStore<phantom QuoteCoinType> has store {
        /// Activation tier, incremented by paying utility coins.
        tier: u8,
        /// Collected fees, in quote coins for given market.
        coins: Coin<QuoteCoinType>
    }

    /// All of an integrator's `IntegratorFeeStore`s for given
    /// `QuoteCoinType`.
    struct IntegratorFeeStores<phantom QuoteCoinType> has key {
        /// Map from market ID to `IntegratorFeeStore`, enabling
        /// duplicate checks and iterable indexing.
        map: Tablist<u64, IntegratorFeeStore<QuoteCoinType>>
    }

    /// Integrator fee store tier parameters for a given tier.
    struct IntegratorFeeStoreTierParameters has drop, store {
        /// Nominal amount divisor for taker quote coin fee reserved for
        /// integrators having activated their fee store to the given
        /// tier. For example, if a transaction involves a quote coin
        /// fill of 1000000 units and the fee share divisor at the given
        /// tier is 4000, integrators get 1/4000th (0.025%) of the
        /// nominal amount (250 quote coin units) in fees at the given
        /// tier. Instituted as a divisor for optimized calculations.
        /// May not be larger than the
        /// `IncentiveParameters.taker_fee_divisor`, since the
        /// integrator fee share is deducted from the taker fee (with
        /// the remaining proceeds going to an `EconiaFeeStore` for the
        /// given market).
        fee_share_divisor: u64,
        /// Cumulative cost, in utility coin units, to activate to the
        /// current tier. For example, if an integrator has already
        /// activated to tier 3, which has a tier activation fee of 1000
        /// units, and tier 4 has a tier activation fee of 10000 units,
        /// the integrator only has to pay 9000 units to activate to
        /// tier 4.
        tier_activation_fee: u64,
        /// Cost, in utility coin units, to withdraw from an integrator
        /// fee store. Shall never be nonzero, since a disincentive is
        /// required to prevent excessively-frequent withdrawals and
        /// thus transaction collisions with the matching engine.
        withdrawal_fee: u64
    }

    /// Container for utility coin fees collected by Econia.
    struct UtilityCoinStore<phantom CoinType> has key {
        /// Coins collected as utility fees.
        coins: Coin<CoinType>
    }

    // Structs <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Genesis parameters >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Genesis parameter.
    const MARKET_REGISTRATION_FEE: u64 =  204918032;
    /// Genesis parameter.
    const UNDERWRITER_REGISTRATION_FEE: u64 = 81967;
    /// Genesis parameter.
    const CUSTODIAN_REGISTRATION_FEE: u64 =   81967;
    /// Genesis parameter.
    const TAKER_FEE_DIVISOR: u64 =             2000;
    /// Genesis parameter.
    const FEE_SHARE_DIVISOR_0: u64 =          10000;
    /// Genesis parameter.
    const FEE_SHARE_DIVISOR_1: u64 =           8333;
    /// Genesis parameter.
    const FEE_SHARE_DIVISOR_2: u64 =           7692;
    /// Genesis parameter.
    const FEE_SHARE_DIVISOR_3: u64 =           7143;
    /// Genesis parameter.
    const FEE_SHARE_DIVISOR_4: u64 =           6667;
    /// Genesis parameter.
    const FEE_SHARE_DIVISOR_5: u64 =           6250;
    /// Genesis parameter.
    const FEE_SHARE_DIVISOR_6: u64 =           5882;
    /// Genesis parameter.
    const TIER_ACTIVATION_FEE_0: u64 =            0;
    /// Genesis parameter.
    const TIER_ACTIVATION_FEE_1: u64 =      1639344;
    /// Genesis parameter.
    const TIER_ACTIVATION_FEE_2: u64 =     24590163;
    /// Genesis parameter.
    const TIER_ACTIVATION_FEE_3: u64 =    327868852;
    /// Genesis parameter.
    const TIER_ACTIVATION_FEE_4: u64 =   4098360655;
    /// Genesis parameter.
    const TIER_ACTIVATION_FEE_5: u64 =  49180327868;
    /// Genesis parameter.
    const TIER_ACTIVATION_FEE_6: u64 = 573770491803;
    /// Genesis parameter.
    const WITHDRAWAL_FEE_0: u64 =           1639344;
    /// Genesis parameter.
    const WITHDRAWAL_FEE_1: u64 =           1557377;
    /// Genesis parameter.
    const WITHDRAWAL_FEE_2: u64 =           1475409;
    /// Genesis parameter.
    const WITHDRAWAL_FEE_3: u64 =           1393442;
    /// Genesis parameter.
    const WITHDRAWAL_FEE_4: u64 =           1311475;
    /// Genesis parameter.
    const WITHDRAWAL_FEE_5: u64 =           1229508;
    /// Genesis parameter.
    const WITHDRAWAL_FEE_6: u64 =           1147540;

    // Genesis parameters <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Error codes >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Caller is not Econia, but should be.
    const E_NOT_ECONIA: u64 = 0;
    /// Type does not correspond to an initialized coin.
    const E_NOT_COIN: u64 = 1;
    /// Passed fee store tiers vector is empty.
    const E_EMPTY_FEE_STORE_TIERS: u64 = 2;
    /// Indicated fee share divisor for given tier is too big.
    const E_FEE_SHARE_DIVISOR_TOO_BIG: u64 = 3;
    /// The indicated fee share divisor for a given tier is less than
    /// the indicated taker fee divisor.
    const E_FEE_SHARE_DIVISOR_TOO_SMALL: u64 = 4;
    /// Market registration fee is less than the minimum.
    const E_MARKET_REGISTRATION_FEE_LESS_THAN_MIN: u64 = 5;
    /// Custodian registration fee is less than the minimum.
    const E_CUSTODIAN_REGISTRATION_FEE_LESS_THAN_MIN: u64 = 6;
    /// Taker fee divisor is less than the minimum.
    const E_TAKER_DIVISOR_LESS_THAN_MIN: u64 = 7;
    /// The wrong number of fields are passed for a given tier.
    const E_TIER_FIELDS_WRONG_LENGTH: u64 = 8;
    /// The indicated tier activation fee is too small.
    const E_ACTIVATION_FEE_TOO_SMALL: u64 = 9;
    /// The indicated withdrawal fee is too big.
    const E_WITHDRAWAL_FEE_TOO_BIG: u64 = 10;
    /// The indicated withdrawal fee is too small.
    const E_WITHDRAWAL_FEE_TOO_SMALL: u64 = 11;
    /// Type is not the utility coin type.
    const E_INVALID_UTILITY_COIN_TYPE: u64 = 12;
    /// Not enough utility coins provided.
    const E_NOT_ENOUGH_UTILITY_COINS: u64 = 13;
    /// Too many integrator fee store tiers indicated.
    const E_TOO_MANY_TIERS: u64 = 14;
    /// Indicated tier is not higher than existing tier.
    const E_NOT_AN_UPGRADE: u64 = 15;
    /// An update to the incentive parameters set indicates a reduction
    /// in fee store tiers.
    const E_FEWER_TIERS: u64 = 16;
    /// The cost to activate to tier 0 is nonzero.
    const E_FIRST_TIER_ACTIVATION_FEE_NONZERO: u64 = 17;
    /// Custodian registration fee is less than the minimum.
    const E_UNDERWRITER_REGISTRATION_FEE_LESS_THAN_MIN: u64 = 18;
    /// Depositing to an integrator fee store would result in an
    /// overflow.
    const E_INTEGRATOR_FEE_STORE_OVERFLOW: u64 = 19;
    /// Depositing to an Econia fee store would result in an overflow.
    const E_ECONIA_FEE_STORE_OVERFLOW: u64 = 20;
    /// Depositing to a utility coin store would result in an overflow.
    const E_UTILITY_COIN_STORE_OVERFLOW: u64 = 21;
    /// There is no tier with given number.
    const E_INVALID_TIER: u64 = 22;
    /// Cumulative activation fee for new tier is not greater than that
    /// of current tier.
    const E_TIER_COST_NOT_INCREASE: u64 = 23;

    // Error codes <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Constants >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Buy direction flag, as defined in `market.move`.
    const BUY: bool = false;
    /// Index of fee share in vectorized representation of an
    /// `IntegratorFeeStoreTierParameters`.
    const FEE_SHARE_DIVISOR_INDEX: u64 = 0;
    /// `u64` bitmask with all bits set, generated in Python via
    /// `hex(int('1' * 64, 2))`.
    const HI_64: u64 = 0xffffffffffffffff;
    /// Maximum number of integrator fee store tiers is largest number
    /// that can fit in a `u8`.
    const MAX_INTEGRATOR_FEE_STORE_TIERS: u64 = 0xff;
    /// Minimum possible divisor for avoiding divide-by-zero error,
    /// including during denominator calculation for a `SELL` in
    /// `calculate_max_quote_match()`.
    const MIN_DIVISOR: u64 = 2;
    /// Minimum possible flat fee, required to disincentivize excessive
    /// bogus transactions.
    const MIN_FEE: u64 = 1;
    /// Number of fields in an `IntegratorFeeStoreTierParameters`.
    const N_TIER_FIELDS: u64 = 3;
    /// Sell direction flag, as defined in `market.move`.
    const SELL: bool = true;
    /// Index of tier activation fee in vectorized representation of an
    /// `IntegratorFeeStoreTierParameters`.
    const TIER_ACTIVATION_FEE_INDEX: u64 = 1;
    /// Index of withdrawal fee in vectorized representation of an
    /// `IntegratorFeeStoreTierParameters`.
    const WITHDRAWAL_FEE_INDEX: u64 = 2;

    // Constants <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // View functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[view]
    /// Calculate cost to upgrade `IntegratorFeeStore` to higher tier.
    ///
    /// # Type parameters
    ///
    /// * `QuoteCoinType`: The quote coin type for market.
    /// * `UtilityCoinType`: The utility coin type.
    ///
    /// # Parameters
    ///
    /// * `integrator_address`: Integrator address.
    /// * `market_id`: Market ID for corresponding market.
    /// * `new_tier`: Tier to upgrade to.
    ///
    /// # Returns
    ///
    /// * `u64`: Cost, in utility coins, to upgrade to given tier,
    ///   calculated as the difference between the cumulative activation
    ///   cost for each tier. For example, if it costs 1000 to activate
    ///   to tier 3 and 100 to activate to tier 1, it costs 900 to
    ///   upgrade from tier 1 to tier 3.
    ///
    /// # Aborts
    ///
    /// * `E_NOT_AN_UPGRADE`: `new_tier` is not higher than the one
    ///    that the `IntegratorFeeStore` is already activated to.
    /// * `E_TIER_COST_NOT_INCREASE`: Cumulative activation fee for new
    ///   tier is not greater than that of current tier.
    ///
    /// # Restrictions
    ///
    /// * Restricted to private view function to prevent excessive
    ///   public queries on an `IntegratorFeeStore` and thus transaction
    ///   collisions with the matching engine.
    ///
    /// # Testing
    ///
    /// * `test_get_cost_to_upgrade_integrator_fee_store_not_increase()`
    /// * `test_get_cost_to_upgrade_integrator_fee_store_not_upgrade()`
    fun get_cost_to_upgrade_integrator_fee_store_view<
        QuoteCoinType,
        UtilityCoinType
    >(
        integrator_address: address,
        market_id: u64,
        new_tier: u8
    ): u64
    acquires
        IncentiveParameters,
        IntegratorFeeStores
    {
        // Immutably borrow integrator fee stores map for given quote
        // coin type.
        let integrator_fee_stores_map_ref =
            &borrow_global<IntegratorFeeStores<QuoteCoinType>>(
                integrator_address).map;
        // Immutably borrow corresponding integrator fee store for
        // given market ID.
        let integrator_fee_store_ref = tablist::borrow(
            integrator_fee_stores_map_ref, market_id);
        // Get current tier number.
        let current_tier = integrator_fee_store_ref.tier;
        // Assert actually attempting to upgrade to new tier.
        assert!(new_tier > current_tier, E_NOT_AN_UPGRADE);
        // Get cumulative activation fee for current tier.
        let current_tier_fee = get_tier_activation_fee(current_tier);
        // Get cumulative activation fee for new tier.
        let new_tier_fee = get_tier_activation_fee(new_tier);
        // Assert new tier fee is greater than current tier fee.
        assert!(new_tier_fee > current_tier_fee, E_TIER_COST_NOT_INCREASE);
        // Return difference in cumulative cost to upgrade.
        new_tier_fee - current_tier_fee
    }

    #[view]
    /// Return custodian registration fee.
    ///
    /// # Testing
    ///
    /// * `test_init_update_get_incentives()`
    public fun get_custodian_registration_fee():
    u64
    acquires IncentiveParameters {
        borrow_global<IncentiveParameters>(@econia).custodian_registration_fee
    }

    #[view]
    /// Return integrator fee share divisor for `tier`.
    ///
    /// # Testing
    ///
    /// * `test_get_fee_share_divisor_invalid_tier()`
    /// * `test_init_update_get_incentives()`
    public fun get_fee_share_divisor(
        tier: u8
    ): u64
    acquires IncentiveParameters {
        // Borrow immutable reference to integrator fee store tiers
        // vector.
        let integrator_fee_store_tiers_ref =
            &borrow_global<IncentiveParameters>(@econia).
                integrator_fee_store_tiers;
        // Assert provided 0-indexed tier number is within range.
        assert!((tier as u64) < vector::length(integrator_fee_store_tiers_ref),
                E_INVALID_TIER);
        // Borrow immutable reference to indicated tier parameters.
        let integrator_fee_store_tier_ref = vector::borrow(
            integrator_fee_store_tiers_ref, (tier as u64));
        // Return corresponding fee share divisor.
        integrator_fee_store_tier_ref.fee_share_divisor
    }

    #[view]
    /// Return withdrawal fee for given `integrator_address` and
    /// `market_id`.
    ///
    /// # Restrictions
    ///
    /// * Restricted to private view function to prevent excessive
    ///   public queries on an `IntegratorFeeStore` and thus transaction
    ///   collisions with the matching engine.
    fun get_integrator_withdrawal_fee_view<QuoteCoinType>(
        integrator_address: address,
        market_id: u64,
    ): u64
    acquires
        IncentiveParameters,
        IntegratorFeeStores
    {
        // Borrow mutable reference to integrator fee stores map for
        // quote coin type.
        let integrator_fee_stores_map_ref = &borrow_global<
            IntegratorFeeStores<QuoteCoinType>>(integrator_address).map;
        // Borrow mutable reference to integrator fee store for given
        // market ID.
        let integrator_fee_store_ref = tablist::borrow(
            integrator_fee_stores_map_ref, market_id);
        // Return withdrawal fee for given tier.
        get_tier_withdrawal_fee(integrator_fee_store_ref.tier)
    }

    #[view]
    /// Return market registration fee.
    ///
    /// # Testing
    ///
    /// * `test_init_update_get_incentives()`
    public fun get_market_registration_fee():
    u64
    acquires IncentiveParameters {
        borrow_global<IncentiveParameters>(@econia).market_registration_fee
    }

    #[view]
    /// Return number of fee store tiers.
    ///
    /// # Testing
    ///
    /// * `test_init_update_get_incentives()`
    public fun get_n_fee_store_tiers():
    u64
    acquires IncentiveParameters {
        // Borrow immutable reference to integrator fee store tiers
        // vector.
        let integrator_fee_store_tiers_ref =
            &borrow_global<IncentiveParameters>(@econia).
                integrator_fee_store_tiers;
        // Return its vector length
        vector::length(integrator_fee_store_tiers_ref)
    }

    #[view]
    /// Return taker fee divisor.
    ///
    /// # Testing
    ///
    /// * `test_init_update_get_incentives()`
    public fun get_taker_fee_divisor():
    u64
    acquires IncentiveParameters {
        borrow_global<IncentiveParameters>(@econia).taker_fee_divisor
    }

    #[view]
    /// Return fee to activate an `IntegratorFeeStore` to given `tier`.
    ///
    /// # Testing
    ///
    /// * `test_get_tier_activation_fee_invalid_tier()`
    /// * `test_init_update_get_incentives()`
    public fun get_tier_activation_fee(
        tier: u8
    ): u64
    acquires IncentiveParameters {
        // Borrow immutable reference to integrator fee store tiers
        // vector.
        let integrator_fee_store_tiers_ref =
            &borrow_global<IncentiveParameters>(@econia).
                integrator_fee_store_tiers;
        // Assert provided 0-indexed tier number is within range.
        assert!((tier as u64) < vector::length(integrator_fee_store_tiers_ref),
                E_INVALID_TIER);
        // Borrow immutable reference to given tier.
        let integrator_fee_store_tier_ref = vector::borrow(
            integrator_fee_store_tiers_ref, (tier as u64));
        // Return its activation fee.
        integrator_fee_store_tier_ref.tier_activation_fee
    }

    #[view]
    /// Return fee to withdraw from `IntegratorFeeStore` activated to
    /// given `tier`.
    ///
    /// # Testing
    ///
    /// * `test_get_tier_withdrawal_fee_invalid_tier()`
    /// * `test_init_update_get_incentives()`
    public fun get_tier_withdrawal_fee(
        tier: u8
    ): u64
    acquires IncentiveParameters {
        // Borrow immutable reference to integrator fee store tiers
        // vector.
        let integrator_fee_store_tiers_ref =
            &borrow_global<IncentiveParameters>(@econia).
                integrator_fee_store_tiers;
        // Assert provided 0-indexed tier number is within range.
        assert!((tier as u64) < vector::length(integrator_fee_store_tiers_ref),
                E_INVALID_TIER);
        // Borrow immutable reference to given tier.
        let integrator_fee_store_tier_ref = vector::borrow(
            integrator_fee_store_tiers_ref, (tier as u64));
        // Return its withdrawal fee.
        integrator_fee_store_tier_ref.withdrawal_fee
    }

    #[view]
    /// Return underwriter registration fee.
    ///
    /// # Testing
    ///
    /// * `test_init_update_get_incentives()`
    public fun get_underwriter_registration_fee():
    u64
    acquires IncentiveParameters {
        borrow_global<IncentiveParameters>(@econia).
            underwriter_registration_fee
    }

    #[view]
    /// Return `true` if `T` is the utility coin type.
    ///
    /// # Testing
    ///
    /// * `test_init_update_get_incentives()`
    public fun is_utility_coin_type<T>():
    bool
    acquires IncentiveParameters {
        // Return if provided type info is that of the utility coin.
        type_info::type_of<T>() ==
            borrow_global<IncentiveParameters>(@econia).utility_coin_type_info
    }

    // View functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Public function wrapper for
    /// `get_cost_to_upgrade_integrator_fee_store_view()`, requiring
    /// integrator signature to prevent runtime transaction collisions.
    ///
    /// # Testing
    ///
    /// * `test_get_cost_to_upgrade_integrator_fee_store_not_increase()`
    /// * `test_get_cost_to_upgrade_integrator_fee_store_not_upgrade()`
    public fun get_cost_to_upgrade_integrator_fee_store<
        QuoteCoinType,
        UtilityCoinType
    >(
        integrator: &signer,
        market_id: u64,
        new_tier: u8,
    ): u64
    acquires
        IncentiveParameters,
        IntegratorFeeStores
    {
        get_cost_to_upgrade_integrator_fee_store_view<
            QuoteCoinType, UtilityCoinType>(
                address_of(integrator), market_id, new_tier)
    }

    /// Public function wrapper for
    /// `get_integrator_withdrawal_fee_view()`, requiring integrator
    /// signature to prevent runtime transaction collisions.
    public fun get_integrator_withdrawal_fee<QuoteCoinType>(
        integrator: &signer,
        market_id: u64,
    ): u64
    acquires
        IncentiveParameters,
        IntegratorFeeStores
    {
        get_integrator_withdrawal_fee_view<QuoteCoinType>(
                address_of(integrator), market_id)
    }

    /// Upgrade `IntegratorFeeStore` to a higher tier.
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
    /// * `new_tier`: Tier to upgrade to.
    /// * `utility_coins`: Utility coins paid for upgrade.
    public fun upgrade_integrator_fee_store<
        QuoteCoinType,
        UtilityCoinType
    >(
        integrator: &signer,
        market_id: u64,
        new_tier: u8,
        utility_coins: coin::Coin<UtilityCoinType>
    ) acquires
        IncentiveParameters,
        IntegratorFeeStores,
        UtilityCoinStore
    {
        // Get cost to upgrade to new tier.
        let cost = get_cost_to_upgrade_integrator_fee_store<QuoteCoinType,
            UtilityCoinType>(integrator, market_id, new_tier);
        // Deposit verified amount and type of utility coins.
        deposit_utility_coins_verified<UtilityCoinType>(utility_coins, cost);
        // Get integrator address.
        let integrator_address = address_of(integrator);
        // Borrow mutable reference to integrator fee stores map for
        // quote coin type.
        let integrator_fee_stores_map_ref_mut =
            &mut borrow_global_mut<IntegratorFeeStores<QuoteCoinType>>(
                integrator_address).map;
        // Borrow mutable reference to integrator fee store for given
        // market ID.
        let integrator_fee_store_ref_mut = tablist::borrow_mut(
            integrator_fee_stores_map_ref_mut, market_id);
        // Set the new tier.
        integrator_fee_store_ref_mut.tier = new_tier;
    }

    /// Assert `T` is utility coin type.
    ///
    /// # Aborts
    ///
    /// * `E_INVALID_UTILITY_COIN_TYPE`: `T` is not utility coin type.
    ///
    /// # Testing
    ///
    /// * `test_verify_utility_coin_type()`
    public fun verify_utility_coin_type<T>()
    acquires IncentiveParameters {
        assert!(is_utility_coin_type<T>(), E_INVALID_UTILITY_COIN_TYPE);
    }

    /// Withdraw `amount` of fee coins from an `EconiaFeeStore` of given
    /// `QuoteCoinType` and having `market_id`, under authority of
    /// `econia`.
    ///
    /// See inner function `withdraw_econia_fees_internal()`.
    ///
    /// Testing
    ///
    /// * `test_register_assess_withdraw()`
    /// * `test_withdraw_econia_fees_not_econia()`
    public fun withdraw_econia_fees<QuoteCoinType>(
        econia: &signer,
        market_id: u64,
        amount: u64
    ): coin::Coin<QuoteCoinType>
    acquires
        EconiaFeeStore
    {
        withdraw_econia_fees_internal<QuoteCoinType>(
            econia, market_id, false, amount)
    }

    /// Withdraw all fee coins from an `EconiaFeeStore` of given
    /// `QuoteCoinType` and having `market_id`, under authority of
    /// `econia`.
    ///
    /// See inner function `withdraw_econia_fees_internal()`.
    ///
    /// Testing
    ///
    /// * `test_register_assess_withdraw()`
    /// * `test_withdraw_econia_fees_all_not_econia()`
    public fun withdraw_econia_fees_all<QuoteCoinType>(
        econia: &signer,
        market_id: u64,
    ): coin::Coin<QuoteCoinType>
    acquires
        EconiaFeeStore
    {
        withdraw_econia_fees_internal<QuoteCoinType>(
            econia, market_id, true, 0)
    }

    /// Withdraw all fees from an `IntegratorFeeStore`.
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
    /// * `utility_coins`: Utility coins paid in order to make
    ///   withdrawal, required to disincentivize excessively frequent
    ///   withdrawals and thus transaction collisions with the matching
    ///   engine.
    ///
    /// # Returns
    ///
    /// * `coin::Coin<QuoteCoinType>`: Quote coin fees for given market.
    public fun withdraw_integrator_fees<
        QuoteCoinType,
        UtilityCoinType
    >(
        integrator: &signer,
        market_id: u64,
        utility_coins: coin::Coin<UtilityCoinType>
    ): coin::Coin<QuoteCoinType>
    acquires
        IncentiveParameters,
        IntegratorFeeStores,
        UtilityCoinStore
    {
        // Borrow mutable reference to integrator fee stores map for
        // quote coin type.
        let integrator_fee_stores_map_ref_mut = &mut borrow_global_mut<
            IntegratorFeeStores<QuoteCoinType>>(address_of(integrator)).map;
        // Borrow mutable reference to integrator fee store for given
        // market ID.
        let integrator_fee_store_ref_mut = tablist::borrow_mut(
            integrator_fee_stores_map_ref_mut, market_id);
        // Get fee to withdraw from fee store at given tier.
        let withdrawal_fee = get_tier_withdrawal_fee(
            integrator_fee_store_ref_mut.tier);
        // Deposit verified amount and type of utility coins.
        deposit_utility_coins_verified(utility_coins, withdrawal_fee);
        // Extract and return all coins in integrator fee store.
        coin::extract_all(&mut integrator_fee_store_ref_mut.coins)
    }

    /// Withdraw `amount` of utility coins from the `UtilityCoinStore`,
    /// under authority of `econia`.
    ///
    /// See inner function `withdraw_utility_coins_internal()`.
    ///
    /// # Testing
    ///
    /// * `test_deposit_withdraw_utility_coins()`
    /// * `test_register_assess_withdraw()`
    /// * `test_withdraw_utility_coins_not_econia()`
    public fun withdraw_utility_coins<UtilityCoinType>(
        econia: &signer,
        amount: u64
    ): coin::Coin<UtilityCoinType>
    acquires
        UtilityCoinStore
    {
        withdraw_utility_coins_internal<UtilityCoinType>(econia, false, amount)
    }

    /// Withdraw all utility coins from the `UtilityCoinStore`, under
    /// authority of `econia`.
    ///
    /// See inner function `withdraw_utility_coins_internal()`.
    ///
    /// # Testing
    ///
    /// * `test_deposit_withdraw_utility_coins()`
    /// * `test_register_assess_withdraw()`
    /// * `test_withdraw_utility_coins_all_not_econia()`
    public fun withdraw_utility_coins_all<UtilityCoinType>(
        econia: &signer
    ): coin::Coin<UtilityCoinType>
    acquires
        UtilityCoinStore
    {
        withdraw_utility_coins_internal<UtilityCoinType>(econia, true, 0)
    }

    // Public functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public entry functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Wrapped call to `set_incentive_parameters()`, when calling after
    /// initialization.
    ///
    /// Accepts same arguments as `set_incentive_parameters()`.
    ///
    /// # Testing
    ///
    /// * `test_init_update_get_incentives()`
    /// * `test_update_incentives_fewer_tiers()`
    public entry fun update_incentives<UtilityCoinType>(
        econia: &signer,
        market_registration_fee: u64,
        underwriter_registration_fee: u64,
        custodian_registration_fee: u64,
        taker_fee_divisor: u64,
        integrator_fee_store_tiers: vector<vector<u64>>
    ) acquires
        IncentiveParameters
    {
        set_incentive_parameters<UtilityCoinType>(econia,
            market_registration_fee, underwriter_registration_fee,
            custodian_registration_fee, taker_fee_divisor,
            &integrator_fee_store_tiers, true);
    }

    /// Wrapped call to `upgrade_integrator_fee_store()`, for paying
    /// utility coins from an `aptos_framework::Coin::CoinStore`.
    ///
    /// See wrapped function `upgrade_integrator_fee_store()`.
    ///
    /// # Testing
    ///
    /// * `upgrade_integrator_fee_store_via_coinstore()`
    public entry fun upgrade_integrator_fee_store_via_coinstore<
        QuoteCoinType,
        UtilityCoinType
    >(
        integrator: &signer,
        market_id: u64,
        new_tier: u8,
    ) acquires
        IncentiveParameters,
        IntegratorFeeStores,
        UtilityCoinStore
    {
        // Get cost to upgrade to new tier.
        let cost = get_cost_to_upgrade_integrator_fee_store<QuoteCoinType,
            UtilityCoinType>(integrator, market_id, new_tier);
        // Upgrade integrator fee store, paying cost from coin store.
        upgrade_integrator_fee_store<QuoteCoinType, UtilityCoinType>(
            integrator, market_id, new_tier, coin::withdraw(
                integrator, cost));
    }

    /// Wrapped call to `withdraw_econia_fees_to_coin_store_internal()`,
    /// similar to `withdraw_econia_fees_all()`.
    ///
    /// # Testing
    ///
    /// * `test_withdraw_to_coin_store_econia()`
    public entry fun withdraw_econia_fees_all_to_coin_store<QuoteCoinType>(
        econia: &signer,
        market_id: u64,
    ) acquires
        EconiaFeeStore
    {
        withdraw_econia_fees_to_coin_store_internal<QuoteCoinType>(
            econia, market_id, true, 0);
    }

    /// Wrapped call to `withdraw_econia_fees_to_coin_store_internal()`,
    /// similar to `withdraw_econia_fees()`.
    ///
    /// # Testing
    ///
    /// * `test_withdraw_to_coin_store_econia()`
    public entry fun withdraw_econia_fees_to_coin_store<QuoteCoinType>(
        econia: &signer,
        market_id: u64,
        amount: u64
    ) acquires
        EconiaFeeStore
    {
        withdraw_econia_fees_to_coin_store_internal<QuoteCoinType>(
            econia, market_id, false, amount);
    }

    /// Wrapped call to `get_withdraw_integrator_fees()`, for paying
    /// utility coins from an `aptos_framework::Coin::CoinStore` and
    /// depositing quote coins to one too.
    ///
    /// See wrapped function `withdraw_integrator_fees()`.
    ///
    /// # Type parameters
    ///
    /// * `QuoteCoinType`: The quote coin type for market.
    /// * `UtilityCoinType`: The utility coin type.
    ///
    /// # Parameters
    ///
    /// * `integrator`: Integrator account.
    /// * `market_id`: Market ID of corresponding market.
    ///
    /// Testing
    ///
    /// * `test_register_assess_withdraw()`
    public entry fun withdraw_integrator_fees_via_coinstores<
        QuoteCoinType,
        UtilityCoinType
    >(
        integrator: &signer,
        market_id: u64
    ) acquires
        IncentiveParameters,
        IntegratorFeeStores,
        UtilityCoinStore
    {
        // Get fee to withdraw from integrator fee coin store.
        let withdrawal_fee = get_integrator_withdrawal_fee<QuoteCoinType>(
            integrator, market_id);
        // Withdraw enough utility coins to pay fee.
        let utility_coins = coin::withdraw<UtilityCoinType>(
            integrator, withdrawal_fee);
        let quote_coins = // Withdraw integrator fees (quote coins).
            withdraw_integrator_fees<QuoteCoinType, UtilityCoinType>(
                integrator, market_id, utility_coins);
        // Get integrator address.
        let integrator_address = address_of(integrator);
        // If integrator does not have quote coin store, register one.
        if (!coin::is_account_registered<QuoteCoinType>(integrator_address))
            coin::register<QuoteCoinType>(integrator);
        // Deposit quote coins to integrator quote coin store.
        coin::deposit(address_of(integrator), quote_coins);
    }

    /// Wrapped `withdraw_utility_coins_to_coin_store_internal()` call,
    /// similar to `withdraw_utility_coins_all()`.
    ///
    /// # Testing
    ///
    /// * `test_withdraw_to_coin_store_econia()`
    public entry fun withdraw_utility_coins_all_to_coin_store<UtilityCoinType>(
        econia: &signer,
    ) acquires
        UtilityCoinStore
    {
        withdraw_utility_coins_to_coin_store_internal<UtilityCoinType>(
            econia, true, 0);
    }

    /// Wrapped `withdraw_utility_coins_to_coin_store_internal()` call,
    /// similar to `withdraw_utility_coins()`.
    ///
    /// # Testing
    ///
    /// * `test_withdraw_to_coin_store_econia()`
    public entry fun withdraw_utility_coins_to_coin_store<UtilityCoinType>(
        econia: &signer,
        amount: u64
    ) acquires
        UtilityCoinStore
    {
        withdraw_utility_coins_to_coin_store_internal<UtilityCoinType>(
            econia, false, amount);
    }

    // Public entry functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Public friend functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Assess fees after a taker fill.
    ///
    /// First attempts to assess an integrator's share of taker fees,
    /// then provides Econia with the remaining share. If the
    /// `integrator_address` does not have an `IntegratorFeeStore` for
    /// the given `market_id` and `QuoteCoinType`, all taker fees are
    /// passed on to Econia. Otherwise the integrator's fee share is
    /// determined based on their tier for the given market.
    ///
    /// # Type parameters
    ///
    /// * `QuoteCoinType`: Quote coin type for market.
    ///
    /// # Parameters
    ///
    /// * `market_id`: Market ID for corresponding market.
    /// * `integrator_address`: Integrator's address. May be
    ///   intentionally marked an address known not to be an integrator,
    ///   for example `@0x0` or `@econia`, in the service of diverting
    ///   all fees to Econia.
    /// * `taker_fee_divisor`: Taker fee divisor.
    /// * `quote_fill`: Amount of quote coins filled during taker match.
    /// * `quote_coins`: Quote coins to withdraw fees from.
    ///
    /// # Returns
    ///
    /// * `coin::Coin<QuoteCoinType>`: Remaining quote coins after fees
    ///   assessed.
    /// * `u64`: Amount of fees assessed.
    ///
    /// # Aborts
    ///
    /// * `E_INTEGRATOR_FEE_STORE_OVERFLOW`: Depositing to integrator
    ///   fee store would result in an overflow. Rather than relying on
    ///   the underlying coin operation to abort, this check is
    ///   performed to provide additional feedback in the unlikely event
    ///   that a coin with a supply far in excess of `HI_64` is the
    ///   quote coin for a market.
    /// * `E_ECONIA_FEE_STORE_OVERFLOW`: Depositing to Econia fee store
    ///   would result in an overflow per above.
    ///
    /// # Assumptions
    ///
    /// * `taker_fee_divisor` is nonzero.
    ///
    /// Testing
    ///
    /// * `test_register_assess_withdraw()`
    public(friend) fun assess_taker_fees<QuoteCoinType>(
        market_id: u64,
        integrator_address: address,
        taker_fee_divisor: u64,
        quote_fill: u64,
        quote_coins: coin::Coin<QuoteCoinType>,
    ): (
        coin::Coin<QuoteCoinType>,
        u64
    ) acquires
        EconiaFeeStore,
        IncentiveParameters,
        IntegratorFeeStores
    {
        // Declare tracker for amount of fees collected by integrator.
        let integrator_fee_share = 0;
        // Calculate total taker fee.
        let total_fee = quote_fill / taker_fee_divisor;
        // If integrator fee stores map for quote coin type exists at
        // indicated integrator address:
        if (exists<IntegratorFeeStores<QuoteCoinType>>(integrator_address)) {
            // Borrow mutable reference to integrator fee stores map.
            let integrator_fee_stores_map_ref_mut =
                &mut borrow_global_mut<IntegratorFeeStores<QuoteCoinType>>(
                    integrator_address).map;
            // Determine if the fee stores map contains an entry for the
            // given market ID.
            let contains_market_id_entry = tablist::contains(
                integrator_fee_stores_map_ref_mut, market_id);
            // If fee stores map contains an entry for given market ID:
            if (contains_market_id_entry) {
                // Borrow mutable reference to corresponding fee store.
                let integrator_fee_store_ref_mut = tablist::borrow_mut(
                    integrator_fee_stores_map_ref_mut, market_id);
                // Get fee share divisor for given tier.
                let fee_share_divisor = get_fee_share_divisor(
                    integrator_fee_store_ref_mut.tier);
                // Calculate resultant integrator fee share.
                integrator_fee_share = quote_fill / fee_share_divisor;
                // Verify merge will not overflow integrator fee store.
                range_check_coin_merge(
                    integrator_fee_share, &integrator_fee_store_ref_mut.coins,
                    E_INTEGRATOR_FEE_STORE_OVERFLOW);
                // Extract resultant amount from supplied quote coins.
                let integrator_fees =
                    coin::extract(&mut quote_coins, integrator_fee_share);
                // Merge the fees into the corresponding fee store.
                coin::merge(&mut integrator_fee_store_ref_mut.coins,
                    integrator_fees);
            }
        }; // Integrator fee share has been assessed.
        // Fee share remaining for Econia is the total taker fee amount
        // less the integrator fee share.
        let econia_fee_share = total_fee - integrator_fee_share;
        // Extract resultant amount from supplied quote coins.
        let econia_fees = coin::extract(&mut quote_coins, econia_fee_share);
        // Get fee account address.
        let fee_account_address = resource_account::get_address();
        // Borrow mutable reference to Econia fee store map for given
        // quote coin type.
        let econia_fee_store_map_ref_mut =
            &mut borrow_global_mut<EconiaFeeStore<QuoteCoinType>>(
                fee_account_address).map;
        // Borrow mutable reference to fees for given market ID.
        let econia_fee_store_coins_ref_mut = tablist::borrow_mut(
            econia_fee_store_map_ref_mut, market_id);
        // Verify merge will not overflow Econia fee store.
        range_check_coin_merge(
            econia_fee_share, econia_fee_store_coins_ref_mut,
            E_ECONIA_FEE_STORE_OVERFLOW);
        // Merge the Econia fees into the fee store.
        coin::merge(econia_fee_store_coins_ref_mut, econia_fees);
        (quote_coins, total_fee) // Return coins, fee paid.
    }

    /// Get max quote coin match amount, per user input and fee divisor.
    ///
    /// Whether a taker buy or sell, users specify a maximum quote coin
    /// amount when initiating the transaction. This amount indicates
    /// the maximum amount of quote coins they are willing to spend in
    /// the case of a taker buy, and the maximum amount of quote coins
    /// they are willing to receive in the case of a taker sell. The
    /// user-specified amount refers to the net change in taker's quote
    /// coin holdings due to matching and fees, which are assessed after
    /// matching concludes. Hence it is necessary to calculate a maximum
    /// quote match amount prior to matching.
    ///
    /// # Example buy
    ///
    /// * Taker is willing to spend 105 quote coins.
    /// * Fee is 5% (divisor of 20).
    /// * Max match is thus 100 quote coins.
    /// * Matching engine halts after 100 quote coins filled.
    /// * 5% fee then assessed, withdrawn from takers's quote coins.
    /// * Taker has spent 105 quote coins.
    ///
    /// # Example sell
    ///
    /// * Taker is willing to receive 100 quote coins.
    /// * Fee is 4% (divisor of 25).
    /// * Max match is thus 104 quote coins.
    /// * Matching engine halts after 104 quote coins filled.
    /// * 4% fee then assessed, withdrawn from quote coins received.
    /// * Taker has received 100 quote coins.
    ///
    /// # Variables
    ///
    /// The relationship between user-indicated maximum quote coin trade
    /// amount, taker fee divisor, and the amount of quote coins matched
    /// can be described with the following variables:
    ///
    /// * $\Delta_t$: Change in quote coins seen by taker.
    /// * $d_t$: Taker fee divisor.
    /// * $q_m$: Quote coins matched.
    /// * $f = \frac{q_m}{d_t}$: Fees assessed.
    ///
    /// # Equations
    ///
    /// ## Buy
    ///
    /// $$q_m = \Delta_t - f = \Delta_t - \frac{q_m}{d_t}$$
    ///
    /// $$\Delta_t = q_m + \frac{q_m}{d_t} = q_m(1 + \frac{1}{d_t})$$
    ///
    /// $$ q_m = \frac{\Delta_t}{1 + \frac{1}{d_t}} $$
    ///
    /// $$ q_m = \frac{d_t \Delta_t}{d_t + 1}$$
    ///
    /// ## Sell
    ///
    /// $$q_m = \Delta_t + f = \Delta_t + \frac{q_m}{d_t}$$
    ///
    /// $$\Delta_t = q_m - \frac{q_m}{d_t} = q_m(1 - \frac{1}{d_t})$$
    ///
    /// $$ q_m = \frac{\Delta_t}{1 - \frac{1}{d_t}} $$
    ///
    /// $$ q_m = \frac{d_t \Delta_t}{d_t - 1}$$
    ///
    /// # Overflow correction
    ///
    /// Per above, if a taker specifies that they are willing to receive
    /// `HI_64` coins during a sell, the corresponding max quote match
    /// amount will overflow a `u64`, since more than `HI_64` quote
    /// coins will need to be matched before a fee is assessed. Hence if
    /// the maximum quote match amount for a sell is calculated to be
    /// in excess of `HI_64`, the maximum quote match amount is simply
    /// corrected to `HI_64`. Here, the maximum user-specified amount
    /// that will not require such correction, $\Delta_{t, m}$ , is
    /// defined in terms of the maximum possible quote match amount
    /// $q_{m, m} = 2^{63} - 1$ (`HI_64`), and the taker fee divisor:
    ///
    /// $$ \Delta_{t, m} + \frac{q_{m, m}}{d_t} = q_{m, m} $$
    ///
    /// $$ \Delta_{t, m} = q_{m, m} - \frac{q_{m, m}}{d_t} $$
    ///
    /// Such an overflow correction does not apply in the case of a
    /// taker buy because the maximum quote match amount is strictly
    /// smaller than the user-specified change in quote coins (the
    /// amount of quote coins the taker is willing to spend).
    ///
    /// # Parameters
    ///
    /// * `direction`: `BUY` or `SELL`.
    /// * `taker_fee_divisor`: Taker fee divisor.
    /// * `max_quote_delta_user`: Maximum change in quote coins seen by
    ///   user: spent if a `BUY` and received if a `SELL`.
    ///
    /// # Returns
    ///
    /// * `u64`: Maximum amount of quote coins to match.
    ///
    /// # Assumptions
    ///
    /// * Taker fee divisor is greater than 1.
    ///
    /// # Testing
    ///
    /// * `test_calculate_max_quote_match()`
    /// * `test_calculate_max_quote_match_overflow()`
    public(friend) fun calculate_max_quote_match(
        direction: bool,
        taker_fee_divisor: u64,
        max_quote_delta_user: u64
    ): u64 {
        // Calculate numerator for both buy and sell equations.
        let numerator = (taker_fee_divisor as u128) *
            (max_quote_delta_user as u128);
        // Calculate denominator based on direction.
        let denominator = if (direction == BUY)
            (taker_fee_divisor + 1 as u128) else
            (taker_fee_divisor - 1 as u128);
        // Calculate maximum quote coins to match.
        let max_quote_match = numerator / denominator;
        // Return corrected sell overflow match amount if needed,
        if (max_quote_match > (HI_64 as u128)) HI_64 else
            (max_quote_match as u64) // Else max quote match amount.
    }

    /// Deposit `coins` of `UtilityCoinType`, verifying that the proper
    /// amount is supplied for custodian registration.
    ///
    /// # Testing
    ///
    /// * `test_deposit_registration_fees_mixed()`
    public(friend) fun deposit_custodian_registration_utility_coins<
        UtilityCoinType
    >(
        coins: coin::Coin<UtilityCoinType>
    ) acquires
        IncentiveParameters,
        UtilityCoinStore
    {
        deposit_utility_coins_verified<UtilityCoinType>(coins,
            get_custodian_registration_fee());
    }

    /// Deposit `coins` of `UtilityCoinType`, verifying that the proper
    /// amount is supplied for market registration.
    ///
    /// # Testing
    ///
    /// * `test_deposit_registration_fees_mixed()`
    public(friend) fun deposit_market_registration_utility_coins<
        UtilityCoinType
    >(
        coins: coin::Coin<UtilityCoinType>
    ) acquires
        IncentiveParameters,
        UtilityCoinStore
    {
        deposit_utility_coins_verified<UtilityCoinType>(coins,
            get_market_registration_fee());
    }

    /// Deposit `coins` of `UtilityCoinType`, verifying that the proper
    /// amount is supplied for underwriter registration.
    ///
    /// # Testing
    ///
    /// * `test_deposit_registration_fees_mixed()`
    public(friend) fun deposit_underwriter_registration_utility_coins<
        UtilityCoinType
    >(
        coins: coin::Coin<UtilityCoinType>
    ) acquires
        IncentiveParameters,
        UtilityCoinStore
    {
        deposit_utility_coins_verified<UtilityCoinType>(coins,
            get_underwriter_registration_fee());
    }

    /// Register an `EconiaFeeStore` entry for given `market_id` and
    /// `QuoteCoinType`.
    ///
    /// Testing
    ///
    /// * `test_register_assess_withdraw()`
    public(friend) fun register_econia_fee_store_entry<QuoteCoinType>(
        market_id: u64
    ) acquires
        EconiaFeeStore,
    {
        // Get fee account signer.
        let fee_account = resource_account::get_signer();
        // Get fee account address.
        let fee_account_address = address_of(&fee_account);
        // If an Econia fee store for the quote coin type has not
        // already been initialized at the fee account:
        if (!exists<EconiaFeeStore<QuoteCoinType>>(fee_account_address))
            // Move to the Econia fee account an empty one.
            move_to<EconiaFeeStore<QuoteCoinType>>(&fee_account,
                EconiaFeeStore{map: tablist::new()});
        // Borrow mutable reference to Econia fee store map for
        // given quote coin type.
        let econia_fee_store_map_ref_mut =
            &mut borrow_global_mut<EconiaFeeStore<QuoteCoinType>>(
                fee_account_address).map;
        // Declare zero coins of quote coin type
        let zero_coins = coin::zero<QuoteCoinType>();
        // Add to fee store map an entry given market ID and no coins.
        tablist::add(econia_fee_store_map_ref_mut, market_id, zero_coins);
    }

    /// Register an `IntegratorFeeStore` entry for given `integrator`.
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
    /// * `tier`: `IntegratorFeeStore` tier to activate to.
    /// * `utility_coins`: Utility coins paid to activate to given tier.
    ///
    /// Testing
    ///
    /// * `test_register_assess_withdraw()`
    /// * `test_upgrade_integrator_fee_store_via_coinstore()`
    public(friend) fun register_integrator_fee_store<
        QuoteCoinType,
        UtilityCoinType
    >(
        integrator: &signer,
        market_id: u64,
        tier: u8,
        utility_coins: coin::Coin<UtilityCoinType>
    ) acquires
        IncentiveParameters,
        IntegratorFeeStores,
        UtilityCoinStore
    {
        // Get tier activation fee for given tier.
        let tier_activation_fee = get_tier_activation_fee(tier);
        // Deposit utility coins, verifying sufficient amount provided.
        // Deposit verified amount and type of utility coins.
        deposit_utility_coins_verified(utility_coins, tier_activation_fee);
        // Get integrator address.
        let integrator_address = address_of(integrator);
        // If an integrator fee store for the quote coin type has not
        // already been initialized at the integrator account:
        if (!exists<IntegratorFeeStores<QuoteCoinType>>(integrator_address))
            // Move to the integrator account an empty one.
            move_to<IntegratorFeeStores<QuoteCoinType>>(integrator,
                IntegratorFeeStores{map: tablist::new()});
        // Declare integrator fee store for given tier, with no coins.
        let integrator_fee_store =
            IntegratorFeeStore{tier, coins: coin::zero<QuoteCoinType>()};
        // Borrow mutable reference to integrator fee stores map for
        // given quote coin type.
        let integrator_fee_stores_map_ref_mut =
            &mut borrow_global_mut<IntegratorFeeStores<QuoteCoinType>>(
                integrator_address).map;
        // Add to the map an entry having with given market ID and
        // generated integrator fee store.
        tablist::add(integrator_fee_stores_map_ref_mut, market_id,
            integrator_fee_store);
    }

    // Public friend functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Private functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    /// Deposit `coins` to the Econia `UtilityCoinStore`.
    ///
    /// # Aborts
    ///
    /// * `E_UTILITY_COIN_STORE_OVERFLOW`: Depositing to utility coin
    ///   store would result in an overflow. Rather than relying on the
    ///   underlying coin operation to abort, this check is performed to
    ///   provide additional feedback in the unlikely event that a coin
    ///   with a supply far in excess of `HI_64` is used as a utility
    ///   coin.
    ///
    /// # Testing
    ///
    /// * `test_deposit_withdraw_utility_coins()`
    fun deposit_utility_coins<UtilityCoinType>(
        coins: coin::Coin<UtilityCoinType>
    ) acquires
        UtilityCoinStore
    {
        // Get fee account address.
        let fee_account_address = resource_account::get_address();
        // Borrow mutable reference to coins in utility coin store.
        let utility_coins_ref_mut =
            &mut borrow_global_mut<UtilityCoinStore<UtilityCoinType>>(
                fee_account_address).coins;
        // Verify merge will not overflow utility coin store.
        range_check_coin_merge(coin::value(&coins),
            utility_coins_ref_mut, E_UTILITY_COIN_STORE_OVERFLOW);
        // Merge in deposited coins.
        coin::merge(utility_coins_ref_mut, coins);
    }

    /// Verify that `UtilityCoinType` is the utility coin type and that
    /// `coins` has at least the `min_amount`, then deposit all utility
    /// coins to `UtilityCoinStore`.
    ///
    /// # Aborts
    ///
    /// * `E_NOT_ENOUGH_UTILITY_COINS`: Insufficient utility coins
    ///   provided.
    ///
    /// # Testing
    ///
    /// * `test_deposit_utility_coins_verified_not_enough()`
    fun deposit_utility_coins_verified<UtilityCoinType>(
        coins: coin::Coin<UtilityCoinType>,
        min_amount: u64
    ) acquires
        IncentiveParameters,
        UtilityCoinStore
    {
        // Verify utility coin type.
        verify_utility_coin_type<UtilityCoinType>();
        // Assert sufficient utility coins provided.
        assert!(coin::value(&coins) >= min_amount, E_NOT_ENOUGH_UTILITY_COINS);
        // Deposit all utility coins to utility coin store.
        deposit_utility_coins(coins);
    }

    /// Initialize incentives during first-time publication.
    ///
    /// Uses hard-coded genesis parameters that can be updated later.
    ///
    /// # Testing
    ///
    /// * `test_init_update_get_incentives()`
    fun init_module(
        econia: &signer
    ) acquires
        IncentiveParameters
    {
        // Vectorize fee store tier parameters.
        let integrator_fee_store_tiers = vector[
            vector[FEE_SHARE_DIVISOR_0,
                   TIER_ACTIVATION_FEE_0,
                   WITHDRAWAL_FEE_0],
            vector[FEE_SHARE_DIVISOR_1,
                   TIER_ACTIVATION_FEE_1,
                   WITHDRAWAL_FEE_1],
            vector[FEE_SHARE_DIVISOR_2,
                   TIER_ACTIVATION_FEE_2,
                   WITHDRAWAL_FEE_2],
            vector[FEE_SHARE_DIVISOR_3,
                   TIER_ACTIVATION_FEE_3,
                   WITHDRAWAL_FEE_3],
            vector[FEE_SHARE_DIVISOR_4,
                   TIER_ACTIVATION_FEE_4,
                   WITHDRAWAL_FEE_4],
            vector[FEE_SHARE_DIVISOR_5,
                   TIER_ACTIVATION_FEE_5,
                   WITHDRAWAL_FEE_5],
            vector[FEE_SHARE_DIVISOR_6,
                   TIER_ACTIVATION_FEE_6,
                   WITHDRAWAL_FEE_6]];
        // // Set incentive parameters for the first time.
        // set_incentive_parameters<AptosCoin>(econia,
        //     MARKET_REGISTRATION_FEE, UNDERWRITER_REGISTRATION_FEE,
        //     CUSTODIAN_REGISTRATION_FEE, TAKER_FEE_DIVISOR,
        //     &integrator_fee_store_tiers, false);
        // Set incentive parameters for the first time.
        set_incentive_parameters<UC>(econia,
            MARKET_REGISTRATION_FEE, UNDERWRITER_REGISTRATION_FEE,
            CUSTODIAN_REGISTRATION_FEE, TAKER_FEE_DIVISOR,
            &integrator_fee_store_tiers, false);
    }

    /// Initialize a `UtilityCoinStore` under the Econia fee account.
    ///
    /// Returns without initializing if a `UtilityCoinStore` already
    /// exists for given `CoinType`, which may happen in the case of
    /// switching back to a utility coin type after having abandoned it.
    ///
    /// # Type Parameters
    ///
    /// * `CoinType`: Utility coin phantom type.
    ///
    /// # Parameters
    ///
    /// * `fee_account`: Econia fee account `signer`.
    ///
    /// # Aborts
    ///
    /// * `E_NOT_COIN`: `CoinType` does not correspond to an initialized
    ///   `aptos_framework::coin::Coin`.
    ///
    /// # Testing
    ///
    /// * `test_init_utility_coin_store()`
    /// * `test_init_utility_coin_store_not_coin()`
    fun init_utility_coin_store<CoinType>(
        fee_account: &signer
    ) {
        // Assert coin type corresponds to initialized coin.
        assert!(coin::is_coin_initialized<CoinType>(), E_NOT_COIN);
        // If a utility coin store does not already exist at account,
        if(!exists<UtilityCoinStore<CoinType>>(address_of(fee_account)))
            // Move to the fee account an initialized one.
            move_to<UtilityCoinStore<CoinType>>(fee_account,
                UtilityCoinStore{coins: coin::zero<CoinType>()});
    }

    /// Verify that attempting to merge `amount` into `target_coins`
    /// does not overflow a `u64`, aborting with `error_code` if it
    /// does.
    ///
    /// Since coins can be minted in excess of a `HI_64` supply, this
    /// is an unlikely but potentially catastrophic event, especially
    /// if the overflowed account blocks other transactions from
    /// proceeding. Hence the extra feedback in this module, in the
    /// form of a custom error code for the given operation, which
    /// allows for diagnosis in extreme cases.
    ///
    /// # Aborts
    ///
    /// * `error_code`: Proposed coin merge overflows a `u64`.
    ///
    /// # Testing
    ///
    /// * `test_range_check_coin_merge()`
    fun range_check_coin_merge<CoinType>(
        amount: u64,
        target_coins: &coin::Coin<CoinType>,
        error_code: u64
    ) {
        // Get value of target coins.
        let target_value = coin::value(target_coins);
        // Assert merge does not overflow a u64.
        assert!((amount as u128) + (target_value as u128) <= (HI_64 as u128),
            error_code);
    }

    /// Set all fields for `IncentiveParameters` under Econia account.
    ///
    /// Rather than pass-by-value a
    /// `vector<IntegratorFeeStoreTierParameters>`, mutably reassigns
    /// the values of `IncentiveParameters.integrator_fee_store_tiers`
    /// via `set_incentive_parameters_parse_tiers_vector()`.
    ///
    /// # Type Parameters
    ///
    /// * `UtilityCoinType`: Utility coin phantom type.
    ///
    /// # Parameters
    ///
    /// * `econia`: Econia account `signer`.
    /// * `market_registration_fee`: Market registration fee to set.
    /// * `underwriter_registration_fee`: Underwriter registration fee
    ///   to set.
    /// * `custodian_registration_fee`: Custodian registration fee to
    ///   set.
    /// * `taker_fee_divisor`: Taker fee divisor to set.
    /// * `integrator_fee_store_tiers_ref`: Immutable reference to
    ///   0-indexed vector of 3-element vectors, with each 3-element
    ///   vector containing fields for a corresponding
    ///   `IntegratorFeeStoreTierParameters`.
    /// * `updating`: `true` if updating incentive parameters that have
    ///   already been set, `false` if setting parameters for the first
    ///   time.
    ///
    /// # Assumptions
    ///
    /// * If `updating` is `true`, an `IncentiveParameters` already
    ///   exists at the Econia account.
    /// * If `updating` is `false`, an `IncentiveParameters` does not
    ///   exist at the Econia account.
    ///
    /// # Aborts
    ///
    /// * `E_FEWER_TIERS`: `updating` is `true` and the new parameter
    ///   set indicates a reduction in the number of fee store
    ///   activation tiers, which would mean that integrators who had
    ///   previously upgraded to the highest tier would become subject
    ///   to undefined behavior.
    fun set_incentive_parameters<UtilityCoinType>(
        econia: &signer,
        market_registration_fee: u64,
        underwriter_registration_fee: u64,
        custodian_registration_fee: u64,
        taker_fee_divisor: u64,
        integrator_fee_store_tiers_ref: &vector<vector<u64>>,
        updating: bool
    ) acquires
        IncentiveParameters
    {
        // Range check inputs.
        set_incentive_parameters_range_check_inputs(econia,
            market_registration_fee, underwriter_registration_fee,
            custodian_registration_fee, taker_fee_divisor,
            integrator_fee_store_tiers_ref);
        // Get fee account signer.
        let fee_account = resource_account::get_signer();
        // Initialize a utility coin store under the fee account (aborts
        // if not an initialized coin type).
        init_utility_coin_store<UtilityCoinType>(&fee_account);
        if (updating) { // If updating previously-set values:
            // Get number of tiers before upgrade.
            let n_old_tiers = get_n_fee_store_tiers();
            // Get number of tiers in new parameter set.
            let n_new_tiers = vector::length(integrator_fee_store_tiers_ref);
            // Assert new parameter set indicates at least as many fee
            // store tiers as the set from before the upgrade.
            assert!(n_new_tiers >= n_old_tiers, E_FEWER_TIERS);
            // Borrow a mutable reference to the incentive parameters
            // resource at the Econia account.
            let incentive_parameters_ref_mut =
                borrow_global_mut<IncentiveParameters>(address_of(econia));
            // Set integrator fee stores to empty vector before
            // moving from.
            incentive_parameters_ref_mut.integrator_fee_store_tiers =
                vector::empty();
            // Move from and drop the existing incentive parameters
            // resource at the Econia account.
            move_from<IncentiveParameters>(address_of(econia));
        };
        // Get utility coin type info.
        let utility_coin_type_info = type_info::type_of<UtilityCoinType>();
        // Declare integrator fee store tiers vector as empty.
        let integrator_fee_store_tiers = vector::empty();
        // Initialize an incentive parameters resource with
        // range-checked inputs and empty tiers vector.
        move_to<IncentiveParameters>(econia, IncentiveParameters{
            utility_coin_type_info, market_registration_fee,
            underwriter_registration_fee, custodian_registration_fee,
            taker_fee_divisor, integrator_fee_store_tiers});
        // Borrow a mutable reference to the incentive parameters
        // resource at the Econia account.
        let incentive_parameters_ref_mut =
            borrow_global_mut<IncentiveParameters>(@econia);
        // Parse in integrator fee store tier parameters.
        set_incentive_parameters_parse_tiers_vector(
            taker_fee_divisor, integrator_fee_store_tiers_ref,
            &mut incentive_parameters_ref_mut.integrator_fee_store_tiers);
    }

    /// Parse vectorized fee store tier parameters passed to
    /// `set_incentive_parameters()`.
    ///
    /// * `taker_fee_divisor`: Taker fee divisor just set.
    /// * `integrator_fee_store_tiers_ref`: Immutable reference to
    ///   0-indexed vector of 3-element vectors, with each 3-element
    ///   vector containing fields for a corresponding
    ///   `IntegratorFeeStoreTierParameters`.
    /// * `integrator_fee_store_tiers_target_ref_mut`: Mutable reference
    ///   to the `IncentiveParameters.integrator_fee_store_tiers` field
    ///   to parse into.
    ///
    /// # Aborts
    ///
    /// * `E_TIER_FIELDS_WRONG_LENGTH`: An indicated inner vector from
    ///   `integrator_fee_store_tiers_ref` is the wrong length.
    /// * `E_FEE_SHARE_DIVISOR_TOO_BIG`: Fee share divisor does not
    ///   decrease with tier number.
    /// * `E_FEE_SHARE_DIVISOR_TOO_SMALL`: A fee share divisor is less
    ///   than taker fee divisor.
    /// * `E_FIRST_TIER_ACTIVATION_FEE_NONZERO`: Tier activation fee for
    ///   first tier is nonzero.
    /// * `E_ACTIVATION_FEE_TOO_SMALL`: Tier activation fee does not
    ///   increase with tier number.
    /// * `E_WITHDRAWAL_FEE_TOO_BIG`: Withdrawal fee does not decrease
    ///   with tier number.
    /// * `E_WITHDRAWAL_FEE_TOO_SMALL`: The withdrawal fee for a given
    ///   tier does not meet minimum threshold.
    ///
    /// # Assumptions
    ///
    /// * `taker_fee_divisor` has been range-checked via
    ///   `set_incentive_parameters_range_check_inputs()`.
    /// * An `IncentiveParameters` exists at the Econia account.
    /// * `integrator_fee_store_tiers_ref` does not indicate an empty
    ///   vector.
    /// * `integrator_fee_store_tiers_target_ref_mut` indicates an empty
    ///   vector.
    ///
    /// # Testing
    ///
    /// * `test_set_incentive_params_parse_tiers_vec_activate_0()`
    /// * `test_set_incentive_params_parse_tiers_vec_activate_1()`
    /// * `test_set_incentive_params_parse_tiers_vec_divisor_big_0()`
    /// * `test_set_incentive_params_parse_tiers_vec_divisor_big_1()`
    /// * `test_set_incentive_params_parse_tiers_vec_divisor_small()`
    /// * `test_set_incentive_params_parse_tiers_vec_withdraw_big_0()`
    /// * `test_set_incentive_params_parse_tiers_vec_withdraw_big_1()`
    /// * `test_set_incentive_params_parse_tiers_vec_withdraw_small()`
    /// * `test_set_incentive_params_parse_tiers_vec_wrong_length()`
    fun set_incentive_parameters_parse_tiers_vector(
        taker_fee_divisor: u64,
        integrator_fee_store_tiers_ref: &vector<vector<u64>>,
        integrator_fee_store_tiers_target_ref_mut:
            &mut vector<IntegratorFeeStoreTierParameters>
    ) {
        // Initialize tracker variables for the fee store parameters of
        // the last parsed tier.
        let (divisor_last, activation_fee_last, withdrawal_fee_last) = (
                    HI_64,                   0,               HI_64);
        // Get number of specified integrator fee store tiers.
        let n_tiers = vector::length(integrator_fee_store_tiers_ref);
        let i = 0; // Declare counter for loop variable.
        while (i < n_tiers) { // Loop over all specified tiers
            // Borrow immutable reference to fields for given tier.
            let tier_fields_ref =
                vector::borrow(integrator_fee_store_tiers_ref, i);
            // Assert containing vector is correct length.
            assert!(vector::length(tier_fields_ref) == N_TIER_FIELDS,
                E_TIER_FIELDS_WRONG_LENGTH);
            // Borrow immutable reference to fee share divisor.
            let fee_share_divisor_ref =
                vector::borrow(tier_fields_ref, FEE_SHARE_DIVISOR_INDEX);
            // Assert indicated fee share divisor is less than divisor
            // from last tier.
            assert!(*fee_share_divisor_ref < divisor_last,
                E_FEE_SHARE_DIVISOR_TOO_BIG);
            // Assert indicated fee share divisor is greater than or
            // equal to taker fee divisor.
            assert!(*fee_share_divisor_ref >= taker_fee_divisor,
                E_FEE_SHARE_DIVISOR_TOO_SMALL);
            // Borrow immutable reference to tier activation fee.
            let tier_activation_fee_ref =
                vector::borrow(tier_fields_ref, TIER_ACTIVATION_FEE_INDEX);
            if (i == 0) { // If parsing parameters for first tier:
                // Assert activation fee is 0.
                assert!(*tier_activation_fee_ref == 0,
                    E_FIRST_TIER_ACTIVATION_FEE_NONZERO);
            } else { // If parameters for tier that is not first:
                // Assert activation fee greater than that of last tier.
                assert!(*tier_activation_fee_ref > activation_fee_last,
                    E_ACTIVATION_FEE_TOO_SMALL);
            };
            // Borrow immutable reference to withdrawal fee.
            let withdrawal_fee_ref =
                vector::borrow(tier_fields_ref, WITHDRAWAL_FEE_INDEX);
            // Assert withdrawal fee is less than that of last tier.
            assert!(*withdrawal_fee_ref < withdrawal_fee_last,
                E_WITHDRAWAL_FEE_TOO_BIG);
            // Assert withdrawal fee meets minimum threshold.
            assert!(*withdrawal_fee_ref >= MIN_FEE,
                    E_WITHDRAWAL_FEE_TOO_SMALL);
            // Mark indicated tier in target tiers vector.
            vector::push_back(integrator_fee_store_tiers_target_ref_mut,
                IntegratorFeeStoreTierParameters{
                    fee_share_divisor: *fee_share_divisor_ref,
                    tier_activation_fee: *tier_activation_fee_ref,
                    withdrawal_fee: *withdrawal_fee_ref});
            // Store divisor for comparison during next iteration.
            divisor_last = *fee_share_divisor_ref;
            // Store activation fee to compare during next iteration.
            activation_fee_last = *tier_activation_fee_ref;
            // Store withdrawal fee to compare during next iteration.
            withdrawal_fee_last = *withdrawal_fee_ref;
            i = i + 1; // Increment loop counter
        };
    }

    /// Range check inputs for `set_incentive_parameters()`.
    ///
    /// # Parameters
    ///
    /// * `econia`: Econia account `signer`.
    /// * `market_registration_fee`: Market registration fee to set.
    /// * `underwriter_registration_fee`: Underwriter registration fee
    ///   to set.
    /// * `custodian_registration_fee`: Custodian registration fee to
    ///   set.
    /// * `taker_fee_divisor`: Taker fee divisor to set.
    /// * `integrator_fee_store_tiers_ref`: Immutable reference to
    ///   0-indexed vector of 3-element vectors, with each 3-element
    ///   vector containing fields for a corresponding
    ///   `IntegratorFeeStoreTierParameters`.
    ///
    /// # Aborts
    ///
    /// * `E_NOT_ECONIA`: `econia` is not Econia account.
    /// * `E_MARKET_REGISTRATION_FEE_LESS_THAN_MIN`:
    ///   `market_registration_fee` does not meet minimum threshold.
    /// * `E_UNDERWRITER_REGISTRATION_FEE_LESS_THAN_MIN`:
    ///   `underwriter_registration_fee` does not meet minimum
    ///   threshold.
    /// * `E_CUSTODIAN_REGISTRATION_FEE_LESS_THAN_MIN`:
    ///   `custodian_registration_fee` does not meet minimum threshold.
    /// * `E_TAKER_DIVISOR_LESS_THAN_MIN`: `taker_fee_divisor` does not
    ///   meet minimum threshold.
    /// * `E_EMPTY_FEE_STORE_TIERS`: `integrator_fee_store_tiers_ref`
    ///   indicates an empty vector.
    /// * `E_TOO_MANY_TIERS`: `integrator_fee_store_tiers_ref` indicates
    ///   a vector that is too long.
    ///
    /// # Testing
    ///
    /// * `test_set_incentive_params_range_check_inputs_custodian_fee()`
    /// * `test_set_incentive_params_range_check_inputs_divisor()`
    /// * `test_set_incentive_params_range_check_inputs_market_fee()`
    /// * `test_set_incentive_params_range_check_inputs_not_econia()`
    /// * `test_set_incentive_params_range_check_inputs_underwriter()`
    /// * `test_set_incentive_params_range_check_inputs_vector_empty()`
    /// * `test_set_incentive_params_range_check_inputs_vector_long()`
    fun set_incentive_parameters_range_check_inputs(
        econia: &signer,
        market_registration_fee: u64,
        underwriter_registration_fee: u64,
        custodian_registration_fee: u64,
        taker_fee_divisor: u64,
        integrator_fee_store_tiers_ref: &vector<vector<u64>>
    ) {
        // Assert signer is from Econia account.
        assert!(address_of(econia) == @econia, E_NOT_ECONIA);
        // Assert market registration fee meets minimum threshold.
        assert!(market_registration_fee >= MIN_FEE,
            E_MARKET_REGISTRATION_FEE_LESS_THAN_MIN);
        // Assert underwriter registration fee meets minimum threshold.
        assert!(underwriter_registration_fee >= MIN_FEE,
            E_UNDERWRITER_REGISTRATION_FEE_LESS_THAN_MIN);
        // Assert custodian registration fee meets minimum threshold.
        assert!(custodian_registration_fee >= MIN_FEE,
            E_CUSTODIAN_REGISTRATION_FEE_LESS_THAN_MIN);
        // Assert taker fee divisor is meets minimum threshold.
        assert!(taker_fee_divisor >= MIN_DIVISOR,
            E_TAKER_DIVISOR_LESS_THAN_MIN);
        // Assert integrator fee store parameters vector not empty.
        assert!(!vector::is_empty(integrator_fee_store_tiers_ref),
            E_EMPTY_FEE_STORE_TIERS);
        // Assert integrator fee store parameters vector not too long.
        assert!(vector::length(integrator_fee_store_tiers_ref) <=
            MAX_INTEGRATOR_FEE_STORE_TIERS, E_TOO_MANY_TIERS);
    }

    /// Withdraw all fee coins from an `EconiaFeeStore` for given
    /// `QuoteCoinType` and `market_id` if `all` is `true`, otherwise
    /// withdraw `amount` (which may correspond to all coins), aborting
    /// if `account` is not Econia.
    ///
    /// # Aborts
    ///
    /// * `E_NOT_ECONIA`: `account` is not Econia account.
    fun withdraw_econia_fees_internal<QuoteCoinType>(
        account: &signer,
        market_id: u64,
        all: bool,
        amount: u64
    ): coin::Coin<QuoteCoinType>
    acquires
        EconiaFeeStore
    {
        // Assert account is Econia.
        assert!(address_of(account) == @econia, E_NOT_ECONIA);
        // Get fee account address.
        let fee_account_address = resource_account::get_address();
        // Borrow mutable reference to Econia fee store map for given
        // quote coin type.
        let econia_fee_store_map_ref_mut =
            &mut borrow_global_mut<EconiaFeeStore<QuoteCoinType>>(
                fee_account_address).map;
        // Borrow mutable reference to fees for given market ID.
        let fee_coins_ref_mut = tablist::borrow_mut(
            econia_fee_store_map_ref_mut, market_id);
        // If flagged to extract all, extract all and return.
        if (all) coin::extract_all(fee_coins_ref_mut) else
            // Else extract specified amount and return.
            coin::extract(fee_coins_ref_mut, amount)
    }

    /// Wrapped call to `withdraw_econia_fees_internal()`, for
    /// depositing withdrawn coins to an
    /// `aptos_framework::coin::CoinStore`.
    ///
    /// # Testing
    ///
    /// * `test_withdraw_to_coin_store_econia()`
    fun withdraw_econia_fees_to_coin_store_internal<QuoteCoinType>(
        econia: &signer,
        market_id: u64,
        all: bool,
        amount: u64
    ) acquires EconiaFeeStore {
        // Withdraw coins from fee store, verifying Econia signer.
        let coins = withdraw_econia_fees_internal<QuoteCoinType>(
            econia, market_id, all, amount);
        // If Econia does not have coin store for coin type:
        if (!coin::is_account_registered<QuoteCoinType>(@econia))
            // Register one.
            coin::register<QuoteCoinType>(econia);
        // Deposit quote coins to coin store under Econia account.
        coin::deposit(@econia, coins);
    }

    /// Withdraw all utility coins from the `UtilityCoinStore` if `all`
    /// is `true`, otherwise withdraw `amount` (which may correspond to
    /// all coins), aborting if `account` is not Econia.
    ///
    /// # Aborts
    ///
    /// * `E_NOT_ECONIA`: `account` is not Econia account.
    fun withdraw_utility_coins_internal<UtilityCoinType>(
        account: &signer,
        all: bool,
        amount: u64
    ): coin::Coin<UtilityCoinType>
    acquires
        UtilityCoinStore
    {
        // Assert account is Econia.
        assert!(address_of(account) == @econia, E_NOT_ECONIA);
        // Get fee account address.
        let fee_account_address = resource_account::get_address();
        // Borrow mutable reference to coins in utility coin store.
        let utility_coins_ref_mut =
            &mut borrow_global_mut<UtilityCoinStore<UtilityCoinType>>(
                fee_account_address).coins;
        // If flagged to extract all, extract all and return.
        if (all) coin::extract_all(utility_coins_ref_mut) else
            // Else extract specified amount and return.
            coin::extract(utility_coins_ref_mut, amount)
    }

    /// Wrapped call to `withdraw_utility_coins_internal()`, for
    /// depositing withdrawn coins to an
    /// `aptos_framework::coin::CoinStore`.
    ///
    /// # Testing
    ///
    /// * `test_withdraw_to_coin_store_econia()`
    fun withdraw_utility_coins_to_coin_store_internal<UtilityCoinType>(
        econia: &signer,
        all: bool,
        amount: u64
    ) acquires UtilityCoinStore {
        // Withdraw coins from fee store, verifying Econia signer.
        let coins = withdraw_utility_coins_internal<UtilityCoinType>(
            econia, all, amount);
        // If Econia does not have coin store for coin type:
        if (!coin::is_account_registered<UtilityCoinType>(@econia))
            // Register one.
            coin::register<UtilityCoinType>(econia);
        // Deposit utility coins to coin store under Econia account.
        coin::deposit(@econia, coins);
    }

    // Private functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Test-only functions >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test_only]
    /// Return `BUY`, for testing synchronization with `market.move`.
    public fun get_BUY_test(): bool {BUY}

    #[test_only]
    /// Return amount of quote coins in `EconiaFeeStore` for given
    /// `QuoteCoinType` and `market_id`.
    ///
    /// # Restrictions
    ///
    /// * Restricted to test-only to prevent excessive public queries
    ///   and thus transaction collisions.
    public fun get_econia_fee_store_balance_test<QuoteCoinType>(
        market_id: u64
    ): u64
    acquires
        EconiaFeeStore
    {
        coin::value(tablist::borrow(
            &borrow_global<EconiaFeeStore<QuoteCoinType>>(
                resource_account::get_address()).map, market_id))
    }

    #[test_only]
    /// Return amount of quote coins in `IntegratorFeeStore` for given
    /// `QuoteCoinType` and `market_id`.
    ///
    /// # Restrictions
    ///
    /// * Restricted to test-only to prevent excessive public queries
    ///   and thus transaction collisions.
    public fun get_integrator_fee_store_balance_test<QuoteCoinType>(
        integrator: address,
        market_id: u64
    ): u64
    acquires
        IntegratorFeeStores
    {
        coin::value(&tablist::borrow(
            &borrow_global<IntegratorFeeStores<QuoteCoinType>>(integrator).map,
                market_id).coins)
    }

    #[test_only]
    /// Return activation tier of `IntegratorFeeStore` for given
    /// `QuoteCoinType` and `market_id`.
    ///
    /// # Restrictions
    ///
    /// * Restricted to test-only to prevent excessive public queries
    ///   and thus transaction collisions.
    public fun get_integrator_fee_store_tier_test<QuoteCoinType>(
        integrator: address,
        market_id: u64
    ): u8
    acquires
        IntegratorFeeStores
    {
        tablist::borrow(&borrow_global<IntegratorFeeStores<QuoteCoinType>>(
            integrator).map, market_id).tier
    }

    #[test_only]
    /// Return `SELL`, for testing synchronization with `market.move`.
    public fun get_SELL_test(): bool {SELL}

    #[test_only]
    /// Return amount of utility coins in `UtilityCoinStore` for utility
    /// coin type `UC`.
    ///
    /// # Restrictions
    ///
    /// * Restricted to test-only to prevent excessive public queries
    ///   and thus transaction collisions.
    public fun get_utility_coin_store_balance_test():
    u64
    acquires
        UtilityCoinStore
    {
        coin::value(&borrow_global<UtilityCoinStore<UC>>(
            resource_account::get_address()).coins)
    }

    #[test_only]
    /// Initialize incentives with `UC` utility coin type.
    public fun init_test()
    acquires
        IncentiveParameters
    {
        assets::init_coin_types_test(); // Initialize coin types.
        // Get signer for Econia account.
        let econia = account::create_signer_with_capability(
            &account::create_test_signer_cap(@econia));
        resource_account::init_test(); // Init fee account.
        // Vectorize fee store tier parameters.
        let integrator_fee_store_tiers = vector[
            vector[
                FEE_SHARE_DIVISOR_0,
                TIER_ACTIVATION_FEE_0,
                WITHDRAWAL_FEE_0
            ],
            vector[
                FEE_SHARE_DIVISOR_1,
                TIER_ACTIVATION_FEE_1,
                WITHDRAWAL_FEE_1
            ]
        ];
        // Initialize incentives with mock utility coin.
        set_incentive_parameters<UC>(&econia, MARKET_REGISTRATION_FEE,
            UNDERWRITER_REGISTRATION_FEE, CUSTODIAN_REGISTRATION_FEE,
            TAKER_FEE_DIVISOR, &integrator_fee_store_tiers, false);
    }

    #[test_only]
    /// Return `true` if `integrator` has an `IntegratorFeeStore` for
    /// given `QuoteCoinType` and `market_id`.
    ///
    /// # Restrictions
    ///
    /// * Restricted to test-only to prevent excessive public queries
    ///   and thus transaction collisions.
    public fun has_integrator_fee_store_test<QuoteCoinType>(
        integrator: address,
        market_id: u64
    ): bool
    acquires
        IntegratorFeeStores
    {
        // Return false if integrator does not have integrator fee
        // stores map for given quote coin type.
        if (!exists<IntegratorFeeStores<QuoteCoinType>>(integrator))
            return false;
        // Immutably borrow integrator fee stores map.
        let integrator_fee_stores_map_ref =
            &borrow_global<IntegratorFeeStores<QuoteCoinType>>(integrator).map;
        // Return true if integrator fee stores map has entry for given
        // market ID.
        tablist::contains(integrator_fee_stores_map_ref, market_id)
    }

    // Test-only functions <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

    // Tests >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    #[test]
    /// Verify max quote match amounts.
    fun test_calculate_max_quote_match() {
        // Declare matching parameters.
        let direction = BUY;
        let taker_fee_divisor = 20;
        let max_quote_delta_user = 105;
        let max_quote_match_expected = 100;
        // Calculate max quote match value.
        let max_quote_match = calculate_max_quote_match(
            direction, taker_fee_divisor, max_quote_delta_user);
        // Assert calculated amount.
        assert!(max_quote_match == max_quote_match_expected, 0);
        // Repeat for a sell.
        direction = SELL;
        taker_fee_divisor = 25;
        max_quote_delta_user = 100;
        max_quote_match_expected = 104;
        // Calculate max quote match value.
        max_quote_match = calculate_max_quote_match(
            direction, taker_fee_divisor, max_quote_delta_user);
        // Assert calculated amount.
        assert!(max_quote_match == max_quote_match_expected, 0);
    }

    #[test]
    /// Verify correction for overflowing quote match amount.
    fun test_calculate_max_quote_match_overflow() {
        // Declare matching parameters.
        let direction = SELL;
        // Define taker fee divisor as a power of two to avoid
        // truncation.
        let taker_fee_divisor = 16;
        let max_quote_delta_user = HI_64 - HI_64 / taker_fee_divisor;
        // Calculate max quote match value for critical amount.
        let max_quote_match = calculate_max_quote_match(
            direction, taker_fee_divisor, max_quote_delta_user);
        // Assert calculated amount.
        assert!(max_quote_match == HI_64, 0);
        // Calculate max quote match value for one more than critical
        // amount.
        max_quote_match = calculate_max_quote_match(
            direction, taker_fee_divisor, max_quote_delta_user + 1);
        // Assert corrected amount.
        assert!(max_quote_match == HI_64, 0);
        // Calculate max quote match value for highest possible input.
        max_quote_match = calculate_max_quote_match(
            direction, taker_fee_divisor, HI_64);
        // Assert corrected amount.
        assert!(max_quote_match == HI_64, 0);
        // Calculate max quote match value for one less than critical
        // amount.
        max_quote_match = calculate_max_quote_match(
            direction, taker_fee_divisor, max_quote_delta_user - 1);
        // Calculate expected return.
        let max_quote_match_expected = ((taker_fee_divisor as u128) *
             ((max_quote_delta_user - 1) as u128)) /
             ((taker_fee_divisor - 1) as u128);
        // Assert expected return below max possible u64.
        assert!(max_quote_match_expected < (HI_64 as u128), 0);
        // Assert expected return.
        assert!(max_quote_match == (max_quote_match_expected as u64), 0);
    }

    #[test]
    /// Verify deposits for mixed registration fees.
    fun test_deposit_registration_fees_mixed()
    acquires
        IncentiveParameters,
        UtilityCoinStore
    {
        init_test(); // Initialize incentives.
        // Get registration fees.
        let market_registration_fee = get_market_registration_fee();
        let underwriter_registration_fee = get_underwriter_registration_fee();
        let custodian_registration_fee = get_custodian_registration_fee();
        // Deposit fees.
        deposit_market_registration_utility_coins<UC>(assets::mint_test(
            market_registration_fee));
        deposit_underwriter_registration_utility_coins<UC>(assets::mint_test(
            underwriter_registration_fee));
        deposit_custodian_registration_utility_coins<UC>(assets::mint_test(
            custodian_registration_fee));
        // Assert total amount.
        assert!(get_utility_coin_store_balance_test() ==
            MARKET_REGISTRATION_FEE + UNDERWRITER_REGISTRATION_FEE +
            CUSTODIAN_REGISTRATION_FEE, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_NOT_ENOUGH_UTILITY_COINS)]
    /// Verify failure for not enough utility coins.
    fun test_deposit_utility_coins_verified_not_enough()
    acquires
        IncentiveParameters,
        UtilityCoinStore
    {
        init_test(); // Init incentives.
        // Attempt invalid invocation.
        deposit_utility_coins_verified(coin::zero<UC>(), 1);
    }

    #[test(econia = @econia)]
    /// Verify deposit and withdrawal of utility coins.
    fun test_deposit_withdraw_utility_coins(
        econia: &signer
    ) acquires
        IncentiveParameters,
        UtilityCoinStore
    {
        init_test(); // Initialize incentives.
        // Deposit utility coins.
        deposit_utility_coins(assets::mint_test<UC>(100));
        // Withdraw some utility coins.
        let coins = withdraw_utility_coins<UC>(econia, 40);
        assert!(coin::value(&coins) == 40, 0); // Assert value.
        assets::burn(coins); // Burn coins
        // Withdraw all utility coins
        coins = withdraw_utility_coins_all<UC>(econia);
        assert!(coin::value(&coins) == 60, 0); // Assert value.
        assets::burn(coins); // Burn coins
    }

    #[test(integrator = @user)]
    #[expected_failure(abort_code = E_TIER_COST_NOT_INCREASE)]
    /// Verify expected failure for not an increase in tier cost.
    fun test_get_cost_to_upgrade_integrator_fee_store_not_increase(
        integrator: &signer
    ) acquires
        IncentiveParameters,
        IntegratorFeeStores,
        UtilityCoinStore
    {
        init_test(); // Init incentives.
        // Declare market ID, tier numbers.
        let (market_id, tier_0, tier_1) = (0, 0, 1);
        // Register to tier 0.
        register_integrator_fee_store<QC, UC>(integrator, market_id, tier_0,
            assets::mint_test(get_tier_activation_fee(tier_0)));
        // Get cumulative fee to activate to tier 0.
        let tier_0_fee = get_tier_activation_fee(tier_0);
        // Mutably borrow incentive parameters.
        let incentive_parameters_ref_mut =
            borrow_global_mut<IncentiveParameters>(@econia);
        // Mutably borrow integrator fee store tiers.
        let integrator_fee_store_tiers_ref_mut =
            &mut incentive_parameters_ref_mut.integrator_fee_store_tiers;
        // Mutably borrow tier 1.
        let tier_1_ref_mut = vector::borrow_mut(
            integrator_fee_store_tiers_ref_mut, (tier_1 as u64));
        // Manually set fee to that of previous tier.
        tier_1_ref_mut.tier_activation_fee = tier_0_fee;
        // Attempt invalid query against modified tier 1.
        get_cost_to_upgrade_integrator_fee_store<QC, UC>(
            integrator, market_id, tier_1);
    }

    #[test(integrator = @user)]
    #[expected_failure(abort_code = E_NOT_AN_UPGRADE)]
    /// Verify expected failure for not an upgrade.
    fun test_get_cost_to_upgrade_integrator_fee_store_not_upgrade(
        integrator: &signer
    ) acquires
        IncentiveParameters,
        IntegratorFeeStores,
        UtilityCoinStore
    {
        init_test(); // Init incentives.
        let (market_id, tier) = (0, 0); // Declare market ID, tier.
        // Register to given tier.
        register_integrator_fee_store<QC, UC>(integrator, market_id, tier,
            assets::mint_test(get_tier_activation_fee(tier)));
        // Attempt invalid query.
        get_cost_to_upgrade_integrator_fee_store<QC, UC>(
            integrator, market_id, tier);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_TIER)]
    /// Verify failure for invalid tier number.
    fun test_get_fee_share_divisor_invalid_tier()
    acquires IncentiveParameters {
        init_test(); // Init for testing.
        // Get maximum 0-indexed tier number.
        let max_tier_number = (get_n_fee_store_tiers() as u8) - 1;
        // Attempt invalid invocation.
        get_fee_share_divisor(max_tier_number + 1);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_TIER)]
    fun test_get_tier_activation_fee_invalid_tier()
    acquires IncentiveParameters {
        init_test(); // Init for testing.
        // Get maximum 0-indexed tier number.
        let max_tier_number = (get_n_fee_store_tiers() as u8) - 1;
        // Attempt invalid invocation.
        get_tier_activation_fee(max_tier_number + 1);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_TIER)]
    fun test_get_tier_withdrawal_fee_invalid_tier()
    acquires IncentiveParameters {
        init_test(); // Init for testing.
        // Get maximum 0-indexed tier number.
        let max_tier_number = (get_n_fee_store_tiers() as u8) - 1;
        // Attempt invalid invocation.
        get_tier_withdrawal_fee(max_tier_number + 1);
    }

    #[test(econia = @econia)]
    /// Verify initializing, updating, and getting incentive parameters.
    fun test_init_update_get_incentives(
        econia: &signer
    ) acquires
        IncentiveParameters
    {
        assets::init_coin_types_test(); // Init coin types.
        resource_account::init_test(); // Init fee account.
        init_module(econia); // Initialize incentives.
        // Assert state.
        verify_utility_coin_type<AptosCoin>();
        assert!(!is_utility_coin_type<QC>(), 0);
        assert!(get_market_registration_fee() == MARKET_REGISTRATION_FEE, 0);
        assert!(get_underwriter_registration_fee() ==
            UNDERWRITER_REGISTRATION_FEE, 0);
        assert!(get_custodian_registration_fee() ==
            CUSTODIAN_REGISTRATION_FEE, 0);
        assert!(get_taker_fee_divisor() == TAKER_FEE_DIVISOR, 0);
        assert!(get_n_fee_store_tiers() == 7, 0);
        assert!(get_fee_share_divisor((0 as u8)) == FEE_SHARE_DIVISOR_0, 0);
        assert!(get_fee_share_divisor((1 as u8)) == FEE_SHARE_DIVISOR_1, 0);
        assert!(get_fee_share_divisor((2 as u8)) == FEE_SHARE_DIVISOR_2, 0);
        assert!(get_fee_share_divisor((3 as u8)) == FEE_SHARE_DIVISOR_3, 0);
        assert!(get_fee_share_divisor((4 as u8)) == FEE_SHARE_DIVISOR_4, 0);
        assert!(get_fee_share_divisor((5 as u8)) == FEE_SHARE_DIVISOR_5, 0);
        assert!(get_fee_share_divisor((6 as u8)) == FEE_SHARE_DIVISOR_6, 0);
        assert!(get_tier_activation_fee((0 as u8)) ==
            TIER_ACTIVATION_FEE_0, 0);
        assert!(get_tier_activation_fee((1 as u8)) ==
            TIER_ACTIVATION_FEE_1, 0);
        assert!(get_tier_activation_fee((2 as u8)) ==
            TIER_ACTIVATION_FEE_2, 0);
        assert!(get_tier_activation_fee((3 as u8)) ==
            TIER_ACTIVATION_FEE_3, 0);
        assert!(get_tier_activation_fee((4 as u8)) ==
            TIER_ACTIVATION_FEE_4, 0);
        assert!(get_tier_activation_fee((5 as u8)) ==
            TIER_ACTIVATION_FEE_5, 0);
        assert!(get_tier_activation_fee((6 as u8)) ==
            TIER_ACTIVATION_FEE_6, 0);
        assert!(get_tier_withdrawal_fee((0 as u8)) == WITHDRAWAL_FEE_0, 0);
        assert!(get_tier_withdrawal_fee((1 as u8)) == WITHDRAWAL_FEE_1, 0);
        assert!(get_tier_withdrawal_fee((2 as u8)) == WITHDRAWAL_FEE_2, 0);
        assert!(get_tier_withdrawal_fee((3 as u8)) == WITHDRAWAL_FEE_3, 0);
        assert!(get_tier_withdrawal_fee((4 as u8)) == WITHDRAWAL_FEE_4, 0);
        assert!(get_tier_withdrawal_fee((5 as u8)) == WITHDRAWAL_FEE_5, 0);
        assert!(get_tier_withdrawal_fee((6 as u8)) == WITHDRAWAL_FEE_6, 0);
        assert!(exists<UtilityCoinStore<AptosCoin>>(
            resource_account::get_address()), 0);
        // Update incentive parameters.
        let market_registration_fee =           MARKET_REGISTRATION_FEE + 5;
        let underwriter_registration_fee = UNDERWRITER_REGISTRATION_FEE + 5;
        let custodian_registration_fee =     CUSTODIAN_REGISTRATION_FEE + 5;
        let taker_fee_divisor =                       TAKER_FEE_DIVISOR + 5;
        let fee_share_divisor_0 =                   FEE_SHARE_DIVISOR_0 + 5;
        let tier_activation_fee_0 =                   TIER_ACTIVATION_FEE_0;
        let withdrawal_fee_0 =                         WITHDRAWAL_FEE_0 + 5;
        let fee_share_divisor_1 =                   fee_share_divisor_0 - 1;
        let tier_activation_fee_1 =               tier_activation_fee_0 + 1;
        let withdrawal_fee_1 =                         withdrawal_fee_0 - 1;
        let fee_share_divisor_2 =                   fee_share_divisor_1 - 1;
        let tier_activation_fee_2 =               tier_activation_fee_1 + 1;
        let withdrawal_fee_2 =                         withdrawal_fee_1 - 1;
        let fee_share_divisor_3 =                   fee_share_divisor_2 - 1;
        let tier_activation_fee_3 =               tier_activation_fee_2 + 1;
        let withdrawal_fee_3 =                         withdrawal_fee_2 - 1;
        let fee_share_divisor_4 =                   fee_share_divisor_3 - 1;
        let tier_activation_fee_4 =               tier_activation_fee_3 + 1;
        let withdrawal_fee_4 =                         withdrawal_fee_3 - 1;
        let fee_share_divisor_5 =                   fee_share_divisor_4 - 1;
        let tier_activation_fee_5 =               tier_activation_fee_4 + 1;
        let withdrawal_fee_5 =                         withdrawal_fee_4 - 1;
        let fee_share_divisor_6 =                   fee_share_divisor_5 - 1;
        let tier_activation_fee_6 =               tier_activation_fee_5 + 1;
        let withdrawal_fee_6 =                         withdrawal_fee_5 - 1;
        // Vectorize fee store tier parameters.
        let integrator_fee_store_tiers = vector[
            vector[fee_share_divisor_0,
                   tier_activation_fee_0,
                   withdrawal_fee_0],
            vector[fee_share_divisor_1,
                   tier_activation_fee_1,
                   withdrawal_fee_1],
            vector[fee_share_divisor_2,
                   tier_activation_fee_2,
                   withdrawal_fee_2],
            vector[fee_share_divisor_3,
                   tier_activation_fee_3,
                   withdrawal_fee_3],
            vector[fee_share_divisor_4,
                   tier_activation_fee_4,
                   withdrawal_fee_4],
            vector[fee_share_divisor_5,
                   tier_activation_fee_5,
                   withdrawal_fee_5],
            vector[fee_share_divisor_6,
                   tier_activation_fee_6,
                   withdrawal_fee_6]];
        // Update incentives.
        update_incentives<QC>(econia, market_registration_fee,
            underwriter_registration_fee, custodian_registration_fee,
            taker_fee_divisor, integrator_fee_store_tiers);
        // Assert state.
        verify_utility_coin_type<QC>();
        assert!(!is_utility_coin_type<UC>(), 0);
        assert!(get_market_registration_fee() == market_registration_fee, 0);
        assert!(get_underwriter_registration_fee() ==
            underwriter_registration_fee, 0);
        assert!(get_custodian_registration_fee() ==
            custodian_registration_fee, 0);
        assert!(get_taker_fee_divisor() == taker_fee_divisor, 0);
        assert!(get_fee_share_divisor((0 as u8)) == fee_share_divisor_0, 0);
        assert!(get_fee_share_divisor((1 as u8)) == fee_share_divisor_1, 0);
        assert!(get_fee_share_divisor((2 as u8)) == fee_share_divisor_2, 0);
        assert!(get_fee_share_divisor((3 as u8)) == fee_share_divisor_3, 0);
        assert!(get_fee_share_divisor((4 as u8)) == fee_share_divisor_4, 0);
        assert!(get_fee_share_divisor((5 as u8)) == fee_share_divisor_5, 0);
        assert!(get_fee_share_divisor((6 as u8)) == fee_share_divisor_6, 0);
        assert!(get_tier_activation_fee((0 as u8)) ==
            tier_activation_fee_0, 0);
        assert!(get_tier_activation_fee((1 as u8)) ==
            tier_activation_fee_1, 0);
        assert!(get_tier_activation_fee((2 as u8)) ==
            tier_activation_fee_2, 0);
        assert!(get_tier_activation_fee((3 as u8)) ==
            tier_activation_fee_3, 0);
        assert!(get_tier_activation_fee((4 as u8)) ==
            tier_activation_fee_4, 0);
        assert!(get_tier_activation_fee((5 as u8)) ==
            tier_activation_fee_5, 0);
        assert!(get_tier_activation_fee((6 as u8)) ==
            tier_activation_fee_6, 0);
        assert!(get_tier_withdrawal_fee((0 as u8)) == withdrawal_fee_0, 0);
        assert!(get_tier_withdrawal_fee((1 as u8)) == withdrawal_fee_1, 0);
        assert!(get_tier_withdrawal_fee((2 as u8)) == withdrawal_fee_2, 0);
        assert!(get_tier_withdrawal_fee((3 as u8)) == withdrawal_fee_3, 0);
        assert!(get_tier_withdrawal_fee((4 as u8)) == withdrawal_fee_4, 0);
        assert!(get_tier_withdrawal_fee((5 as u8)) == withdrawal_fee_5, 0);
        assert!(get_tier_withdrawal_fee((6 as u8)) == withdrawal_fee_6, 0);
        assert!(
            exists<UtilityCoinStore<QC>>(resource_account::get_address()), 0);
    }

    #[test]
    /// Verify successful `UtilityCoinStore` initialization.
    fun test_init_utility_coin_store() {
        assets::init_coin_types_test(); // Init coin types.
        resource_account::init_test(); // Init fee account.
        // Get fee account signer.
        let fee_account = resource_account::get_signer();
        // Init utility coin store under fee account.
        init_utility_coin_store<QC>(&fee_account);
        // Verify can call re-init for when already initialized.
        init_utility_coin_store<QC>(&fee_account);
        // Assert a utility coin store exists under fee account.
        assert!(exists<UtilityCoinStore<QC>>(address_of(&fee_account)), 0);
    }

    #[test(account = @user)]
    #[expected_failure(abort_code = E_NOT_COIN)]
    /// Verify failure for attempting to initialize with non-coin type.
    fun test_init_utility_coin_store_not_coin(
        account: &signer
    ) {
        // Attempt invalid invocation.
        init_utility_coin_store<IncentiveParameters>(account);
    }

    #[test]
    #[expected_failure(abort_code = 12345, location = Self)]
    /// Verify failure for overflow.
    fun test_range_check_coin_merge() {
        let target_coins = assets::mint_test<QC>(HI_64); // Mint coins.
        // Attempt invalid invocation.
        range_check_coin_merge(1, &target_coins, 12345);
        assets::burn(target_coins); // Burn target coins.
    }

    #[test(
        econia = @econia,
        integrator = @user
    )]
    /// Verify registration of assorted coin stores, fee assessment, and
    /// withdrawal scenarios.
    fun test_register_assess_withdraw(
        econia: &signer,
        integrator: &signer
    ) acquires
        EconiaFeeStore,
        IncentiveParameters,
        IntegratorFeeStores,
        UtilityCoinStore
    {
        init_test(); // Init incentives.
        // Declare market IDs.
        let (market_id_0, market_id_1, market_id_2) = (0, 1, 2);
        // Declare integrator fee store tiers.
        let (tier_0, tier_1) = (0, 1);
        // Get taker fee divisor.
        let taker_fee_divisor = get_taker_fee_divisor();
        // Declare utility coin balance after integrator registration.
        let utility_coin_balance_0 = get_tier_activation_fee(tier_0) +
            get_tier_activation_fee(tier_1);
        // Declare utility coin balance after integrator fee withdrawal.
        let utility_coin_balance_1 = utility_coin_balance_0 +
            get_tier_withdrawal_fee(tier_0);
        let quote_fill_0 = 12345; // Declare quote fill amount, fill 0.
        // Calculate integrator fee share for fill 0.
        let integrator_fees_0 = quote_fill_0 / get_fee_share_divisor(tier_0);
        // Calculate taker fees assessed on fill 0.
        let taker_fees_0 = quote_fill_0 / taker_fee_divisor;
        // Calculate Econia fees assessed on fill 0.
        let econia_fees_0 = taker_fees_0 - integrator_fees_0;
        let quote_fill_1 = 54321; // Declare quote fill amount, fill 1.
        // Declare Econia fees for fill 1, where integrator does not
        // have a fee stores map for given quote coin types
        let econia_fees_1 = quote_fill_1 / taker_fee_divisor;
        let quote_fill_2 = 23456; // Declare quote fill amount, fill 2.
        // Declare Econia fees for fill 2, where integrator does not
        // have a fee store for given market ID.
        let econia_fees_2 = quote_fill_2 / taker_fee_divisor;
        // Register an Econia fee store for all markets.
        register_econia_fee_store_entry<QC>(market_id_0);
        register_econia_fee_store_entry<QC>(market_id_1);
        register_econia_fee_store_entry<QC>(market_id_2);
        // Register an integrator fee store for first two markets.
        register_integrator_fee_store<QC, UC>(integrator, market_id_0,
            tier_0, assets::mint_test(get_tier_activation_fee(tier_0)));
        register_integrator_fee_store<QC, UC>(integrator, market_id_1,
            tier_1, assets::mint_test(get_tier_activation_fee(tier_1)));
        // Assert tiers.
        assert!(get_integrator_fee_store_tier_test<QC>(@user, market_id_0) ==
            tier_0, 0);
        assert!(get_integrator_fee_store_tier_test<QC>(@user, market_id_1) ==
            tier_1, 0);
        // Assert utility coins deposited.
        assert!(get_utility_coin_store_balance_test() ==
            utility_coin_balance_0, 0);
        // Mint enough quote coins to cover taker fees for fill 0.
        let quote_coins = assets::mint_test(taker_fees_0);
        // Assess fees on fill 0.
        let (quote_coins, taker_fees) = assess_taker_fees<QC>(
            market_id_0, @user, taker_fee_divisor, quote_fill_0, quote_coins);
        // Assert fee amount.
        assert!(taker_fees == taker_fees_0, 0);
        // Destroy empty coins, asserting that all taker fees assessed.
        coin::destroy_zero(quote_coins);
        assert!(get_econia_fee_store_balance_test<QC>(market_id_0) ==
            econia_fees_0, 0); // Assert Econia fee share.
        assert!(get_integrator_fee_store_balance_test<QC>(@user, market_id_0)
            == integrator_fees_0, 0); // Assert integrator fee share.
        // Mint enough quote coins to cover taker fees for fill 1.
        quote_coins = assets::mint_test(econia_fees_1);
        // Assess fees on fill 1.
        (quote_coins, taker_fees) = assess_taker_fees<QC>(
            market_id_1, @econia, taker_fee_divisor, quote_fill_1,
            quote_coins);
        // Assert fee amount.
        assert!(taker_fees == econia_fees_1, 0);
        // Destroy empty coins, asserting that all taker fees assessed.
        coin::destroy_zero(quote_coins);
        assert!(get_econia_fee_store_balance_test<QC>(market_id_1) ==
            econia_fees_1, 0); // Assert Econia fee share.
        // Mint enough quote coins to cover taker fees for fill 2.
        quote_coins = assets::mint_test(econia_fees_2);
        // Assess fees on fill 2.
        (quote_coins, taker_fees) = assess_taker_fees<QC>(
            market_id_2, @user, taker_fee_divisor, quote_fill_2, quote_coins);
        // Assert fee amount.
        assert!(taker_fees == econia_fees_2, 0);
        // Destroy empty coins, asserting that all taker fees assessed.
        coin::destroy_zero(quote_coins);
        assert!(get_econia_fee_store_balance_test<QC>(market_id_2) ==
            econia_fees_2, 0); // Assert Econia fee share.
        // Register account for integrator.
        account::create_account_for_test(@user);
        // Register utility coin store for integrator.
        coin::register<UC>(integrator);
        // Deposit sufficient utility coins to pay fees
        coin::deposit<UC>(@user,
            assets::mint_test<UC>(get_tier_withdrawal_fee(tier_0)));
        // Have integrator withdraw all fees for market ID 0.
        withdraw_integrator_fees_via_coinstores<QC, UC>(integrator,
            market_id_0);
        // Assert integrator got all coins.
        assert!(coin::balance<QC>(@user) == integrator_fees_0, 0);
        // Assert utility coins deposited.
        assert!(get_utility_coin_store_balance_test() ==
            utility_coin_balance_1, 0);
        // Have Econia withdraw 1 coin for market ID 0.
        quote_coins = withdraw_econia_fees<QC>(econia, market_id_0, 1);
        // Assert 1 coin withdrawn.
        assert!(coin::value(&quote_coins) == 1, 0);
        assets::burn(quote_coins); // Burn coins.
        // Have Econia withdraw all coins for market ID 0.
        quote_coins = withdraw_econia_fees_all<QC>(econia, market_id_0);
        // Assert remaining coins withdrawn.
        assert!(coin::value(&quote_coins) == econia_fees_0 - 1, 0);
        assets::burn(quote_coins); // Burn coins.
        // Have Econia withdraw 1 utility coin.
        let utility_coins = withdraw_utility_coins<UC>(econia, 1);
        // Assert 1 coin withdrawn.
        assert!(coin::value(&utility_coins) == 1, 0);
        assets::burn(utility_coins); // Burn coins.
        // Have Econia withdraw all utility coins.
        utility_coins = withdraw_utility_coins_all<UC>(econia);
        // Assert remaining coins withdrawn.
        assert!(coin::value(&utility_coins) == utility_coin_balance_1 - 1, 0);
        assets::burn(utility_coins); // Burn coins.
        // Deposit sufficient utility coins to integrator to pay
        // withdrawal fees a second time.
        coin::deposit<UC>(@user,
            assets::mint_test<UC>(get_tier_withdrawal_fee(tier_0)));
        // Have integrator withdraw fees for market ID 0
        withdraw_integrator_fees_via_coinstores<QC, UC>(integrator,
            market_id_0);
        // Assert integrator quote coin balance unchanged.
        assert!(coin::balance<QC>(@user) == integrator_fees_0, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_FIRST_TIER_ACTIVATION_FEE_NONZERO)]
    /// Verify failure for nonzero activation fee on first tier.
    fun test_set_incentive_params_parse_tiers_vec_activate_0() {
        // Declare mock inputs.
        let taker_fee_divisor = 2345;
        // Divisor.
        let tier_0 = vector::singleton(taker_fee_divisor + 1);
        vector::push_back(&mut tier_0, 1); // Activation fee.
        vector::push_back(&mut tier_0, HI_64 - 1); // Withdrawal fee.
        let integrator_fee_store_tiers = vector::singleton(tier_0);
        let integrator_fee_store_tiers_target = vector::empty();
        set_incentive_parameters_parse_tiers_vector(
            taker_fee_divisor, &integrator_fee_store_tiers,
            &mut integrator_fee_store_tiers_target);
    }

    #[test]
    #[expected_failure(abort_code = E_ACTIVATION_FEE_TOO_SMALL)]
    /// Verify failure for activation fee too small on 1st tier.
    fun test_set_incentive_params_parse_tiers_vec_activate_1() {
        // Declare mock inputs.
        let taker_fee_divisor = 2345;
        // Divisor.
        let tier_0 = vector::singleton(taker_fee_divisor + 2);
        // Activation fee.
        vector::push_back(&mut tier_0, TIER_ACTIVATION_FEE_0);
        vector::push_back(&mut tier_0, HI_64 - 1); // Withdrawal fee.
        // Divisor.
        let tier_1 = vector::singleton(taker_fee_divisor + 1);
        // Activation fee.
        vector::push_back(&mut tier_1, TIER_ACTIVATION_FEE_0);
        vector::push_back(&mut tier_1, HI_64 - 2); // Withdrawal fee.
        let integrator_fee_store_tiers = vector::singleton(tier_0);
        vector::push_back(&mut integrator_fee_store_tiers, tier_1);
        let integrator_fee_store_tiers_target = vector::empty();
        set_incentive_parameters_parse_tiers_vector(
            taker_fee_divisor, &integrator_fee_store_tiers,
            &mut integrator_fee_store_tiers_target);
    }

    #[test]
    #[expected_failure(abort_code = E_FEE_SHARE_DIVISOR_TOO_BIG)]
    /// Verify failure for fee share divisor too big on 0th tier.
    fun test_set_incentive_params_parse_tiers_vec_divisor_big_0() {
        // Declare mock inputs.
        let taker_fee_divisor = 2345;
        let tier_0 = vector::singleton(HI_64); // Divisor.
        vector::push_back(&mut tier_0, 0); // Activation fee.
        vector::push_back(&mut tier_0, 0); // Withdrawal fee.
        let integrator_fee_store_tiers = vector::singleton(tier_0);
        let integrator_fee_store_tiers_target = vector::empty();
        set_incentive_parameters_parse_tiers_vector(
            taker_fee_divisor, &integrator_fee_store_tiers,
            &mut integrator_fee_store_tiers_target);
    }

    #[test]
    #[expected_failure(abort_code = E_FEE_SHARE_DIVISOR_TOO_BIG)]
    /// Verify failure for fee share divisor too big on 1st tier.
    fun test_set_incentive_params_parse_tiers_vec_divisor_big_1() {
        // Declare mock inputs.
        let taker_fee_divisor = 2345;
        // Divisor.
        let tier_0 = vector::singleton(taker_fee_divisor + 1);
        // Activation fee.
        vector::push_back(&mut tier_0, TIER_ACTIVATION_FEE_0);
        vector::push_back(&mut tier_0, HI_64 - 1); // Withdrawal fee.
        // Divisor.
        let tier_1 = vector::singleton(taker_fee_divisor + 1);
        vector::push_back(&mut tier_1, 2); // Activation fee.
        vector::push_back(&mut tier_1, HI_64 - 2); // Withdrawal fee.
        let integrator_fee_store_tiers = vector::singleton(tier_0);
        vector::push_back(&mut integrator_fee_store_tiers, tier_1);
        let integrator_fee_store_tiers_target = vector::empty();
        set_incentive_parameters_parse_tiers_vector(
            taker_fee_divisor, &integrator_fee_store_tiers,
            &mut integrator_fee_store_tiers_target);
    }

    #[test]
    #[expected_failure(abort_code = E_FEE_SHARE_DIVISOR_TOO_SMALL)]
    /// Verify failure for fee share divisor too small.
    fun test_set_incentive_params_parse_tiers_vec_divisor_small() {
        // Declare mock inputs.
        let taker_fee_divisor = 2345;
        // Divisor.
        let tier_0 = vector::singleton(taker_fee_divisor - 1);
        // Activation fee.
        vector::push_back(&mut tier_0, TIER_ACTIVATION_FEE_0);
        vector::push_back(&mut tier_0, 0); // Withdrawal fee.
        let integrator_fee_store_tiers = vector::singleton(tier_0);
        let integrator_fee_store_tiers_target = vector::empty();
        set_incentive_parameters_parse_tiers_vector(
            taker_fee_divisor, &integrator_fee_store_tiers,
            &mut integrator_fee_store_tiers_target);
    }

    #[test]
    #[expected_failure(abort_code = E_WITHDRAWAL_FEE_TOO_BIG)]
    /// Verify failure for withdrawal fee too big on 0th tier.
    fun test_set_incentive_params_parse_tiers_vec_withdraw_big_0() {
        // Declare mock inputs.
        let taker_fee_divisor = 2345;
        // Divisor.
        let tier_0 = vector::singleton(taker_fee_divisor + 2);
        // Activation fee.
        vector::push_back(&mut tier_0, TIER_ACTIVATION_FEE_0);
        vector::push_back(&mut tier_0, HI_64); // Withdrawal fee.
        let integrator_fee_store_tiers = vector::singleton(tier_0);
        let integrator_fee_store_tiers_target = vector::empty();
        set_incentive_parameters_parse_tiers_vector(
            taker_fee_divisor, &integrator_fee_store_tiers,
            &mut integrator_fee_store_tiers_target);
    }

    #[test]
    #[expected_failure(abort_code = E_WITHDRAWAL_FEE_TOO_BIG)]
    /// Verify failure for withdrawal fee too big on 1st tier.
    fun test_set_incentive_params_parse_tiers_vec_withdraw_big_1() {
        // Declare mock inputs.
        let taker_fee_divisor = 2345;
        // Divisor.
        let tier_0 = vector::singleton(taker_fee_divisor + 2);
        // Activation fee.
        vector::push_back(&mut tier_0, TIER_ACTIVATION_FEE_0);
        vector::push_back(&mut tier_0, HI_64 - 1); // Withdrawal fee.
        // Divisor.
        let tier_1 = vector::singleton(taker_fee_divisor + 1);
        vector::push_back(&mut tier_1, 2); // Activation fee.
        vector::push_back(&mut tier_1, HI_64 - 1); // Withdrawal fee.
        let integrator_fee_store_tiers = vector::singleton(tier_0);
        vector::push_back(&mut integrator_fee_store_tiers, tier_1);
        let integrator_fee_store_tiers_target = vector::empty();
        set_incentive_parameters_parse_tiers_vector(
            taker_fee_divisor, &integrator_fee_store_tiers,
            &mut integrator_fee_store_tiers_target);
    }

    #[test]
    #[expected_failure(abort_code = E_WITHDRAWAL_FEE_TOO_SMALL)]
    /// Verify failure for withdrawal fee too small.
    fun test_set_incentive_params_parse_tiers_vec_withdraw_small() {
        // Declare mock inputs.
        let taker_fee_divisor = 2345;
        // Divisor.
        let tier_0 = vector::singleton(taker_fee_divisor + 1);
        // Activation fee.
        vector::push_back(&mut tier_0, TIER_ACTIVATION_FEE_0);
        vector::push_back(&mut tier_0, 0); // Withdrawal fee.
        let integrator_fee_store_tiers = vector::singleton(tier_0);
        let integrator_fee_store_tiers_target = vector::empty();
        set_incentive_parameters_parse_tiers_vector(
            taker_fee_divisor, &integrator_fee_store_tiers,
            &mut integrator_fee_store_tiers_target);
    }

    #[test]
    #[expected_failure(abort_code = E_TIER_FIELDS_WRONG_LENGTH)]
    /// Verify failure for wrong length of inner vector.
    fun test_set_incentive_params_parse_tiers_vec_wrong_length() {
        // Declare mock inputs.
        let taker_fee_divisor = 2345;
        let integrator_fee_store_tiers = vector::singleton(vector::empty());
        let integrator_fee_store_tiers_target = vector::empty();
        set_incentive_parameters_parse_tiers_vector(
            taker_fee_divisor, &integrator_fee_store_tiers,
            &mut integrator_fee_store_tiers_target);
    }

    #[test(econia = @econia)]
    #[expected_failure(
        abort_code = E_CUSTODIAN_REGISTRATION_FEE_LESS_THAN_MIN
    )]
    /// Verify failure for custodian registration fee too low.
    fun test_set_incentive_params_range_check_inputs_custodian_fee(
        econia: &signer
    ) {
        // Attempt invalid invocation.
        set_incentive_parameters_range_check_inputs(econia, 1, 1, 0, 0,
            &vector::empty());
    }

    #[test(econia = @econia)]
    #[expected_failure(abort_code = E_TAKER_DIVISOR_LESS_THAN_MIN)]
    /// Verify failure for divisor too low.
    fun test_set_incentive_params_range_check_inputs_divisor(
        econia: &signer
    ) {
        // Attempt invalid invocation.
        set_incentive_parameters_range_check_inputs(econia, 1, 1, 1, 0,
            &vector::empty());
    }

    #[test(econia = @econia)]
    #[expected_failure(abort_code = E_MARKET_REGISTRATION_FEE_LESS_THAN_MIN)]
    /// Verify failure for market registration fee too low.
    fun test_set_incentive_params_range_check_inputs_market_fee(
        econia: &signer
    ) {
        // Attempt invalid invocation.
        set_incentive_parameters_range_check_inputs(econia, 0, 0, 0, 0,
            &vector::empty());
    }

    #[test(account = @user)]
    #[expected_failure(abort_code = E_NOT_ECONIA)]
    /// Verify failure for not Econia account.
    fun test_set_incentive_params_range_check_inputs_not_econia(
        account: &signer
    ) {
        // Attempt invalid invocation.
        set_incentive_parameters_range_check_inputs(account, 0, 0, 0, 0,
            &vector::empty());
    }

    #[test(econia = @econia)]
    #[expected_failure(
        abort_code = E_UNDERWRITER_REGISTRATION_FEE_LESS_THAN_MIN
    )]
    /// Verify failure for underwriter registration fee too low.
    fun test_set_incentive_params_range_check_inputs_underwriter(
        econia: &signer
    ) {
        // Attempt invalid invocation.
        set_incentive_parameters_range_check_inputs(econia, 1, 0, 0, 0,
            &vector::empty());
    }

    #[test(econia = @econia)]
    #[expected_failure(abort_code = E_EMPTY_FEE_STORE_TIERS)]
    /// Verify failure for empty fee store tiers.
    fun test_set_incentive_params_range_check_inputs_vector_empty(
        econia: &signer
    ) {
        // Attempt invalid invocation.
        set_incentive_parameters_range_check_inputs(econia, 1, 1, 1, 2,
            &vector::empty());
    }

    #[test(econia = @econia)]
    #[expected_failure(abort_code = E_TOO_MANY_TIERS)]
    /// Verify failure for too many elements in fee store tiers vector.
    fun test_set_incentive_params_range_check_inputs_vector_long(
        econia: &signer
    ) {
        // Declare empty integrator fee store tiers vector.
        let integrator_fee_store_tiers = vector::empty();
        let i = 0; // Declare loop counter.
        // For one iteration more than the number of max tiers:
        while (i < MAX_INTEGRATOR_FEE_STORE_TIERS + 1) {
            // Push back an empty vector onto fee store tiers vector.
            vector::push_back(&mut integrator_fee_store_tiers,
                vector::empty());
            i = i + 1; // Increment loop counter.
        };
        // Attempt invalid invocation.
        set_incentive_parameters_range_check_inputs(econia, 1, 1, 1, 2,
            &integrator_fee_store_tiers);
    }

    #[test(econia = @econia)]
    #[expected_failure(abort_code = E_FEWER_TIERS)]
    /// Verify failure for attempting to update incentive parameters
    /// with fewer integrator fee store tiers than before.
    fun test_update_incentives_fewer_tiers(
        econia: &signer
    ) acquires
        IncentiveParameters
    {
        assets::init_coin_types_test(); // Init coin types.
        resource_account::init_test(); // Init fee account.
        init_module(econia); // Initialize incentives.
        // Vectorize fee store tier parameters.
        let tier_0 = vector::singleton(FEE_SHARE_DIVISOR_0);
        vector::push_back(&mut tier_0, TIER_ACTIVATION_FEE_0);
        vector::push_back(&mut tier_0, WITHDRAWAL_FEE_0);
        let integrator_fee_store_tiers = vector::singleton(tier_0);
        // Attempt invalid update to incentive parameter set.
        update_incentives<QC>(econia, MARKET_REGISTRATION_FEE,
            UNDERWRITER_REGISTRATION_FEE, CUSTODIAN_REGISTRATION_FEE,
            TAKER_FEE_DIVISOR, integrator_fee_store_tiers);
    }

    #[test(integrator = @user)]
    /// Verify upgrade and fee assessment.
    fun test_upgrade_integrator_fee_store_via_coinstore(
        integrator: &signer
    ) acquires
        IncentiveParameters,
        IntegratorFeeStores,
        UtilityCoinStore
    {
        init_test(); // Init incentives.
        // Declare market ID, tier.
        let (market_id, tier_start, tier_upgrade) = (0, 0, 1);
        // Declare activation fee for start and upgrade tiers.
        let (fee_start, fee_upgrade) = (get_tier_activation_fee(tier_start),
            get_tier_activation_fee(tier_upgrade));
        // Register to start tier.
        register_integrator_fee_store<QC, UC>(integrator, market_id,
            tier_start, assets::mint_test(fee_start));
        // Assert start tier.
        assert!(get_integrator_fee_store_tier_test<QC>(@user, market_id) ==
            tier_start, 0);
        // Register account for given integrator.
        account::create_account_for_test(@user);
        // Register integrator with coinstore for utility coin.
        coin::register<UC>(integrator);
        // Deposit enough utility coins to pay for upgrade.
        coin::deposit<UC>(@user, assets::mint_test(fee_upgrade));
        // Upgrade to upgrade tier.
        upgrade_integrator_fee_store_via_coinstore<QC, UC>(integrator,
            market_id, tier_upgrade);
        // Assert fees assessed for cumulative amount required to
        // activate to upgrade tier.
        assert!(get_utility_coin_store_balance_test() == fee_upgrade, 0);
        // Assert upgrade tier.
        assert!(get_integrator_fee_store_tier_test<QC>(@user, market_id) ==
            tier_upgrade, 0);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_UTILITY_COIN_TYPE)]
    /// Verify failure for wrong type.
    fun test_verify_utility_coin_type()
    acquires
        IncentiveParameters
    {
        init_test(); // Initialize incentives for testing.
        verify_utility_coin_type<QC>(); // Attempt invalid invocation.
    }

    #[test(account = @user)]
    #[expected_failure(abort_code = E_NOT_ECONIA)]
    /// Verify failure for account is not Econia.
    fun test_withdraw_econia_fees_all_not_econia(
        account: &signer
    ) acquires
        EconiaFeeStore
    {
        // Attempt invalid invocation.
        let fees = withdraw_econia_fees_all<UC>(account, 0);
        assets::burn(fees); // Burn fees.
    }

    #[test(account = @user)]
    #[expected_failure(abort_code = E_NOT_ECONIA)]
    /// Verify failure for account is not Econia.
    fun test_withdraw_econia_fees_not_econia(
        account: &signer
    ) acquires
        EconiaFeeStore
    {
        // Attempt invalid invocation.
        let fees = withdraw_econia_fees<UC>(account, 0, 0);
        assets::burn(fees); // Burn fees.
    }

    #[test]
    /// Verify state updates for withdrawing to a standard coin store,
    /// from Econia's fee and utility coin stores.
    fun test_withdraw_to_coin_store_econia()
    acquires
        EconiaFeeStore,
        IncentiveParameters,
        UtilityCoinStore
    {
        init_test(); // Init incentives.
        // Declare coin amounts, mock market ID.
        let utility_coin_amount = 123;
        let fee_coin_amount = 321;
        let market_id = 456;
        // Deposit utility coins.
        deposit_utility_coins<UC>(
            assets::mint_test(utility_coin_amount));
        // Register Econia account.
        let econia = account::create_account_for_test(@econia);
        // Withdraw 1 coin, registering coin store.
        withdraw_utility_coins_to_coin_store<UC>(&econia, 1);
        // Assert coin store balance.
        assert!(coin::balance<UC>(@econia) == 1, 0);
        // Withdraw remaining coins.
        withdraw_utility_coins_all_to_coin_store<UC>(&econia,);
        // Assert coin store balance.
        assert!(coin::balance<UC>(@econia) == utility_coin_amount, 0);
        // Register Econia fee store.
        register_econia_fee_store_entry<QC>(market_id);
        // Mutably borrow Econia fee store map for quote coin type.
        let econia_fee_store_map_ref_mut =
            &mut borrow_global_mut<EconiaFeeStore<QC>>(
                resource_account::get_address()).map;
        // Borrow mutable reference to fees for given market ID.
        let econia_fee_store_coins_ref_mut = tablist::borrow_mut(
            econia_fee_store_map_ref_mut, market_id);
        // Merge simulated fees into the fee store.
        coin::merge(econia_fee_store_coins_ref_mut,
                    assets::mint_test<QC>(fee_coin_amount));
        // Withdraw 1 coin, registering coin store.
        withdraw_econia_fees_to_coin_store<QC>(&econia, market_id, 1);
        // Assert coin store balance.
        assert!(coin::balance<QC>(@econia) == 1, 0);
        // Withdraw remaining coins.
        withdraw_econia_fees_all_to_coin_store<QC>(&econia, market_id);
        // Assert coin store balance.
        assert!(coin::balance<QC>(@econia) == fee_coin_amount, 0);
    }

    #[test(account = @user)]
    #[expected_failure(abort_code = E_NOT_ECONIA)]
    /// Verify failure for account is not Econia.
    fun test_withdraw_utility_coins_all_not_econia(
        account: &signer
    ): coin::Coin<UC>
    acquires
        UtilityCoinStore
    {
        // Attempt invalid invocation.
        withdraw_utility_coins_all<UC>(account)
    }

    #[test(account = @user)]
    #[expected_failure(abort_code = E_NOT_ECONIA)]
    /// Verify failure for account is not Econia.
    fun test_withdraw_utility_coins_not_econia(
        account: &signer
    ): coin::Coin<UC>
    acquires
        UtilityCoinStore
    {
        // Attempt invalid invocation.
        withdraw_utility_coins<UC>(account, 1234)
    }

    // Tests <<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<

}