spec aptos_framework::aptos_account {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Check if the bytes of the auth_key is 32.
    /// The Account does not exist under the auth_key before creating the account.
    /// Limit the address of auth_key is not @vm_reserved / @aptos_framework / @aptos_toke.
    spec create_account(auth_key: address) {
        include CreateAccount;

        ensures exists<account::Account>(auth_key);
        ensures exists<coin::CoinStore<AptosCoin>>(auth_key);
    }

    spec schema CreateAccount {
        auth_key: address;

        aborts_if exists<account::Account>(auth_key);
        aborts_if length_judgment(auth_key);
        aborts_if auth_key == @vm_reserved || auth_key == @aptos_framework || auth_key == @aptos_token;
        aborts_if exists<coin::CoinStore<AptosCoin>>(auth_key);
    }

    spec fun length_judgment(auth_key: address): bool {
        use std::bcs;

        let authentication_key = bcs::to_bytes(auth_key);
        len(authentication_key) != 32
    }

    spec transfer(source: &signer, to: address, amount: u64) {
        pragma verify = false;
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
}
