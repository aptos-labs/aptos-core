script {
    use aptos_token::token::{Self, transfer};
    use std::string::String;

    fun main(
        from: &signer,
        creator: address,
        collection_name: String,
        token_name: String,
        token_property_version: u64,
        to: address,
        amount: u64,
    ) {
        let token_id = token::create_token_id_raw(creator, collection_name, token_name, token_property_version);
        transfer(from, token_id, to, amount);
    }
}