spec aptos_framework::aptos_account {
    spec module {
        pragma aborts_if_is_strict;
    }

    /// Check if the bytes of the auth_key is 32.
    /// The Account does not exist under the auth_key before creating the account.
    /// Limit the address of auth_key is not @vm_reserved / @aptos_framework / @aptos_toke.
    spec create_account(auth_key: address) {
        include CreateAccountAbortsIf;
        ensures exists<account::Account>(auth_key);
        ensures exists<coin::CoinStore<AptosCoin>>(auth_key);
    }
    spec schema CreateAccountAbortsIf {
        auth_key: address;
        aborts_if exists<account::Account>(auth_key);
        aborts_if length_judgment(auth_key);
        aborts_if auth_key == @vm_reserved || auth_key == @aptos_framework || auth_key == @aptos_token;
    }

    spec fun length_judgment(auth_key: address): bool {
        use std::bcs;

        let authentication_key = bcs::to_bytes(auth_key);
        len(authentication_key) != 32
    }

    spec transfer(source: &signer, to: address, amount: u64) {
        let account_addr_source = signer::address_of(source);
        let coin_store_to = global<coin::CoinStore<AptosCoin>>(to);

        // The 'from' addr is implictly not equal to 'to' addr
        requires account_addr_source != to;

        include CreateAccountTransferAbortsIf;
        include GuidAbortsIf<AptosCoin>;
        include WithdrawAbortsIf<AptosCoin>{from: source};

        aborts_if exists<coin::CoinStore<AptosCoin>>(to) && global<coin::CoinStore<AptosCoin>>(to).frozen;
        ensures exists<aptos_framework::account::Account>(to);
        ensures exists<coin::CoinStore<AptosCoin>>(to);
    }

    spec assert_account_exists(addr: address) {
        aborts_if !account::exists_at(addr);
    }

    /// Check if the address existed.
    /// Check if the AptosCoin under the address existed.
    spec assert_account_is_registered_for_apt(addr: address) {
        aborts_if !account::exists_at(addr);
        aborts_if !coin::is_account_registered<AptosCoin>(addr);
    }

    spec set_allow_direct_coin_transfers(account: &signer, allow: bool) {
        let addr = signer::address_of(account);
        include !exists<DirectTransferConfig>(addr) ==> account::NewEventHandleAbortsIf;
    }

    spec batch_transfer(source: &signer, recipients: vector<address>, amounts: vector<u64>) {
        //TODO: Can't verify the loop invariant in enumerate
        pragma verify = false;
        let account_addr_source = signer::address_of(source);
        let coin_store_source = global<coin::CoinStore<AptosCoin>>(account_addr_source);
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
                !account::exists_at(recipients[i]) && (recipients[i] == @vm_reserved || recipients[i] == @aptos_framework || recipients[i] == @aptos_token);
        ensures forall i in 0..len(recipients):
                (!account::exists_at(recipients[i]) ==> !length_judgment(recipients[i])) &&
                    (!account::exists_at(recipients[i]) ==> (recipients[i] != @vm_reserved && recipients[i] != @aptos_framework && recipients[i] != @aptos_token));

        // coin::withdraw properties
        aborts_if exists i in 0..len(recipients):
            !exists<coin::CoinStore<AptosCoin>>(account_addr_source);
        aborts_if exists i in 0..len(recipients):
            coin_store_source.frozen;
        aborts_if exists i in 0..len(recipients):
            global<coin::CoinStore<AptosCoin>>(account_addr_source).coin.value < amounts[i];

        // deposit properties
        aborts_if exists i in 0..len(recipients):
            exists<coin::CoinStore<AptosCoin>>(recipients[i]) && global<coin::CoinStore<AptosCoin>>(recipients[i]).frozen;

        // guid properties
        aborts_if exists i in 0..len(recipients):
            account::exists_at(recipients[i]) && !exists<coin::CoinStore<AptosCoin>>(recipients[i]) && global<account::Account>(recipients[i]).guid_creation_num + 2 >= account::MAX_GUID_CREATION_NUM;
        aborts_if exists i in 0..len(recipients):
            account::exists_at(recipients[i]) && !exists<coin::CoinStore<AptosCoin>>(recipients[i]) && global<account::Account>(recipients[i]).guid_creation_num + 2 > MAX_U64;
    }

    spec can_receive_direct_coin_transfers(account: address): bool {
        aborts_if false;
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

        aborts_if len(recipients) != len(amounts);

        //create account properties
        aborts_if exists i in 0..len(recipients):
                !account::exists_at(recipients[i]) && length_judgment(recipients[i]);
        aborts_if exists i in 0..len(recipients):
                !account::exists_at(recipients[i]) && (recipients[i] == @vm_reserved || recipients[i] == @aptos_framework || recipients[i] == @aptos_token);
        ensures forall i in 0..len(recipients):
                (!account::exists_at(recipients[i]) ==> !length_judgment(recipients[i])) &&
                    (!account::exists_at(recipients[i]) ==> (recipients[i] != @vm_reserved && recipients[i] != @aptos_framework && recipients[i] != @aptos_token));

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
            !coin::is_account_registered<CoinType>(recipients[i]) && !type_info::spec_is_struct<CoinType>();
        aborts_if exists i in 0..len(recipients):
            !coin::is_account_registered<CoinType>(recipients[i]) && !can_receive_direct_coin_transfers(recipients[i]);

    }

    spec deposit_coins<CoinType>(to: address, coins: Coin<CoinType>) {
        include CreateAccountTransferAbortsIf;
        include GuidAbortsIf<CoinType>;
        include RegistCoinAbortsIf<CoinType>;

        aborts_if exists<coin::CoinStore<CoinType>>(to) && global<coin::CoinStore<CoinType>>(to).frozen;
        ensures exists<aptos_framework::account::Account>(to);
        ensures exists<aptos_framework::coin::CoinStore<CoinType>>(to);
    }

    spec transfer_coins<CoinType>(from: &signer, to: address, amount: u64) {
        let account_addr_source = signer::address_of(from);
        let coin_store_to = global<coin::CoinStore<CoinType>>(to);

        //The 'from' addr is implictly not equal to 'to' addr
        requires account_addr_source != to;

        include CreateAccountTransferAbortsIf;
        include WithdrawAbortsIf<CoinType>;
        include GuidAbortsIf<CoinType>;
        include RegistCoinAbortsIf<CoinType>;

        aborts_if exists<coin::CoinStore<CoinType>>(to) && global<coin::CoinStore<CoinType>>(to).frozen;
        ensures exists<aptos_framework::account::Account>(to);
        ensures exists<aptos_framework::coin::CoinStore<CoinType>>(to);
    }

    spec schema CreateAccountTransferAbortsIf {
        to: address;
        aborts_if !account::exists_at(to) && length_judgment(to);
        aborts_if !account::exists_at(to) && (to == @vm_reserved || to == @aptos_framework || to == @aptos_token);
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
        aborts_if !coin::is_account_registered<CoinType>(to) && !type_info::spec_is_struct<CoinType>();
        aborts_if exists<aptos_framework::account::Account>(to)
            && !coin::is_account_registered<CoinType>(to) && !can_receive_direct_coin_transfers(to);
        aborts_if type_info::type_of<CoinType>() != type_info::type_of<AptosCoin>()
            && !coin::is_account_registered<CoinType>(to) && !can_receive_direct_coin_transfers(to);
    }
}
