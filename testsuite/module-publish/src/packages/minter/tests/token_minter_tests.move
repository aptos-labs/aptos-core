#[test_only]
module token_minter::token_minter_tests {
    use std::bcs;
    use std::option;
    use std::signer;
    use std::string;
    use std::string::String;
    use std::vector;
    use aptos_framework::object;

    use aptos_token_objects::royalty;
    use aptos_token_objects::token;

    use token_minter::token_minter;
    use token_minter::token_minter_utils;

    #[test(creator = @0x123, user = @0x456)]
    /// When public mint is enabled, anyone can mint tokens.
    fun test_mint_public_mint(creator: &signer, user: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        // Assert creator owns the token minter.
        assert!(object::owns(token_minter, signer::address_of(creator)), 0);
        // Assert creator owns the collection.
        let collection = token_minter::collection(token_minter);
        assert!(object::owns(collection, signer::address_of(creator)), 0);

        let previous_minted_amount = token_minter::tokens_minted(token_minter);
        let amount = 1;

        let tokens = &token_minter::mint_tokens_object(
            user,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            amount,
            vector[vector<String>[string::utf8(b"attack"), string::utf8(b"num_of_use")]],
            vector[vector<String>[string::utf8(b"u64"), string::utf8(b"u64")]],
            vector[vector<vector<u8>>[bcs::to_bytes<u64>(&10), bcs::to_bytes<u64>(&5)]],
            vector[signer::address_of(creator)],
        );

        let i = 0;
        while (i < vector::length(tokens)) {
            // Assert `allow_ungated_transfer` is true, i.e. not soulbound
            assert!(object::ungated_transfer_allowed(*vector::borrow(tokens, i)), 0);
            i = i + 1;
        };

        // No guards added, so we expect a seamless mint.
        assert!(token_minter::tokens_minted(token_minter) == previous_minted_amount + amount, 0);

        // Test royalty object creation
        let expected_royalty = royalty::create(500, 10000, signer::address_of(creator));
        assert!(option::some(expected_royalty) == royalty::get(collection), 0);
    }

    #[test(creator = @0x123, user = @0x456)]
    #[expected_failure(abort_code = 0x010006, location = token_minter::token_minter)]
    fun test_mint_token_when_not_creator(creator: &signer, user: &signer) {
        // Set creator mint to true, so when `user` mints, it reverts
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, true, false);

        token_minter::mint_tokens_object(
            user,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            1,
            vector[vector<String>[string::utf8(b"attack"), string::utf8(b"num_of_use")]],
            vector[vector<String>[string::utf8(b"u64"), string::utf8(b"u64")]],
            vector[vector<vector<u8>>[bcs::to_bytes<u64>(&10), bcs::to_bytes<u64>(&5)]],
            vector[signer::address_of(creator)],
        );
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x030004, location = token_minter::token_minter)]
    fun test_mint_when_paused(creator: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        // Set `paused` to true, so minting should fail
        token_minter::set_paused(creator, token_minter, true);

        token_minter::mint_tokens_object(
            creator,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            1,
            vector[vector<String>[string::utf8(b"attack"), string::utf8(b"num_of_use")]],
            vector[vector<String>[string::utf8(b"u64"), string::utf8(b"u64")]],
            vector[vector<vector<u8>>[bcs::to_bytes<u64>(&10), bcs::to_bytes<u64>(&5)]],
            vector[signer::address_of(creator)],
        );
    }

    #[test(creator = @0x123)]
    fun test_mint_soulbound(creator: &signer) {
        // Set `soulbound` to true, this will mint soulbound NFTs.
        let soulbound = true;
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, soulbound);

        let tokens = &token_minter::mint_tokens_object(
            creator,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            1,
            vector[vector<String>[string::utf8(b"attack"), string::utf8(b"num_of_use")]],
            vector[vector<String>[string::utf8(b"u64"), string::utf8(b"u64")]],
            vector[vector<vector<u8>>[bcs::to_bytes<u64>(&10), bcs::to_bytes<u64>(&5)]],
            vector[signer::address_of(creator)],
        );

        let i = 0;
        while (i < vector::length(tokens)) {
            // Assert `allow_ungated_transfer` is false for all minted tokens.
            assert!(!object::ungated_transfer_allowed(*vector::borrow(tokens, i)), 0);
            i = i + 1;
        }
    }

    #[test(creator = @0x123)]
    fun test_destroy_token_minter(creator: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        token_minter::destroy_token_minter(creator, token_minter);
    }

    #[test(creator = @0x123)]
    fun test_set_token_description(creator: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        let tokens = &token_minter::mint_tokens_object(
            creator,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            1,
            vector[vector<String>[string::utf8(b"attack"), string::utf8(b"num_of_use")]],
            vector[vector<String>[string::utf8(b"u64"), string::utf8(b"u64")]],
            vector[vector<vector<u8>>[bcs::to_bytes<u64>(&10), bcs::to_bytes<u64>(&5)]],
            vector[signer::address_of(creator)],
        );
        let minted_token = *vector::borrow(tokens, 0);
        token_minter::set_token_description<token::Token>(creator, minted_token, string::utf8(b"UpdatedTestToken"));
        let minted_token = *vector::borrow(tokens, 0);
        assert!(token::description(minted_token) == string::utf8(b"UpdatedTestToken"), 0);
    }

    #[test(creator = @0x123, user = @0x456)]
    #[expected_failure(abort_code = 0x010006, location = token_minter::token_minter)]
    fun test_destroy_token_minter_fails_when_not_creator(creator: &signer, user: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        token_minter::destroy_token_minter(user, token_minter);
    }
}
