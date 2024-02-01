module 0x1337::token_transfers {
    struct PendingClaims has key {
        pending_claims: 0x1::table::Table<TokenOfferId, 0x1337::token::Token>,
        offer_events: 0x1::event::EventHandle<TokenOfferEvent>,
        cancel_offer_events: 0x1::event::EventHandle<TokenCancelOfferEvent>,
        claim_events: 0x1::event::EventHandle<TokenClaimEvent>,
    }
    
    struct TokenCancelOfferEvent has drop, store {
        to_address: address,
        token_id: 0x1337::token::TokenId,
        amount: u64,
    }
    
    struct TokenClaimEvent has drop, store {
        to_address: address,
        token_id: 0x1337::token::TokenId,
        amount: u64,
    }
    
    struct TokenOfferEvent has drop, store {
        to_address: address,
        token_id: 0x1337::token::TokenId,
        amount: u64,
    }
    
    struct TokenOfferId has copy, drop, store {
        to_addr: address,
        token_id: 0x1337::token::TokenId,
    }
    
    public fun cancel_offer(arg0: &signer, arg1: address, arg2: 0x1337::token::TokenId) acquires PendingClaims {
        let v0 = 0x1::signer::address_of(arg0);
        assert!(exists<PendingClaims>(v0), 1);
        let v1 = &mut borrow_global_mut<PendingClaims>(v0).pending_claims;
        let v2 = 0x1::table::remove<TokenOfferId, 0x1337::token::Token>(v1, create_token_offer_id(arg1, arg2));
        let v3 = 0x1337::token::get_token_amount(&v2);
        0x1337::token::deposit_token(arg0, v2);
        let v4 = &mut borrow_global_mut<PendingClaims>(v0).cancel_offer_events;
        let v5 = TokenCancelOfferEvent{
            to_address : arg1, 
            token_id   : arg2, 
            amount     : v3,
        };
        0x1::event::emit_event<TokenCancelOfferEvent>(v4, v5);
    }
    
    public entry fun cancel_offer_script(arg0: signer, arg1: address, arg2: address, arg3: 0x1::string::String, arg4: 0x1::string::String, arg5: u64) acquires PendingClaims {
        cancel_offer(&arg0, arg1, 0x1337::token::create_token_id_raw(arg2, arg3, arg4, arg5));
    }
    
    public fun claim(arg0: &signer, arg1: address, arg2: 0x1337::token::TokenId) acquires PendingClaims {
        assert!(exists<PendingClaims>(arg1), 1);
        let v0 = &mut borrow_global_mut<PendingClaims>(arg1).pending_claims;
        let v1 = create_token_offer_id(0x1::signer::address_of(arg0), arg2);
        assert!(0x1::table::contains<TokenOfferId, 0x1337::token::Token>(v0, v1), 0x1::error::not_found(1));
        let v2 = 0x1::table::remove<TokenOfferId, 0x1337::token::Token>(v0, v1);
        let v3 = 0x1337::token::get_token_amount(&v2);
        0x1337::token::deposit_token(arg0, v2);
        let v4 = &mut borrow_global_mut<PendingClaims>(arg1).claim_events;
        let v5 = TokenClaimEvent{
            to_address : 0x1::signer::address_of(arg0), 
            token_id   : arg2, 
            amount     : v3,
        };
        0x1::event::emit_event<TokenClaimEvent>(v4, v5);
    }
    
    public entry fun claim_script(arg0: signer, arg1: address, arg2: address, arg3: 0x1::string::String, arg4: 0x1::string::String, arg5: u64) acquires PendingClaims {
        claim(&arg0, arg1, 0x1337::token::create_token_id_raw(arg2, arg3, arg4, arg5));
    }
    
    fun create_token_offer_id(arg0: address, arg1: 0x1337::token::TokenId) : TokenOfferId {
        TokenOfferId{
            to_addr  : arg0, 
            token_id : arg1,
        }
    }
    
    fun initialize_token_transfers(arg0: &signer) {
        let v0 = 0x1::table::new<TokenOfferId, 0x1337::token::Token>();
        let v1 = 0x1::account::new_event_handle<TokenOfferEvent>(arg0);
        let v2 = 0x1::account::new_event_handle<TokenCancelOfferEvent>(arg0);
        let v3 = 0x1::account::new_event_handle<TokenClaimEvent>(arg0);
        let v4 = PendingClaims{
            pending_claims      : v0, 
            offer_events        : v1, 
            cancel_offer_events : v2, 
            claim_events        : v3,
        };
        move_to<PendingClaims>(arg0, v4);
    }
    
    public fun offer(arg0: &signer, arg1: address, arg2: 0x1337::token::TokenId, arg3: u64) acquires PendingClaims {
        let v0 = 0x1::signer::address_of(arg0);
        if (!exists<PendingClaims>(v0)) {
            initialize_token_transfers(arg0);
        };
        let v1 = &mut borrow_global_mut<PendingClaims>(v0).pending_claims;
        let v2 = create_token_offer_id(arg1, arg2);
        let v3 = 0x1337::token::withdraw_token(arg0, arg2, arg3);
        if (!0x1::table::contains<TokenOfferId, 0x1337::token::Token>(v1, v2)) {
            0x1::table::add<TokenOfferId, 0x1337::token::Token>(v1, v2, v3);
        } else {
            0x1337::token::merge(0x1::table::borrow_mut<TokenOfferId, 0x1337::token::Token>(v1, v2), v3);
        };
        let v4 = TokenOfferEvent{
            to_address : arg1, 
            token_id   : arg2, 
            amount     : arg3,
        };
        0x1::event::emit_event<TokenOfferEvent>(&mut borrow_global_mut<PendingClaims>(v0).offer_events, v4);
    }
    
    public entry fun offer_script(arg0: signer, arg1: address, arg2: address, arg3: 0x1::string::String, arg4: 0x1::string::String, arg5: u64, arg6: u64) acquires PendingClaims {
        offer(&arg0, arg1, 0x1337::token::create_token_id_raw(arg2, arg3, arg4, arg5), arg6);
    }
    
    // decompiled from Move bytecode v6
}
