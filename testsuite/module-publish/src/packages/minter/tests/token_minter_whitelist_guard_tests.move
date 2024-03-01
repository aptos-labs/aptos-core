#[test_only]
module token_minter::token_minter_with_guards_test {
    use std::bcs;
    use std::signer;
    use std::string;
    use std::string::String;

    use token_minter::token_minter;
    use token_minter::token_minter_utils;
    use token_minter::whitelist;

    #[test(creator = @0x123, user = @0x456)]
    fun test_add_whitelist_guard_and_mint(creator: &signer, user: &signer) {
        let whitelist_amount = 2;
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        let whitelisted_addrs = vector[signer::address_of(creator), signer::address_of(user)];
        let whitelisted_amounts_per_user = vector[whitelist_amount, whitelist_amount];
        token_minter::add_or_update_whitelist(creator, token_minter, whitelisted_addrs, whitelisted_amounts_per_user);

        // Whitelist should be enabled as we added whitelisted addresses.
        assert!(whitelist::is_whitelist_enabled(token_minter), 0);
        // Assert whitelisted allowances
        assert!(whitelist::allowance(token_minter, signer::address_of(creator)) == whitelist_amount, 0);
        assert!(whitelist::allowance(token_minter, signer::address_of(user)) == whitelist_amount, 0);

        let previous_minted_amount = token_minter::tokens_minted(token_minter);
        let user_addr = signer::address_of(user);
        token_minter::mint_tokens(
            user,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            whitelist_amount,
            vector[vector<String>[string::utf8(b"attack")], vector<String>[string::utf8(b"defense")]],
            vector[vector<String>[string::utf8(b"u64")], vector<String>[string::utf8(b"u64")]],
            vector[vector[bcs::to_bytes<u64>(&20)], vector[bcs::to_bytes<u64>(&10)]],
            vector[user_addr, user_addr],
        );

        // User should mint 2 tokens as user was whitelisted.
        assert!(token_minter::tokens_minted(token_minter) == previous_minted_amount + whitelist_amount, 0);
    }

    #[test(creator = @0x123, user = @0x456)]
    #[expected_failure(abort_code = 0x060002, location = token_minter::whitelist)]
    fun test_user_not_whitelisted(creator: &signer, user: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        token_minter::add_or_update_whitelist(creator, token_minter, vector[signer::address_of(creator)], vector[2]);

        let user_addr = signer::address_of(user);
        token_minter::mint_tokens(
            user,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            2,
            vector[vector<String>[string::utf8(b"attack")], vector<String>[string::utf8(b"defense")]],
            vector[vector<String>[string::utf8(b"u64")], vector<String>[string::utf8(b"u64")]],
            vector[vector[bcs::to_bytes<u64>(&20)], vector[bcs::to_bytes<u64>(&10)]],
            vector[user_addr, user_addr],
        );
    }

    #[test(creator = @0x123, user = @0x456)]
    #[expected_failure(abort_code = 0x030004, location = token_minter::whitelist)]
    fun test_set_whitelist_allowance_to_zero_and_mint(creator: &signer, user: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        let whitelisted_addrs = vector[signer::address_of(creator), signer::address_of(user)];
        let whitelisted_amounts_per_user = vector[2, 2];
        // Create and add whitelist members
        token_minter::add_or_update_whitelist(creator, token_minter, whitelisted_addrs, whitelisted_amounts_per_user);

        // Decrease whitelisted amount for user to zero
        token_minter::add_or_update_whitelist(creator, token_minter, whitelisted_addrs, vector[0, 0]);

        // Attempt to mint after zero whitelisted amount, expect abort
        let user_addr = signer::address_of(user);
        token_minter::mint_tokens(
            user,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            2,
            vector[vector<String>[string::utf8(b"attack")], vector<String>[string::utf8(b"defense")]],
            vector[vector<String>[string::utf8(b"u64")], vector<String>[string::utf8(b"u64")]],
            vector[vector[bcs::to_bytes<u64>(&20)], vector[bcs::to_bytes<u64>(&10)]],
            vector[user_addr, user_addr],
        );
    }

