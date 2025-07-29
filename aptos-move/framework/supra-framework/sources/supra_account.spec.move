spec supra_framework::supra_account {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: During the creation of an Supra account the following rules should hold: (1) the authentication key
    /// should be 32 bytes in length, (2) an Supra account should not already exist for that authentication key, and (3)
    /// the address of the authentication key should not be equal to a reserved address (0x0, 0x1, or 0x3).
    /// Criticality: Critical
    /// Implementation: The authentication key which is passed in as an argument to create_account should satisfy all
    /// necessary conditions.
    /// Enforcement: Formally verified via [high-level-req-1](CreateAccountAbortsIf).
    ///
    /// No.: 2
    /// Requirement: After creating an Supra account, the account should become registered to receive SupraCoin.
    /// Criticality: Critical
    /// Implementation: The create_account function creates a new account for the particular address and registers
    /// SupraCoin.
    /// Enforcement: Formally verified via [high-level-req-2](create_account).
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
    /// Requirement: The transfer function should ensure an account is created for the provided destination if one does not
    /// exist; then, register SupraCoin for that account if a particular is unregistered before transferring the amount.
    /// Criticality: Critical
    /// Implementation: The transfer function checks if the recipient account exists. If the account does not exist,
    /// the function creates one and registers the account to SupraCoin if not registered.
    /// Enforcement: Formally verified via [high-level-req-5](transfer).
    ///
    /// No.: 6
    /// Requirement: Creating an account for the provided destination and registering it for that particular CoinType
    /// should be the only way to enable depositing coins, provided the account does not already exist.
    /// Criticality: Critical
    /// Implementation: The deposit_coins function verifies if the recipient account exists. If the account does not
    /// exist, the function creates one and ensures that the account becomes registered for the specified CointType.
    /// Enforcement: Formally verified via [high-level-req-6](deposit_coins).
    ///
    /// No.: 7
    /// Requirement: When performing a batch transfer of Supra Coin and/or a batch transfer of a custom coin type, it
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
    /// Limit the address of auth_key is not @vm_reserved / @supra_framework / @aptos_toke.
    spec create_account(auth_key: address) {
        /// [high-level-req-1]
        pragma aborts_if_is_partial;
        include CreateAccountAbortsIf;
        ensures exists<account::Account>(auth_key);
    }
    spec schema CreateAccountAbortsIf {
        auth_key: address;
        aborts_if exists<account::Account>(auth_key);
        aborts_if length_judgment(auth_key);
        aborts_if auth_key == @vm_reserved || auth_key == @supra_framework || auth_key == @aptos_token;
    }

    spec fun length_judgment(auth_key: address): bool {
        use std::bcs;

        let authentication_key = bcs::to_bytes(auth_key);
        len(authentication_key) != 32
    }

    spec transfer(source: &signer, to: address, amount: u64) {
        // TODO(fa_migration)
        pragma verify = false;
        let account_addr_source = signer::address_of(source);

        // The 'from' addr is implictly not equal to 'to' addr
        requires account_addr_source != to;

        include CreateAccountTransferAbortsIf;
        include GuidAbortsIf<SupraCoin>;
        include WithdrawAbortsIf<SupraCoin>{from: source};
        include TransferEnsures<SupraCoin>;

        aborts_if exists<coin::CoinStore<SupraCoin>>(to) && global<coin::CoinStore<SupraCoin>>(to).frozen;
        /// [high-level-req-5]
        ensures exists<supra_framework::account::Account>(to);
        ensures exists<coin::CoinStore<SupraCoin>>(to);
    }

    spec assert_account_exists(addr: address) {
        aborts_if !account::exists_at(addr);
    }

    /// Check if the address existed.
    /// Check if the SupraCoin under the address existed.
    spec assert_account_is_registered_for_supra(addr: address) {
        pragma aborts_if_is_partial;
        aborts_if !account::exists_at(addr);
        aborts_if !coin::spec_is_account_registered<SupraCoin>(addr);
    }

    spec set_allow_direct_coin_transfers(account: &signer, allow: bool) {
        // TODO(fa_migration)
        pragma verify = false;
        // let addr = signer::address_of(account);
        // include !exists<DirectTransferConfig>(addr) ==> account::NewEventHandleAbortsIf;
        // ensures global<DirectTransferConfig>(addr).allow_arbitrary_coin_transfers == allow;
    }

    spec batch_transfer(source: &signer, recipients: vector<address>, amounts: vector<u64>) {
        //TODO: Can't verify the loop invariant in enumerate
        pragma verify = false;
        let account_addr_source = signer::address_of(source);
        let coin_store_source = global<coin::CoinStore<SupraCoin>>(account_addr_source);
        let balance_source = coin_store_source.coin.value;

        requires forall i in 0..len(recipients):
            recipients[i] != account_addr_source;
        requires exists i in 0..len(recipients):
            amounts[i] > 0;

        // create account properties
        aborts_if len(recipients) != len(amounts);
        aborts_if exists i in 0..len(recipients):
                !account::exists_at(recipients[i]) && length_judgment(recipients[i]);
        aborts_if exists i in 0..len(recipients):
                !account::exists_at(recipients[i]) && (recipients[i] == @vm_reserved || recipients[i] == @supra_framework || recipients[i] == @aptos_token);
        ensures forall i in 0..len(recipients):
                (!account::exists_at(recipients[i]) ==> !length_judgment(recipients[i])) &&
                    (!account::exists_at(recipients[i]) ==> (recipients[i] != @vm_reserved && recipients[i] != @supra_framework && recipients[i] != @aptos_token));

        // coin::withdraw properties
        aborts_if exists i in 0..len(recipients):
            !exists<coin::CoinStore<SupraCoin>>(account_addr_source);
        aborts_if exists i in 0..len(recipients):
            coin_store_source.frozen;
        aborts_if exists i in 0..len(recipients):
            global<coin::CoinStore<SupraCoin>>(account_addr_source).coin.value < amounts[i];

        // deposit properties
        aborts_if exists i in 0..len(recipients):
            exists<coin::CoinStore<SupraCoin>>(recipients[i]) && global<coin::CoinStore<SupraCoin>>(recipients[i]).frozen;

        // guid properties
        aborts_if exists i in 0..len(recipients):
            account::exists_at(recipients[i]) && !exists<coin::CoinStore<SupraCoin>>(recipients[i]) && global<account::Account>(recipients[i]).guid_creation_num + 2 >= account::MAX_GUID_CREATION_NUM;
        aborts_if exists i in 0..len(recipients):
            account::exists_at(recipients[i]) && !exists<coin::CoinStore<SupraCoin>>(recipients[i]) && global<account::Account>(recipients[i]).guid_creation_num + 2 > MAX_U64;
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
        //TODO: Can't verify the loop invariant in enumerate
        use aptos_std::type_info;
        pragma verify = false;
        let account_addr_source = signer::address_of(from);
        let coin_store_source = global<coin::CoinStore<CoinType>>(account_addr_source);
        let balance_source = coin_store_source.coin.value;

        requires forall i in 0..len(recipients):
            recipients[i] != account_addr_source;

        requires exists i in 0..len(recipients):
            amounts[i] > 0;

        /// [high-level-req-7]
        aborts_if len(recipients) != len(amounts);

        //create account properties
        aborts_if exists i in 0..len(recipients):
                !account::exists_at(recipients[i]) && length_judgment(recipients[i]);
        aborts_if exists i in 0..len(recipients):
                !account::exists_at(recipients[i]) && (recipients[i] == @vm_reserved || recipients[i] == @supra_framework || recipients[i] == @aptos_token);
        ensures forall i in 0..len(recipients):
                (!account::exists_at(recipients[i]) ==> !length_judgment(recipients[i])) &&
                    (!account::exists_at(recipients[i]) ==> (recipients[i] != @vm_reserved && recipients[i] != @supra_framework && recipients[i] != @aptos_token));

        // coin::withdraw properties
        aborts_if exists i in 0..len(recipients):
            !exists<coin::CoinStore<CoinType>>(account_addr_source);
        aborts_if exists i in 0..len(recipients):
            coin_store_source.frozen;
        aborts_if exists i in 0..len(recipients):
            global<coin::CoinStore<CoinType>>(account_addr_source).coin.value < amounts[i];

        // deposit properties
        aborts_if exists i in 0..len(recipients):
            exists<coin::CoinStore<CoinType>>(recipients[i]) && global<coin::CoinStore<CoinType>>(recipients[i]).frozen;

        // guid properties
        aborts_if exists i in 0..len(recipients):
            account::exists_at(recipients[i]) && !exists<coin::CoinStore<CoinType>>(recipients[i]) && global<account::Account>(recipients[i]).guid_creation_num + 2 >= account::MAX_GUID_CREATION_NUM;
        aborts_if exists i in 0..len(recipients):
            account::exists_at(recipients[i]) && !exists<coin::CoinStore<CoinType>>(recipients[i]) && global<account::Account>(recipients[i]).guid_creation_num + 2 > MAX_U64;

        // register_coin properties
        aborts_if exists i in 0..len(recipients):
            !coin::spec_is_account_registered<CoinType>(recipients[i]) && !type_info::spec_is_struct<CoinType>();
    }

    spec deposit_coins<CoinType>(to: address, coins: Coin<CoinType>) {
        // TODO(fa_migration)
        pragma verify = true;
        pragma aborts_if_is_partial;
        // include CreateAccountTransferAbortsIf;
        // include GuidAbortsIf<CoinType>;
        // include RegistCoinAbortsIf<CoinType>;

        let if_exist_coin = exists<coin::CoinStore<CoinType>>(to);
        // aborts_if if_exist_coin && global<coin::CoinStore<CoinType>>(to).frozen;
        /// [high-level-spec-6]
        ensures exists<supra_framework::account::Account>(to);
        ensures exists<supra_framework::coin::CoinStore<CoinType>>(to);

        let coin_store_to = global<coin::CoinStore<CoinType>>(to).coin.value;
        let post post_coin_store_to = global<coin::CoinStore<CoinType>>(to).coin.value;
        ensures if_exist_coin ==> post_coin_store_to == coin_store_to + coins.value;
    }

    spec transfer_coins<CoinType>(from: &signer, to: address, amount: u64) {
        // TODO(fa_migration)
        pragma verify = true;
        pragma aborts_if_is_partial;
        let account_addr_source = signer::address_of(from);

        //The 'from' addr is implictly not equal to 'to' addr
        requires account_addr_source != to;

        // include CreateAccountTransferAbortsIf;
        // include WithdrawAbortsIf<CoinType>;
        // include GuidAbortsIf<CoinType>;
        // include RegistCoinAbortsIf<CoinType>;
        // include TransferEnsures<CoinType>;

        // aborts_if exists<coin::CoinStore<CoinType>>(to) && global<coin::CoinStore<CoinType>>(to).frozen;
        ensures exists<supra_framework::account::Account>(to);
        ensures exists<supra_framework::coin::CoinStore<CoinType>>(to);
    }

    spec register_supra(account_signer: &signer) {
        // TODO: temporary mockup.
        pragma verify = false;
    }

    spec fungible_transfer_only(source: &signer, to: address, amount: u64) {
        // TODO: temporary mockup.
        pragma verify = false;
    }

    spec is_fungible_balance_at_least(account: address, amount: u64): bool {
        // TODO: temporary mockup.
        pragma verify = false;
    }

    spec burn_from_fungible_store(
        ref: &BurnRef,
        account: address,
        amount: u64,
    ) {
        // TODO: temporary mockup.
        pragma verify = false;
    }

    spec schema CreateAccountTransferAbortsIf {
        to: address;
        aborts_if !account::exists_at(to) && length_judgment(to);
        aborts_if !account::exists_at(to) && (to == @vm_reserved || to == @supra_framework || to == @aptos_token);
    }

    spec schema WithdrawAbortsIf<CoinType> {
        from: &signer;
        amount: u64;
        let account_addr_source = signer::address_of(from);
        let coin_store_source = global<coin::CoinStore<CoinType>>(account_addr_source);
        let balance_source = coin_store_source.coin.value;
        aborts_if !exists<coin::CoinStore<CoinType>>(account_addr_source);
        aborts_if coin_store_source.frozen;
        aborts_if balance_source < amount;
    }

    spec schema GuidAbortsIf<CoinType> {
        to: address;
        let acc = global<account::Account>(to);
        aborts_if account::exists_at(to) && !exists<coin::CoinStore<CoinType>>(to) && acc.guid_creation_num + 2 >= account::MAX_GUID_CREATION_NUM;
        aborts_if account::exists_at(to) && !exists<coin::CoinStore<CoinType>>(to) && acc.guid_creation_num + 2 > MAX_U64;
    }

    spec schema RegistCoinAbortsIf<CoinType> {
        use aptos_std::type_info;
        to: address;
        aborts_if !coin::spec_is_account_registered<CoinType>(to) && !type_info::spec_is_struct<CoinType>();
        aborts_if exists<supra_framework::account::Account>(to);
        aborts_if type_info::type_of<CoinType>() != type_info::type_of<SupraCoin>();
    }

    spec schema TransferEnsures<CoinType> {
        to: address;
        account_addr_source: address;
        amount: u64;

        let if_exist_account = exists<account::Account>(to);
        let if_exist_coin = exists<coin::CoinStore<CoinType>>(to);
        let coin_store_to = global<coin::CoinStore<CoinType>>(to);
        let coin_store_source = global<coin::CoinStore<CoinType>>(account_addr_source);
        let post p_coin_store_to = global<coin::CoinStore<CoinType>>(to);
        let post p_coin_store_source = global<coin::CoinStore<CoinType>>(account_addr_source);
        ensures coin_store_source.coin.value - amount == p_coin_store_source.coin.value;
        ensures if_exist_account && if_exist_coin ==> coin_store_to.coin.value + amount == p_coin_store_to.coin.value;
    }

    spec assert_account_is_registered_for_apt {
        pragma aborts_if_is_strict = false;
    }
}
