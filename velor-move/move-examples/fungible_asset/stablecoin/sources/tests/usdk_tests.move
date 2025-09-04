#[test_only]
module stablecoin::usdk_tests {
    use std::signer;
    use velor_framework::primary_fungible_store;
    use velor_framework::dispatchable_fungible_asset;
    use velor_framework::fungible_asset::{Self, FungibleStore};
    use stablecoin::usdk;
    use velor_framework::object;

    #[test(creator = @0xcafe, minter = @0xface, master_minter = @0xbab, denylister = @0xcade)]
    fun test_basic_flow(creator: &signer, minter: &signer, master_minter: &signer, denylister: &signer) {
        usdk::init_for_test(creator);
        let receiver_address = @0xcafe1;
        let minter_address = signer::address_of(minter);

        // set minter and have minter call mint, check balance
        usdk::add_minter(master_minter, minter_address);
        usdk::mint(minter, minter_address, 100);
        let asset = usdk::metadata();
        assert!(primary_fungible_store::balance(minter_address, asset) == 100, 0);

        // transfer from minter to receiver, check balance
        let minter_store = primary_fungible_store::ensure_primary_store_exists(minter_address, asset);
        let receiver_store = primary_fungible_store::ensure_primary_store_exists(receiver_address, asset);
        dispatchable_fungible_asset::transfer(minter, minter_store, receiver_store, 10);

        // denylist account, check if account is denylisted
        usdk::denylist(denylister, receiver_address);
        assert!(primary_fungible_store::is_frozen(receiver_address, asset), 0);
        usdk::undenylist(denylister, receiver_address);
        assert!(!primary_fungible_store::is_frozen(receiver_address, asset), 0);

        // burn tokens, check balance
        usdk::burn(minter, minter_address, 90);
        assert!(primary_fungible_store::balance(minter_address, asset) == 0, 0);
    }


    #[test(creator = @0xcafe, pauser = @0xdafe, minter = @0xface, master_minter = @0xbab)]
    #[expected_failure(abort_code = 2, location = stablecoin::usdk)]
    fun test_pause(creator: &signer, pauser: &signer, minter: &signer, master_minter: &signer) {
        usdk::init_for_test(creator);
        let minter_address = signer::address_of(minter);
        usdk::set_pause(pauser, true);
        usdk::add_minter(master_minter, minter_address);
    }

    // test the ability of a denylisted account to transfer out newly created store
    #[test(creator = @0xcafe, denylister = @0xcade, receiver = @0xdead)]
    #[expected_failure(abort_code = 327683, location = velor_framework::object)]
    fun test_untransferrable_store(creator: &signer, denylister: &signer, receiver: &signer) {
        usdk::init_for_test(creator);
        let receiver_address = signer::address_of(receiver);
        let asset = usdk::metadata();

        usdk::denylist(denylister, receiver_address);
        assert!(primary_fungible_store::is_frozen(receiver_address, asset), 0);

        let constructor_ref = object::create_object(receiver_address);
        fungible_asset::create_store(&constructor_ref, asset);
        let store = object::object_from_constructor_ref<FungibleStore>(&constructor_ref);

        object::transfer(receiver, store, @0xdeadbeef);
    }
}
