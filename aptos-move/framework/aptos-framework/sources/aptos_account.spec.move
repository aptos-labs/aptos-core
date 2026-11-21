spec aptos_framework::aptos_account {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: During the creation of an Aptos account the following rules should hold: (1) the authentication key
    /// should be 32 bytes in length, (2) an Aptos account should not already exist for that authentication key, and (3)
    /// the address of the authentication key should not be equal to a reserved address (0x0, 0x1, or 0x3).
    /// Criticality: Critical
    /// Implementation: The authentication key which is passed in as an argument to create_account should satisfy all
    /// necessary conditions.
    /// Enforcement: Formally verified via [high-level-req-1](create_account).
    ///
    /// No.: 2
    /// Requirement: After creating an Aptos account, the account should become registered to receive AptosCoin.
    /// Criticality: Critical
    /// Implementation: The create_account function creates a new account for the particular address and registers
    /// AptosCoin.
    /// Enforcement: Audited that create_account creates the APT primary fungible store of the new account.
    ///
    /// No.: 3
    /// Requirement: An account may receive a direct transfer of coins they have not registered for if and only if the
    /// transfer of arbitrary coins is enabled. By default the option should always set to be enabled for an account.
    /// Criticality: Low
    /// Implementation: Transfers of a coin to an account that has not yet registered for that coin should abort
    /// if and only if the allow_arbitrary_coin_transfers flag is explicitly set to false.
    /// Enforcement: Formally verified via [high-level-req-3](can_receive_direct_coin_transfers).
    ///
    /// No.: 4
    /// Requirement: Setting direct coin transfers may only occur if and only if a direct transfer config is associated
    /// with the provided account address.
    /// Criticality: Low
    /// Implementation: The set_allow_direct_coin_transfers function ensures the DirectTransferConfig structure exists
    /// for the signer.
    /// Enforcement: Formally verified via [high-level-req-4](set_allow_direct_coin_transfers).
    ///
    /// No.: 5
    /// Requirement: After a transfer, the destination account should exist and a fungible store should exist at its
    /// APT primary store address.
    /// Criticality: Critical
    /// Implementation: The transfer function creates the recipient account if `account::exists_at` is false (an
    /// Account resource is created only when default account resources are disabled), then deposits into the
    /// fungible store at the recipient's APT primary store address, creating a store there if missing.
    /// Enforcement: Formally verified via [high-level-req-5.1](transfer) and [high-level-req-5.2](AptTransfer).
    ///
    /// No.: 6
    /// Requirement: Depositing coins should create the destination account if needed and land the coins in the
    /// destination's primary store paired with CoinType.
    /// Criticality: Critical
    /// Implementation: The deposit_coins function creates the recipient account if `account::exists_at` is false (an
    /// Account resource is created only when default account resources are disabled); when the deposit runs without
    /// a hook, the paired primary store exists afterwards and holds the deposited amount.
    /// Enforcement: Formally verified via [high-level-req-6.1](deposit_coins) and [high-level-req-6.2](deposit_coins).
    ///
    /// No.: 7
    /// Requirement: When performing a batch transfer of Aptos Coin and/or a batch transfer of a custom coin type, it
    /// should ensure that the vector containing destination addresses and the vector containing the corresponding
    /// amounts are equal in length.
    /// Criticality: Low
    /// Implementation: The batch_transfer and batch_transfer_coins functions verify that the length of the recipient
    /// addresses vector matches the length of the amount vector through an assertion.
    /// Enforcement: Formally verified via [high-level-req-7](batch_transfer_coins).
    /// </high-level-req>
    ///
    spec module {
        pragma aborts_if_is_strict;
    }

    /// Check if the bytes of the auth_key is 32.
    /// The Account does not exist under the auth_key before creating the account.
    /// Limit the address of auth_key is not @vm_reserved / @aptos_framework / @aptos_toke.
    spec create_account(auth_key: address) {
        pragma aborts_if_is_partial;
        /// [high-level-req-1]
        include CreateAccountAbortsIf;
        // ensures exists<account::Account>(auth_key);
    }
    spec schema CreateAccountAbortsIf {
        auth_key: address;
        aborts_if exists<account::Account>(auth_key);
        // aborts_if length_judgment(auth_key);
        aborts_if auth_key == @vm_reserved || auth_key == @aptos_framework || auth_key == @aptos_token;
    }

    spec fun length_judgment(auth_key: address): bool {
        use std::bcs;

        let authentication_key = bcs::to_bytes(auth_key);
        len(authentication_key) != 32
    }

    spec transfer(source: &signer, to: address, amount: u64) {
        // Partial: the abort conditions of creating the primary stores are not specified.
        pragma aborts_if_is_partial;
        include CreateAccountTransferAbortsIf;
        include AptTransfer;
        /// [high-level-req-5.1]
        ensures account::spec_exists_at(to);
    }

    spec assert_account_exists(addr: address) {
        aborts_if !account::spec_exists_at(addr);
    }

    /// Check if the address existed.
    /// Check if the AptosCoin under the address existed.
    spec assert_account_is_registered_for_apt(addr: address) {
        aborts_if !account::spec_exists_at(addr);
        aborts_if !exists<coin::CoinInfo<AptosCoin>>(@aptos_framework);
    }

    spec set_allow_direct_coin_transfers(account: &signer, allow: bool) {
        let addr = signer::address_of(account);
        include !exists<DirectTransferConfig>(addr) ==>
            account::NewEventHandleAbortsIf { account };
        modifies global<DirectTransferConfig>(addr);
        modifies global<account::Account>(addr);
        /// [high-level-req-4]
        ensures exists<DirectTransferConfig>(addr);
        ensures global<DirectTransferConfig>(addr).allow_arbitrary_coin_transfers == allow;
    }

    spec batch_transfer(source: &signer, recipients: vector<address>, amounts: vector<u64>) {
        // Partial: only the length check is specified; each transfer is specified by `transfer`.
        pragma aborts_if_is_partial;
        aborts_if len(recipients) != len(amounts);
    }

    spec can_receive_direct_coin_transfers(account: address): bool {
        aborts_if false;
        /// [high-level-req-3]
        ensures result == (
            !exists<DirectTransferConfig>(account) ||
                global<DirectTransferConfig>(account).allow_arbitrary_coin_transfers
        );
    }

    spec batch_transfer_coins<CoinType>(from: &signer, recipients: vector<address>, amounts: vector<u64>) {
        // Partial: only the length check is specified.
        pragma aborts_if_is_partial;
        /// [high-level-req-7]
        aborts_if len(recipients) != len(amounts);
    }

    /// The coin is deposited into the paired primary store of `to`, creating the account if needed.
    spec deposit_coins<CoinType>(to: address, coins: Coin<CoinType>) {
        use aptos_std::type_info;
        // Partial: the abort conditions of creating the pairing or the store, of the supply updates and of a
        // deposit hook are not specified.
        pragma aborts_if_is_partial;
        let paired = coin::spec_paired_metadata(type_info::type_of<CoinType>());
        let store_addr = coin::spec_paired_store_address(to, paired);
        let existing = coin::spec_has_paired_store(to, paired);
        include CreateAccountTransferAbortsIf;
        include coin::AbortsIfNotExistCoinInfo<CoinType>;
        aborts_if existing && global<fungible_asset::FungibleStore>(store_addr).frozen;
        /// [high-level-req-6.1]
        ensures account::spec_exists_at(to);
        let no_alias = spec_no_apt_store_alias(to, std::option::borrow(paired).inner, store_addr);
        // A deposit that pairs the coin creates the store for the new metadata.
        let post post_paired = coin::spec_paired_metadata(type_info::type_of<CoinType>());
        let post post_no_alias = old(spec_no_apt_store_alias(
            to, std::option::borrow(post_paired).inner, coin::spec_paired_store_address(to, post_paired)
        ));
        /// [high-level-req-6.2]
        include (if (std::option::is_some(paired)) { no_alias } else { post_no_alias }) ==>
            coin::PairedDepositBalance<CoinType> { owner: to, amount: coins.value };
    }

    /// The asset is deposited into the primary store of `to`, creating the account if needed.
    spec deposit_fungible_assets(to: address, fa: FungibleAsset) {
        // Partial: the abort conditions of creating the store and of a deposit hook are not specified.
        pragma aborts_if_is_partial;
        let store_addr = primary_fungible_store::spec_primary_store_address(to, fa.metadata);
        let existing = exists<fungible_asset::FungibleStore>(store_addr);
        include CreateAccountTransferAbortsIf;
        aborts_if existing && global<fungible_asset::FungibleStore>(store_addr).frozen;
        ensures account::spec_exists_at(to);
        let no_alias = spec_no_apt_store_alias(to, fa.metadata.inner, store_addr);
        include no_alias ==> primary_fungible_store::PrimaryDepositBalance { owner: to };
    }

    /// Mints `amount` into the APT primary store of `account`, ignoring its frozen flag.
    spec mint_to_fungible_store_for_gas(ref: &MintRef, account: address, amount: u64) {
        // Partial: the abort conditions of creating the store and of the supply update are not specified.
        pragma aborts_if_is_partial;
        let store_addr = spec_apt_store_address(account);
        let existing = exists<fungible_asset::FungibleStore>(store_addr);
        aborts_if amount != 0 && existing && global<fungible_asset::FungibleStore>(store_addr).metadata != ref.metadata;
        // A nonzero mint creates a missing store.
        ensures existing || amount != 0 ==> exists<fungible_asset::FungibleStore>(store_addr)
            && fungible_asset::spec_balance(store_addr)
                == (if (existing) { old(fungible_asset::spec_balance(store_addr)) } else { 0 }) + amount;
        ensures amount != 0 ==> fungible_asset::spec_supply(ref.metadata.inner)
            == old(fungible_asset::spec_supply(ref.metadata.inner)) + amount;
    }

    /// Moves `amount` from the primary store of `from` into the primary store of `to`, creating the account if
    /// needed.
    spec transfer_fungible_assets(from: &signer, metadata: Object<Metadata>, to: address, amount: u64) {
        // Partial: the abort conditions of creating the stores, unburning, the ownership check and the dispatch
        // hooks are not specified.
        pragma aborts_if_is_partial;
        let from_store = primary_fungible_store::spec_primary_store_address(signer::address_of(from), metadata);
        let to_store = primary_fungible_store::spec_primary_store_address(to, metadata);
        // The withdrawn asset is deposited into the primary store of its own metadata.
        let both = exists<fungible_asset::FungibleStore>(from_store) && exists<fungible_asset::FungibleStore>(to_store)
            && global<fungible_asset::FungibleStore>(from_store).metadata == metadata;
        let plain = exists<fungible_asset::FungibleStore>(from_store)
            && global<fungible_asset::FungibleStore>(from_store).metadata == metadata
            && primary_fungible_store::spec_is_plain_transfer_to(from_store, to_store, metadata.inner)
            && spec_no_apt_store_alias(to, metadata.inner, to_store);
        aborts_if exists<fungible_asset::FungibleStore>(from_store)
            && global<fungible_asset::FungibleStore>(from_store).frozen;
        aborts_if both && fungible_asset::spec_store_no_withdraw_hook(from_store)
            && global<fungible_asset::FungibleStore>(to_store).frozen;
        aborts_if plain && fungible_asset::spec_balance(from_store) < amount;
        ensures account::spec_exists_at(to);
        include primary_fungible_store::PlainTransferBalancesToNew { from: from_store, to: to_store, amount, plain };
    }

    spec batch_transfer_fungible_assets(from: &signer, metadata: Object<Metadata>, recipients: vector<address>, amounts: vector<u64>) {
        // Partial: only the length check is specified; each transfer is specified by `transfer_fungible_assets`.
        pragma aborts_if_is_partial;
        aborts_if len(recipients) != len(amounts);
    }

    /// Moves `amount` between the paired primary stores of `from` and `to`, creating the account if needed.
    spec transfer_coins<CoinType>(from: &signer, to: address, amount: u64) {
        use aptos_std::type_info;
        // Partial: the abort conditions of creating the pairing or the stores, unburning, the ownership check, the
        // dispatch hooks and the supply updates are not specified.
        pragma aborts_if_is_partial;
        let paired = coin::spec_paired_metadata(type_info::type_of<CoinType>());
        let from_store = coin::spec_paired_store_address(signer::address_of(from), paired);
        let to_store = coin::spec_paired_store_address(to, paired);
        let plain = coin::spec_has_paired_store(signer::address_of(from), paired)
            && primary_fungible_store::spec_is_plain_transfer_to(
                from_store, to_store, std::option::borrow(paired).inner
            ) && spec_no_apt_store_alias(to, std::option::borrow(paired).inner, to_store);
        aborts_if coin::spec_has_paired_store(signer::address_of(from), paired)
            && global<fungible_asset::FungibleStore>(from_store).frozen;
        aborts_if plain && fungible_asset::spec_balance(from_store) < amount;
        ensures account::spec_exists_at(to);
        include primary_fungible_store::PlainTransferBalancesToNew { from: from_store, to: to_store, amount, plain };
    }

    /// Creates the APT primary store of the signer if missing.
    spec register_apt(account_signer: &signer) {
        pragma aborts_if_is_partial = false;
        let store_addr = spec_apt_store_address(signer::address_of(account_signer));
        let m = @aptos_fungible_asset;
        let missing = !exists<fungible_asset::FungibleStore>(store_addr);
        aborts_if missing && (!exists<object::ObjectCore>(m) || !exists<fungible_asset::Metadata>(m));
        aborts_if missing && !exists<primary_fungible_store::DeriveRefPod>(m);
        aborts_if missing && exists<object::ObjectCore>(store_addr);
        aborts_if missing && exists<object::Untransferable>(store_addr);
        aborts_if missing && std::features::spec_is_enabled(std::features::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE)
            && exists<fungible_asset::ConcurrentFungibleBalance>(store_addr);
        ensures exists<fungible_asset::FungibleStore>(store_addr);
    }

    spec fungible_transfer_only(source: &signer, to: address, amount: u64) {
        // Partial: the abort conditions of creating the primary stores are not specified.
        pragma aborts_if_is_partial;
        include AptTransfer;
    }

    spec is_fungible_balance_at_least(account: address, amount: u64): bool {
        ensures result == fungible_asset::spec_is_address_balance_at_least(spec_apt_store_address(account), amount);
    }

    spec burn_from_fungible_store_for_gas(
        ref: &BurnRef,
        account: address,
        amount: u64,
    ) {
        include BurnFromAptStore;
    }

    /// Burns `amount` from the APT primary store of `account`, ignoring its frozen flag.
    spec schema BurnFromAptStore {
        ref: BurnRef;
        account: address;
        amount: u64;
        let store_addr = spec_apt_store_address(account);
        let metadata_addr = global<fungible_asset::FungibleStore>(store_addr).metadata.inner;
        aborts_if amount != 0 && !exists<fungible_asset::FungibleStore>(store_addr);
        aborts_if amount != 0 && fungible_asset::spec_balance(store_addr) < amount;
        aborts_if amount != 0 && ref.metadata != global<fungible_asset::FungibleStore>(store_addr).metadata;
        aborts_if amount != 0 && !exists<fungible_asset::ConcurrentSupply>(metadata_addr)
            && !exists<fungible_asset::Supply>(metadata_addr);
        aborts_if amount != 0 && fungible_asset::spec_supply(metadata_addr) < amount;
        ensures fungible_asset::spec_balance(store_addr) == old(fungible_asset::spec_balance(store_addr)) - amount;
        ensures amount != 0 ==>
            fungible_asset::spec_supply(metadata_addr) == old(fungible_asset::spec_supply(metadata_addr)) - amount;
    }


    /// Creating `owner`'s account adds its APT store, which may share the derived address `store_addr` of the
    /// primary store for asset `m` only if that store already exists or `m` is APT.
    spec fun spec_no_apt_store_alias(owner: address, m: address, store_addr: address): bool {
        account::spec_exists_at(owner) || exists<fungible_asset::FungibleStore>(store_addr)
            || m == @aptos_fungible_asset || store_addr != spec_apt_store_address(owner)
    }

    /// The APT primary store of `account`.
    spec fun spec_apt_store_address(account: address): address {
        object::spec_create_user_derived_object_address(account, @aptos_fungible_asset)
    }

    /// Moves `amount` between the APT primary stores of `source` and `to`, ignoring frozen flags and hooks; a missing
    /// store at `to` is created.
    spec schema AptTransfer {
        source: &signer;
        to: address;
        amount: u64;
        let from_store = spec_apt_store_address(signer::address_of(source));
        let to_store = spec_apt_store_address(to);
        let existing = exists<fungible_asset::FungibleStore>(from_store);
        let both = existing && exists<fungible_asset::FungibleStore>(to_store);
        aborts_if existing && fungible_asset::spec_balance(from_store) < amount;
        aborts_if both && global<fungible_asset::FungibleStore>(from_store).metadata
            != global<fungible_asset::FungibleStore>(to_store).metadata;
        include primary_fungible_store::PlainTransferBalancesToNew { from: from_store, to: to_store, amount, plain: existing };
        /// [high-level-req-5.2]
        ensures exists<fungible_asset::FungibleStore>(to_store);
    }

    spec schema CreateAccountTransferAbortsIf {
        to: address;
        aborts_if !account::spec_exists_at(to) && length_judgment(to);
        aborts_if !account::spec_exists_at(to) && (to == @vm_reserved || to == @aptos_framework || to == @aptos_token);
    }
}
