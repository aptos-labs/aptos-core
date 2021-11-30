module Sender::TestNFT {
    use Sender::NFTStandard;
    use Std::Signer;

    struct TestNFT has drop, store {}

    public(script) fun create_nft(account: signer, content_uri: vector<u8>) {
        NFTStandard::initialize<TestNFT>(&account); // assumes account is sender/creator/ADMIN
        let token = TestNFT{};
        let instance = NFTStandard::create<TestNFT>(
            &account,
            token,
            content_uri,
        );
        NFTStandard::add(Signer::address_of(&account), instance);
    }
}
