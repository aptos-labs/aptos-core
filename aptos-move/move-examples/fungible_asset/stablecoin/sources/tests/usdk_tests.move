#[test_only]
module stablecoin::usdk_tests {
    use std::signer;
    use aptos_framework::primary_fungible_store;
    use stablecoin::usdk;

    #[test(creator = @0xcafe)]
    fun test_basic_flow(creator: &signer) {
        usdk::init_for_test(creator);
        let creator_address = signer::address_of(creator);
        let receiver_address = @0xcafe1;

        usdk::mint(creator, creator_address, 100);
        let asset = usdk::metadata();
        assert!(primary_fungible_store::balance(creator_address, asset) == 100, 0);
        primary_fungible_store::transfer(creator, asset, receiver_address, 10);
        assert!(primary_fungible_store::balance(receiver_address, asset) == 10, 0);

        usdk::blacklist(creator, creator_address);
        assert!(primary_fungible_store::is_frozen(creator_address, asset), 0);
        usdk::unblacklist(creator, creator_address);
        assert!(!primary_fungible_store::is_frozen(creator_address, asset), 0);
        usdk::burn(creator, creator_address, 90);
    }
}
