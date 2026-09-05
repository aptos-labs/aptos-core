// Ported from https://github.com/aave/aptos-aave-v3, module
// `aave_mock_underlyings::mock_underlying_token_factory`.
// Copyright (c) Aave DAO and contributors.
// SPDX-License-Identifier: Apache-2.0

/// Underlyings for the benchmark, as real fungible assets.
///
/// Metadata objects are named objects: `fungible_asset::add_fungibility`
/// rejects anything that can generate a delete ref, and a named object cannot.
/// Every transfer therefore goes through derived primary-store addresses and
/// the object resource group, which is the point of using real assets here
/// instead of an internal ledger.
module bench::aave_mock_fa {
    use std::option;
    use std::signer;
    use std::string;
    use aptos_framework::fungible_asset::{Self, BurnRef, Metadata, MintRef, TransferRef};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;

    const ENOT_ADMIN: u64 = 30;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct AssetRefs has key {
        mint_ref: MintRef,
        burn_ref: BurnRef,
        transfer_ref: TransferRef
    }

    public fun create_asset(
        admin: &signer, symbol: vector<u8>, decimals: u8
    ): Object<Metadata> {
        assert!(signer::address_of(admin) == @bench, ENOT_ADMIN);
        let ctor = object::create_named_object(admin, symbol);
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            &ctor,
            option::none(),
            string::utf8(symbol),
            string::utf8(symbol),
            decimals,
            string::utf8(b""),
            string::utf8(b"")
        );
        let metadata_signer = object::generate_signer(&ctor);
        move_to(
            &metadata_signer,
            AssetRefs {
                mint_ref: fungible_asset::generate_mint_ref(&ctor),
                burn_ref: fungible_asset::generate_burn_ref(&ctor),
                transfer_ref: fungible_asset::generate_transfer_ref(&ctor)
            }
        );
        object::object_from_constructor_ref<Metadata>(&ctor)
    }

    public entry fun create_asset_entry(
        admin: &signer, symbol: vector<u8>, decimals: u8
    ) {
        create_asset(admin, symbol, decimals);
    }

    public fun asset_address(symbol: vector<u8>): address {
        object::create_object_address(&@bench, symbol)
    }

    public fun metadata(asset: address): Object<Metadata> {
        object::address_to_object<Metadata>(asset)
    }

    public entry fun mint(
        admin: &signer, asset: address, to: address, amount: u64
    ) acquires AssetRefs {
        assert!(signer::address_of(admin) == @bench, ENOT_ADMIN);
        let refs = borrow_global<AssetRefs>(asset);
        primary_fungible_store::mint(&refs.mint_ref, to, amount);
    }

    public fun decimals(asset: address): u8 {
        fungible_asset::decimals(metadata(asset))
    }

    public fun balance_of(owner: address, asset: address): u64 {
        primary_fungible_store::balance(owner, metadata(asset))
    }
}
