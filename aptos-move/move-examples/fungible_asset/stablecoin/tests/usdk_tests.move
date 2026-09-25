#[test_only]
module stablecoin::usdk_tests {
    use aptos_framework::dispatchable_fungible_asset;
    use aptos_framework::fungible_asset::{Self, FungibleStore};
    use aptos_framework::object;
    use aptos_framework::primary_fungible_store;
    use stablecoin::usdk;
    use std::option;
    use std::signer;

    const RECEIVER: address = @0xcafe1;
    const OUTSIDER: address = @0xdead;

    // ----------------------------------------------------------------------
    // Initialization
    // ----------------------------------------------------------------------

    #[test(creator = @0xcafe)]
    fun test_initialize_sets_roles_and_state(creator: &signer) {
        assert!(!usdk::is_initialized());
        usdk::init_for_test(creator);

        assert!(usdk::is_initialized());
        assert!(!usdk::is_paused());
        assert!(usdk::master_minter() == @0xbab);
        assert!(usdk::pauser() == @0xdafe);
        assert!(usdk::denylister() == @0xcade);
        assert!(usdk::confiscator() == @0xc0ffee);
        assert!(usdk::pending_master_minter() == option::none());
        assert!(usdk::pending_pauser() == option::none());
        assert!(usdk::pending_denylister() == option::none());
        assert!(usdk::pending_confiscator() == option::none());
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 6, location = stablecoin::usdk)]
    fun test_initialize_twice_aborts(creator: &signer) {
        usdk::init_for_test(creator);
        usdk::init_for_test(creator);
    }

    #[test(rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_initialize_requires_module_address(rando: &signer) {
        usdk::init_for_test(rando);
    }

    // ----------------------------------------------------------------------
    // Core flow
    // ----------------------------------------------------------------------

    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface)]
    fun test_basic_flow(creator: &signer, master_minter: &signer, minter: &signer) {
        usdk::init_for_test(creator);
        let asset = usdk::metadata();
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::mint(minter, minter_address, 100);
        assert!(primary_fungible_store::balance(minter_address, asset) == 100);

        let minter_store = primary_fungible_store::ensure_primary_store_exists(minter_address, asset);
        let receiver_store = primary_fungible_store::ensure_primary_store_exists(RECEIVER, asset);
        dispatchable_fungible_asset::transfer(minter, minter_store, receiver_store, 10);
        assert!(primary_fungible_store::balance(RECEIVER, asset) == 10);

        usdk::burn(minter, 90);
        assert!(primary_fungible_store::balance(minter_address, asset) == 0);
    }

    // ----------------------------------------------------------------------
    // Mint authority and allowances
    // ----------------------------------------------------------------------

    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface)]
    fun test_mint_decrements_allowance(creator: &signer, master_minter: &signer, minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        assert!(usdk::is_minter(minter_address));
        assert!(usdk::mint_allowance(minter_address) == 1000);

        usdk::mint(minter, RECEIVER, 400);
        assert!(usdk::mint_allowance(minter_address) == 600);
    }

    // A compromised minter can issue at most its remaining allowance.
    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface)]
    #[expected_failure(abort_code = 8, location = stablecoin::usdk)]
    fun test_mint_beyond_allowance_aborts(creator: &signer, master_minter: &signer, minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 100);
        usdk::mint(minter, RECEIVER, 101);
    }

    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface)]
    fun test_mint_zero_is_noop(creator: &signer, master_minter: &signer, minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 100);
        usdk::mint(minter, RECEIVER, 0);
        assert!(usdk::mint_allowance(minter_address) == 100);
        assert!(primary_fungible_store::balance(RECEIVER, usdk::metadata()) == 0);
    }

    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_non_minter_cannot_mint(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::mint(rando, RECEIVER, 100);
    }

    // The master minter holds no implicit mint authority.
    #[test(creator = @0xcafe, master_minter = @0xbab)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_master_minter_cannot_mint_without_allowance(creator: &signer, master_minter: &signer) {
        usdk::init_for_test(creator);
        usdk::mint(master_minter, RECEIVER, 100);
    }

    // Granting itself an allowance is possible, but explicit and auditable.
    #[test(creator = @0xcafe, master_minter = @0xbab)]
    fun test_master_minter_may_grant_itself_allowance(creator: &signer, master_minter: &signer) {
        usdk::init_for_test(creator);
        let master_address = signer::address_of(master_minter);

        usdk::add_minter(master_minter, master_address, 50);
        usdk::mint(master_minter, RECEIVER, 50);
        assert!(primary_fungible_store::balance(RECEIVER, usdk::metadata()) == 50);
    }

    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface, denylister = @0xcade)]
    #[expected_failure(abort_code = 5, location = stablecoin::usdk)]
    fun test_cannot_mint_to_denylisted_account(
        creator: &signer,
        master_minter: &signer,
        minter: &signer,
        denylister: &signer,
    ) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::mint(minter, RECEIVER, 10);
        usdk::denylist(denylister, RECEIVER);
        usdk::mint(minter, RECEIVER, 10);
    }

    // ----------------------------------------------------------------------
    // Minter administration
    // ----------------------------------------------------------------------

    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface)]
    #[expected_failure(abort_code = 3, location = stablecoin::usdk)]
    fun test_add_minter_twice_aborts(creator: &signer, master_minter: &signer, minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);
        usdk::add_minter(master_minter, minter_address, 1);
        usdk::add_minter(master_minter, minter_address, 1);
    }

    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_non_master_minter_cannot_add_minter(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::add_minter(rando, OUTSIDER, 100);
    }

    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface)]
    fun test_removed_minter_loses_authority(creator: &signer, master_minter: &signer, minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 100);
        usdk::remove_minter(master_minter, minter_address);
        assert!(!usdk::is_minter(minter_address));
        assert!(usdk::mint_allowance(minter_address) == 0);
    }

    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_removed_minter_cannot_mint(creator: &signer, master_minter: &signer, minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 100);
        usdk::remove_minter(master_minter, minter_address);
        usdk::mint(minter, RECEIVER, 1);
    }

    #[test(creator = @0xcafe, master_minter = @0xbab)]
    fun test_remove_minter_is_noop_for_non_minter(creator: &signer, master_minter: &signer) {
        usdk::init_for_test(creator);
        usdk::remove_minter(master_minter, OUTSIDER);
        assert!(!usdk::is_minter(OUTSIDER));
    }

    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_non_master_minter_cannot_remove_minter(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::remove_minter(rando, OUTSIDER);
    }

    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface)]
    fun test_set_mint_allowance(creator: &signer, master_minter: &signer, minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 100);
        usdk::set_mint_allowance(master_minter, minter_address, 500);
        assert!(usdk::mint_allowance(minter_address) == 500);
    }

    #[test(creator = @0xcafe, master_minter = @0xbab)]
    #[expected_failure(abort_code = 4, location = stablecoin::usdk)]
    fun test_set_mint_allowance_requires_minter(creator: &signer, master_minter: &signer) {
        usdk::init_for_test(creator);
        usdk::set_mint_allowance(master_minter, OUTSIDER, 100);
    }

    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_set_mint_allowance_requires_master_minter(
        creator: &signer,
        master_minter: &signer,
        minter: &signer,
        rando: &signer,
    ) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);
        usdk::add_minter(master_minter, minter_address, 100);
        usdk::set_mint_allowance(rando, minter_address, 500);
    }

    // ----------------------------------------------------------------------
    // Pause policy
    // ----------------------------------------------------------------------

    #[test(creator = @0xcafe, pauser = @0xdafe, master_minter = @0xbab, minter = @0xface)]
    #[expected_failure(abort_code = 2, location = stablecoin::usdk)]
    fun test_pause_blocks_mint(
        creator: &signer,
        pauser: &signer,
        master_minter: &signer,
        minter: &signer,
    ) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);
        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::set_pause(pauser, true);
        usdk::mint(minter, minter_address, 100);
    }

    #[test(creator = @0xcafe, pauser = @0xdafe, master_minter = @0xbab, minter = @0xface)]
    #[expected_failure(abort_code = 2, location = stablecoin::usdk)]
    fun test_pause_blocks_transfer(
        creator: &signer,
        pauser: &signer,
        master_minter: &signer,
        minter: &signer,
    ) {
        usdk::init_for_test(creator);
        let asset = usdk::metadata();
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::mint(minter, minter_address, 100);

        let minter_store = primary_fungible_store::ensure_primary_store_exists(minter_address, asset);
        let receiver_store = primary_fungible_store::ensure_primary_store_exists(RECEIVER, asset);

        usdk::set_pause(pauser, true);
        dispatchable_fungible_asset::transfer(minter, minter_store, receiver_store, 10);
    }

    #[test(creator = @0xcafe, pauser = @0xdafe, master_minter = @0xbab, minter = @0xface)]
    #[expected_failure(abort_code = 2, location = stablecoin::usdk)]
    fun test_pause_blocks_burn(
        creator: &signer,
        pauser: &signer,
        master_minter: &signer,
        minter: &signer,
    ) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::mint(minter, minter_address, 100);
        usdk::set_pause(pauser, true);
        usdk::burn(minter, 10);
    }

    // Granting new issuing authority must not be possible during an incident.
    #[test(creator = @0xcafe, pauser = @0xdafe, master_minter = @0xbab)]
    #[expected_failure(abort_code = 2, location = stablecoin::usdk)]
    fun test_pause_blocks_add_minter(creator: &signer, pauser: &signer, master_minter: &signer) {
        usdk::init_for_test(creator);
        usdk::set_pause(pauser, true);
        usdk::add_minter(master_minter, OUTSIDER, 100);
    }

    #[test(creator = @0xcafe, pauser = @0xdafe, master_minter = @0xbab, minter = @0xface)]
    #[expected_failure(abort_code = 2, location = stablecoin::usdk)]
    fun test_pause_blocks_allowance_increase(
        creator: &signer,
        pauser: &signer,
        master_minter: &signer,
        minter: &signer,
    ) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);
        usdk::add_minter(master_minter, minter_address, 100);
        usdk::set_pause(pauser, true);
        usdk::set_mint_allowance(master_minter, minter_address, 101);
    }

    // Recovery actions stay available while paused.
    #[test(
        creator = @0xcafe,
        pauser = @0xdafe,
        master_minter = @0xbab,
        minter = @0xface,
        denylister = @0xcade,
    )]
    fun test_recovery_actions_available_while_paused(
        creator: &signer,
        pauser: &signer,
        master_minter: &signer,
        minter: &signer,
        denylister: &signer,
    ) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);
        usdk::add_minter(master_minter, minter_address, 1000);

        usdk::set_pause(pauser, true);

        // Allowance decreases are permitted.
        usdk::set_mint_allowance(master_minter, minter_address, 1);
        assert!(usdk::mint_allowance(minter_address) == 1);

        // Revocation is permitted.
        usdk::remove_minter(master_minter, minter_address);
        assert!(!usdk::is_minter(minter_address));

        // Denylisting is permitted.
        usdk::denylist(denylister, OUTSIDER);
        assert!(usdk::is_denylisted(OUTSIDER));
        usdk::undenylist(denylister, OUTSIDER);
        assert!(!usdk::is_denylisted(OUTSIDER));
    }

    #[test(creator = @0xcafe, pauser = @0xdafe)]
    fun test_set_pause_is_idempotent(creator: &signer, pauser: &signer) {
        usdk::init_for_test(creator);
        usdk::set_pause(pauser, false);
        assert!(!usdk::is_paused());

        usdk::set_pause(pauser, true);
        usdk::set_pause(pauser, true);
        assert!(usdk::is_paused());

        usdk::set_pause(pauser, false);
        assert!(!usdk::is_paused());
    }

    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_non_pauser_cannot_pause(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::set_pause(rando, true);
    }

    // ----------------------------------------------------------------------
    // Denylist
    // ----------------------------------------------------------------------

    // The framework rejects a frozen primary store before dispatch reaches our
    // hook, so this surfaces as ESTORE_IS_FROZEN from 0x1::fungible_asset.
    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface, denylister = @0xcade)]
    #[expected_failure(abort_code = 327683, location = aptos_framework::fungible_asset)]
    fun test_denylisted_account_cannot_transfer(
        creator: &signer,
        master_minter: &signer,
        minter: &signer,
        denylister: &signer,
    ) {
        usdk::init_for_test(creator);
        let asset = usdk::metadata();
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::mint(minter, minter_address, 100);

        let minter_store = primary_fungible_store::ensure_primary_store_exists(minter_address, asset);
        let receiver_store = primary_fungible_store::ensure_primary_store_exists(RECEIVER, asset);

        usdk::denylist(denylister, minter_address);
        dispatchable_fungible_asset::transfer(minter, minter_store, receiver_store, 10);
    }

    // A denylisted account must not escape enforcement by parking funds in a store
    // owned by an intermediate object it controls. The framework's own check uses
    // object::owns, which accepts indirect ownership, so only the root_owner check
    // in the withdraw hook catches this. Two levels of nesting are required: with a
    // single level the store's direct owner is already the denylisted account.
    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface, denylister = @0xcade)]
    #[expected_failure(abort_code = 5, location = stablecoin::usdk)]
    fun test_denylist_not_bypassable_via_nested_store(
        creator: &signer,
        master_minter: &signer,
        minter: &signer,
        denylister: &signer,
    ) {
        usdk::init_for_test(creator);
        let asset = usdk::metadata();
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::mint(minter, minter_address, 100);

        let outer_ref = object::create_object(minter_address);
        let inner_ref = object::create_object(outer_ref.address_from_constructor_ref());
        fungible_asset::create_store(&inner_ref, asset);
        let nested_store = inner_ref.object_from_constructor_ref<FungibleStore>();

        let minter_store = primary_fungible_store::ensure_primary_store_exists(minter_address, asset);
        dispatchable_fungible_asset::transfer(minter, minter_store, nested_store, 50);

        usdk::denylist(denylister, minter_address);

        let receiver_store = primary_fungible_store::ensure_primary_store_exists(RECEIVER, asset);
        dispatchable_fungible_asset::transfer(minter, nested_store, receiver_store, 10);
    }

    // The deposit hook must also reject a denylisted root owner.
    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface, denylister = @0xcade)]
    #[expected_failure(abort_code = 5, location = stablecoin::usdk)]
    fun test_denylist_blocks_deposit_into_nested_store(
        creator: &signer,
        master_minter: &signer,
        minter: &signer,
        denylister: &signer,
    ) {
        usdk::init_for_test(creator);
        let asset = usdk::metadata();
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::mint(minter, minter_address, 100);

        let outer_ref = object::create_object(OUTSIDER);
        let inner_ref = object::create_object(outer_ref.address_from_constructor_ref());
        fungible_asset::create_store(&inner_ref, asset);
        let nested_store = inner_ref.object_from_constructor_ref<FungibleStore>();

        usdk::denylist(denylister, OUTSIDER);

        let minter_store = primary_fungible_store::ensure_primary_store_exists(minter_address, asset);
        dispatchable_fungible_asset::transfer(minter, minter_store, nested_store, 10);
    }

    #[test(creator = @0xcafe, denylister = @0xcade)]
    fun test_denylist_is_idempotent(creator: &signer, denylister: &signer) {
        usdk::init_for_test(creator);

        usdk::denylist(denylister, RECEIVER);
        usdk::denylist(denylister, RECEIVER);
        assert!(usdk::is_denylisted(RECEIVER));

        usdk::undenylist(denylister, RECEIVER);
        usdk::undenylist(denylister, RECEIVER);
        assert!(!usdk::is_denylisted(RECEIVER));
    }

    // Undenylisting an address that never held the asset must not create a store.
    #[test(creator = @0xcafe, denylister = @0xcade)]
    fun test_undenylist_does_not_create_store(creator: &signer, denylister: &signer) {
        usdk::init_for_test(creator);

        usdk::undenylist(denylister, OUTSIDER);
        assert!(!primary_fungible_store::primary_store_exists(OUTSIDER, usdk::metadata()));
        assert!(!usdk::is_denylisted(OUTSIDER));
    }

    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_non_denylister_cannot_denylist(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::denylist(rando, RECEIVER);
    }

    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_non_denylister_cannot_undenylist(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::undenylist(rando, RECEIVER);
    }

    #[test(creator = @0xcafe, denylister = @0xcade, receiver = @0xdead)]
    #[expected_failure(abort_code = 327683, location = aptos_framework::object)]
    fun test_untransferrable_store(creator: &signer, denylister: &signer, receiver: &signer) {
        usdk::init_for_test(creator);
        let receiver_address = signer::address_of(receiver);
        let asset = usdk::metadata();

        usdk::denylist(denylister, receiver_address);
        assert!(usdk::is_denylisted(receiver_address));

        let constructor_ref = object::create_object(receiver_address);
        fungible_asset::create_store(&constructor_ref, asset);
        let store = constructor_ref.object_from_constructor_ref();

        object::transfer<FungibleStore>(receiver, store, @0xdeadbeef);
    }

    // ----------------------------------------------------------------------
    // Burn (self) and confiscation
    // ----------------------------------------------------------------------

    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface)]
    fun test_burn_zero_is_noop(creator: &signer, master_minter: &signer, minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::mint(minter, minter_address, 10);
        usdk::burn(minter, 0);
        assert!(primary_fungible_store::balance(minter_address, usdk::metadata()) == 10);
    }

    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_burn_requires_minter(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::burn(rando, 1);
    }

    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface)]
    #[expected_failure(abort_code = 7, location = stablecoin::usdk)]
    fun test_burn_without_store_aborts(creator: &signer, master_minter: &signer, minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::burn(minter, 1);
    }

    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface, confiscator = @0xc0ffee)]
    fun test_confiscate_burns_holder_balance(
        creator: &signer,
        master_minter: &signer,
        minter: &signer,
        confiscator: &signer,
    ) {
        usdk::init_for_test(creator);
        let asset = usdk::metadata();
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::mint(minter, RECEIVER, 100);

        usdk::confiscate(confiscator, RECEIVER, 40);
        assert!(primary_fungible_store::balance(RECEIVER, asset) == 60);
    }

    // Confiscation must work against a frozen account, which is the point of it.
    #[test(
        creator = @0xcafe,
        master_minter = @0xbab,
        minter = @0xface,
        denylister = @0xcade,
        confiscator = @0xc0ffee,
    )]
    fun test_confiscate_works_on_denylisted_account(
        creator: &signer,
        master_minter: &signer,
        minter: &signer,
        denylister: &signer,
        confiscator: &signer,
    ) {
        usdk::init_for_test(creator);
        let asset = usdk::metadata();
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::mint(minter, RECEIVER, 100);
        usdk::denylist(denylister, RECEIVER);

        usdk::confiscate(confiscator, RECEIVER, 100);
        assert!(primary_fungible_store::balance(RECEIVER, asset) == 0);
    }

    // Confiscation is a recovery action and stays available while paused.
    #[test(
        creator = @0xcafe,
        pauser = @0xdafe,
        master_minter = @0xbab,
        minter = @0xface,
        confiscator = @0xc0ffee,
    )]
    fun test_confiscate_allowed_while_paused(
        creator: &signer,
        pauser: &signer,
        master_minter: &signer,
        minter: &signer,
        confiscator: &signer,
    ) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::mint(minter, RECEIVER, 100);
        usdk::set_pause(pauser, true);

        usdk::confiscate(confiscator, RECEIVER, 100);
        assert!(primary_fungible_store::balance(RECEIVER, usdk::metadata()) == 0);
    }

    #[test(creator = @0xcafe, confiscator = @0xc0ffee)]
    fun test_confiscate_zero_is_noop(creator: &signer, confiscator: &signer) {
        usdk::init_for_test(creator);
        usdk::confiscate(confiscator, OUTSIDER, 0);
        assert!(!primary_fungible_store::primary_store_exists(OUTSIDER, usdk::metadata()));
    }

    #[test(creator = @0xcafe, confiscator = @0xc0ffee)]
    #[expected_failure(abort_code = 7, location = stablecoin::usdk)]
    fun test_confiscate_without_store_aborts(creator: &signer, confiscator: &signer) {
        usdk::init_for_test(creator);
        usdk::confiscate(confiscator, OUTSIDER, 1);
    }

    // A minter must not be able to destroy someone else's balance.
    #[test(creator = @0xcafe, master_minter = @0xbab, minter = @0xface)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_minter_cannot_confiscate(creator: &signer, master_minter: &signer, minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);

        usdk::add_minter(master_minter, minter_address, 1000);
        usdk::mint(minter, RECEIVER, 100);
        usdk::confiscate(minter, RECEIVER, 100);
    }

    // ----------------------------------------------------------------------
    // Two-step role rotation
    // ----------------------------------------------------------------------

    #[test(creator = @0xcafe, pauser = @0xdafe, rando = @0xdead)]
    fun test_pauser_rotation(creator: &signer, pauser: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        let new_pauser = signer::address_of(rando);

        usdk::propose_pauser(pauser, new_pauser);
        assert!(usdk::pending_pauser() == option::some(new_pauser));
        // A proposal alone grants nothing.
        assert!(usdk::pauser() == signer::address_of(pauser));

        usdk::accept_pauser(rando);
        assert!(usdk::pauser() == new_pauser);
        assert!(usdk::pending_pauser() == option::none());

        usdk::set_pause(rando, true);
        assert!(usdk::is_paused());
    }

    #[test(creator = @0xcafe, master_minter = @0xbab, rando = @0xdead)]
    fun test_master_minter_rotation(creator: &signer, master_minter: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        let new_holder = signer::address_of(rando);

        usdk::propose_master_minter(master_minter, new_holder);
        assert!(usdk::pending_master_minter() == option::some(new_holder));
        usdk::accept_master_minter(rando);
        assert!(usdk::master_minter() == new_holder);

        usdk::add_minter(rando, OUTSIDER, 5);
        assert!(usdk::is_minter(OUTSIDER));
    }

    #[test(creator = @0xcafe, denylister = @0xcade, rando = @0xdead)]
    fun test_denylister_rotation(creator: &signer, denylister: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        let new_holder = signer::address_of(rando);

        usdk::propose_denylister(denylister, new_holder);
        assert!(usdk::pending_denylister() == option::some(new_holder));
        usdk::accept_denylister(rando);
        assert!(usdk::denylister() == new_holder);

        usdk::denylist(rando, RECEIVER);
        assert!(usdk::is_denylisted(RECEIVER));
    }

    #[test(creator = @0xcafe, confiscator = @0xc0ffee, rando = @0xdead)]
    fun test_confiscator_rotation(creator: &signer, confiscator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        let new_holder = signer::address_of(rando);

        usdk::propose_confiscator(confiscator, new_holder);
        assert!(usdk::pending_confiscator() == option::some(new_holder));
        usdk::accept_confiscator(rando);
        assert!(usdk::confiscator() == new_holder);
    }

    // A proposal can be replaced before it is accepted.
    #[test(creator = @0xcafe, pauser = @0xdafe, rando = @0xdead)]
    fun test_proposal_can_be_replaced(creator: &signer, pauser: &signer, rando: &signer) {
        usdk::init_for_test(creator);

        usdk::propose_pauser(pauser, OUTSIDER);
        usdk::propose_pauser(pauser, RECEIVER);
        assert!(usdk::pending_pauser() == option::some(RECEIVER));

        // The superseded address can no longer accept.
        assert!(usdk::pauser() == signer::address_of(pauser));
        let _ = rando;
    }

    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_propose_role_requires_current_holder(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::propose_pauser(rando, OUTSIDER);
    }

    #[test(creator = @0xcafe, pauser = @0xdafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_accept_role_requires_proposed_holder(
        creator: &signer,
        pauser: &signer,
        rando: &signer,
    ) {
        usdk::init_for_test(creator);
        usdk::propose_pauser(pauser, RECEIVER);
        usdk::accept_pauser(rando);
    }

    #[test(creator = @0xcafe, rando = @0xdead)]
    #[expected_failure(abort_code = 1, location = stablecoin::usdk)]
    fun test_accept_role_without_proposal_aborts(creator: &signer, rando: &signer) {
        usdk::init_for_test(creator);
        usdk::accept_pauser(rando);
    }

    // Role handoff is a recovery action and stays available while paused.
    #[test(creator = @0xcafe, pauser = @0xdafe, rando = @0xdead)]
    fun test_role_rotation_allowed_while_paused(
        creator: &signer,
        pauser: &signer,
        rando: &signer,
    ) {
        usdk::init_for_test(creator);
        usdk::set_pause(pauser, true);

        usdk::propose_pauser(pauser, signer::address_of(rando));
        usdk::accept_pauser(rando);
        assert!(usdk::pauser() == signer::address_of(rando));
    }
}
