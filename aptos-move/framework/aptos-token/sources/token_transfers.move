/// This module provides the foundation for transferring of Tokens
module aptos_token::token_transfers {
    use std::signer;
    use std::string::String;
    use aptos_std::table_with_length::{Self, TableWithLength};
    use aptos_token::token::{Self, Token, TokenId};

    struct TokenTransfers has key {
        pending_claims: TableWithLength<address, TableWithLength<TokenId, Token>>,
    }

    fun initialize_token_transfers(account: &signer) {
        move_to(
            account,
            TokenTransfers {
                pending_claims: table_with_length::new<address, TableWithLength<TokenId, Token>>(),
            }
        )
    }

    public entry fun offer_script(
        sender: signer,
        receiver: address,
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
        amount: u64,
    ) acquires TokenTransfers {
        let token_id = token::create_token_id_raw(creator, collection, name, property_version);
        offer(&sender, receiver, token_id, amount);
    }

    // Make an entry into pending transfers and extract from gallery
    public fun offer(
        sender: &signer,
        receiver: address,
        token_id: TokenId,
        amount: u64,
    ) acquires TokenTransfers {
        let sender_addr = signer::address_of(sender);
        if (!exists<TokenTransfers>(sender_addr)) {
            initialize_token_transfers(sender)
        };

        let pending_claims =
            &mut borrow_global_mut<TokenTransfers>(sender_addr).pending_claims;
        if (!table_with_length::contains(pending_claims, receiver)) {
            table_with_length::add(pending_claims, receiver, table_with_length::new())
        };
        let addr_pending_claims = table_with_length::borrow_mut(pending_claims, receiver);

        let token = token::withdraw_token(sender, token_id, amount);
        let token_id = *token::token_id(&token);
        if (table_with_length::contains(addr_pending_claims, token_id)) {
            let dst_token = table_with_length::borrow_mut(addr_pending_claims, token_id);
            token::merge(dst_token, token)
        } else {
            table_with_length::add(addr_pending_claims, token_id, token)
        }
    }

    public entry fun claim_script(
        receiver: signer,
        sender: address,
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
    ) acquires TokenTransfers {
        let token_id = token::create_token_id_raw(creator, collection, name, property_version);
        claim(&receiver, sender, token_id);
    }

    // Pull from someone else's pending transfers and insert into our gallery
    public fun claim(
        receiver: &signer,
        sender: address,
        token_id: TokenId,
    ) acquires TokenTransfers {
        let receiver_addr = signer::address_of(receiver);
        let pending_claims =
            &mut borrow_global_mut<TokenTransfers>(sender).pending_claims;
        let pending_tokens = table_with_length::borrow_mut(pending_claims, receiver_addr);
        let token = table_with_length::remove(pending_tokens, token_id);

        if (table_with_length::length(pending_tokens) == 0) {
            let real_pending_claims = table_with_length::remove(pending_claims, receiver_addr);
            table_with_length::destroy_empty(real_pending_claims)
        };

        token::deposit_token(receiver, token)
    }

    public entry fun cancel_offer_script(
        sender: signer,
        receiver: address,
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
    ) acquires TokenTransfers {
        let token_id = token::create_token_id_raw(creator, collection, name, property_version);
        cancel_offer(&sender, receiver, token_id);
    }

    // Extra from our pending_claims and return to gallery
    public fun cancel_offer(
        sender: &signer,
        receiver: address,
        token_id: TokenId,
    ) acquires TokenTransfers {
        let sender_addr = signer::address_of(sender);
        let pending_claims =
            &mut borrow_global_mut<TokenTransfers>(sender_addr).pending_claims;
        let pending_tokens = table_with_length::borrow_mut(pending_claims, receiver);
        let token = table_with_length::remove(pending_tokens, token_id);

        if (table_with_length::length(pending_tokens) == 0) {
            let real_pending_claims = table_with_length::remove(pending_claims, receiver);
            table_with_length::destroy_empty(real_pending_claims)
        };

        token::deposit_token(sender, token)
    }

    #[test(creator = @0x1, owner = @0x2)]
    public fun test_nft(creator: signer, owner: signer) acquires TokenTransfers {
        let token_id = create_token(&creator, 1);

        let creator_addr = signer::address_of(&creator);
        let owner_addr = signer::address_of(&owner);
        offer(&creator, owner_addr, token_id, 1);
        claim(&owner, creator_addr, token_id);

        offer(&owner, creator_addr, token_id, 1);
        cancel_offer(&owner, creator_addr, token_id);
    }

    #[test(creator = @0x1, owner0 = @0x2, owner1 = @0x3)]
    public fun test_editions(
        creator: signer,
        owner0: signer,
        owner1: signer,
    ) acquires TokenTransfers {
        let token_id = create_token(&creator, 2);

        let creator_addr = signer::address_of(&creator);
        let owner0_addr = signer::address_of(&owner0);
        let owner1_addr = signer::address_of(&owner1);

        offer(&creator, owner0_addr, token_id, 1);
        assert!(table_with_length::length(&borrow_global<TokenTransfers>(creator_addr).pending_claims) == 1, 1);
        offer(&creator, owner1_addr, token_id, 1);
        assert!(table_with_length::length(&borrow_global<TokenTransfers>(creator_addr).pending_claims) == 2, 2);
        claim(&owner0, creator_addr, token_id);
        assert!(table_with_length::length(&borrow_global<TokenTransfers>(creator_addr).pending_claims) == 1, 3);
        claim(&owner1, creator_addr, token_id);
        assert!(table_with_length::length(&borrow_global<TokenTransfers>(creator_addr).pending_claims) == 0, 4);

        offer(&owner0, owner1_addr, token_id, 1);
        claim(&owner1, owner0_addr, token_id);

        offer(&owner1, creator_addr, token_id, 1);
        offer(&owner1, creator_addr, token_id, 1);
        claim(&creator, owner1_addr, token_id);
    }

    #[test_only]
    public entry fun create_token(creator: &signer, amount: u64): TokenId {
        use std::string::{Self, String};

        let collection_name = string::utf8(b"Hello, World");
        let collection_mutation_setting = vector<bool>[false, false, false];

        token::create_collection(
            creator,
            *&collection_name,
            string::utf8(b"Collection: Hello, World"),
            string::utf8(b"https://aptos.dev"),
            1,
            collection_mutation_setting,
        );

        let token_mutation_setting = vector<bool>[false, false, false, false, true];
        let default_keys = vector<String>[string::utf8(b"attack"), string::utf8(b"num_of_use")];
        let default_vals = vector<vector<u8>>[b"10", b"5"];
        let default_types = vector<String>[string::utf8(b"integer"), string::utf8(b"integer")];
        token::create_token_script(
            creator,
            *&collection_name,

            string::utf8(b"Token: Hello, Token"),
            string::utf8(b"Hello, Token"),
            amount,
            amount,
            string::utf8(b"https://aptos.dev"),
            signer::address_of(creator),
            100,
            0,
            token_mutation_setting,
            default_keys,
            default_vals,
            default_types,
        );
        token::create_token_id_raw(
            signer::address_of(creator),
            *&collection_name,
            string::utf8(b"Token: Hello, Token"),
            0
        )
    }
}
