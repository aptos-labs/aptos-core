module token_minter::token_helper {

    use std::error;
    use std::string::String;
    use std::vector;
    use aptos_framework::object;
    use aptos_framework::object::{ConstructorRef, Object};

    use aptos_token_objects::token::Token;

    friend token_minter::token_minter;

    /// The property keys, types, and values for minting do not match
    const EMINT_PROPERTIES_ARGUMENT_MISMATCH: u64 = 1;

    public(friend) fun validate_token_properties(
        amount: u64,
        property_keys: &vector<vector<String>>,
        property_types: &vector<vector<String>>,
        property_values: &vector<vector<vector<u8>>>,
        recipient_addrs: &vector<address>,
    ) {
        assert!(
            vector::length(property_keys) == amount
                && vector::length(property_types) == amount
                && vector::length(property_values) == amount
                && vector::length(recipient_addrs) == amount,
            error::invalid_argument(EMINT_PROPERTIES_ARGUMENT_MISMATCH),
        );
    }

    public(friend) fun transfer_token(
        owner: &signer,
        to: address,
        soulbound: bool,
        token_constructor_ref: &ConstructorRef,
    ): Object<Token> {
        let token = object::object_from_constructor_ref(token_constructor_ref);
        if (soulbound) {
            transfer_soulbound_token(to, token_constructor_ref);
        } else {
            object::transfer(owner, token, to);
        };

        token
    }

    fun transfer_soulbound_token(to: address, token_constructor_ref: &ConstructorRef) {
        let transfer_ref = &object::generate_transfer_ref(token_constructor_ref);
        let linear_transfer_ref = object::generate_linear_transfer_ref(transfer_ref);
        object::transfer_with_ref(linear_transfer_ref, to);
        object::disable_ungated_transfer(transfer_ref);
    }
}
