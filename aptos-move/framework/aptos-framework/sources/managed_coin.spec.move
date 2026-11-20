spec aptos_framework::managed_coin {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The initializing account should hold the capabilities to operate the coin.
    /// Criticality: Critical
    /// Implementation: The capabilities are stored under the initializing account under the Capabilities resource,
    /// which is distinct for a distinct type of coin.
    /// Enforcement: Enforced via [high-level-req-1](initialize).
    ///
    /// No.: 2
    /// Requirement: A new coin should be properly initialized.
    /// Criticality: High
    /// Implementation: In the initialize function, a new coin is initialized via the coin module with the specified
    /// properties.
    /// Enforcement: Enforced via [coin::high-level-req-2](initialize_internal).
    ///
    /// No.: 3
    /// Requirement: Minting/Burning should only be done by the account who hold the valid capabilities.
    /// Criticality: High
    /// Implementation: The mint and burn capabilities are moved under the initializing account and retrieved, while
    /// minting/burning
    /// Enforcement: Enforced via: [high-level-req-3.1](initialize), [high-level-req-3.2](burn),
    /// [high-level-req-3.3](mint).
    ///
    /// No.: 4
    /// Requirement: If the total supply of coins is being monitored, burn and mint operations will appropriately adjust
    /// the total supply.
    /// Criticality: High
    /// Implementation: The coin::burn and coin::mint functions, when tracking the supply, adjusts the total coin
    /// supply accordingly.
    /// Enforcement: Formally verified via [high-level-req-4.1](burn), [high-level-req-4.2](mint) and
    /// [high-level-req-4.3](mint) that burn and mint adjust the supply of the paired fungible asset by the amount (for
    /// burn, when the withdrawal runs without a withdraw hook), while the tracked coin supply and the value held in
    /// coins are unchanged (coin::CoinSupplyUnchanged). Formally verified via [high-level-req-4.4](mint) and
    /// [high-level-req-4.5](burn) that, for a tracked coin supply and an existing pairing, the coin supply plus the
    /// paired asset supply grows by the minted amount, and drops by the burned amount when the withdrawal runs without
    /// a withdraw hook from a store of the paired asset.
    ///
    /// No.: 5
    /// Requirement: Before burning coins, exact amount of coins are withdrawn.
    /// Criticality: High
    /// Implementation: The burn function withdraws amount via coin::withdraw from the paired primary store of the
    /// signer and burns it; when the withdrawal runs without a hook, the store balance drops by exactly amount.
    /// Enforcement: Enforced via [high-level-req-5](burn).
    ///
    /// No.: 6
    /// Requirement: Minted coins are deposited to the provided destination address.
    /// Criticality: High
    /// Implementation: After the coins are minted via coin::mint they are deposited into the coinstore of the
    /// destination address.
    /// Enforcement: Enforced via [high-level-req-6](mint).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_partial;
    }

    /// Withdraws `amount` from the paired primary store of the signer and burns it.
    spec burn<CoinType>(
        account: &signer,
        amount: u64,
    ) {
        use aptos_std::type_info;
        use aptos_framework::fungible_asset;
        use std::option;
        // Partial: the abort conditions of the withdrawal (see `coin::withdraw`) and of the burn are not fully
        // specified.
        let account_addr = signer::address_of(account);
        let paired = coin::spec_paired_metadata(type_info::type_of<CoinType>());
        let store_addr = coin::spec_paired_store_address(account_addr, paired);
        let existing = coin::spec_has_paired_store(account_addr, paired);
        let plain = existing && fungible_asset::spec_store_no_withdraw_hook(store_addr);

        // Resource Capabilities<CoinType> should exists in the signer address.
        /// [high-level-req-3.2]
        aborts_if !exists<Capabilities<CoinType>>(account_addr);

        // The paired store should not be frozen and should have sufficient balance.
        aborts_if !exists<coin::CoinConversionMap>(@aptos_framework);
        aborts_if existing && global<fungible_asset::FungibleStore>(store_addr).frozen;
        aborts_if plain && fungible_asset::spec_balance(store_addr) < amount;
        /// [high-level-req-5]
        ensures plain ==>
            fungible_asset::spec_balance(store_addr) == old(fungible_asset::spec_balance(store_addr)) - amount;
        // The coin's tracked supply is unchanged; the burned amount is removed from the store's asset supply.
        let fa_addr = global<fungible_asset::FungibleStore>(store_addr).metadata.inner;
        include coin::CoinSupplyUnchanged<CoinType>;
        /// [high-level-req-4.1]
        ensures plain ==> fungible_asset::spec_supply(fa_addr) == old(fungible_asset::spec_supply(fa_addr)) - amount;
        /// [high-level-req-4.5]
        include plain && fa_addr == option::borrow(paired).inner ==>
            TotalSupplyChange<CoinType> { added: 0, removed: amount };
    }

    /// For a tracked coin supply and an existing pairing, the coin supply plus the paired asset supply changes by
    /// `added - removed`.
    spec schema TotalSupplyChange<CoinType> {
        use aptos_std::type_info;
        use std::option;
        added: num;
        removed: num;
        let coin_type = type_info::type_of<CoinType>();
        let tracked = option::is_some(global<coin::CoinInfo<CoinType>>(coin_type.account_address).supply)
            && option::is_some(coin::spec_paired_metadata(coin_type));
        ensures tracked ==> coin::spec_total_supply<CoinType>(coin_type)
            == old(coin::spec_total_supply<CoinType>(coin_type)) + added - removed;
    }

    /// Make sure `name` and `symbol` are legal length.
    /// Only the creator of `CoinType` can initialize.
    /// The 'name' and 'symbol' should be valid utf8 bytes
    /// The Capabilities<CoinType> should not be under the signer before creating;
    /// The Capabilities<CoinType> should be under the signer after creating;
    spec initialize<CoinType>(
        account: &signer,
        name: vector<u8>,
        symbol: vector<u8>,
        decimals: u8,
        monitor_supply: bool,
    ) {
        include coin::InitializeInternalSchema<CoinType>;
        aborts_if !string::spec_internal_check_utf8(name);
        aborts_if !string::spec_internal_check_utf8(symbol);
        aborts_if exists<Capabilities<CoinType>>(signer::address_of(account));
        /// [high-level-req-1]
        /// [high-level-req-3.1]
        ensures exists<Capabilities<CoinType>>(signer::address_of(account));
    }

    /// The Capabilities<CoinType> should exist in the signer address.
    /// The paired primary store of `dst_addr` should not be frozen.
    spec mint<CoinType>(
        account: &signer,
        dst_addr: address,
        amount: u64,
    ) {
        use aptos_std::type_info;
        use aptos_framework::fungible_asset;
        use std::option;
        // Partial: the abort conditions of creating the pairing or the store, of the supply updates and of a
        // deposit hook are not specified.
        let account_addr = signer::address_of(account);
        /// [high-level-req-3.3]
        aborts_if !exists<Capabilities<CoinType>>(account_addr);
        let addr = type_info::type_of<CoinType>().account_address;
        aborts_if (amount != 0) && !exists<coin::CoinInfo<CoinType>>(addr);
        let paired = coin::spec_paired_metadata(type_info::type_of<CoinType>());
        let store_addr = coin::spec_paired_store_address(dst_addr, paired);
        aborts_if !exists<coin::CoinConversionMap>(@aptos_framework);
        aborts_if coin::spec_has_paired_store(dst_addr, paired)
            && global<fungible_asset::FungibleStore>(store_addr).frozen;
        /// [high-level-req-6]
        include coin::PairedDepositBalance<CoinType> { owner: dst_addr, amount };
        // The coin's tracked supply is unchanged; the minted amount is added to the paired asset's supply.
        include coin::CoinSupplyUnchanged<CoinType>;
        /// [high-level-req-4.2]
        ensures option::is_some(paired) ==> fungible_asset::spec_supply(option::borrow(paired).inner)
            == old(fungible_asset::spec_supply(option::borrow(paired).inner)) + amount;
        /// [high-level-req-4.4]
        include TotalSupplyChange<CoinType> { added: amount, removed: 0 };
        // On the first mint the minted amount is added to the supply of the newly paired asset.
        let post post_fa_addr = option::borrow(coin::spec_paired_metadata(type_info::type_of<CoinType>())).inner;
        /// [high-level-req-4.3]
        ensures option::is_none(paired) ==> fungible_asset::spec_supply(post_fa_addr)
            == (if (old(exists<fungible_asset::ConcurrentSupply>(post_fa_addr))) {
                old(fungible_asset::spec_supply(post_fa_addr))
            } else { 0 }) + amount;
    }

    /// Registering requires an initialized coin.
    spec register<CoinType>(account: &signer) {
        pragma aborts_if_is_partial = false;
        include coin::AbortsIfNotExistCoinInfo<CoinType>;
    }

    spec remove_caps<CoinType>(account: &signer): (BurnCapability<CoinType>, FreezeCapability<CoinType>, MintCapability<CoinType>) {
        let account_addr = signer::address_of(account);
        aborts_if !exists<Capabilities<CoinType>>(account_addr);
        ensures !exists<Capabilities<CoinType>>(account_addr);
    }

    spec destroy_caps <CoinType>(account: &signer) {
        let account_addr = signer::address_of(account);
        aborts_if !exists<Capabilities<CoinType>>(account_addr);
        ensures !exists<Capabilities<CoinType>>(account_addr);
    }
}
