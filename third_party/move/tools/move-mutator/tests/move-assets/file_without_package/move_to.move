module 0x2::Collection {
    struct Item has store {}

    struct Collection has key {
        item: Item
    }

    public fun start_collection(account: &signer) {
        move_to<Collection>(account, Collection {
            item: Item {}
        });

    }

}