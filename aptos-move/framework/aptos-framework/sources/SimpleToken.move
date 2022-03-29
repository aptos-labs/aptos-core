/// This exists to demonstrate how one could define their own TokenMetadata
module AptosFramework::SimpleToken {
    use AptosFramework::Token;

    struct SimpleToken has copy, drop, store {
        magic_number: u64,
    }

    // Create a single Token
    public(script) fun create_simple_token(
        account: signer,
        collection_name: vector<u8>,
        description: vector<u8>,
        name: vector<u8>,
        supply: u64,
        uri: vector<u8>,
    ) {
      Token::create_token_with_metadata_script<SimpleToken>(
          account,
          collection_name,
          description,
          name,
          supply,
          uri,
          SimpleToken { magic_number: 42 },
      );
    }
}
