/// Mintable fungible assets for the DEX aggregator benchmark, plus the pool
/// vaults the four backends keep their reserves in.
///
/// The assets are real `aptos_framework::fungible_asset` metadata objects with
/// primary stores enabled, so balances live in resource groups and store
/// addresses are derived rather than stored.
module bench::dexr_assets {
    use std::bcs;
    use std::option;
    use std::signer;
    use std::string;
    use std::vector;
    use aptos_framework::fungible_asset::{
        Self,
        BurnRef,
        FungibleAsset,
        FungibleStore,
        Metadata,
        MintRef,
        TransferRef,
    };
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;

    /// Kept under the metadata object of the asset they control.
    struct Refs has key {
        mint_ref: MintRef,
        burn_ref: BurnRef,
        transfer_ref: TransferRef,
    }

    /// Create an asset under `admin` at the named object for `symbol`, using
    /// the symbol as the name as well. A pool settles a fill out of a vault
    /// whose signer nobody holds, so `transfer_ref` is kept alongside the
    /// other two.
    public fun create_asset(
        admin: &signer, symbol: vector<u8>, decimals: u8
    ): Object<Metadata> {
        let constructor_ref = object::create_named_object(admin, symbol);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            &constructor_ref,
            option::none(),
            string::utf8(symbol),
            string::utf8(symbol),
            decimals,
            string::utf8(b""),
            string::utf8(b""),
        );
        move_to(
            &object::generate_signer(&constructor_ref),
            Refs {
                mint_ref: fungible_asset::generate_mint_ref(&constructor_ref),
                burn_ref: fungible_asset::generate_burn_ref(&constructor_ref),
                transfer_ref: fungible_asset::generate_transfer_ref(
                    &constructor_ref),
            },
        );
        object::object_from_constructor_ref<Metadata>(&constructor_ref)
    }

    public entry fun create_asset_entry(
        admin: &signer, symbol: vector<u8>, decimals: u8
    ) {
        create_asset(admin, symbol, decimals);
    }

    /// Address of the asset `admin` created for `symbol`.
    public fun asset_address(admin: address, symbol: vector<u8>): address {
        object::create_object_address(&admin, symbol)
    }

    public fun asset(admin: address, symbol: vector<u8>): Object<Metadata> {
        object::address_to_object<Metadata>(asset_address(admin, symbol))
    }

    public fun metadata_at(asset: address): Object<Metadata> {
        object::address_to_object<Metadata>(asset)
    }

    /// Unpermissioned mint. A harness drives thousands of independent accounts
    /// that each need funding, and routing every one through the admin would
    /// serialize them on the admin's sequence number.
    public fun faucet(asset: Object<Metadata>, to: address, amount: u64) acquires Refs {
        if (amount == 0) {
            return
        };
        let refs = borrow_global<Refs>(object::object_address(&asset));
        primary_fungible_store::mint(&refs.mint_ref, to, amount);
    }

    /// Top `owner` up to `amount` of `asset`. Every hop calls this before it
    /// withdraws, which is what lets a route run forever without checking
    /// whether the account still holds the leg it is about to trade.
    public fun ensure_primary(
        asset: Object<Metadata>, owner: address, amount: u64
    ) acquires Refs {
        let held = primary_fungible_store::balance(owner, asset);
        if (held < amount) {
            faucet(asset, owner, amount - held);
        };
    }

    /// Withdraw from any store of `asset`.
    public fun withdraw(
        asset: Object<Metadata>, store: Object<FungibleStore>, amount: u64
    ): FungibleAsset acquires Refs {
        let refs = borrow_global<Refs>(object::object_address(&asset));
        fungible_asset::withdraw_with_ref(&refs.transfer_ref, store, amount)
    }

    public fun deposit(
        asset: Object<Metadata>,
        store: Object<FungibleStore>,
        fa: FungibleAsset,
    ) acquires Refs {
        let refs = borrow_global<Refs>(object::object_address(&asset));
        fungible_asset::deposit_with_ref(&refs.transfer_ref, store, fa);
    }

    /// The named-object seed a backend's vault sits at. Backends pass distinct
    /// prefixes and never reuse a pool id, so the object can never collide.
    public fun vault_seed(prefix: vector<u8>, pool_id: u64, side: u8): vector<u8> {
        let seed = prefix;
        vector::append(&mut seed, bcs::to_bytes(&pool_id));
        vector::push_back(&mut seed, side);
        seed
    }

    /// Create one side of a pool's vault and fund it with `amount`.
    public fun create_vault(
        admin: &signer,
        asset: address,
        prefix: vector<u8>,
        pool_id: u64,
        side: u8,
        amount: u64,
    ): Object<FungibleStore> acquires Refs {
        let metadata = metadata_at(asset);
        let constructor_ref =
            object::create_named_object(admin, vault_seed(prefix, pool_id, side));
        let store = fungible_asset::create_store(&constructor_ref, metadata);
        fund_vault(asset, store, amount);
        store
    }

    /// Mint `amount` straight into a vault. Reserves are topped up this way
    /// rather than out of anybody's balance, so a rebalance never depends on
    /// who signed it.
    public fun fund_vault(
        asset: address, store: Object<FungibleStore>, amount: u64
    ) acquires Refs {
        if (amount == 0) {
            return
        };
        let refs = borrow_global<Refs>(asset);
        let fa = fungible_asset::mint(&refs.mint_ref, amount);
        fungible_asset::deposit_with_ref(&refs.transfer_ref, store, fa);
    }

    public fun vault_balance(store: Object<FungibleStore>): u64 {
        fungible_asset::balance(store)
    }

    public fun primary_balance(owner: address, asset: Object<Metadata>): u64 {
        primary_fungible_store::balance(owner, asset)
    }

    /// Move one hop's legs: `in_amount` of `in_asset` from the trader into the
    /// pool's vault, then `out_amount` of `out_asset` back out of it. The
    /// output is clamped to what the vault actually holds, so a pool whose
    /// bookkeeping has drifted returns less rather than aborting the route.
    public fun settle(
        user: &signer,
        in_asset: address,
        in_store: Object<FungibleStore>,
        in_amount: u64,
        out_asset: address,
        out_store: Object<FungibleStore>,
        out_amount: u64,
    ) acquires Refs {
        let owner = signer::address_of(user);
        if (in_amount > 0) {
            let metadata = metadata_at(in_asset);
            ensure_primary(metadata, owner, in_amount);
            let fa = primary_fungible_store::withdraw(user, metadata, in_amount);
            deposit(metadata, in_store, fa);
        };
        let available = fungible_asset::balance(out_store);
        let out_amount = if (out_amount > available) { available } else { out_amount };
        if (out_amount > 0) {
            let fa = withdraw(metadata_at(out_asset), out_store, out_amount);
            primary_fungible_store::deposit(owner, fa);
        };
    }
}
