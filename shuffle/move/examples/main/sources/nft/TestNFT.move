module Sender::TestNFT {
    use Sender::NFT;
    use Std::Signer;

    struct TestNFT has drop, store {}

    public(script) fun create_nft(account: signer, content_uri: vector<u8>) {
        NFT::initialize<TestNFT>(&account); // assumes account is sender/creator/ADMIN
        let token = TestNFT{};
        let instance = NFT::create<TestNFT>(
            &account,
            token,
            content_uri,
        );
        NFT::add(Signer::address_of(&account), instance);
    }
}
