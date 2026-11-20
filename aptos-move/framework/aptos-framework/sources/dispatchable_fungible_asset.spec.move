spec aptos_framework::dispatchable_fungible_asset {
    spec module {
        pragma verify = true;
    }

    /// The asset of the store at `store_addr` has a withdraw hook (no APT short-circuit).
    spec fun spec_has_withdraw_hook(store_addr: address): bool {
        !fungible_asset::spec_store_no_withdraw_hook(store_addr)
    }

    /// The asset of the store at `store_addr` has a deposit hook (no APT short-circuit).
    spec fun spec_has_deposit_hook(store_addr: address): bool {
        !fungible_asset::spec_store_no_deposit_hook(store_addr)
    }

    /// The metadata of the store at `store_addr` holds a `TransferRefStore`.
    spec fun spec_has_transfer_ref_store(store_addr: address): bool {
        exists<TransferRefStore>(global<fungible_asset::FungibleStore>(store_addr).metadata.inner)
    }

    /// The store balance as `fungible_asset::balance` reads it; a missing store counts as an empty one.
    spec fun spec_balance_or_zero(store_addr: address): u64 {
        if (exists<fungible_asset::FungibleStore>(store_addr)) { fungible_asset::spec_balance(store_addr) } else { 0 }
    }

    // The dispatch natives forward their arguments to the registered hook, which is user code:
    // its result and aborts are unknown and its writes cannot be bounded, so nothing is stated.
    // The native's own `ENOT_LOADED` abort cannot occur, as the module is loaded right before.

    spec dispatchable_withdraw {
        pragma opaque;
    }

    spec dispatchable_deposit {
        pragma opaque;
    }

    spec dispatchable_derived_balance {
        pragma opaque;
    }

    spec dispatchable_derived_supply {
        pragma opaque;
    }

    // Opaque, mirroring the natives they replace.

    spec dispatch_withdraw_hook {
        pragma opaque;
    }

    spec dispatch_deposit_hook {
        pragma opaque;
    }

    spec dispatch_derived_balance_hook {
        pragma opaque;
    }

    spec dispatch_derived_supply_hook {
        pragma opaque;
    }

    /// Without a withdraw hook, withdraws as `fungible_asset::withdraw`.
    spec withdraw<T: key>(owner: &signer, store: Object<T>, amount: u64): FungibleAsset {
        use aptos_framework::fungible_asset::FungibleStore;
        // Partial: the aborts of a withdraw hook are not known.
        pragma aborts_if_is_partial;
        let store_addr = store.inner;
        let owner_address = std::signer::address_of(owner);
        let hook = spec_has_withdraw_hook(store_addr);
        include WithdrawSanityAbortsIf;
        aborts_if hook && !spec_has_transfer_ref_store(store_addr);
        aborts_if !hook && fungible_asset::spec_balance(store_addr) < amount;
        ensures !hook ==> fungible_asset::spec_balance(store_addr) == old(fungible_asset::spec_balance(store_addr)) - amount;
        ensures !hook ==> result == FungibleAsset { metadata: old(global<FungibleStore>(store_addr).metadata), amount };
        ensures !hook ==> global<FungibleStore>(store_addr).metadata == old(global<FungibleStore>(store_addr).metadata);
        ensures !hook ==> global<FungibleStore>(store_addr).frozen == old(global<FungibleStore>(store_addr).frozen);
        ensures !hook ==> fungible_asset::spec_supply(old(global<FungibleStore>(store_addr).metadata.inner))
            == old(fungible_asset::spec_supply(global<FungibleStore>(store_addr).metadata.inner));
    }

    /// The checks `fungible_asset::withdraw_sanity_check` makes before any hook runs.
    spec schema WithdrawSanityAbortsIf {
        use aptos_framework::fungible_asset::FungibleStore;
        store_addr: address;
        owner_address: address;
        aborts_if object::spec_owns_aborts(store_addr, owner_address);
        aborts_if !object::spec_owns(store_addr, owner_address);
        aborts_if !exists<FungibleStore>(store_addr);
        aborts_if global<FungibleStore>(store_addr).frozen;
    }

    /// The checks `fungible_asset::deposit_sanity_check` makes before any hook runs.
    spec schema DepositSanityAbortsIf {
        use aptos_framework::fungible_asset::FungibleStore;
        store_addr: address;
        aborts_if !exists<FungibleStore>(store_addr);
        aborts_if global<FungibleStore>(store_addr).frozen;
    }

    /// Without a deposit hook, deposits as `fungible_asset::deposit`.
    spec deposit<T: key>(store: Object<T>, fa: FungibleAsset) {
        use aptos_framework::fungible_asset::FungibleStore;
        // Partial: the aborts of a deposit hook are not known.
        pragma aborts_if_is_partial;
        let store_addr = store.inner;
        let hook = spec_has_deposit_hook(store_addr);
        include DepositSanityAbortsIf;
        aborts_if hook && !spec_has_transfer_ref_store(store_addr);
        include !hook ==> fungible_asset::UncheckedDepositAbortsIf;
        ensures !hook ==> fungible_asset::spec_balance(store_addr) == old(fungible_asset::spec_balance(store_addr)) + fa.amount;
        ensures !hook ==> global<FungibleStore>(store_addr).metadata == old(global<FungibleStore>(store_addr).metadata);
        ensures !hook ==> global<FungibleStore>(store_addr).frozen == old(global<FungibleStore>(store_addr).frozen);
        ensures !hook ==> fungible_asset::spec_supply(fa.metadata.inner) == old(fungible_asset::spec_supply(fa.metadata.inner));
    }

    /// Without hooks, moves `amount` from `from` to `to`.
    spec transfer<T: key>(sender: &signer, from: Object<T>, to: Object<T>, amount: u64) {
        // Partial: the aborts of the hooks are not known.
        pragma aborts_if_is_partial;
        let from_addr = from.inner;
        let to_addr = to.inner;
        let hooks = spec_has_withdraw_hook(from_addr) || spec_has_deposit_hook(to_addr);
        include TransferAbortsIf;
        ensures !hooks && from_addr != to_addr ==>
            fungible_asset::spec_balance(from_addr) == old(fungible_asset::spec_balance(from_addr)) - amount
            && fungible_asset::spec_balance(to_addr) == old(fungible_asset::spec_balance(to_addr)) + amount;
        ensures !hooks && from_addr == to_addr ==>
            fungible_asset::spec_balance(from_addr) == old(fungible_asset::spec_balance(from_addr));
    }

    /// Aborts of a transfer that occur before or without a hook.
    spec schema TransferAbortsIf {
        use aptos_framework::fungible_asset::FungibleStore;
        sender: signer;
        from_addr: address;
        to_addr: address;
        amount: u64;
        let withdraw_hook = spec_has_withdraw_hook(from_addr);
        include WithdrawSanityAbortsIf { store_addr: from_addr, owner_address: std::signer::address_of(sender) };
        aborts_if withdraw_hook && !spec_has_transfer_ref_store(from_addr);
        aborts_if !withdraw_hook && fungible_asset::spec_balance(from_addr) < amount;
        // Without a withdraw hook the deposit sees `to` as before, unless it is `from`.
        include !withdraw_hook && from_addr != to_addr ==> DepositSanityAbortsIf { store_addr: to_addr };
        aborts_if !withdraw_hook && from_addr != to_addr && spec_has_deposit_hook(to_addr)
            && !spec_has_transfer_ref_store(to_addr);
        include !withdraw_hook && from_addr != to_addr && !spec_has_deposit_hook(to_addr)
            ==> fungible_asset::UncheckedDepositAbortsIf {
                store_addr: to_addr,
                fa: FungibleAsset { metadata: global<FungibleStore>(from_addr).metadata, amount }
            };
    }

    /// The balance of `to` grows by at least `expected`, whatever the hooks do.
    spec transfer_assert_minimum_deposit<T: key>(
        sender: &signer,
        from: Object<T>,
        to: Object<T>,
        amount: u64,
        expected: u64
    ) {
        use aptos_framework::fungible_asset::FungibleStore;
        // Partial: the aborts of the hooks are not known.
        pragma aborts_if_is_partial;
        let from_addr = from.inner;
        let to_addr = to.inner;
        let hooks = spec_has_withdraw_hook(from_addr) || spec_has_deposit_hook(to_addr);
        aborts_if exists<FungibleStore>(to_addr)
            && fungible_asset::spec_has_balance_dispatch_function(global<FungibleStore>(to_addr).metadata);
        include TransferAbortsIf;
        aborts_if !hooks && from_addr != to_addr && amount < expected;
        aborts_if !hooks && from_addr == to_addr && expected > 0;
        ensures spec_balance_or_zero(to_addr) >= old(spec_balance_or_zero(to_addr)) + expected;
    }

    /// Without a derived-balance hook, the store balance.
    spec derived_balance<T: key>(store: Object<T>): u64 {
        // Partial: the aborts of a derived-balance hook are not known.
        pragma aborts_if_is_partial;
        let store_addr = store.inner;
        aborts_if !exists<fungible_asset::FungibleStore>(store_addr);
        ensures !fungible_asset::spec_store_has_balance_hook(store_addr) ==>
            result == fungible_asset::spec_balance(store_addr);
    }

    /// Without a derived-balance hook, a snapshot of the store balance.
    spec derived_balance_snapshot<T: key>(store: Object<T>): AggregatorSnapshot<u64> {
        // Partial: the aborts of a derived-balance hook are not known.
        pragma aborts_if_is_partial;
        let store_addr = store.inner;
        aborts_if !exists<fungible_asset::FungibleStore>(store_addr);
        ensures !fungible_asset::spec_store_has_balance_hook(store_addr) ==>
            aggregator_v2::spec_read_snapshot(result) == fungible_asset::spec_balance(store_addr);
    }

    /// Without a derived-balance hook, whether the store balance is at least `amount`.
    spec is_derived_balance_at_least<T: key>(store: Object<T>, amount: u64): bool {
        // Partial: the aborts of a derived-balance hook are not known.
        pragma aborts_if_is_partial;
        let store_addr = store.inner;
        aborts_if !exists<fungible_asset::FungibleStore>(store_addr);
        ensures !fungible_asset::spec_store_has_balance_hook(store_addr) ==>
            result == (fungible_asset::spec_balance(store_addr) >= amount);
    }

    /// Without a derived-supply hook, the supply.
    spec derived_supply<T: key>(metadata: Object<T>): Option<u128> {
        use aptos_framework::fungible_asset::DeriveSupply;
        // Partial: the aborts of a derived-supply hook are not known.
        pragma aborts_if_is_partial;
        let addr = metadata.inner;
        let hook = exists<DeriveSupply>(addr) && std::option::is_some(global<DeriveSupply>(addr).dispatch_function);
        // A `DeriveSupply` without a function still makes `fungible_asset::supply` abort.
        aborts_if !hook && addr != @aptos_fungible_asset && exists<DeriveSupply>(addr);
        ensures !hook ==> result == if (exists<fungible_asset::ConcurrentSupply>(addr) || exists<fungible_asset::Supply>(addr)) {
            std::option::spec_some(fungible_asset::spec_supply(addr))
        } else {
            std::option::spec_none()
        };
    }
}
