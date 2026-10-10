/// Mintable fungible assets for the CDP liquidation benchmark.
///
/// The assets are real `aptos_framework::fungible_asset` metadata objects with
/// primary stores enabled, so balances live in resource groups and store
/// addresses are derived rather than stored.
module bench::cdp_assets {
    use std::option;
    use std::string;
    use aptos_framework::fungible_asset::{Self, BurnRef, Metadata, MintRef};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;

    /// Kept under the metadata object of the asset they control.
    struct Refs has key {
        mint_ref: MintRef,
        burn_ref: BurnRef,
    }

    /// Create an asset under `admin` at the named object for `symbol`, using
    /// the symbol as the name as well. Liquidation burns stable out of the
    /// stability pool's store, so the burn ref is kept alongside the mint one.
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

    /// Unpermissioned mint. A harness drives thousands of independent accounts
    /// that each need funding, and routing every one through the admin would
    /// serialize them on the admin's sequence number.
    public fun faucet(
        asset: Object<Metadata>, to: address, amount: u64
    ) acquires Refs {
        let refs = borrow_global<Refs>(object::object_address(&asset));
        primary_fungible_store::mint(&refs.mint_ref, to, amount);
    }

    /// Burn out of `owner`'s primary store. Callers clamp `amount` to the
    /// balance first, since the framework aborts on a short burn.
    public fun burn_from(
        asset: Object<Metadata>, owner: address, amount: u64
    ) acquires Refs {
        let refs = borrow_global<Refs>(object::object_address(&asset));
        primary_fungible_store::burn(&refs.burn_ref, owner, amount);
    }

    /// Zero when `owner` has never held the asset, rather than an abort.
    public fun primary_balance(owner: address, asset: Object<Metadata>): u64 {
        primary_fungible_store::balance(owner, asset)
    }
}
