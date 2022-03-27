/// This exists to provide convenient access to Tokens within Aptos for folks that do not want
/// additional features within the metadata.
module AptosFramework::SimpleToken {
    use AptosFramework::Token;
    use AptosFramework::TokenTransfers;

    struct NoMetadata has copy, drop, store { }

    // Create a single Token
    public(script) fun create_simple_token(
        account: signer,
        collection_name: vector<u8>,
        description: vector<u8>,
        name: vector<u8>,
        supply: u64,
        uri: vector<u8>,
    ) {
      Token::create_token_script<NoMetadata>(
          account,
          collection_name,
          description,
          name,
          supply,
          uri,
          NoMetadata { },
      );
    }

    // Creates a collection with a bounded number of tokens in it
    public(script) fun create_finite_simple_collection(
        account: signer,
        description: vector<u8>,
        name: vector<u8>,
        uri: vector<u8>,
        maximum: u64,
    ) {
        Token::create_finite_collection_script<NoMetadata>(
            account,
            description,
            name,
            uri,
            maximum,
        );
    }

    // Creates a collection with a unbounded number of tokens in it
    public(script) fun create_unlimited_simple_collection(
        account: signer,
        description: vector<u8>,
        name: vector<u8>,
        uri: vector<u8>,
    ) {
        Token::create_unlimited_collection_script<NoMetadata>(account, description, name, uri);
    }

    // Make the token available to the receipient to claim
    public(script) fun transfer_simple_token_to(
        sender: signer,
        receiver: address,
        creator: address,
        token_creation_num: u64,
        amount: u64,
    ) {
        TokenTransfers::transfer_to_script<NoMetadata>(
            sender,
            receiver,
            creator,
            token_creation_num,
            amount,
        );
    }

    // Claim an offered token
   public(script) fun receive_simple_token_from(
        receiver: signer,
        sender: address,
        creator: address,
        token_creation_num: u64,
    ) {
        TokenTransfers::receive_from_script<NoMetadata>(
            receiver,
            sender,
            creator,
            token_creation_num,
        );
    }

    // Retrieve the offered token and return it to the gallery
    public(script) fun stop_simple_token_transfer_to(
        sender: signer,
        receiver: address,
        creator: address,
        token_creation_num: u64,
    ) {
        TokenTransfers::stop_transfer_to_script<NoMetadata>(
            sender,
            receiver,
            creator,
            token_creation_num,
        );
    }
}
