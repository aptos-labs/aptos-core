/// This module provides the foundation for transferring of Tokens
module aptos_token::token_transfers {
    use std::signer;
    use std::string::String;
    use std::error;
    use aptos_std::table::{Self, Table};
    use aptos_token::token::{Self, Token, TokenId};
    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};

    //
    // Errors.
    //

    /// Token offer doesn't exist
    const ETOKEN_OFFER_NOT_EXIST: u64 = 1;

    struct PendingClaims has key {
        pending_claims: Table<TokenOfferId, Token>,
        offer_events: EventHandle<TokenOfferEvent>,
        cancel_offer_events: EventHandle<TokenCancelOfferEvent>,
        claim_events: EventHandle<TokenClaimEvent>,
    }

    struct TokenOfferId has copy, drop, store {
        to_addr: address,
        token_id: TokenId,
    }

    struct TokenOfferEvent has drop, store {
        to_address: address,
        token_id: TokenId,
        amount: u64,
    }

    struct TokenCancelOfferEvent has drop, store {
        to_address: address,
        token_id: TokenId,
        amount: u64,
    }

    struct TokenClaimEvent has drop, store {
        to_address: address,
        token_id: TokenId,
        amount: u64,
    }

    fun initialize_token_transfers(account: &signer) {
        move_to(
            account,
            PendingClaims {
                pending_claims: table::new<TokenOfferId, Token>(),
                offer_events: account::new_event_handle<TokenOfferEvent>(account),
                cancel_offer_events: account::new_event_handle<TokenCancelOfferEvent>(account),
                claim_events: account::new_event_handle<TokenClaimEvent>(account),
            }
        )
    }

    fun create_token_offer_id(to_addr: address, token_id: TokenId): TokenOfferId {
        TokenOfferId {
            to_addr,
            token_id
        }
    }

    public entry fun offer_script(
        sender: signer,
        receiver: address,
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
        amount: u64,
    ) acquires PendingClaims {
        let token_id = token::create_token_id_raw(creator, collection, name, property_version);
        offer(&sender, receiver, token_id, amount);
    }

    public fun offer(
        sender: &signer,
        receiver: address,
        token_id: TokenId,
        amount: u64,
    ) acquires PendingClaims {
        let sender_addr = signer::address_of(sender);
        if (!exists<PendingClaims>(sender_addr)) {
            initialize_token_transfers(sender)
        };

        let pending_claims =
            &mut borrow_global_mut<PendingClaims>(sender_addr).pending_claims;
        let token_offer_id = create_token_offer_id(receiver, token_id);
        let token = token::withdraw_token(sender, token_id, amount);
        if (!table::contains(pending_claims, token_offer_id)) {
            table::add(pending_claims, token_offer_id, token);
        } else {
            let dst_token = table::borrow_mut(pending_claims, token_offer_id);
            token::merge(dst_token, token);
        };

        event::emit_event<TokenOfferEvent>(
            &mut borrow_global_mut<PendingClaims>(sender_addr).offer_events,
            TokenOfferEvent {
                to_address: receiver,
                token_id,
                amount,
            },
        );
    }

    public entry fun claim_script(
        receiver: signer,
        sender: address,
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
    ) acquires PendingClaims {
        let token_id = token::create_token_id_raw(creator, collection, name, property_version);
        claim(&receiver, sender, token_id);
    }

    public fun claim(
        receiver: &signer,
        sender: address,
        token_id: TokenId,
    ) acquires PendingClaims {
        let pending_claims =
            &mut borrow_global_mut<PendingClaims>(sender).pending_claims;
        let token_offer_id = create_token_offer_id(signer::address_of(receiver), token_id);
        assert!(table::contains(pending_claims, token_offer_id), error::not_found(ETOKEN_OFFER_NOT_EXIST));
        let tokens = table::remove(pending_claims, token_offer_id);
        let amount = token::get_token_amount(&tokens);
        token::deposit_token(receiver, tokens);

        event::emit_event<TokenClaimEvent>(
            &mut borrow_global_mut<PendingClaims>(sender).claim_events,
            TokenClaimEvent {
                to_address: signer::address_of(receiver),
                token_id,
                amount,
            },
        );
    }

    public entry fun cancel_offer_script(
        sender: signer,
        receiver: address,
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
    ) acquires PendingClaims {
        let token_id = token::create_token_id_raw(creator, collection, name, property_version);
        cancel_offer(&sender, receiver, token_id);
    }

    // Extra from our pending_claims and return to gallery
    public fun cancel_offer(
        sender: &signer,
        receiver: address,
        token_id: TokenId,
    ) acquires PendingClaims {
        let sender_addr = signer::address_of(sender);
        let token_offer_id = create_token_offer_id(receiver, token_id);
        let pending_claims =
            &mut borrow_global_mut<PendingClaims>(sender_addr).pending_claims;
        let token = table::remove(pending_claims, token_offer_id);
        let amount = token::get_token_amount(&token);
        token::deposit_token(sender, token);

        event::emit_event<TokenCancelOfferEvent>(
            &mut borrow_global_mut<PendingClaims>(sender_addr).cancel_offer_events,
            TokenCancelOfferEvent {
                to_address: receiver,
                token_id,
                amount,
            },
        );
    }

    #[test(creator = @0x1, owner = @0x2)]
    public fun test_nft(creator: signer, owner: signer) acquires PendingClaims {
        let token_id = create_token(&creator, 1);

        let creator_addr = signer::address_of(&creator);
        let owner_addr = signer::address_of(&owner);
        aptos_framework::account::create_account_for_test(owner_addr);
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
    ) acquires PendingClaims {
        let token_id = create_token(&creator, 2);

        let creator_addr = signer::address_of(&creator);
        let owner0_addr = signer::address_of(&owner0);
        aptos_framework::account::create_account_for_test(owner0_addr);
        let owner1_addr = signer::address_of(&owner1);
        aptos_framework::account::create_account_for_test(owner1_addr);

        offer(&creator, owner0_addr, token_id, 1);
        offer(&creator, owner1_addr, token_id, 1);

        assert!(token::balance_of(signer::address_of(&creator), token_id) == 0, 1);
        claim(&owner0, creator_addr, token_id);
        assert!(token::balance_of(signer::address_of(&owner0), token_id) == 1, 1);
        claim(&owner1, creator_addr, token_id);
        assert!(token::balance_of(signer::address_of(&owner1), token_id) == 1, 1);

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
        aptos_framework::account::create_account_for_test(signer::address_of(creator));

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
