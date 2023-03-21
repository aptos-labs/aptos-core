/// This module provides an addtional abstraction on top of `FungibleSource` that manages the capabilities of mint, burn
/// and transfer for the creator in a simple way. It offers creators to destory any capabilities in an on-demand way too.
/// For more advanced goverance, please build your own module to manage capabilitys extending `FungibleSource`.
module aptos_framework::managed_fungible_asset {
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::String;
    use aptos_framework::object::{Self, Object, ConstructorRef};
    use aptos_framework::fungible_caps::{Self, MintCap, TransferCap, BurnCap};

    /// Mint capability exists or does not exist.
    const EMINT_CAP: u64 = 1;
    /// Transfer capability exists does not exist.
    const EFREEZE_CAP: u64 = 2;
    /// Burn capability exists or does not exist.
    const EBURN_CAP: u64 = 3;
    /// Not the owner.
    const ENOT_OWNER: u64 = 4;
    /// Caps existence errors.
    const EMANAGED_FUNGIBLE_ASSET_CAPS: u64 = 5;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Used to hold capabilities to control the minting, transfer and burning of fungible assets.
    struct ManagingCapabilities has key {
        mint: Option<MintCap>,
        transfer: Option<TransferCap>,
        burn: Option<BurnCap>,
    }

    /// Initialize capabilities of an asset object after initializing `FungibleSource`.
    public fun init_managing_capabilities(
        constructor_ref: &ConstructorRef,
        maximum_supply: u64,
        name: String,
        symbol: String,
        decimals: u8
    ) {
        let (mint_cap, transfer_cap, burn_cap) = fungible_caps::init_fungible_source_with_caps(
            constructor_ref,
            maximum_supply,
            name,
            symbol,
            decimals
        );
        let asset_object_signer = object::generate_signer(constructor_ref);
        move_to(
            &asset_object_signer,
            ManagingCapabilities {
                mint: option::some(mint_cap), transfer: option::some(
                    transfer_cap
                ), burn: option::some(burn_cap)
            }
        )
    }

    /// Mint fungible tokens as the owner of the base asset.
    public fun mint<T: key>(
        asset_owner: &signer,
        asset: &Object<T>,
        amount: u64,
        to: address
    ) acquires ManagingCapabilities {
        assert_owner(asset_owner, asset);
        let mint_cap = borrow_mint_from_caps(asset);
        fungible_caps::mint(mint_cap, amount, to);
    }

    /// Mint fungible tokens as the owner of the base asset.
    public fun transfer<T: key>(
        asset_owner: &signer,
        asset: &Object<T>,
        amount: u64,
        from: address,
        to: address,
    ) acquires ManagingCapabilities {
        assert_owner(asset_owner, asset);
        let transfer_cap = borrow_transfer_from_caps(asset);
        fungible_caps::transfer_with_cap(transfer_cap, amount, from, to);
    }

    /// Burn fungible tokens as the owner of the base asset.
    public fun burn<T: key>(
        asset_owner: &signer,
        asset: &Object<T>,
        amount: u64,
        from: address
    ) acquires ManagingCapabilities {
        assert_owner(asset_owner, asset);
        let burn_cap = borrow_burn_from_caps(asset);
        fungible_caps::burn(burn_cap, amount, from);
    }

    /// Transfer as an owner of fungible asset.
    public fun set_ungated_transfer<T: key>(
        asset_owner: &signer,
        asset: &Object<T>,
        account: address,
        allow: bool
    ) acquires ManagingCapabilities {
        assert_owner(asset_owner, asset);
        let transfer_cap = borrow_transfer_from_caps(asset);
        fungible_caps::set_ungated_transfer(transfer_cap, account, allow);
    }

    /// Self-explanatory.
    public fun owner_can_mint<T: key>(asset: &Object<T>): bool acquires ManagingCapabilities {
        option::is_some(&borrow_caps(asset).mint)
    }

    /// Self-explanatory.
    public fun owner_can_transfer<T: key>(asset: &Object<T>): bool acquires ManagingCapabilities {
        option::is_some(&borrow_caps(asset).transfer)
    }

    /// Self-explanatory.
    public fun owner_can_burn<T: key>(asset: &Object<T>): bool acquires ManagingCapabilities {
        option::is_some(&borrow_caps(asset).burn)
    }

    /// Explicitly waive the mint capability.
    public fun waive_mint<T: key>(
        asset_owner: &signer,
        asset: &Object<T>
    ) acquires ManagingCapabilities {
        let mint_cap = &mut borrow_caps_mut(asset_owner, asset).mint;
        assert!(option::is_some(mint_cap), error::not_found(EMINT_CAP));
        fungible_caps::destroy_mint_cap(option::extract(mint_cap));
    }

    /// Explicitly wavie the transfer capability.
    public fun waive_transfer<T: key>(
        asset_owner: &signer,
        asset: &Object<T>
    ) acquires ManagingCapabilities {
        let transfer_cap = &mut borrow_caps_mut(asset_owner, asset).transfer;
        assert!(option::is_some(transfer_cap), error::not_found(EFREEZE_CAP));
        fungible_caps::destroy_transfer_cap(option::extract(transfer_cap));
    }

