module 0x1337::token_coin_swap {
    struct TokenCoinSwap<phantom T0> has drop, store {
        token_amount: u64,
        min_price_per_token: u64,
    }
    
    struct TokenEscrow has store {
        token: 0x1337::token::Token,
        locked_until_secs: u64,
    }
    
    struct TokenListingEvent has drop, store {
        token_id: 0x1337::token::TokenId,
        amount: u64,
        min_price: u64,
        locked_until_secs: u64,
        coin_type_info: 0x1::type_info::TypeInfo,
    }
    
    struct TokenListings<phantom T0> has key {
        listings: 0x1::table::Table<0x1337::token::TokenId, TokenCoinSwap<T0>>,
        listing_events: 0x1::event::EventHandle<TokenListingEvent>,
        swap_events: 0x1::event::EventHandle<TokenSwapEvent>,
    }
    
    struct TokenStoreEscrow has key {
        token_escrows: 0x1::table::Table<0x1337::token::TokenId, TokenEscrow>,
    }
    
    struct TokenSwapEvent has drop, store {
        token_id: 0x1337::token::TokenId,
        token_buyer: address,
        token_amount: u64,
        coin_amount: u64,
        coin_type_info: 0x1::type_info::TypeInfo,
    }
    
    public fun cancel_token_listing<T0>(arg0: &signer, arg1: 0x1337::token::TokenId, arg2: u64) {
        abort 0x1::error::invalid_argument(8)
    }
    
    public fun deposit_token_to_escrow(arg0: &signer, arg1: 0x1337::token::TokenId, arg2: 0x1337::token::Token, arg3: u64) {
        abort 0x1::error::invalid_argument(8)
    }
    
    public fun does_listing_exist<T0>(arg0: address, arg1: 0x1337::token::TokenId) : bool {
        abort 0x1::error::invalid_argument(8)
    }
    
    public fun exchange_coin_for_token<T0>(arg0: &signer, arg1: u64, arg2: address, arg3: address, arg4: 0x1::string::String, arg5: 0x1::string::String, arg6: u64, arg7: u64) {
        abort 0x1::error::invalid_argument(8)
    }
    
    fun initialize_token_listing<T0>(arg0: &signer) {
        abort 0x1::error::invalid_argument(8)
    }
    
    fun initialize_token_store_escrow(arg0: &signer) {
        abort 0x1::error::invalid_argument(8)
    }
    
    public entry fun list_token_for_swap<T0>(arg0: &signer, arg1: address, arg2: 0x1::string::String, arg3: 0x1::string::String, arg4: u64, arg5: u64, arg6: u64, arg7: u64) {
        abort 0x1::error::invalid_argument(8)
    }
    
    public fun withdraw_token_from_escrow(arg0: &signer, arg1: 0x1337::token::TokenId, arg2: u64) : 0x1337::token::Token {
        abort 0x1::error::invalid_argument(8)
    }
    
    fun withdraw_token_from_escrow_internal(arg0: address, arg1: 0x1337::token::TokenId, arg2: u64) : 0x1337::token::Token {
        abort 0x1::error::invalid_argument(8)
    }
    
    // decompiled from Move bytecode v6
}
