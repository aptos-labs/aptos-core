#[test_only]
module stablecoin::usdk_tests {
    use aptos_framework::primary_fungible_store;
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::fungible_asset::{Self, FungibleStore};
    use aptos_framework::object;
    use stablecoin::usdk;
    use std::signer;

    #[test(creator = @0xcafe, minter = @0xface, master_minter = @0xbab, denylister = @0xcade)]
    fun test_basic_flow(creator: &signer, minter: &signer, master_minter: &signer, denylister: &signer) {
        usdk::init_for_test(creator);
        let receiver_address = @0xcafe1;
        let minter_address = signer::address_of(minter);

        // set minter and have minter call mint, check balance
        usdk::add_minter(master_minter, minter_address);
        usdk::mint(minter, minter_address, 100);
        let asset = usdk::metadata();
        assert!(primary_fungible_store::balance(minter_address, asset) == 100);

        // transfer from minter to receiver, check balance
        let minter_store = primary_fungible_store::ensure_primary_store_exists(minter_address, asset);
        let receiver_store = primary_fungible_store::ensure_primary_store_exists(receiver_address, asset);
        dispatchable_fungible_asset::transfer(minter, minter_store, receiver_store, 10);

        // denylist account, check if account is denylisted
        usdk::denylist(denylister, receiver_address);
        assert!(primary_fungible_store::is_frozen(receiver_address, asset));
        usdk::undenylist(denylister, receiver_address);
        assert!(!primary_fungible_store::is_frozen(receiver_address, asset));

        // burn tokens, check balance
        usdk::burn(minter, minter_address, 90);
        assert!(primary_fungible_store::balance(minter_address, asset) == 0);
    }


