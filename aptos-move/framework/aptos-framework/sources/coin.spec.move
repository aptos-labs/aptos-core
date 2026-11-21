spec aptos_framework::coin {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: Only the owner of a coin may mint, burn or freeze coins.
    /// Criticality: Critical
    /// Implementation: Acquiring capabilities for a particular CoinType may only occur if the caller has a signer for
    /// the module declaring that type. The initialize function returns these capabilities to the caller.
    /// Enforcement: Formally Verified via [high-level-req-1.1](upgrade_supply) and [high-level-req-1.2](initialize).
    ///
    /// No.: 2
    /// Requirement: Each coin may only be created exactly once.
    /// Criticality: Medium
    /// Implementation: The initialization function may only be called once.
    /// Enforcement: Formally Verified via [high-level-req-2](initialize).
    ///
    /// No.: 3
    /// Requirement: The merging of coins may only be done on coins of the same type.
    /// Criticality: Critical
    /// Implementation: The merge function is limited to merging coins of the same type only.
    /// Enforcement: Formally Verified via [high-level-req-3](merge).
    ///
    /// No.: 4
    /// Requirement: The supply of a coin is only affected by burn and mint operations.
    /// Criticality: High
    /// Implementation: Only mint and burn operations on a coin alter the total supply of coins.
    /// Enforcement: Formally Verified via [high-level-req-4.1](CoinSupplyUnchanged) that every function of this
    /// module except mint, burn, initialization, the conversions between coin and fungible asset and the functions
    /// converting internally (deposit, deposit_with_signer, deposit_for_gas_fee, withdraw and the migrations) keeps
    /// the supply recorded in `CoinInfo` and the value held in coins unchanged; the excepted functions change both by
    /// exactly the minted, burned or converted amount. Formally Verified via [high-level-req-4.2](TotalSupplyUnchanged)
    /// that, for a tracked coin supply and an existing pairing, every function of this module except mint, burn,
    /// burn_from and initialization keeps the coin supply plus the paired fungible asset supply unchanged; for
    /// [high-level-req-4.3](fungible_asset_to_coin) and withdraw this is verified when the converted asset is the
    /// paired one and, for withdraw, the store has no withdraw hook. Audited: conversions that create the pairing,
    /// migrate_coin_store_to_fungible_store, and mint and burn of the paired fungible asset through refs held outside
    /// this module.
    ///
    /// No.: 5
    /// Requirement: Users may register an account for a coin multiple times idempotently.
    /// Criticality: Medium
    /// Implementation: The register function should work idempotently. Importantly, it should not abort if the coin is already registered.
    /// Enforcement: Formally verified via aborts_if on [high-level-req-5](register).
    ///
    /// No.: 6
    /// Requirement: Coin operations should fail if the user has not registered for the coin.
    /// Criticality: Medium
    /// Implementation: Coin operations may succeed only on valid user coin registration.
    /// Enforcement: Formally Verified via [high-level-req-6.2](burn_from), [high-level-req-6.3](freeze_coin_store) and
    /// [high-level-req-6.4](unfreeze_coin_store). Audited that balance, transfer and withdraw need no registration:
    /// balance reads 0 from an absent paired store, and transfer and withdraw create it on demand.
    ///
    /// No.: 7
    /// Requirement: It should always be possible to (1) determine if a coin exists, and (2) determine if a user registered
    /// an account with a particular coin. If a coin exists, it should always be possible to request the following
    /// information of the coin: (1) Name, (2) Symbol, and (3) Supply.
    /// Criticality: Low
    /// Implementation: The following functions should never abort: (1) is_coin_initialized, and (2) is_account_registered. The following functions should not abort if the coin exists: (1) name, (2) symbol, and (3) supply.
    /// Enforcement: Formally Verified in corresponding functions: [high-level-req-7.1](is_coin_initialized), [high-level-req-7.3](name), [high-level-req-7.4](symbol) and [high-level-req-7.5](coin_supply).
    /// Audited that is_account_registered aborts only if the coin does not exist.
    ///
    /// No.: 8
    /// Requirement: Coin operations should fail if the user's CoinStore is frozen.
    /// Criticality: Medium
    /// Implementation: If the CoinStore of an address is frozen, coin operations are disallowed.
    /// Enforcement: Formally Verified via [high-level-req-8.1](withdraw),
    /// [high-level-req-8.2](TransferAbortsIfAndEnsures) and [high-level-req-8.3](deposit).
    ///
    /// No.: 9
    /// Requirement: Utilizing AggregatableCoins does not violate other critical invariants, such as (4).
    /// Criticality: High
    /// Implementation: Utilizing AggregatableCoin does not change the real-supply of any token.
    /// Enforcement: Audited that no operation uses AggregatableCoin.
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_partial;
        /// The total value of all `Coin<CoinType>` values.
        global supply<CoinType>: num;
        // TODO: `migrate_coin_store_to_fungible_store` migrates in an inline `for_each`
        // loop; the schema needs a loop invariant that cannot be attached there.
        apply TotalSupplyTracked<CoinType> to *<CoinType> except initialize, initialize_internal, initialize_with_parallelizable_supply, migrate_coin_store_to_fungible_store;
        /// [high-level-req-4.1]
        apply CoinSupplyUnchanged<CoinType> to *<CoinType> except mint, mint_internal, burn, burn_internal,
            coin_to_fungible_asset, fungible_asset_to_coin, deposit, deposit_with_signer, deposit_for_gas_fee, withdraw,
            maybe_convert_to_fungible_store, migrate_to_fungible_store, migrate_coin_store_to_fungible_store,
            initialize, initialize_internal, initialize_with_parallelizable_supply;
        /// [high-level-req-4.2]
        apply TotalSupplyUnchanged<CoinType> to *<CoinType> except mint, mint_internal, burn, burn_internal,
            burn_from, burn_from_for_gas, fungible_asset_to_coin, withdraw, migrate_coin_store_to_fungible_store,
            initialize, initialize_internal, initialize_with_parallelizable_supply;
    }

    spec fun spec_fun_supply_tracked<CoinType>(val: u64, supply: Option<OptionalAggregator>): bool {
        option::is_some(supply) ==>
            val
                == optional_aggregator::optional_aggregator_value(
                    option::borrow(supply)
                )
    }

    spec schema TotalSupplyTracked<CoinType> {
        ensures old(
            spec_fun_supply_tracked<CoinType>(
                supply<CoinType>,
                global<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address)
                .supply
            )
        ) ==>
            spec_fun_supply_tracked<CoinType>(
                supply<CoinType>,
                global<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address)
                .supply
            );
    }

    /// The supply recorded in `CoinInfo<CoinType>`, none if it is not tracked.
    spec fun spec_coin_supply<CoinType>(coin_addr: address): Option<u128> {
        let maybe_supply = global<CoinInfo<CoinType>>(coin_addr).supply;
        if (option::is_some(maybe_supply)) {
            option::spec_some(optional_aggregator::optional_aggregator_value(option::borrow(maybe_supply)))
        } else {
            option::spec_none()
        }
    }

    /// Neither the recorded supply nor the value held in coins changes.
    spec schema CoinSupplyUnchanged<CoinType> {
        let coin_addr = type_info::type_of<CoinType>().account_address;
        ensures spec_coin_supply<CoinType>(coin_addr) == old(spec_coin_supply<CoinType>(coin_addr));
        ensures supply<CoinType> == old(supply<CoinType>);
    }

    /// The recorded coin supply plus the supply of the paired fungible asset, if any.
    spec fun spec_total_supply<CoinType>(coin_type: TypeInfo): num {
        let maybe_supply = global<CoinInfo<CoinType>>(coin_type.account_address).supply;
        let metadata = spec_paired_metadata(coin_type);
        optional_aggregator::optional_aggregator_value(option::borrow(maybe_supply))
            + if (option::is_some(metadata)) { fungible_asset::spec_supply(option::borrow(metadata).inner) } else { 0 }
    }

    /// If the coin supply is tracked and the coin is paired, the total supply does not change.
    spec schema TotalSupplyUnchanged<CoinType> {
        let coin_type = type_info::type_of<CoinType>();
        let tracked = option::is_some(global<CoinInfo<CoinType>>(coin_type.account_address).supply)
            && option::is_some(spec_paired_metadata(coin_type));
        ensures tracked ==> spec_total_supply<CoinType>(coin_type) == old(spec_total_supply<CoinType>(coin_type));
    }

    /// The recorded supply, if tracked, changes by `added - removed`.
    spec schema CoinSupplyDelta<CoinType> {
        added: num;
        removed: num;
        let coin_addr = type_info::type_of<CoinType>().account_address;
        let maybe_supply = spec_coin_supply<CoinType>(coin_addr);
        let post post_maybe_supply = spec_coin_supply<CoinType>(coin_addr);
        ensures option::is_some(post_maybe_supply) == option::is_some(maybe_supply);
        ensures option::is_some(maybe_supply) ==>
            option::borrow(post_maybe_supply) == option::borrow(maybe_supply) + added - removed;
    }

    spec AggregatableCoin {
        use aptos_framework::aggregator;
        invariant aggregator::spec_get_limit(value) == MAX_U64;
    }

    spec mint {
        // Partial: the supply overflow abort is assumed away in `mint_internal`.
        let addr = type_info::type_of<CoinType>().account_address;
        modifies global<CoinInfo<CoinType>>(addr);
        aborts_if (amount != 0) && !type_info::spec_is_struct<CoinType>();
        aborts_if (amount != 0) && !exists<CoinInfo<CoinType>>(addr);
        ensures supply<CoinType> == old(supply<CoinType>) + amount;
        include CoinSupplyDelta<CoinType> { added: amount, removed: 0 };
        ensures result.value == amount;
        ensures exists<CoinInfo<CoinType>>(addr) == old(exists<CoinInfo<CoinType>>(addr));
    }

    spec mint_internal {
        // Partial: the supply overflow abort is assumed away in `mint_internal`.
        let addr = type_info::type_of<CoinType>().account_address;
        modifies global<CoinInfo<CoinType>>(addr);
        aborts_if (amount != 0) && !type_info::spec_is_struct<CoinType>();
        aborts_if (amount != 0) && !exists<CoinInfo<CoinType>>(addr);
        ensures supply<CoinType> == old(supply<CoinType>) + amount;
        include CoinSupplyDelta<CoinType> { added: amount, removed: 0 };
        ensures result.value == amount;
        ensures exists<CoinInfo<CoinType>>(addr) == old(exists<CoinInfo<CoinType>>(addr));
    }

    /// Get address by reflection.
    spec coin_address<CoinType>(): address {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if !type_info::spec_is_struct<CoinType>();
        ensures result == type_info::type_of<CoinType>().account_address;
    }

    /// Can only be updated by `@aptos_framework`.
    spec allow_supply_upgrades(_aptos_framework: &signer, _allowed: bool) {
        pragma aborts_if_is_partial = false;
        aborts_if true;
    }

    /// The balance left in a not yet migrated `CoinStore`.
    spec fun spec_coin_balance<CoinType>(owner: address): u64 {
        if (exists<CoinStore<CoinType>>(owner)) { global<CoinStore<CoinType>>(owner).coin.value } else { 0 }
    }

    /// The primary store of `owner` for the paired fungible asset `paired`; meaningful only if it is some.
    spec fun spec_paired_store_address(owner: address, paired: Option<Object<Metadata>>): address {
        primary_fungible_store::spec_primary_store_address(owner, option::borrow(paired))
    }

    spec fun spec_has_paired_store(owner: address, paired: Option<Object<Metadata>>): bool {
        option::is_some(paired) && exists<fungible_asset::FungibleStore>(spec_paired_store_address(owner, paired))
    }

    spec fun spec_fa_balance(owner: address, paired: Option<Object<Metadata>>): u64 {
        if (spec_has_paired_store(owner, paired)) {
            fungible_asset::spec_balance(spec_paired_store_address(owner, paired))
        } else { 0 }
    }

    /// The balance of `owner`'s paired store is computed by a dispatch hook rather than read from the store.
    spec fun spec_has_balance_hook(owner: address, paired: Option<Object<Metadata>>): bool {
        spec_has_paired_store(owner, paired)
            && fungible_asset::spec_store_has_balance_hook(spec_paired_store_address(owner, paired))
    }

    /// The coin balance plus the balance of the paired primary store.
    spec balance<CoinType>(owner: address): u64 {
        // Partial: the result and aborts of a derived-balance hook are not specified.
        let paired = spec_paired_metadata(type_info::type_of<CoinType>());
        let total = spec_coin_balance<CoinType>(owner) + spec_fa_balance(owner, paired);
        aborts_if !spec_has_balance_hook(owner, paired) && total > MAX_U64;
        ensures !spec_has_balance_hook(owner, paired) ==> result == total;
    }

    spec is_balance_at_least<CoinType>(owner: address, amount: u64): bool {
        // Partial: the result and aborts of a derived-balance hook are not specified.
        let paired = spec_paired_metadata(type_info::type_of<CoinType>());
        ensures !spec_has_balance_hook(owner, paired) ==>
            result == (spec_coin_balance<CoinType>(owner) + spec_fa_balance(owner, paired) >= amount);
    }

    spec is_coin_initialized<CoinType>(): bool {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        /// [high-level-req-7.1]
        aborts_if !type_info::spec_is_struct<CoinType>();
        ensures result == exists<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address);
    }

    spec is_account_registered<CoinType>(_account_addr: address): bool {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if !type_info::spec_is_struct<CoinType>();
        aborts_if !exists<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address);
        ensures result == true;
    }

    /// The fungible asset metadata paired with the coin type `coin_type`, if any.
    spec fun spec_paired_metadata(coin_type: TypeInfo): Option<Object<Metadata>> {
        if (exists<CoinConversionMap>(@aptos_framework)) {
            let map =
                global<CoinConversionMap>(@aptos_framework).coin_to_fungible_asset_map;
            if (table::spec_contains(map, coin_type)) {
                let metadata = table::spec_get(map, coin_type);
                option::spec_some(metadata)
            } else {
                option::spec_none()
            }
        } else {
            option::spec_none()
        }
    }

    spec paired_metadata<CoinType>(): Option<Object<Metadata>> {
        pragma aborts_if_is_partial = false;
        aborts_if exists<CoinConversionMap>(@aptos_framework) && !type_info::spec_is_struct<CoinType>();
        ensures result == spec_paired_metadata(type_info::type_of<CoinType>());
    }

    spec create_coin_conversion_map(aptos_framework: &signer) {
        pragma aborts_if_is_partial = false;
        aborts_if signer::address_of(aptos_framework) != @aptos_framework;
        ensures exists<CoinConversionMap>(@aptos_framework);
    }

    /// `CoinType` is paired and its metadata object holds the paired refs.
    spec schema PairedRefsAbortsIf<CoinType> {
        let paired = spec_paired_metadata(type_info::type_of<CoinType>());
        aborts_if !type_info::spec_is_struct<CoinType>();
        aborts_if option::is_none(paired);
        aborts_if !exists<PairedFungibleAssetRefs>(option::borrow(paired).inner);
    }

    spec get_paired_mint_copy_ref<CoinType>(mint_cap: &MintCapability<CoinType>): MintRef {
        pragma aborts_if_is_partial = false;
        let metadata = option::borrow(spec_paired_metadata(type_info::type_of<CoinType>()));
        let refs = global<PairedFungibleAssetRefs>(metadata.inner);
        include PairedRefsAbortsIf<CoinType>;
        aborts_if option::is_none(refs.mint_ref_opt);
        ensures result == option::borrow(refs.mint_ref_opt);
    }

    spec get_paired_mint_ref {
        pragma aborts_if_is_partial = false;
        let metadata = option::borrow(spec_paired_metadata(type_info::type_of<CoinType>()));
        let refs = global<PairedFungibleAssetRefs>(metadata.inner);
        modifies global<PairedFungibleAssetRefs>(metadata.inner);
        include PairedRefsAbortsIf<CoinType>;
        aborts_if option::is_none(refs.mint_ref_opt);
        ensures result_1 == option::borrow(refs.mint_ref_opt);
        ensures result_2 == MintRefReceipt { metadata };
        ensures global<PairedFungibleAssetRefs>(metadata.inner)
            == update_field(refs, mint_ref_opt, option::spec_none());
    }

    spec return_paired_mint_ref(mint_ref: MintRef, receipt: MintRefReceipt) {
        pragma aborts_if_is_partial = false;
        let addr = receipt.metadata.inner;
        let refs = global<PairedFungibleAssetRefs>(addr);
        modifies global<PairedFungibleAssetRefs>(addr);
        aborts_if mint_ref.metadata != receipt.metadata;
        aborts_if !exists<PairedFungibleAssetRefs>(addr);
        aborts_if option::is_some(refs.mint_ref_opt);
        ensures global<PairedFungibleAssetRefs>(addr)
            == update_field(refs, mint_ref_opt, option::spec_some(mint_ref));
    }

    spec get_paired_transfer_ref {
        pragma aborts_if_is_partial = false;
        let metadata = option::borrow(spec_paired_metadata(type_info::type_of<CoinType>()));
        let refs = global<PairedFungibleAssetRefs>(metadata.inner);
        modifies global<PairedFungibleAssetRefs>(metadata.inner);
        include PairedRefsAbortsIf<CoinType>;
        aborts_if option::is_none(refs.transfer_ref_opt);
        ensures result_1 == option::borrow(refs.transfer_ref_opt);
        ensures result_2 == TransferRefReceipt { metadata };
        ensures global<PairedFungibleAssetRefs>(metadata.inner)
            == update_field(refs, transfer_ref_opt, option::spec_none());
    }

    spec return_paired_transfer_ref(transfer_ref: TransferRef, receipt: TransferRefReceipt) {
        pragma aborts_if_is_partial = false;
        let addr = receipt.metadata.inner;
        let refs = global<PairedFungibleAssetRefs>(addr);
        modifies global<PairedFungibleAssetRefs>(addr);
        aborts_if transfer_ref.metadata != receipt.metadata;
        aborts_if !exists<PairedFungibleAssetRefs>(addr);
        aborts_if option::is_some(refs.transfer_ref_opt);
        ensures global<PairedFungibleAssetRefs>(addr)
            == update_field(refs, transfer_ref_opt, option::spec_some(transfer_ref));
    }

    spec get_paired_burn_copy_ref<CoinType>(burn_cap: &BurnCapability<CoinType>): BurnRef {
        pragma aborts_if_is_partial = false;
        let metadata = option::borrow(spec_paired_metadata(type_info::type_of<CoinType>()));
        let refs = global<PairedFungibleAssetRefs>(metadata.inner);
        include PairedRefsAbortsIf<CoinType>;
        aborts_if option::is_none(refs.burn_ref_opt);
        ensures result == option::borrow(refs.burn_ref_opt);
    }

    spec get_paired_burn_ref {
        pragma aborts_if_is_partial = false;
        let metadata = option::borrow(spec_paired_metadata(type_info::type_of<CoinType>()));
        let refs = global<PairedFungibleAssetRefs>(metadata.inner);
        modifies global<PairedFungibleAssetRefs>(metadata.inner);
        include PairedRefsAbortsIf<CoinType>;
        aborts_if option::is_none(refs.burn_ref_opt);
        ensures result_1 == option::borrow(refs.burn_ref_opt);
        ensures result_2 == BurnRefReceipt { metadata };
        ensures global<PairedFungibleAssetRefs>(metadata.inner)
            == update_field(refs, burn_ref_opt, option::spec_none());
    }

    spec convert_and_take_paired_burn_ref<CoinType>(burn_cap: BurnCapability<CoinType>): BurnRef {
        pragma aborts_if_is_partial = false;
        let metadata = option::borrow(spec_paired_metadata(type_info::type_of<CoinType>()));
        let refs = global<PairedFungibleAssetRefs>(metadata.inner);
        modifies global<PairedFungibleAssetRefs>(metadata.inner);
        include PairedRefsAbortsIf<CoinType>;
        aborts_if option::is_none(refs.burn_ref_opt);
        ensures result == option::borrow(refs.burn_ref_opt);
        ensures global<PairedFungibleAssetRefs>(metadata.inner)
            == update_field(refs, burn_ref_opt, option::spec_none());
    }

    spec return_paired_burn_ref(burn_ref: BurnRef, receipt: BurnRefReceipt) {
        pragma aborts_if_is_partial = false;
        let addr = receipt.metadata.inner;
        let refs = global<PairedFungibleAssetRefs>(addr);
        modifies global<PairedFungibleAssetRefs>(addr);
        aborts_if burn_ref.metadata != receipt.metadata;
        aborts_if !exists<PairedFungibleAssetRefs>(addr);
        aborts_if option::is_some(refs.burn_ref_opt);
        ensures global<PairedFungibleAssetRefs>(addr)
            == update_field(refs, burn_ref_opt, option::spec_some(burn_ref));
    }

    spec schema CoinSubAbortsIf<CoinType> {
        use aptos_framework::optional_aggregator;
        amount: u64;
        let addr = type_info::type_of<CoinType>().account_address;
        let maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        include (option::is_some(
            maybe_supply
        )) ==> optional_aggregator::SubAbortsIf { optional_aggregator: option::borrow(maybe_supply), value: amount };
    }

    spec schema CoinAddAbortsIf<CoinType> {
        use aptos_framework::optional_aggregator;
        amount: u64;
        let addr = type_info::type_of<CoinType>().account_address;
        let maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        include (option::is_some(
            maybe_supply
        )) ==> optional_aggregator::AddAbortsIf { optional_aggregator: option::borrow(maybe_supply), value: amount };
    }

    spec schema AbortsIfNotExistCoinInfo<CoinType> {
        let addr = type_info::type_of<CoinType>().account_address;
        aborts_if !type_info::spec_is_struct<CoinType>();
        aborts_if !exists<CoinInfo<CoinType>>(addr);
    }

    spec name<CoinType>(): string::String {
        pragma aborts_if_is_partial = false;
        /// [high-level-req-7.3]
        include AbortsIfNotExistCoinInfo<CoinType>;
    }

    spec symbol<CoinType>(): string::String {
        pragma aborts_if_is_partial = false;
        /// [high-level-req-7.4]
        include AbortsIfNotExistCoinInfo<CoinType>;
    }

    spec decimals<CoinType>(): u8 {
        pragma aborts_if_is_partial = false;
        include AbortsIfNotExistCoinInfo<CoinType>;
    }

    /// The coin supply plus the supply of the paired fungible asset, if the coin supply is tracked.
    spec supply<CoinType>(): Option<u128> {
        pragma aborts_if_is_partial = false;
        let maybe_supply = global<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address).supply;
        let coin_value = optional_aggregator::optional_aggregator_value(option::borrow(maybe_supply));
        let metadata = spec_paired_metadata(type_info::type_of<CoinType>());
        let paired = option::is_some(metadata);
        let fa_addr = option::borrow(metadata).inner;
        include AbortsIfNotExistCoinInfo<CoinType>;
        aborts_if paired && fa_addr != @aptos_fungible_asset && exists<fungible_asset::DeriveSupply>(fa_addr);
        aborts_if paired && option::is_some(maybe_supply)
            && !exists<fungible_asset::ConcurrentSupply>(fa_addr) && !exists<fungible_asset::Supply>(fa_addr);
        aborts_if paired && option::is_some(maybe_supply)
            && coin_value + fungible_asset::spec_supply(fa_addr) > MAX_U128;
        ensures option::is_some(result) == option::is_some(maybe_supply);
        ensures option::is_some(result) ==> option::borrow(result)
            == coin_value + if (paired) { fungible_asset::spec_supply(fa_addr) } else { 0 };
    }

    spec coin_supply<CoinType>(): Option<u128> {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        let coin_addr = type_info::type_of<CoinType>().account_address;
        /// [high-level-req-7.5]
        aborts_if !type_info::spec_is_struct<CoinType>();
        aborts_if !exists<CoinInfo<CoinType>>(coin_addr);
        let maybe_supply = global<CoinInfo<CoinType>>(coin_addr).supply;
        let supply = option::borrow(maybe_supply);
        let value = optional_aggregator::optional_aggregator_value(supply);

        ensures if (option::is_some(maybe_supply)) {
            result == option::spec_some(value)
        } else {
            option::is_none(result)
        };
    }

    /// Burning a coin decreases the tracked coin supply by its value.
    spec schema CoinBurn<CoinType> {
        coin: Coin<CoinType>;
        let addr = type_info::type_of<CoinType>().account_address;
        let maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        let post post_maybe_supply = global<CoinInfo<CoinType>>(addr).supply;
        modifies global<CoinInfo<CoinType>>(addr);
        aborts_if coin.value != 0 && !type_info::spec_is_struct<CoinType>();
        aborts_if coin.value != 0 && !exists<CoinInfo<CoinType>>(addr);
        include coin.value != 0 ==> CoinSubAbortsIf<CoinType> { amount: coin.value };
        ensures supply<CoinType> == old(supply<CoinType>) - coin.value;
        ensures if (option::is_some(maybe_supply)) {
            option::is_some(post_maybe_supply)
                && optional_aggregator::optional_aggregator_value(option::borrow(post_maybe_supply))
                    == optional_aggregator::optional_aggregator_value(option::borrow(maybe_supply)) - coin.value
        } else {
            option::is_none(post_maybe_supply)
        };
    }

    spec burn<CoinType>(coin: Coin<CoinType>, _cap: &BurnCapability<CoinType>) {
        pragma aborts_if_is_partial = false;
        include CoinBurn<CoinType>;
    }

    spec burn_internal<CoinType>(coin: Coin<CoinType>): u64 {
        pragma aborts_if_is_partial = false;
        include CoinBurn<CoinType>;
        ensures result == coin.value;
    }

    /// Burns `amount` from the paired primary store of `account_addr`, ignoring its frozen flag.
    spec schema PairedBurnFrom<CoinType> {
        account_addr: address;
        amount: u64;
        let paired = spec_paired_metadata(type_info::type_of<CoinType>());
        let store_addr = spec_paired_store_address(account_addr, paired);
        let fa_addr = global<fungible_asset::FungibleStore>(store_addr).metadata.inner;
        include amount != 0 ==> PairedRefsAbortsIf<CoinType>;
        aborts_if amount != 0
            && option::is_none(global<PairedFungibleAssetRefs>(option::borrow(paired).inner).burn_ref_opt);
        aborts_if amount != 0 && !exists<fungible_asset::FungibleStore>(store_addr);
        aborts_if amount != 0 && fungible_asset::spec_balance(store_addr) < amount;
        ensures fungible_asset::spec_balance(store_addr) == old(fungible_asset::spec_balance(store_addr)) - amount;
        ensures amount != 0 ==>
            fungible_asset::spec_supply(fa_addr) == old(fungible_asset::spec_supply(fa_addr)) - amount;
    }

    spec burn_from<CoinType>(
        account_addr: address, amount: u64, burn_cap: &BurnCapability<CoinType>
    ) {
        // Partial: the abort conditions of a mismatched burn ref and of the supply update are not specified.
        let paired = spec_paired_metadata(type_info::type_of<CoinType>());
        let store_addr = spec_paired_store_address(account_addr, paired);
        include PairedBurnFrom<CoinType>;
        /// [high-level-req-6.2]
        aborts_if amount != 0 && !exists<object::ObjectCore>(store_addr);
    }

    spec burn_from_for_gas<CoinType>(
        account_addr: address, amount: u64, burn_cap: &BurnCapability<CoinType>
    ) {
        // Partial: the abort conditions of a mismatched burn ref and of the supply update are not specified.
        include PairedBurnFrom<CoinType>;
    }

    /// The coin is converted and deposited into the paired primary store of `account_addr`, which is not frozen.
    spec deposit<CoinType>(account_addr: address, coin: Coin<CoinType>) {
        // Partial: the abort conditions of creating the pairing or the store, of the supply updates and of a
        // deposit hook are not specified.
        let paired = spec_paired_metadata(type_info::type_of<CoinType>());
        let store_addr = spec_paired_store_address(account_addr, paired);
        let existing = spec_has_paired_store(account_addr, paired);
        aborts_if !exists<CoinConversionMap>(@aptos_framework);
        /// [high-level-req-8.3]
        aborts_if existing && global<fungible_asset::FungibleStore>(store_addr).frozen;
        include PairedDepositBalance<CoinType> { owner: account_addr, amount: coin.value };
        ensures supply<CoinType> == old(supply<CoinType>) - coin.value;
        include CoinSupplyDelta<CoinType> { added: 0, removed: coin.value };
    }

    spec deposit_with_signer<CoinType>(account: &signer, coin: Coin<CoinType>) {
        // Partial: as for `deposit`.
        ensures supply<CoinType> == old(supply<CoinType>) - coin.value;
        include CoinSupplyDelta<CoinType> { added: 0, removed: coin.value };
    }

    /// Without a deposit hook, `amount` lands in the paired primary store of `owner`, which is created if missing.
    spec schema PairedDepositBalance<CoinType> {
        owner: address;
        amount: u64;
        let paired = spec_paired_metadata(type_info::type_of<CoinType>());
        let store_addr = spec_paired_store_address(owner, paired);
        let existing = exists<fungible_asset::FungibleStore>(store_addr);
        // A missing store is created with the paired metadata.
        let no_hook = if (existing) {
            fungible_asset::spec_store_no_deposit_hook(store_addr)
        } else {
            primary_fungible_store::spec_metadata_no_deposit_hook(option::borrow(paired).inner)
        };
        ensures option::is_some(paired) && no_hook ==> exists<fungible_asset::FungibleStore>(store_addr)
            && global<fungible_asset::FungibleStore>(store_addr).metadata == option::borrow(paired)
            && fungible_asset::spec_balance(store_addr)
                == (if (existing) { old(fungible_asset::spec_balance(store_addr)) } else { 0 }) + amount;
        // When the deposit itself pairs the coin, the amount lands in the store for the new metadata.
        let post post_paired = spec_paired_metadata(type_info::type_of<CoinType>());
        let post post_store = spec_paired_store_address(owner, post_paired);
        let post post_existing = old(exists<fungible_asset::FungibleStore>(post_store));
        let post post_no_hook = if (post_existing) {
            fungible_asset::spec_store_no_deposit_hook(post_store)
        } else {
            primary_fungible_store::spec_metadata_no_deposit_hook(option::borrow(post_paired).inner)
        };
        ensures option::is_none(paired) ==> option::is_some(post_paired);
        ensures option::is_none(paired) && post_no_hook ==>
            exists<fungible_asset::FungibleStore>(post_store)
            && global<fungible_asset::FungibleStore>(post_store).metadata == option::borrow(post_paired)
            && fungible_asset::spec_balance(post_store)
                == (if (post_existing) { old(fungible_asset::spec_balance(post_store)) } else { 0 }) + amount;
    }

    /// Burns the coin and mints the same amount of the paired fungible asset, pairing `CoinType` if needed.
    spec coin_to_fungible_asset<CoinType>(coin: Coin<CoinType>): FungibleAsset {
        // Partial: the abort conditions of creating the pairing and of the supply updates are not specified.
        let metadata = spec_paired_metadata(type_info::type_of<CoinType>());
        let fa_addr = option::borrow(metadata).inner;
        let post post_metadata = spec_paired_metadata(type_info::type_of<CoinType>());
        aborts_if !exists<CoinConversionMap>(@aptos_framework);
        ensures option::is_some(post_metadata);
        ensures result == FungibleAsset { metadata: option::borrow(post_metadata), amount: coin.value };
        ensures supply<CoinType> == old(supply<CoinType>) - coin.value;
        include CoinSupplyDelta<CoinType> { added: 0, removed: coin.value };
        ensures option::is_some(metadata) ==>
            fungible_asset::spec_supply(fa_addr) == old(fungible_asset::spec_supply(fa_addr)) + coin.value;
    }

    /// Burns a fungible asset paired with `CoinType` and mints the same amount of coin.
    spec fungible_asset_to_coin<CoinType>(fungible_asset: FungibleAsset): Coin<CoinType> {
        pragma aborts_if_is_partial = false;
        let metadata_addr = fungible_asset.metadata.inner;
        let amount = fungible_asset.amount;
        aborts_if !exists<object::ObjectCore>(metadata_addr) || !object::spec_exists_at<PairedCoinType>(metadata_addr);
        aborts_if !type_info::spec_is_struct<CoinType>();
        aborts_if global<PairedCoinType>(metadata_addr).type != type_info::type_of<CoinType>();
        aborts_if amount != 0 && !exists<fungible_asset::ConcurrentSupply>(metadata_addr)
            && !exists<fungible_asset::Supply>(metadata_addr);
        aborts_if amount != 0 && fungible_asset::spec_supply(metadata_addr) < amount;
        aborts_if amount != 0 && !exists<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address);
        include amount != 0 ==> CoinAddAbortsIf<CoinType> { amount };
        ensures result.value == fungible_asset.amount;
        ensures supply<CoinType> == old(supply<CoinType>) + fungible_asset.amount;
        include CoinSupplyDelta<CoinType> { added: fungible_asset.amount, removed: 0 };
        /// [high-level-req-4.3]
        include fungible_asset.metadata == option::borrow(spec_paired_metadata(type_info::type_of<CoinType>())) ==>
            TotalSupplyUnchanged<CoinType>;
        ensures fungible_asset::spec_supply(metadata_addr)
            == old(fungible_asset::spec_supply(metadata_addr)) - fungible_asset.amount;
    }

    /// Removes the `CoinStore`, moving its coin and frozen flag into the paired primary store.
    spec maybe_convert_to_fungible_store<CoinType>(account: address) {
        // Partial: the abort conditions of creating the pairing or the store and of the supply updates are not
        // specified.
        let coin_store = global<CoinStore<CoinType>>(account);
        let paired = spec_paired_metadata(type_info::type_of<CoinType>());
        let store_addr = spec_paired_store_address(account, paired);
        let moved = exists<CoinStore<CoinType>>(account) && coin_store.coin.value > 0
            && spec_has_paired_store(account, paired)
            && exists<CoinInfo<CoinType>>(type_info::type_of<CoinType>().account_address);
        ensures !exists<CoinStore<CoinType>>(account);
        ensures moved ==> global<fungible_asset::FungibleStore>(store_addr).frozen == coin_store.frozen
            && fungible_asset::spec_balance(store_addr)
                == old(fungible_asset::spec_balance(store_addr)) + coin_store.coin.value;
        let removed = if (exists<CoinStore<CoinType>>(account)) { coin_store.coin.value } else { 0 };
        ensures supply<CoinType> == old(supply<CoinType>) - removed;
        include CoinSupplyDelta<CoinType> { added: 0, removed };
    }

    spec migrate_to_fungible_store<CoinType>(account: &signer) {
        // Partial: as for `maybe_convert_to_fungible_store`.
        let addr = signer::address_of(account);
        let removed = if (exists<CoinStore<CoinType>>(addr)) { global<CoinStore<CoinType>>(addr).coin.value } else { 0 };
        ensures !exists<CoinStore<CoinType>>(addr);
        ensures supply<CoinType> == old(supply<CoinType>) - removed;
        include CoinSupplyDelta<CoinType> { added: 0, removed };
    }

    /// The coin is converted and deposited into the paired primary store of `account_addr`, ignoring its frozen flag.
    spec deposit_for_gas_fee<CoinType>(account_addr: address, coin: Coin<CoinType>) {
        // Partial: the abort conditions of creating the pairing or the store and of the supply updates are not
        // specified.
        let paired = spec_paired_metadata(type_info::type_of<CoinType>());
        let store_addr = spec_paired_store_address(account_addr, paired);
        let existing = spec_has_paired_store(account_addr, paired);
        aborts_if !exists<CoinConversionMap>(@aptos_framework);
        // A missing store is created; hooks are bypassed.
        ensures option::is_some(paired) ==> exists<fungible_asset::FungibleStore>(store_addr)
            && fungible_asset::spec_balance(store_addr)
                == (if (existing) { old(fungible_asset::spec_balance(store_addr)) } else { 0 }) + coin.value;
        // When the deposit itself pairs the coin, the amount lands in the store for the new metadata.
        let post post_paired = spec_paired_metadata(type_info::type_of<CoinType>());
        let post post_store = spec_paired_store_address(account_addr, post_paired);
        let post post_existing = old(exists<fungible_asset::FungibleStore>(post_store));
        ensures option::is_none(paired) ==> option::is_some(post_paired);
        ensures option::is_none(paired) ==> exists<fungible_asset::FungibleStore>(post_store)
            && fungible_asset::spec_balance(post_store)
                == (if (post_existing) { old(fungible_asset::spec_balance(post_store)) } else { 0 }) + coin.value;
        ensures supply<CoinType> == old(supply<CoinType>) - coin.value;
        include CoinSupplyDelta<CoinType> { added: 0, removed: coin.value };
    }

    /// The value of `zero_coin` must be 0.
    spec destroy_zero<CoinType>(zero_coin: Coin<CoinType>) {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if zero_coin.value > 0;
    }

    spec extract<CoinType>(coin: &mut Coin<CoinType>, amount: u64): Coin<CoinType> {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if coin.value < amount;
        ensures result.value == amount;
        ensures coin.value == old(coin.value) - amount;
    }

    spec extract_all<CoinType>(coin: &mut Coin<CoinType>): Coin<CoinType> {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if false;
        ensures result.value == old(coin).value;
        ensures coin.value == 0;
    }

    spec freeze_coin_store<CoinType>(
        account_addr: address, _freeze_cap: &FreezeCapability<CoinType>
    ) {
        pragma aborts_if_is_partial = false;
        modifies global<CoinStore<CoinType>>(account_addr);
        /// [high-level-req-6.3]
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        let post coin_store = global<CoinStore<CoinType>>(account_addr);
        ensures coin_store.frozen;
    }

    spec unfreeze_coin_store<CoinType>(
        account_addr: address, _freeze_cap: &FreezeCapability<CoinType>
    ) {
        pragma aborts_if_is_partial = false;
        modifies global<CoinStore<CoinType>>(account_addr);
        /// [high-level-req-6.4]
        aborts_if !exists<CoinStore<CoinType>>(account_addr);
        let post coin_store = global<CoinStore<CoinType>>(account_addr);
        ensures !coin_store.frozen;
    }

    /// The creator of `CoinType` must be `@aptos_framework`.
    /// A parallelizable supply is switched to a non-parallelizable one with the same value.
    spec upgrade_supply<CoinType>(account: &signer) {
        pragma aborts_if_is_partial = false;
        let account_addr = signer::address_of(account);
        let maybe_supply = global<CoinInfo<CoinType>>(account_addr).supply;
        let post post_maybe_supply = global<CoinInfo<CoinType>>(account_addr).supply;
        modifies global<CoinInfo<CoinType>>(account_addr);
        aborts_if account_addr != @aptos_framework;
        aborts_if !type_info::spec_is_struct<CoinType>();
        /// [high-level-req-1.1]
        aborts_if type_info::type_of<CoinType>().account_address != account_addr;
        aborts_if !exists<CoinInfo<CoinType>>(account_addr);
        ensures if (option::is_some(maybe_supply)) {
            option::is_some(post_maybe_supply)
                && !optional_aggregator::is_parallelizable(option::borrow(post_maybe_supply))
                && optional_aggregator::optional_aggregator_value(option::borrow(post_maybe_supply))
                    == optional_aggregator::optional_aggregator_value(option::borrow(maybe_supply))
        } else {
            option::is_none(post_maybe_supply)
        };
    }

    spec initialize {
        pragma aborts_if_is_partial = false;
        let account_addr = signer::address_of(account);
        /// [high-level-req-1.2]
        /// [high-level-req-2]
        include InitializeInternalSchema<CoinType> {
            name: name.bytes,
            symbol: symbol.bytes
        };
        modifies global<CoinInfo<CoinType>>(account_addr);
        ensures exists<CoinInfo<CoinType>>(account_addr);
        ensures result_1 == BurnCapability<CoinType> {};
        ensures result_2 == FreezeCapability<CoinType> {};
        ensures result_3 == MintCapability<CoinType> {};
    }

    // `account` must be `@aptos_framework`.
    spec initialize_with_parallelizable_supply<CoinType>(
        account: &signer,
        name: string::String,
        symbol: string::String,
        decimals: u8,
        monitor_supply: bool
    ): (BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>) {
        use aptos_framework::aggregator_factory;
        pragma opaque;
        pragma aborts_if_is_partial = false;
        let addr = signer::address_of(account);
        aborts_if addr != @aptos_framework;
        aborts_if monitor_supply
            && !exists<aggregator_factory::AggregatorFactory>(@aptos_framework);
        include InitializeInternalSchema<CoinType> {
            name: name.bytes,
            symbol: symbol.bytes
        };
        modifies global<CoinInfo<CoinType>>(addr);
        ensures exists<CoinInfo<CoinType>>(addr);
        ensures result_1 == BurnCapability<CoinType> {};
        ensures result_2 == FreezeCapability<CoinType> {};
        ensures result_3 == MintCapability<CoinType> {};
    }

    /// Make sure `name` and `symbol` are legal length.
    /// Only the creator of `CoinType` can initialize.
    spec schema InitializeInternalSchema<CoinType> {
        account: signer;
        name: vector<u8>;
        symbol: vector<u8>;
        decimals: u8;
        aborts_if decimals > MAX_DECIMALS;
        let account_addr = signer::address_of(account);
        let coin_address = type_info::type_of<CoinType>().account_address;
        aborts_if !type_info::spec_is_struct<CoinType>();
        aborts_if coin_address != account_addr;
        aborts_if exists<CoinInfo<CoinType>>(account_addr);
        aborts_if len(name) > MAX_COIN_NAME_LENGTH;
        aborts_if len(symbol) > MAX_COIN_SYMBOL_LENGTH;
    }

    spec initialize_internal<CoinType>(
        account: &signer,
        name: string::String,
        symbol: string::String,
        decimals: u8,
        monitor_supply: bool,
        parallelizable: bool
    ): (BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>) {
        use aptos_framework::aggregator_factory;
        pragma aborts_if_is_partial = false;
        include InitializeInternalSchema<CoinType> {
            name: name.bytes,
            symbol: symbol.bytes
        };
        let account_addr = signer::address_of(account);
        let post coin_info = global<CoinInfo<CoinType>>(account_addr);
        let post supply = option::borrow(coin_info.supply);
        let post value = optional_aggregator::optional_aggregator_value(supply);
        let post limit = optional_aggregator::optional_aggregator_limit(supply);
        modifies global<CoinInfo<CoinType>>(account_addr);
        aborts_if monitor_supply
            && parallelizable
            && !exists<aggregator_factory::AggregatorFactory>(@aptos_framework);
        /// [managed_coin::high-level-req-2]
        ensures exists<CoinInfo<CoinType>>(account_addr)
            && coin_info.name == name
            && coin_info.symbol == symbol && coin_info.decimals == decimals;
        ensures if (monitor_supply) {
            value == 0
                && limit == MAX_U128
                && (parallelizable == optional_aggregator::is_parallelizable(supply))
        } else {
            option::is_none(coin_info.supply)
        };
        ensures result_1 == BurnCapability<CoinType> {};
        ensures result_2 == FreezeCapability<CoinType> {};
        ensures result_3 == MintCapability<CoinType> {};
    }

    spec merge<CoinType>(dst_coin: &mut Coin<CoinType>, source_coin: Coin<CoinType>) {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if dst_coin.value + source_coin.value > MAX_U64;
        /// [high-level-req-3]
        ensures dst_coin.value == old(dst_coin.value) + source_coin.value;
        ensures supply<CoinType> == old(supply<CoinType>);
    }

    /// Registering only requires `CoinType` to be initialized and changes nothing.
    spec register<CoinType>(account: &signer) {
        pragma aborts_if_is_partial = false;
        /// [high-level-req-5]
        include AbortsIfNotExistCoinInfo<CoinType>;
    }

    /// Moves `amount` between the paired primary stores of `from` and `to`.
    spec transfer<CoinType>(from: &signer, to: address, amount: u64) {
        // Partial: the abort conditions of creating the pairing or the stores, unburning, the ownership check and
        // the dispatch hooks are not specified.
        let paired = spec_paired_metadata(type_info::type_of<CoinType>());
        let from_store = spec_paired_store_address(signer::address_of(from), paired);
        let to_store = spec_paired_store_address(to, paired);
        // The withdrawn asset is deposited into the primary store of its own metadata.
        let both = spec_has_paired_store(signer::address_of(from), paired) && spec_has_paired_store(to, paired)
            && global<fungible_asset::FungibleStore>(from_store).metadata == option::borrow(paired);
        include TransferAbortsIfAndEnsures<CoinType> { from: signer::address_of(from) };
        aborts_if both && fungible_asset::spec_store_no_withdraw_hook(from_store)
            && global<fungible_asset::FungibleStore>(to_store).frozen;
    }

    /// Transfer of `amount` between the paired primary stores of `from` and `to`: the source store must not be
    /// frozen and, without hooks, must hold enough; a missing target store is created.
    spec schema TransferAbortsIfAndEnsures<CoinType> {
        from: address;
        to: address;
        amount: u64;
        let paired = spec_paired_metadata(type_info::type_of<CoinType>());
        let from_store = spec_paired_store_address(from, paired);
        let to_store = spec_paired_store_address(to, paired);
        let plain = spec_has_paired_store(from, paired)
            && global<fungible_asset::FungibleStore>(from_store).metadata == option::borrow(paired)
            && primary_fungible_store::spec_is_plain_transfer_to(from_store, to_store, option::borrow(paired).inner);
        aborts_if !exists<CoinConversionMap>(@aptos_framework);
        /// [high-level-req-8.2]
        aborts_if spec_has_paired_store(from, paired) && global<fungible_asset::FungibleStore>(from_store).frozen;
        aborts_if plain && fungible_asset::spec_balance(from_store) < amount;
        include primary_fungible_store::PlainTransferBalancesToNew { from: from_store, to: to_store, amount, plain };
        // A missing source store is created empty, so without hooks only a zero transfer succeeds.
        let m = option::borrow(paired).inner;
        let fresh = option::is_some(paired) && !spec_has_paired_store(from, paired)
            && (!exists<fungible_asset::DispatchFunctionStore>(m)
                || option::is_none(global<fungible_asset::DispatchFunctionStore>(m).withdraw_function))
            && if (exists<fungible_asset::FungibleStore>(to_store)) {
                fungible_asset::spec_store_no_deposit_hook(to_store)
            } else {
                primary_fungible_store::spec_metadata_no_deposit_hook(m)
            };
        ensures fresh ==> amount == 0 && exists<fungible_asset::FungibleStore>(from_store)
            && exists<fungible_asset::FungibleStore>(to_store)
            && fungible_asset::spec_balance(to_store) == (if (old(exists<fungible_asset::FungibleStore>(to_store))) {
                old(fungible_asset::spec_balance(to_store))
            } else { 0 });
    }

    /// Withdraws from the paired primary store of the signer, which must not be frozen and hold enough balance.
    spec withdraw<CoinType>(account: &signer, amount: u64): Coin<CoinType> {
        // Partial: the abort conditions of creating the pairing or the store, unburning, the ownership check, a
        // withdraw hook and the supply updates are not specified.
        let account_addr = signer::address_of(account);
        let paired = spec_paired_metadata(type_info::type_of<CoinType>());
        let store_addr = spec_paired_store_address(account_addr, paired);
        let existing = spec_has_paired_store(account_addr, paired);
        let plain = existing && fungible_asset::spec_store_no_withdraw_hook(store_addr);
        aborts_if !exists<CoinConversionMap>(@aptos_framework);
        /// [high-level-req-8.1]
        aborts_if existing && global<fungible_asset::FungibleStore>(store_addr).frozen;
        aborts_if plain && fungible_asset::spec_balance(store_addr) < amount;
        ensures plain ==> result.value == amount
            && fungible_asset::spec_balance(store_addr) == old(fungible_asset::spec_balance(store_addr)) - amount;
        ensures supply<CoinType> == old(supply<CoinType>) + result.value;
        include CoinSupplyDelta<CoinType> { added: result.value, removed: 0 };
        include plain && global<fungible_asset::FungibleStore>(store_addr).metadata == option::borrow(paired) ==>
            TotalSupplyUnchanged<CoinType>;
    }
}
