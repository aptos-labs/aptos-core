/// This module provides the foundation for transferring of Tokens
module aptos_framework::token_transfers {
    use std::signer;
    use aptos_std::table::{Self, Table};
    use aptos_framework::token::{Self, Token, TokenId};

    struct TokenTransfers has key {
        pending_claims: Table<address, Table<TokenId, Token>>,
    }

    fun initialize_token_transfers(account: &signer) {
        move_to(
            account,
            TokenTransfers {
                pending_claims: table::new<address, Table<TokenId, Token>>(),
            }
        )
    }

    public entry fun offer_script(
        sender: signer,
        receiver: address,
        creator: address,
        collection: vector<u8>,
        name: vector<u8>,
        amount: u64,
    ) acquires TokenTransfers {
        let token_id = token::create_token_id_raw(creator, collection, name);
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
        if (!table::contains(pending_claims, receiver)) {
            table::add(pending_claims, receiver, table::new())
        };
        let addr_pending_claims = table::borrow_mut(pending_claims, receiver);

        let token = token::withdraw_token(sender, token_id, amount);
        let token_id = *token::token_id(&token);
        if (table::contains(addr_pending_claims, token_id)) {
            let dst_token = table::borrow_mut(addr_pending_claims, token_id);
            token::merge(dst_token, token)
        } else {
            table::add(addr_pending_claims, token_id, token)
        }
    }

    public entry fun claim_script(
        receiver: signer,
        sender: address,
        creator: address,
        collection: vector<u8>,
        name: vector<u8>,
    ) acquires TokenTransfers {
        let token_id = token::create_token_id_raw(creator, collection, name);
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
        let pending_tokens = table::borrow_mut(pending_claims, receiver_addr);
        let token = table::remove(pending_tokens, token_id);

        if (table::length(pending_tokens) == 0) {
            let real_pending_claims = table::remove(pending_claims, receiver_addr);
            table::destroy_empty(real_pending_claims)
        };

        token::deposit_token(receiver, token)
    }

    public entry fun cancel_offer_script(
        sender: signer,
        receiver: address,
        creator: address,
        collection: vector<u8>,
        name: vector<u8>,
    ) acquires TokenTransfers {
        let token_id = token::create_token_id_raw(creator, collection, name);
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
        let pending_tokens = table::borrow_mut(pending_claims, receiver);
        let token = table::remove(pending_tokens, token_id);

        if (table::length(pending_tokens) == 0) {
            let real_pending_claims = table::remove(pending_claims, receiver);
            table::destroy_empty(real_pending_claims)
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
        assert!(table::length(&borrow_global<TokenTransfers>(creator_addr).pending_claims) == 1, 1);
        offer(&creator, owner1_addr, token_id, 1);
        assert!(table::length(&borrow_global<TokenTransfers>(creator_addr).pending_claims) == 2, 2);
        claim(&owner0, creator_addr, token_id);
        assert!(table::length(&borrow_global<TokenTransfers>(creator_addr).pending_claims) == 1, 3);
        claim(&owner1, creator_addr, token_id);
        assert!(table::length(&borrow_global<TokenTransfers>(creator_addr).pending_claims) == 0, 4);

        offer(&owner0, owner1_addr, token_id, 1);
        claim(&owner1, owner0_addr, token_id);

        offer(&owner1, creator_addr, token_id, 1);
        offer(&owner1, creator_addr, token_id, 1);
        claim(&creator, owner1_addr, token_id);
    }

    fun create_token(creator: &signer, amount: u64): TokenId {
        use std::string;
        use std::option;

        let collection_name = string::utf8(b"Hello, World");

        token::create_collection(
            creator,
            *&collection_name,
            string::utf8(b"Collection: Hello, World"),
            string::utf8(b"https://aptos.dev"),
            option::some(1),
        );

        token::create_token(
            creator,
            *&collection_name,
            string::utf8(b"Token: Hello, Token"),
            string::utf8(b"Hello, Token"),
            false,
            amount,
            option::none(),
            string::utf8(b"https://aptos.dev"),
            0,
        )
    }
}
