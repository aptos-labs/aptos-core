module token_minter::collection_helper {

    use std::option;
    use std::option::Option;
    use std::string::String;
    use aptos_framework::object::ConstructorRef;

    use aptos_token_objects::collection;
    use aptos_token_objects::royalty::Royalty;

    friend token_minter::token_minter;

    public(friend) fun create_collection(
        object_signer: &signer,
        description: String,
        max_supply: Option<u64>,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        if (option::is_some(&max_supply)) {
            collection::create_fixed_collection(
                object_signer,
                description,
                option::extract(&mut max_supply),
                name,
                royalty,
                uri,
            )
        } else {
            collection::create_unlimited_collection(
                object_signer,
                description,
                name,
                royalty,
                uri,
            )
        }
    }
}