    #[test(creator = @0xcafe, pauser = @0xdafe, minter = @0xface, master_minter = @0xbab)]
    #[expected_failure(abort_code = 2, location = stablecoin::usdk)]
    fun test_pause(creator: &signer, pauser: &signer, minter: &signer, master_minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);
        usdk::add_minter(master_minter, minter_address);
        usdk::set_pause(pauser, true);
        usdk::mint(minter, minter_address, 100);
    }

    // Pausing stops value movement, so an in-flight transfer must abort.
    #[test(creator = @0xcafe, pauser = @0xdafe, minter = @0xface, master_minter = @0xbab)]
    #[expected_failure(abort_code = 2, location = stablecoin::usdk)]
    fun test_pause_blocks_transfer(
        creator: &signer,
        pauser: &signer,
        minter: &signer,
        master_minter: &signer,
    ) {
        usdk::init_for_test(creator);
        let asset = usdk::metadata();
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address);
        usdk::mint(minter, minter_address, 100);

        let minter_store = primary_fungible_store::ensure_primary_store_exists(minter_address, asset);
        let receiver_store = primary_fungible_store::ensure_primary_store_exists(@0xcafe1, asset);

        usdk::set_pause(pauser, true);
        dispatchable_fungible_asset::transfer(minter, minter_store, receiver_store, 10);
    }

    // Pausing must not disable incident response: role administration and denylisting stay available.
    #[test(creator = @0xcafe, pauser = @0xdafe, minter = @0xface, master_minter = @0xbab, denylister = @0xcade)]
    fun test_administration_available_while_paused(
        creator: &signer,
        pauser: &signer,
        minter: &signer,
        master_minter: &signer,
        denylister: &signer,
    ) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::set_pause(pauser, true);

        usdk::add_minter(master_minter, minter_address);
        assert!(usdk::is_minter(minter_address));
        usdk::remove_minter(master_minter, minter_address);
        assert!(!usdk::is_minter(minter_address));

        usdk::denylist(denylister, minter_address);
        assert!(usdk::is_denylisted(minter_address));
    }

    // A denylisted account must not be able to move value out of its own primary store.
    // The frozen flag is caught by the framework's own withdraw_sanity_check before dispatch
    // reaches our hook, so this aborts with ESTORE_IS_FROZEN rather than EDENYLISTED.
    #[test(creator = @0xcafe, minter = @0xface, master_minter = @0xbab, denylister = @0xcade)]
    #[expected_failure(abort_code = 327683, location = aptos_framework::fungible_asset)]
    fun test_denylisted_account_cannot_transfer(
        creator: &signer,
        minter: &signer,
        master_minter: &signer,
        denylister: &signer,
    ) {
        usdk::init_for_test(creator);
        let asset = usdk::metadata();
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address);
        usdk::mint(minter, minter_address, 100);

        let minter_store = primary_fungible_store::ensure_primary_store_exists(minter_address, asset);
        let receiver_store = primary_fungible_store::ensure_primary_store_exists(@0xcafe1, asset);

        usdk::denylist(denylister, minter_address);
        dispatchable_fungible_asset::transfer(minter, minter_store, receiver_store, 10);
    }

    // A denylisted account must not be able to escape the denylist by holding funds in a store
    // owned by an intermediate object it controls. The framework's withdraw_sanity_check uses
    // object::owns, which accepts indirect ownership, so only the root_owner check in our
    // withdraw hook catches this.
    #[test(creator = @0xcafe, minter = @0xface, master_minter = @0xbab, denylister = @0xcade)]
    #[expected_failure(abort_code = 5, location = stablecoin::usdk)]
    fun test_denylist_not_bypassable_via_nested_store(
        creator: &signer,
        minter: &signer,
        master_minter: &signer,
        denylister: &signer,
    ) {
        usdk::init_for_test(creator);
        let asset = usdk::metadata();
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address);
        usdk::mint(minter, minter_address, 100);

        // Park funds in a store owned by an intermediate object, which is in turn owned by the
        // minter. Two levels are required: with a single level the store's direct owner is already
        // the minter, so the store.owner() check alone would catch it.
        let outer_ref = object::create_object(minter_address);
        let inner_ref = object::create_object(outer_ref.address_from_constructor_ref());
        fungible_asset::create_store(&inner_ref, asset);
        let nested_store = inner_ref.object_from_constructor_ref<FungibleStore>();

        let minter_store = primary_fungible_store::ensure_primary_store_exists(minter_address, asset);
        dispatchable_fungible_asset::transfer(minter, minter_store, nested_store, 50);

        usdk::denylist(denylister, minter_address);

        // The nested store itself is not frozen and is only indirectly owned, so the framework
        // check passes; the denylist must still be enforced by our hook.
        let receiver_store = primary_fungible_store::ensure_primary_store_exists(@0xcafe1, asset);
        dispatchable_fungible_asset::transfer(minter, nested_store, receiver_store, 10);
    }

    // Minting to a denylisted account must abort.
    #[test(creator = @0xcafe, minter = @0xface, master_minter = @0xbab, denylister = @0xcade)]
    #[expected_failure(abort_code = 5, location = stablecoin::usdk)]
    fun test_cannot_mint_to_denylisted_account(
        creator: &signer,
        minter: &signer,
        master_minter: &signer,
        denylister: &signer,
    ) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address);
        usdk::denylist(denylister, @0xcafe1);
        usdk::mint(minter, @0xcafe1, 100);
    }

    // Role checks: a non-minter must not be able to mint.
    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_non_minter_cannot_mint(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::mint(rando, signer::address_of(rando), 100);
    }

    // Role checks: a non-master-minter must not be able to grant the minter role.
    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_non_master_minter_cannot_add_minter(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::add_minter(rando, signer::address_of(rando));
    }

    // Role checks: a non-denylister must not be able to denylist.
    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_non_denylister_cannot_denylist(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::denylist(rando, @0xcafe1);
    }

    // Role checks: a non-pauser must not be able to pause.
    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_non_pauser_cannot_pause(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::set_pause(rando, true);
    }

    // Granting the minter role twice must abort rather than duplicate the entry.
    #[test(creator = @0xcafe, minter = @0xface, master_minter = @0xbab)]
    #[expected_failure(abort_code = 3, location = stablecoin::usdk)]
    fun test_add_minter_twice_aborts(creator: &signer, minter: &signer, master_minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);
        usdk::add_minter(master_minter, minter_address);
        usdk::add_minter(master_minter, minter_address);
    }

    // Revoking the minter role must actually stop minting.
    #[test(creator = @0xcafe, minter = @0xface, master_minter = @0xbab)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_removed_minter_cannot_mint(creator: &signer, minter: &signer, master_minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address);
        usdk::mint(minter, minter_address, 100);
        usdk::remove_minter(master_minter, minter_address);

        usdk::mint(minter, minter_address, 100);
    }

    // Burning from an account with no store must abort rather than silently creating one.
    #[test(creator = @0xcafe, minter = @0xface, master_minter = @0xbab)]
    #[expected_failure(abort_code = 7, location = stablecoin::usdk)]
    fun test_burn_without_store_aborts(creator: &signer, minter: &signer, master_minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address);
        usdk::burn(minter, @0xcafe1, 100);

        assert!(!primary_fungible_store::primary_store_exists(@0xcafe1, usdk::metadata()));
    }

    // The view functions must reflect pause, minter and denylist state.
    #[test(creator = @0xcafe, pauser = @0xdafe, minter = @0xface, master_minter = @0xbab, denylister = @0xcade)]
    fun test_views(
        creator: &signer,
        pauser: &signer,
        minter: &signer,
        master_minter: &signer,
        denylister: &signer,
    ) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        assert!(!usdk::is_paused());
        assert!(!usdk::is_minter(minter_address));
        assert!(!usdk::is_denylisted(minter_address));
        assert!(usdk::master_minter() == signer::address_of(master_minter));
        assert!(usdk::pauser() == signer::address_of(pauser));
        assert!(usdk::denylister() == signer::address_of(denylister));
        assert!(usdk::minters().is_empty());

        usdk::add_minter(master_minter, minter_address);
        assert!(usdk::is_minter(minter_address));
        assert!(usdk::minters() == vector[minter_address]);

        // The master minter may mint without being listed explicitly.
        assert!(usdk::is_minter(signer::address_of(master_minter)));

        usdk::set_pause(pauser, true);
        assert!(usdk::is_paused());
        usdk::set_pause(pauser, false);
        assert!(!usdk::is_paused());

        usdk::denylist(denylister, minter_address);
        assert!(usdk::is_denylisted(minter_address));
        usdk::undenylist(denylister, minter_address);
        assert!(!usdk::is_denylisted(minter_address));
    }

    // test the ability of a denylisted account to transfer out newly created store
    #[test(creator = @0xcafe, denylister = @0xcade, receiver = @0xdead)]
    #[expected_failure(abort_code = 327683, location = aptos_framework::object)]
    fun test_untransferrable_store(creator: &signer, denylister: &signer, receiver: &signer) {
        usdk::init_for_test(creator);
        let receiver_address = signer::address_of(receiver);
        let asset = usdk::metadata();

        usdk::denylist(denylister, receiver_address);
        assert!(primary_fungible_store::is_frozen(receiver_address, asset));

        let constructor_ref = object::create_object(receiver_address);
        fungible_asset::create_store(&constructor_ref, asset);
        let store = constructor_ref.object_from_constructor_ref();

        object::transfer<FungibleStore>(receiver, store, @0xdeadbeef);
    }
}
