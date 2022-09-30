/// Deprecated module
module aptos_token::token_coin_swap {
    use std::string::String;
    use std::error;
    use aptos_std::table::Table;
    use aptos_std::type_info::TypeInfo;
    use aptos_framework::event::EventHandle;
    use aptos_token::token::{Token, TokenId};

    //
    // Errors.
    //

    /// Token already listed
    const ETOKEN_ALREADY_LISTED: u64 = 1;

    /// Token listing no longer exists
    const ETOKEN_LISTING_NOT_EXIST: u64 = 2;

    /// Token is not in escrow
    const ETOKEN_NOT_IN_ESCROW: u64 = 3;

    /// Token cannot be moved out of escrow before the lockup time
    const ETOKEN_CANNOT_MOVE_OUT_OF_ESCROW_BEFORE_LOCKUP_TIME: u64 = 4;

    /// Token buy price doesn't match listing price
    const ETOKEN_MIN_PRICE_NOT_MATCH: u64 = 5;

    /// Token buy amount doesn't match listing amount
    const ETOKEN_AMOUNT_NOT_MATCH: u64 = 6;

    /// Not enough coin to buy token
    const ENOT_ENOUGH_COIN: u64 = 7;

    /// Deprecated module
    const EDEPRECATED_MODULE: u64 = 8;

    /// TokenCoinSwap records a swap ask for swapping token_amount with CoinType with a minimal price per token
    struct TokenCoinSwap<phantom CoinType> has store, drop {
        token_amount: u64,
        min_price_per_token: u64,
    }

    /// The listing of all tokens for swapping stored at token owner's account
    struct TokenListings<phantom CoinType> has key {
        // key is the token id for swapping and value is the min price of target coin type.
        listings: Table<TokenId, TokenCoinSwap<CoinType>>,
        listing_events: EventHandle<TokenListingEvent>,
        swap_events: EventHandle<TokenSwapEvent>,
    }

    /// TokenEscrow holds the tokens that cannot be withdrawn or transferred
    struct TokenEscrow has store {
        token: Token,
        // until the locked time runs out, the owner cannot move the token out of the escrow
        // the default value is 0 meaning the owner can move the coin out anytime
        locked_until_secs: u64,
    }

    /// TokenStoreEscrow holds a map of token id to their tokenEscrow
    struct TokenStoreEscrow has key {
        token_escrows: Table<TokenId, TokenEscrow>,
    }

    struct TokenListingEvent has drop, store {
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        locked_until_secs: u64,
        coin_type_info: TypeInfo,
    }

    struct TokenSwapEvent has drop, store {
        token_id: TokenId,
        token_buyer: address,
        token_amount: u64,
        coin_amount: u64,
        coin_type_info: TypeInfo,
    }

    public fun does_listing_exist<CoinType>(
        _token_owner: address,
        _token_id: TokenId
    ): bool {
        abort error::invalid_argument(EDEPRECATED_MODULE)
    }

    /// Coin owner withdraw coin to swap with tokens listed for swapping at the token owner's address.
    public fun exchange_coin_for_token<CoinType>(
        _coin_owner: &signer,
        _coin_amount: u64,
        _token_owner: address,
        _creators_address: address,
        _collection: String,
        _name: String,
        _property_version: u64,
        _token_amount: u64,
    ) {
        abort error::invalid_argument(EDEPRECATED_MODULE)
    }

    /// Token owner lists their token for swapping
    public entry fun list_token_for_swap<CoinType>(
        _token_owner: &signer,
        _creators_address: address,
        _collection: String,
        _name: String,
        _property_version: u64,
        _token_amount: u64,
        _min_coin_per_token: u64,
        _locked_until_secs: u64
    ) {
        abort error::invalid_argument(EDEPRECATED_MODULE)
    }

    /// Initalize the token listing for a token owner
    fun initialize_token_listing<CoinType>(_token_owner: &signer) {
        abort error::invalid_argument(EDEPRECATED_MODULE)
    }

    /// Intialize the token escrow
    fun initialize_token_store_escrow(_token_owner: &signer) {
        abort error::invalid_argument(EDEPRECATED_MODULE)
    }

    /// Put the token into escrow that cannot be transferred or withdrawed by the owner.
    public fun deposit_token_to_escrow(
        _token_owner: &signer,
        _token_id: TokenId,
        _tokens: Token,
        _locked_until_secs: u64
    ) {
        abort error::invalid_argument(EDEPRECATED_MODULE)
    }

    /// Private function for withdraw tokens from an escrow stored in token owner address
    fun withdraw_token_from_escrow_internal(
        _token_owner_addr: address,
        _token_id: TokenId,
        _amount: u64
    ): Token {
        abort error::invalid_argument(EDEPRECATED_MODULE)
    }

    /// Withdraw tokens from the token escrow. It needs a signer to authorize
    public fun withdraw_token_from_escrow(
        _token_owner: &signer,
        _token_id: TokenId,
        _amount: u64
    ): Token {
        abort error::invalid_argument(EDEPRECATED_MODULE)
    }

    /// Cancel token listing for a fixed amount
    public fun cancel_token_listing<CoinType>(
        _token_owner: &signer,
        _token_id: TokenId,
        _token_amount: u64,
    ) {
        abort error::invalid_argument(EDEPRECATED_MODULE)
    }
}
