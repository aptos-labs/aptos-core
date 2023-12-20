module tournament::admin {
    use std::bcs;
    use std::option::{Self, Option};
    use std::signer;
    use aptos_framework::account::{Self, SignerCapability};
    use aptos_framework::object::{Self, ObjectCore};

    use tournament::tournament_manager;

    friend tournament::rock_paper_scissor;
    friend tournament::trivia;
    friend tournament::aptos_tournament;

    /// You are not authorized to do that
    const ENOT_AUTHORIZED: u64 = 0;

    #[test_only]
    friend tournament::rps_utils;

    struct AdminStore has key, drop {
        signer_cap: SignerCapability,
        // An additional admin address to use, so we don't need to use the deploy key
        admin_address: Option<address>
    }

    public(friend) fun setup_admin_signer(deployer: &signer) {
        let (_resource_signer, signer_cap) = account::create_resource_account(
            deployer,
            bcs::to_bytes(&b"aptos tournament admin")
        );
        let admin_store = AdminStore {
            signer_cap,
            admin_address: option::none(),
        };
        move_to(deployer, admin_store);
    }

    public fun assert_admin(caller: &signer) acquires AdminStore {
        let caller_address = signer::address_of(caller);

        let is_tournament = caller_address == @tournament;
        let admin_address = borrow_global<AdminStore>(@tournament).admin_address;

        assert!(
            is_tournament ||
                (option::is_some(&admin_address) && &caller_address == option::borrow(&admin_address)) ||
                signer::address_of(&get_admin_signer()) == caller_address,
            ENOT_AUTHORIZED
        );
    }

    public entry fun set_admin_signer(caller: &signer, admin_address: address) acquires AdminStore {
        assert_admin(caller);
        borrow_global_mut<AdminStore>(@tournament).admin_address = option::some(admin_address);
    }

    public fun get_admin_signer_as_admin(caller: &signer): signer acquires AdminStore {
        assert_admin(caller);
        get_admin_signer()
    }

    public fun get_tournament_owner_signer_as_admin(
        caller: &signer,
        tournament_address: address
    ): signer acquires AdminStore {
        let admin_signer = get_admin_signer_as_admin(caller);
        tournament_manager::get_tournament_signer(&admin_signer, tournament_address)
    }

    public(friend) fun get_admin_signer(): signer acquires AdminStore {
        let admin_store = borrow_global<AdminStore>(@tournament);
        account::create_signer_with_capability(&admin_store.signer_cap)
    }

    public(friend) fun get_tournament_owner_signer(tournament_address: address): signer acquires AdminStore {
        let admin_signer = get_admin_signer();
        tournament_manager::get_tournament_signer(&admin_signer, tournament_address)
    }

    // Assumes the tournament_address is `object::owner(Object<ObjectCore>@object_address)`
    public(friend) fun get_tournament_owner_signer_from_object_owner(
        object_address: address
    ): signer acquires AdminStore {
        get_tournament_owner_signer(
            object::owner(object::address_to_object<ObjectCore>(object_address))
        )
    }
}
