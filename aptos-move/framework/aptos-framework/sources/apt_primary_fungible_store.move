
module aptos_framework::apt_primary_fungible_store {
    use aptos_framework::fungible_asset::{Self, Metadata, BurnRef};
    use aptos_framework::primary_fungible_store;
    use aptos_framework::object;

    use std::signer;

    friend aptos_framework::aptos_account;
    friend aptos_framework::transaction_fee;
    friend aptos_framework::transaction_validation;

    inline fun store_address(account: address): address {
        object::create_user_derived_object_address(account, @aptos_fungible_asset)
    }

    public(friend) fun is_balance_at_least(account: address, amount: u64): bool {
        let store_addr = store_address(account);
        fungible_asset::is_address_balance_at_least(store_addr, amount)
    }

    public(friend) fun burn_from(
        ref: &BurnRef,
        account: address,
        amount: u64,
    ) {
        // Skip burning if amount is zero. This shouldn't error out as it's called as part of transaction fee burning.
        if (amount != 0) {
            let store_addr = store_address(account);
            fungible_asset::address_burn_from(ref, store_addr, amount);
        };
    }

    public(friend) inline fun ensure_primary_store_exists(owner: address): address {
        let store_addr = store_address(owner);
        if (fungible_asset::store_exists(store_addr)) {
            store_addr
        } else {
            object::object_address(&primary_fungible_store::create_primary_store(owner, object::address_to_object<Metadata>(@aptos_fungible_asset)))
        }
    }

    public entry fun transfer(
        sender: &signer,
        recipient: address,
        amount: u64,
    ) {
        let sender_store = ensure_primary_store_exists(signer::address_of(sender));
        let recipient_store = ensure_primary_store_exists(recipient);

        // use internal APIs, as they skip:
        // - owner, frozen and dispatchable checks
        // as APT cannot be frozen or have dispatch, and PFS cannot be transfered
        // (PFS could potentially be burned. regular transfer would permanently unburn the store.
        // Ignoring the check here has the equivalent of unburning, transfers, and then burning again)
        fungible_asset::deposit_internal(recipient_store, fungible_asset::withdraw_internal(sender_store, amount));
    }

}
