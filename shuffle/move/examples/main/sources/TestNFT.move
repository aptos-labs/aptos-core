module Sender::TestNFT {
    struct TestNFT has drop, store {}
    public fun new_test_nft(): TestNFT {
        TestNFT{}
    }
}