    #[test(creator = @0x123, user = @0x456)]
    #[expected_failure(abort_code = 0x030004, location = token_minter::whitelist)]
    fun test_abort_when_mint_more_than_whitelist_allowance(creator: &signer, user: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        let whitelisted_addrs = vector[signer::address_of(creator), signer::address_of(user)];
        let whitelisted_amount = 1;
        let whitelisted_amounts_per_user = vector[whitelisted_amount, whitelisted_amount];
        token_minter::add_or_update_whitelist(creator, token_minter, whitelisted_addrs, whitelisted_amounts_per_user);

        // Attempting to mint more than allowed
        let user_addr = signer::address_of(user);
        token_minter::mint_tokens(
            user,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            whitelisted_amount + 1,
            vector[vector<String>[string::utf8(b"attack")], vector<String>[string::utf8(b"defense")]],
            vector[vector<String>[string::utf8(b"u64")], vector<String>[string::utf8(b"u64")]],
            vector[vector[bcs::to_bytes<u64>(&20)], vector[bcs::to_bytes<u64>(&10)]],
            vector[user_addr, user_addr],
        );
    }

    #[test(creator = @0x123, user = @0x456)]
    fun test_mint_success_when_whitelist_guard_removed(creator: &signer, user: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        let whitelisted_addrs = vector[signer::address_of(creator), signer::address_of(user)];
        let whitelisted_amounts_per_user = vector[2, 2];
        token_minter::add_or_update_whitelist(creator, token_minter, whitelisted_addrs, whitelisted_amounts_per_user);

        token_minter::remove_whitelist_guard(creator, token_minter);

        // Attempting to mint more than allowed
        let user_addr = signer::address_of(user);
        token_minter::mint_tokens(
            user,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            2,
            vector[vector<String>[string::utf8(b"attack")], vector<String>[string::utf8(b"defense")]],
            vector[vector<String>[string::utf8(b"u64")], vector<String>[string::utf8(b"u64")]],
            vector[vector[bcs::to_bytes<u64>(&20)], vector[bcs::to_bytes<u64>(&10)]],
            vector[user_addr, user_addr],
        );
    }

    #[test(creator = @0x123, user = @0x456)]
    fun test_partial_mint_whitelist_allowance(creator: &signer, user: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        let whitelisted_addrs = vector[signer::address_of(creator), signer::address_of(user)];
        let whitelisted_amount = 2;
        let whitelisted_amounts_per_user = vector[whitelisted_amount, whitelisted_amount];
        token_minter::add_or_update_whitelist(creator, token_minter, whitelisted_addrs, whitelisted_amounts_per_user);

        let amount = 1;
        token_minter::mint_tokens(
            user,
            token_minter,
            string::utf8(b"TestToken"),
            string::utf8(b"Token desc"),
            string::utf8(b"http://test.token.uri"),
            amount,
            vector[vector<String>[string::utf8(b"attack")]],
            vector[vector<String>[string::utf8(b"u64")]],
            vector[vector[bcs::to_bytes<u64>(&20)]],
            vector[signer::address_of(user)],
        );

        assert!(whitelist::allowance(token_minter, signer::address_of(user)) == whitelisted_amount - amount, 0);
    }

    #[test(creator = @0x123, user = @0x456)]
    #[expected_failure(abort_code = 0x010006, location = token_minter::token_minter)]
    fun test_non_creator_updating_whitelist(creator: &signer, user: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        token_minter::add_or_update_whitelist(user, token_minter, vector[], vector[]);
    }

    #[test(creator = @0x123, user = @0x456)]
    #[expected_failure(abort_code = 0x010006, location = token_minter::token_minter)]
    fun test_non_creator_removing_whitelist(creator: &signer, user: &signer) {
        let token_minter = token_minter_utils::init_token_minter_object_and_collection(creator, false, false);
        token_minter::remove_whitelist_guard(user, token_minter);
    }
}
