module stablecoin::ownable {
    use std::option::{Self, Option};
    use std::signer;
    use aptos_framework::event;
    use aptos_framework::object::{Self, Object};

    // === Errors ===

    /// Non-existent object.
    const ENON_EXISTENT_OBJECT: u64 = 0;
    /// Address is not the owner.
    const ENOT_OWNER: u64 = 1;
    /// Address is not the pending owner.
    const ENOT_PENDING_OWNER: u64 = 2;
    /// Pending owner is not set.
    const EPENDING_OWNER_NOT_SET: u64 = 3;

    // === Structs ===

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// The current and pending owner addresses state.
    struct OwnerRole has key {
        owner: address,
        pending_owner: Option<address>
    }

    // === Events ===

    #[event]
    /// Emitted when the ownership transfer is started.
    struct OwnershipTransferStarted has drop, store {
        obj_address: address,
        old_owner: address,
        new_owner: address
    }

    #[event]
    /// Emitted when the ownership is transferred to a new address.
    struct OwnershipTransferred has drop, store {
        obj_address: address,
        old_owner: address,
        new_owner: address
    }

    // === View-only functions ===

    #[view]
    /// Returns the active owner address.
    public fun owner(obj: Object<OwnerRole>): address acquires OwnerRole {
        borrow_global<OwnerRole>(object::object_address(&obj)).owner
    }

    #[view]
    /// Returns the pending owner address.
    public fun pending_owner(obj: Object<OwnerRole>): Option<address> acquires OwnerRole {
        borrow_global<OwnerRole>(object::object_address(&obj)).pending_owner
    }

    /// Aborts if the caller is not the owner of the input object
    public fun assert_is_owner(caller: &signer, obj_address: address) acquires OwnerRole {
        let obj = object::address_to_object<OwnerRole>(obj_address);
        assert!(owner(obj) == signer::address_of(caller), ENOT_OWNER);
    }

    // === Write functions ===

    /// Creates and inits a new OwnerRole object.
    public fun new(obj_signer: &signer, owner: address) {
        assert!(object::is_object(signer::address_of(obj_signer)), ENON_EXISTENT_OBJECT);
        move_to(obj_signer, OwnerRole { owner, pending_owner: option::none() });
    }

    /// Starts the ownership transfer of the object by setting the pending owner to the new_owner address.
    entry fun transfer_ownership(caller: &signer, obj: Object<OwnerRole>, new_owner: address) acquires OwnerRole {
        let obj_address = object::object_address(&obj);
        let owner_role = borrow_global_mut<OwnerRole>(obj_address);
        assert!(owner_role.owner == signer::address_of(caller), ENOT_OWNER);

        owner_role.pending_owner = option::some(new_owner);

        event::emit(OwnershipTransferStarted { obj_address, old_owner: owner_role.owner, new_owner });
    }

    /// Transfers the ownership of the object by setting the owner to the pending owner address.
    entry fun accept_ownership(caller: &signer, obj: Object<OwnerRole>) acquires OwnerRole {
        let obj_address = object::object_address(&obj);
        let owner_role = borrow_global_mut<OwnerRole>(obj_address);
        assert!(option::is_some(&owner_role.pending_owner), EPENDING_OWNER_NOT_SET);
        assert!(
            option::contains(&owner_role.pending_owner, &signer::address_of(caller)),
            ENOT_PENDING_OWNER
        );

        let old_owner = owner_role.owner;
        let new_owner = option::extract(&mut owner_role.pending_owner);

        owner_role.owner = new_owner;

        event::emit(OwnershipTransferred { obj_address, old_owner, new_owner });
    }

    // === Test-only ===

    #[test_only]
    public fun test_OwnershipTransferStarted_event(
        obj_address: address, old_owner: address, new_owner: address
    ): OwnershipTransferStarted {
        OwnershipTransferStarted { obj_address, old_owner, new_owner }
    }

    #[test_only]
    public fun test_OwnershipTransferred_event(
        obj_address: address, old_owner: address, new_owner: address
    ): OwnershipTransferred {
        OwnershipTransferred { obj_address, old_owner, new_owner }
    }

    #[test_only]
    public fun test_transfer_ownership(
        caller: &signer, obj: Object<OwnerRole>, new_owner: address
    ) acquires OwnerRole {
        transfer_ownership(caller, obj, new_owner);
    }

    #[test_only]
    public fun test_accept_ownership(caller: &signer, obj: Object<OwnerRole>) acquires OwnerRole {
        accept_ownership(caller, obj);
    }

    #[test_only]
    public fun set_owner_for_testing(obj_address: address, owner: address) acquires OwnerRole {
        let role = borrow_global_mut<OwnerRole>(obj_address);
        role.owner = owner;
    }

    #[test_only]
    public fun set_pending_owner_for_testing(obj_address: address, pending_owner: address) acquires OwnerRole {
        let role = borrow_global_mut<OwnerRole>(obj_address);
        role.pending_owner = option::some(pending_owner);
    }
}