    /// Explicitly destory the burn capability.
    public fun waive_burn<T: key>(
        asset_owner: &signer,
        asset: &Object<T>
    ) acquires ManagingCapabilities {
        let burn_cap = &mut borrow_caps_mut(asset_owner, asset).burn;
        assert!(option::is_some(burn_cap), error::not_found(EFREEZE_CAP));
        fungible_caps::destroy_burn_cap(option::extract(burn_cap));
    }

    /// Borrow the immutable reference of mint capability from `asset`.
    inline fun borrow_mint_from_caps<T: key>(
        asset: &Object<T>,
    ): &MintCap acquires ManagingCapabilities {
        let mint_cap = &borrow_caps(asset).mint;
        assert!(option::is_some(mint_cap), error::not_found(EMINT_CAP));
        option::borrow(mint_cap)
    }

    /// Borrow the immutable reference of transfer capability from `asset`.
    inline fun borrow_transfer_from_caps<T: key>(
        asset: &Object<T>,
    ): &TransferCap acquires ManagingCapabilities {
        let transfer_cap = &borrow_caps(asset).transfer;
        assert!(option::is_some(transfer_cap), error::not_found(EFREEZE_CAP));
        option::borrow(transfer_cap)
    }

    /// Borrow the immutable reference of burn capability from `asset`.
    inline fun borrow_burn_from_caps<T: key>(
        asset: &Object<T>,
    ): &BurnCap acquires ManagingCapabilities {
        let burn_cap = &borrow_caps(asset).burn;
        assert!(option::is_some(burn_cap), error::not_found(EBURN_CAP));
        option::borrow(burn_cap)
    }

    /// Borrow the immutable reference of capabilities from `asset`.
    inline fun borrow_caps<T: key>(
        asset: &Object<T>,
    ): &ManagingCapabilities acquires ManagingCapabilities {
        borrow_global_mut<ManagingCapabilities>(verify(asset))
    }

    /// Borrow the mutable reference of capabilities from `asset`.
    inline fun borrow_caps_mut<T: key>(
        owner: &signer,
        asset: &Object<T>,
    ): &mut ManagingCapabilities acquires ManagingCapabilities {
        assert_owner(owner, asset);
        borrow_global_mut<ManagingCapabilities>(verify(asset))
    }

    /// Verify `asset` indeed has `GoveranceCapabilities` resource associated.
    inline fun verify<T: key>(asset: &Object<T>): address {
        let asset_addr = object::object_address(asset);
        object::address_to_object<ManagingCapabilities>(asset_addr);
        asset_addr
    }

    /// Assert the owner of `asset` is `owner`.
    inline fun assert_owner<T: key>(owner: &signer, asset: &Object<T>) {
        assert!(object::is_owner(*asset, signer::address_of(owner)), error::permission_denied(ENOT_OWNER));
    }

    #[test_only]
    use aptos_framework::fungible_source;
    #[test_only]
    use aptos_framework::fungible_store::{balance, ungated_transfer_allowed};
    #[test_only]
    use std::string;

    #[test_only]
    public fun init_test_managing_capabilities(creator_ref: &ConstructorRef) {
        init_managing_capabilities(
            creator_ref,
            100 /* max supply */,
            string::utf8(b"USDA"),
            string::utf8(b"$$$"),
            0
        )
    }

    #[test(creator = @0xcafe)]
    fun test_basic_flow(
        creator: &signer,
    ) acquires ManagingCapabilities {
        let (creator_ref, asset) = fungible_source::create_test_token(creator);
        init_test_managing_capabilities(&creator_ref);
        let creator_address = signer::address_of(creator);
        let aaron_address = @0xface;

        assert!(owner_can_mint(&asset), 1);
        assert!(owner_can_transfer(&asset), 2);
        assert!(owner_can_burn(&asset), 3);

        mint(creator, &asset, 100, creator_address);
        assert!(balance(creator_address, &asset) == 100, 4);
        set_ungated_transfer(creator, &asset, creator_address, false);
        assert!(!ungated_transfer_allowed(creator_address, &asset), 5);
        transfer(creator, &asset, 10, creator_address, aaron_address);
        assert!(balance(aaron_address, &asset) == 10, 6);

        set_ungated_transfer(creator, &asset, creator_address, true);
        assert!(ungated_transfer_allowed(creator_address, &asset), 7);
        burn(creator, &asset, 90, creator_address);

        waive_mint(creator, &asset);
        waive_transfer(creator, &asset);
        waive_burn(creator, &asset);

        assert!(!owner_can_mint(&asset), 8);
        assert!(!owner_can_transfer(&asset), 9);
        assert!(!owner_can_burn(&asset), 10);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    fun test_permission_denied(
        creator: &signer,
        aaron: &signer
    ) acquires ManagingCapabilities {
        let (creator_ref, asset) = fungible_source::create_test_token(creator);
        init_test_managing_capabilities(&creator_ref);
        let creator_address = signer::address_of(creator);
        assert!(owner_can_mint(&asset), 1);
        mint(aaron, &asset, 100, creator_address);
    }
}
