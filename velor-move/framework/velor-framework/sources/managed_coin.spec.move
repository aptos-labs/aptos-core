spec velor_framework::managed_coin {
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
    /// Enforcement: Enforced via [coin::high-level-req-4](TotalSupplyNoChange).
    ///
    /// No.: 5
    /// Requirement: Before burning coins, exact amount of coins are withdrawn.
    /// Criticality: High
    /// Implementation: After utilizing the coin::withdraw function to withdraw coins, they are then burned,
    /// and the function ensures the precise return of the initially specified coin amount.
    /// Enforcement: Enforced via [coin::high-level-req-5](burn_from).
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

    spec burn<CoinType>(
        account: &signer,
        amount: u64,
    ) {
        use velor_std::type_info;
        // TODO(fa_migration)
        pragma verify = false;

        let account_addr = signer::address_of(account);

        // Resource Capabilities<CoinType> should exists in the signer address.
        aborts_if !exists<Capabilities<CoinType>>(account_addr);
        let coin_store = global<coin::CoinStore<CoinType>>(account_addr);
        let balance = coin_store.coin.value;

        // Resource CoinStore<CoinType> should exists in the signer.
        /// [high-level-req-3.2]
        /// [high-level-req-4.1]
        aborts_if !exists<coin::CoinStore<CoinType>>(account_addr);

        // Account should not be frozen and should have sufficient balance.
        aborts_if coin_store.frozen;
        aborts_if balance < amount;

        let addr =  type_info::type_of<CoinType>().account_address;
        let maybe_supply = global<coin::CoinInfo<CoinType>>(addr).supply;
        // Ensure the amount won't be overflow.
        aborts_if amount == 0;
        aborts_if !exists<coin::CoinInfo<CoinType>>(addr);
        include coin::CoinSubAbortsIf<CoinType> { amount:amount };

        // Ensure that the global 'supply' decreases by 'amount'.
        ensures coin::supply<CoinType> == old(coin::supply<CoinType>) - amount;
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

    /// The Capabilities<CoinType> should not exist in the signer address.
    /// The `dst_addr` should not be frozen.
    spec mint<CoinType>(
        account: &signer,
        dst_addr: address,
        amount: u64,
    ) {
        use velor_std::type_info;
        // TODO(fa_migration)
        pragma verify = false;
        let account_addr = signer::address_of(account);
        /// [high-level-req-3.3]
        aborts_if !exists<Capabilities<CoinType>>(account_addr);
        let addr = type_info::type_of<CoinType>().account_address;
        aborts_if (amount != 0) && !exists<coin::CoinInfo<CoinType>>(addr);
        let coin_store = global<coin::CoinStore<CoinType>>(dst_addr);
        aborts_if !exists<coin::CoinStore<CoinType>>(dst_addr);
        aborts_if coin_store.frozen;
        include coin::CoinAddAbortsIf<CoinType>;
        ensures coin::supply<CoinType> == old(coin::supply<CoinType>) + amount;
        /// [high-level-req-6]
        ensures global<coin::CoinStore<CoinType>>(dst_addr).coin.value == old(global<coin::CoinStore<CoinType>>(dst_addr)).coin.value + amount;
    }

    /// An account can only be registered once.
    /// Updating `Account.guid_creation_num` will not overflow.
    spec register<CoinType>(account: &signer) {
        use velor_framework::account;
        use velor_std::type_info;
        // TODO(fa_migration)
        pragma verify = false;

        let account_addr = signer::address_of(account);
        let acc = global<account::Account>(account_addr);

        aborts_if !exists<coin::CoinStore<CoinType>>(account_addr) && acc.guid_creation_num + 2 >= account::MAX_GUID_CREATION_NUM;
        aborts_if !exists<coin::CoinStore<CoinType>>(account_addr) && acc.guid_creation_num + 2 > MAX_U64;
        aborts_if !exists<coin::CoinStore<CoinType>>(account_addr) && !exists<account::Account>(account_addr);
        aborts_if !exists<coin::CoinStore<CoinType>>(account_addr) && !type_info::spec_is_struct<CoinType>();
        ensures exists<coin::CoinStore<CoinType>>(account_addr);
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
