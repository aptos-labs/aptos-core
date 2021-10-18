script {
    use Sender::NFT;

    // Renamed from Type to NFTType to prevent Type name collision
    // in generated typescript.
    fun initialize_nft<NFTType: store + drop>(account: signer) {
      NFT::initialize<NFTType>(&account);
    }
}
