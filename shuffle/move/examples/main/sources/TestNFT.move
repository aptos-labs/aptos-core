module Sender::TestNFT {
    use Sender::NFT;

    struct TestNFT has drop, store {}

    // warning: Can only create one TestNFT per account bc of 1 resource per
    // account constraint
    // TODO: convert to vec
    public(script) fun create_nft(account: signer, content_uri: vector<u8>) {
      NFT::initialize_<TestNFT>(&account); // assumes account is sender/creator/ADMIN
      let token = TestNFT{};
      let instance = NFT::create<TestNFT>(
        &account,
        token,
        content_uri,
      );
      NFT::publish(&account, instance);
    }
}
