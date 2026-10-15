// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// Plaintext accounting prototype for a future TEE-backed store. This module does not encrypt data.
module encrypted_store::encrypted_fungible_store {
    use std::error;
    use std::signer;
    use aptos_std::table::{Self, Table};
    use aptos_framework::fungible_asset::{Self, FungibleStore, Metadata};
    use aptos_framework::object::{Self, ExtendRef, Object};
    use aptos_framework::primary_fungible_store;

    /// The signer does not own the source store.
    const E_NOT_OWNER: u64 = 1;
    /// A transfer must use stores of the same asset.
    const E_ASSET_MISMATCH: u64 = 2;
    /// The source balance cannot cover the operation.
    const E_INSUFFICIENT_BALANCE: u64 = 3;
    /// Hook-enabled and nontransferable assets need a separate policy design.
    const E_UNSUPPORTED_ASSET: u64 = 4;
    /// The issuer has frozen the pooled reserve.
    const E_RESERVE_FROZEN: u64 = 5;

    /// Proposed confidential resource: persisted in plaintext until the VM supports encryption.
    struct EncryptedFungibleStore has key {
        owner: address,
        metadata: Object<Metadata>,
        balance: u64
    }

    struct Reserves has key {
        assets: Table<address, Reserve>
    }

    struct Reserve has store {
        store: Object<FungibleStore>,
        extend_ref: ExtendRef,
        liability: u64
    }

    fun init_module(publisher: &signer) {
        move_to(publisher, Reserves { assets: table::new() });
    }

    /// Creates a nontransferable store controlled by the owner's signer.
    public fun create_store(
        owner: &signer, metadata: Object<Metadata>
    ): Object<EncryptedFungibleStore> {
        assert_supported(metadata);
        ensure_reserve(metadata);
        let owner = signer::address_of(owner);
        let constructor = object::create_sticky_object(owner);
        object::disable_ungated_transfer(&object::generate_transfer_ref(&constructor));
        move_to(
            &object::generate_signer(&constructor),
            EncryptedFungibleStore { owner, metadata, balance: 0 }
        );
        object::object_from_constructor_ref(&constructor)
    }

    /// Atomically locks public assets and credits an existing recipient store.
    public entry fun shield(
        sender: &signer, recipient: Object<EncryptedFungibleStore>, amount: u64
    ) {
        let store = &mut EncryptedFungibleStore[object::object_address(&recipient)];
        let reserve = active_reserve(&mut Reserves[@encrypted_store], store.metadata);
        let assets = primary_fungible_store::withdraw(sender, store.metadata, amount);
        fungible_asset::deposit(reserve.store, assets);
        store.balance = store.balance + amount;
        reserve.liability = reserve.liability + amount;
    }

    /// Moves backed balances without changing the public reserve.
    public entry fun transfer(
        owner: &signer,
        source: Object<EncryptedFungibleStore>,
        destination: Object<EncryptedFungibleStore>,
        amount: u64
    ) {
        let source_addr = object::object_address(&source);
        let destination_addr = object::object_address(&destination);
        let from = &EncryptedFungibleStore[source_addr];
        assert_owner(owner, from);
        assert!(
            from.metadata == EncryptedFungibleStore[destination_addr].metadata,
            error::invalid_argument(E_ASSET_MISMATCH)
        );
        active_reserve(&mut Reserves[@encrypted_store], from.metadata);
        assert!(from.balance >= amount, error::invalid_argument(E_INSUFFICIENT_BALANCE));
        if (source_addr == destination_addr) { return };
        EncryptedFungibleStore[source_addr].balance = from.balance - amount;
        let to = &mut EncryptedFungibleStore[destination_addr];
        to.balance = to.balance + amount;
    }

    /// Atomically debits a backed balance and releases public assets.
    public entry fun unshield(
        owner: &signer,
        source: Object<EncryptedFungibleStore>,
        recipient: address,
        amount: u64
    ) {
        let store = &mut EncryptedFungibleStore[object::object_address(&source)];
        assert_owner(owner, store);
        let reserve = active_reserve(&mut Reserves[@encrypted_store], store.metadata);
        assert!(store.balance >= amount, error::invalid_argument(E_INSUFFICIENT_BALANCE));
        store.balance = store.balance - amount;
        reserve.liability = reserve.liability - amount;
        let reserve_signer = object::generate_signer_for_extending(&reserve.extend_ref);
        let assets = fungible_asset::withdraw(&reserve_signer, reserve.store, amount);
        primary_fungible_store::deposit(recipient, assets);
    }

    /// Requires owner authorization at the API layer; REST storage remains public in this prototype.
    public fun balance(
        owner: &signer, store: Object<EncryptedFungibleStore>
    ): u64 {
        let store = &EncryptedFungibleStore[object::object_address(&store)];
        assert_owner(owner, store);
        store.balance
    }

    /// Public backing and aggregate outstanding claims, useful for reserve accounting.
    public fun reserve_accounting(metadata: Object<Metadata>): (u64, u64) {
        let reserve =
            table::borrow(
                &Reserves[@encrypted_store].assets,
                object::object_address(&metadata)
            );
        (fungible_asset::balance(reserve.store), reserve.liability)
    }

    fun assert_owner(owner: &signer, store: &EncryptedFungibleStore) {
        assert!(
            signer::address_of(owner) == store.owner,
            error::permission_denied(E_NOT_OWNER)
        );
    }

    fun assert_supported(metadata: Object<Metadata>) {
        assert!(
            !fungible_asset::is_asset_type_dispatchable(metadata)
                && !fungible_asset::is_untransferable(metadata),
            error::invalid_argument(E_UNSUPPORTED_ASSET)
        );
    }

    fun ensure_reserve(metadata: Object<Metadata>) {
        let reserves = &mut Reserves[@encrypted_store].assets;
        let asset = object::object_address(&metadata);
        if (!table::contains(reserves, asset)) {
            // No external signer owns the reserve; only this module retains its ExtendRef.
            let constructor = object::create_sticky_object(@0x0);
            object::disable_ungated_transfer(&object::generate_transfer_ref(&constructor));
            let store = fungible_asset::create_store(&constructor, metadata);
            table::add(
                reserves,
                asset,
                Reserve {
                    store,
                    extend_ref: object::generate_extend_ref(&constructor),
                    liability: 0
                }
            );
        };
    }

    fun active_reserve(
        reserves: &mut Reserves, metadata: Object<Metadata>
    ): &mut Reserve {
        assert_supported(metadata);
        let reserve =
            table::borrow_mut(&mut reserves.assets, object::object_address(&metadata));
        assert!(
            !fungible_asset::is_frozen(reserve.store),
            error::permission_denied(E_RESERVE_FROZEN)
        );
        reserve
    }

    #[test_only]
    public fun initialize_for_test(publisher: &signer) {
        init_module(publisher);
    }

    #[test_only]
    public fun reserve_store_for_test(metadata: Object<Metadata>): Object<FungibleStore> {
        table::borrow(
            &Reserves[@encrypted_store].assets,
            object::object_address(&metadata)
        ).store
    }
}
