/// Mintable fungible assets for the order book benchmark.
///
/// The assets are real `aptos_framework::fungible_asset` metadata objects with
/// primary stores enabled, so balances live in resource groups and store
/// addresses are derived rather than stored.
module bench::clob_mock_fa {
    use std::option;
    use std::string;
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
    /// the symbol as the name as well. The market needs `transfer_ref` to
    /// settle a fill against a maker whose signer it does not have, so all
    /// three refs are kept.
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

    /// Mint into the recipient's primary store, creating it if needed.
    public fun mint(
        _admin: &signer, asset: Object<Metadata>, to: address, amount: u64
    ) acquires Refs {
        let refs = borrow_global<Refs>(object::object_address(&asset));
        primary_fungible_store::mint(&refs.mint_ref, to, amount);
    }

    public entry fun mint_entry(
        admin: &signer, asset: Object<Metadata>, to: address, amount: u64
    ) acquires Refs {
        mint(admin, asset, to, amount);
    }

    public fun burn(asset: Object<Metadata>, fa: FungibleAsset) acquires Refs {
        let refs = borrow_global<Refs>(object::object_address(&asset));
        fungible_asset::burn(&refs.burn_ref, fa);
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

    public fun balance(store: Object<FungibleStore>): u64 {
        fungible_asset::balance(store)
    }

    public fun primary_balance(
        owner: address, asset: Object<Metadata>
    ): u64 {
        primary_fungible_store::balance(owner, asset)
    }
}
