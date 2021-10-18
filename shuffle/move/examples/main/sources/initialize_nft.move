script {
    use Sender::NFT;

    // Example script that initializes that particular NFT Type. Used instead
    // of genesis initialize. There is also a work around shown in
    // TestNFT::create script function.
    // Renamed from Type to NFTType to prevent Type name collision
    // in generated typescript.
    fun initialize_nft<NFTType: store + drop>(account: signer) {
      NFT::initialize<NFTType>(&account);
    }
}
