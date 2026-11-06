// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
#[test_only]
module encrypted_store::encrypted_fungible_store_tests {
    use std::option;
    use std::signer;
    use std::string::utf8;
    use aptos_framework::fungible_asset::{Self, Metadata, MintRef, TransferRef};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use encrypted_store::encrypted_fungible_store::{Self, EncryptedFungibleStore};

    fun asset(creator: &signer): (Object<Metadata>, MintRef, TransferRef) {
        let constructor = object::create_sticky_object(signer::address_of(creator));
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            &constructor,
            option::none(),
            utf8(b"Prototype asset"),
            utf8(b"TEST"),
            6,
            utf8(b""),
            utf8(b"")
        );
        (
            object::object_from_constructor_ref(&constructor),
            fungible_asset::generate_mint_ref(&constructor),
            fungible_asset::generate_transfer_ref(&constructor)
        )
    }

    fun setup(
        publisher: &signer, alice: &signer
    ): (Object<Metadata>, Object<EncryptedFungibleStore>, TransferRef) {
        encrypted_fungible_store::initialize_for_test(publisher);
        let (metadata, mint_ref, transfer_ref) = asset(publisher);
        primary_fungible_store::mint(&mint_ref, signer::address_of(alice), 100);
        let store = encrypted_fungible_store::create_store(alice, metadata);
        encrypted_fungible_store::shield(alice, store, 70);
        (metadata, store, transfer_ref)
    }

    #[test(publisher = @encrypted_store, alice = @0xA11CE, bob = @0xB0B)]
    fun lifecycle(publisher: &signer, alice: &signer, bob: &signer) {
        let (metadata, alice_store, _) = setup(publisher, alice);
        let bob_store = encrypted_fungible_store::create_store(bob, metadata);
        assert!(encrypted_fungible_store::balance(alice, alice_store) == 70, 0);
        assert!(
            primary_fungible_store::balance(signer::address_of(alice), metadata) == 30,
            1
        );
        encrypted_fungible_store::transfer(alice, alice_store, bob_store, 25);
        assert!(encrypted_fungible_store::balance(alice, alice_store) == 45, 2);
        assert!(encrypted_fungible_store::balance(bob, bob_store) == 25, 3);
        let (backing, liability) = encrypted_fungible_store::reserve_accounting(metadata);
        assert!(backing == 70 && liability == 70, 4);
        encrypted_fungible_store::transfer(alice, alice_store, alice_store, 45);
        assert!(encrypted_fungible_store::balance(alice, alice_store) == 45, 5);
        encrypted_fungible_store::unshield(bob, bob_store, signer::address_of(bob), 10);
        assert!(encrypted_fungible_store::balance(alice, alice_store) == 45, 6);
        assert!(encrypted_fungible_store::balance(bob, bob_store) == 15, 7);
        assert!(
            primary_fungible_store::balance(signer::address_of(bob), metadata) == 10,
            8
        );
        let (backing, liability) = encrypted_fungible_store::reserve_accounting(metadata);
        assert!(backing == 60 && liability == 60, 9);
        encrypted_fungible_store::unshield(
            alice,
            alice_store,
            signer::address_of(alice),
            45
        );
        encrypted_fungible_store::unshield(bob, bob_store, signer::address_of(bob), 15);
        let (backing, liability) = encrypted_fungible_store::reserve_accounting(metadata);
        assert!(backing == 0 && liability == 0, 10);
        assert!(encrypted_fungible_store::balance(alice, alice_store) == 0, 11);
        assert!(encrypted_fungible_store::balance(bob, bob_store) == 0, 12);
        assert!(
            primary_fungible_store::balance(signer::address_of(alice), metadata) == 75,
            13
        );
        assert!(
            primary_fungible_store::balance(signer::address_of(bob), metadata) == 25,
            14
        );
    }

    #[test(publisher = @encrypted_store, alice = @0xA11CE, bob = @0xB0B)]
    #[expected_failure(
        abort_code = 0x50001, location = encrypted_store::encrypted_fungible_store
    )]
    fun unauthorized_transfer(
        publisher: &signer, alice: &signer, bob: &signer
    ) {
        let (_, source, _) = setup(publisher, alice);
        encrypted_fungible_store::transfer(bob, source, source, 1);
    }

    #[test(publisher = @encrypted_store, alice = @0xA11CE, bob = @0xB0B)]
    #[expected_failure(
        abort_code = 0x50001, location = encrypted_store::encrypted_fungible_store
    )]
    fun unauthorized_unshield(
        publisher: &signer, alice: &signer, bob: &signer
    ) {
        let (_, source, _) = setup(publisher, alice);
        encrypted_fungible_store::unshield(bob, source, signer::address_of(bob), 1);
    }

    #[test(publisher = @encrypted_store, alice = @0xA11CE, bob = @0xB0B)]
    #[expected_failure(
        abort_code = 0x50001, location = encrypted_store::encrypted_fungible_store
    )]
    fun unauthorized_balance(
        publisher: &signer, alice: &signer, bob: &signer
    ) {
        let (_, source, _) = setup(publisher, alice);
        encrypted_fungible_store::balance(bob, source);
    }

    #[test(publisher = @encrypted_store, alice = @0xA11CE, bob = @0xB0B)]
    #[expected_failure(
        abort_code = 0x10003, location = encrypted_store::encrypted_fungible_store
    )]
    fun overdraft(publisher: &signer, alice: &signer, bob: &signer) {
        let (metadata, source, _) = setup(publisher, alice);
        let destination = encrypted_fungible_store::create_store(bob, metadata);
        encrypted_fungible_store::transfer(alice, source, destination, 71);
    }

    #[test(publisher = @encrypted_store, alice = @0xA11CE)]
    #[expected_failure(
        abort_code = 0x10003, location = encrypted_store::encrypted_fungible_store
    )]
    fun unshield_overdraft(publisher: &signer, alice: &signer) {
        let (_, source, _) = setup(publisher, alice);
        encrypted_fungible_store::unshield(alice, source, signer::address_of(alice), 71);
    }

    #[test(publisher = @encrypted_store, alice = @0xA11CE, bob = @0xB0B)]
    #[expected_failure(
        abort_code = 0x10002, location = encrypted_store::encrypted_fungible_store
    )]
    fun cross_asset_transfer(
        publisher: &signer, alice: &signer, bob: &signer
    ) {
        let (_, source, _) = setup(publisher, alice);
        let (other, _, _) = asset(publisher);
        let destination = encrypted_fungible_store::create_store(bob, other);
        encrypted_fungible_store::transfer(alice, source, destination, 1);
    }

    #[test(publisher = @encrypted_store, alice = @0xA11CE, bob = @0xB0B)]
    #[expected_failure(
        abort_code = 0x50005, location = encrypted_store::encrypted_fungible_store
    )]
    fun frozen_reserve(publisher: &signer, alice: &signer, bob: &signer) {
        let (metadata, source, transfer_ref) = setup(publisher, alice);
        let destination = encrypted_fungible_store::create_store(bob, metadata);
        fungible_asset::set_frozen_flag(
            &transfer_ref,
            encrypted_fungible_store::reserve_store_for_test(metadata),
            true
        );
        encrypted_fungible_store::transfer(alice, source, destination, 1);
    }

    #[test(publisher = @encrypted_store, alice = @0xA11CE)]
    #[expected_failure(abort_code = 0x50008, location = aptos_framework::fungible_asset)]
    fun publisher_cannot_withdraw_reserve(
        publisher: &signer, alice: &signer
    ) {
        let (metadata, _, _) = setup(publisher, alice);
        let assets =
            fungible_asset::withdraw(
                publisher,
                encrypted_fungible_store::reserve_store_for_test(metadata),
                1
            );
        primary_fungible_store::deposit(signer::address_of(publisher), assets);
    }

    #[test(publisher = @encrypted_store)]
    #[expected_failure(
        abort_code = 0x10004, location = encrypted_store::encrypted_fungible_store
    )]
    fun nontransferable_asset(publisher: &signer) {
        encrypted_fungible_store::initialize_for_test(publisher);
        let constructor = object::create_sticky_object(signer::address_of(publisher));
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            &constructor,
            option::none(),
            utf8(b"Nontransferable"),
            utf8(b"SOUL"),
            0,
            utf8(b""),
            utf8(b"")
        );
        fungible_asset::set_untransferable(&constructor);
        encrypted_fungible_store::create_store(
            publisher,
            object::object_from_constructor_ref<Metadata>(&constructor)
        );
    }
}
