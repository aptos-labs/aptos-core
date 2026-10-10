/// Mintable fungible asset for the airdrop benchmark.
///
/// The asset is a real `aptos_framework::fungible_asset` metadata object with
/// primary stores enabled, so balances live in resource groups and store
/// addresses are derived rather than stored.
module bench::ad_assets {
    use std::option;
    use std::signer;
    use std::string;
    use aptos_framework::fungible_asset::{
        Self,
        BurnRef,
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
    /// the symbol as the name as well. The framework hands the refs out once,
    /// at creation, so all three are kept for whatever the mix needs later.
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

    /// Calling this again is a no-op, so a re-run of the setup does not abort.
    public entry fun create_asset_entry(
        admin: &signer, symbol: vector<u8>, decimals: u8
    ) {
        let addr = asset_address(signer::address_of(admin), symbol);
        if (object::object_exists<Metadata>(addr)) { return };
        create_asset(admin, symbol, decimals);
    }

    /// Address of the asset `admin` created for `symbol`.
    public fun asset_address(admin: address, symbol: vector<u8>): address {
        object::create_object_address(&admin, symbol)
    }

    public fun asset(admin: address, symbol: vector<u8>): Object<Metadata> {
        object::address_to_object<Metadata>(asset_address(admin, symbol))
    }

    /// Mint into the recipient's primary store, creating it if needed.
    public fun mint(
        _admin: &signer, asset: Object<Metadata>, to: address, amount: u64
    ) acquires Refs {
        faucet(asset, to, amount);
    }

    /// Unpermissioned mint. A harness drives thousands of independent accounts
    /// that each need funding, and routing every one through the admin would
    /// serialize them on the admin's sequence number.
    public fun faucet(asset: Object<Metadata>, to: address, amount: u64) acquires Refs {
        let refs = borrow_global<Refs>(object::object_address(&asset));
        primary_fungible_store::mint(&refs.mint_ref, to, amount);
    }

    public entry fun mint_entry(
        admin: &signer, asset: Object<Metadata>, to: address, amount: u64
    ) acquires Refs {
        mint(admin, asset, to, amount);
    }

    /// Move `amount` between primary stores, creating either side as needed.
    public fun transfer_primary(
        from: &signer, asset: Object<Metadata>, to: address, amount: u64
    ) {
        primary_fungible_store::transfer(from, asset, to, amount);
    }

    /// Burn from a primary store without its owner's signer.
    public fun burn_from(
        asset: Object<Metadata>, owner: address, amount: u64
    ) acquires Refs {
        let refs = borrow_global<Refs>(object::object_address(&asset));
        primary_fungible_store::burn(&refs.burn_ref, owner, amount);
    }

    public fun primary_balance(owner: address, asset: Object<Metadata>): u64 {
        primary_fungible_store::balance(owner, asset)
    }
}
