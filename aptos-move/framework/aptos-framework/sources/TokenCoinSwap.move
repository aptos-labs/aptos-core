// A basic utility module for swapping a fixed amount of coins with a fixed amount of tokens.
module AptosFramework::TokenCoinSwap {
    use Std::Signer;
    use Std::Option::Option;
    use AptosFramework::Table::{Self, Table};
    use AptosFramework::Token::{Self, Token, TokenId};
    use AptosFramework::Coin::{Self, Coin};
    use AptosFramework::SimpleMap::{SimpleMap, Self};
    use AptosFramework::Timestamp;

    const ETOKEN_ALREADY_LISTED: u64 = 1;
    const ETOKEN_LISTING_NOT_EXIST: u64 = 2;
    const ETOKEN_NOT_IN_ESCROW: u64 = 3;
    const ETOKEN_CANNOT_MOVE_OUT_OF_ESCROW_BEFORE_LOCKUP_TIME: u64 = 4;
    const ETOKEN_MIN_PRICE_NOT_MATCH: u64 = 5;
    const ETOKEN_AMOUNT_NOT_MATCH: u64 = 6;
    const ENOT_ENOUGH_COIN: u64 = 7;

    struct TokenCoinSwap<phantom CoinType> has store, drop {
        token_amount: u64,
        min_price_per_token: u64,
    }

    // stored at token owner's account
    struct TokenListings<phantom CoinType> has key {
        // key is the token id for swapping and value is the min price of target coin type.
        listings: Table<TokenId, TokenCoinSwap<CoinType>>
    }

    // stored at coin owner's account
    struct CoinListings<phantom CoinType> has key {
        // key is the token id for swapping and value is the min price of target coin type.
        listings: Table<TokenId, TokenCoinSwap<CoinType>>
    }

    // stored at owner's account
    struct CoinEscrow<phantom CoinType> has key {
        coin: Coin<CoinType>,
        // until the locked time runs out, the owner cannot move the coin out of the escrow
        // the default value is 0 meaning the owner can move the coin out anytime
        locked_until_secs: u64,
    }

    struct TokenEscrow has store {
        token: Token,
        // until the locked time runs out, the owner cannot move the token out of the escrow
        // the default value is 0 meaning the owner can move the coin out anytime
        locked_until_secs: u64
    }

    // stored at owner's account
    struct TokenStoreEscrow has key {
        token_escrows: Table<TokenId, TokenEscrow>,
    }

    public fun exchange_coin_for_token<CoinType>(
        coin_owner: &signer,
        coin_amount: u64,
        token_owner: address,
        token_id: TokenId,
        token_amount: u64
    ) acquires TokenListings, TokenStoreEscrow {
        // valide listing existing and coin owner has sufficient balance
        let token_listing = borrow_global_mut<TokenListings<CoinType>>(token_owner);
        assert!(Table::contains(&token_listing.listings, token_id), ETOKEN_LISTING_NOT_EXIST);
        assert!(Coin::balance<CoinType>(Signer::address_of(coin_owner)) >= coin_amount, ENOT_ENOUGH_COIN);
        // validate min price and amount
        let token_swap = Table::borrow_mut(&mut token_listing.listings, token_id);
        assert!(token_swap.min_price_per_token * token_amount <= coin_amount, ETOKEN_MIN_PRICE_NOT_MATCH);
        assert!(token_swap.token_amount >= token_amount, ETOKEN_AMOUNT_NOT_MATCH);

        // withdraw from token escrow of tokens
        let tokens = withdraw_token_from_escrow_internal(token_owner, token_id, token_amount);

        // deposite tokens to the coin_owner
        Token::deposit_token(coin_owner, tokens);

        // deposite coin to the token_owner
        let coin = Coin::withdraw<CoinType>(coin_owner, token_swap.min_price_per_token * token_amount);
        Coin::deposit(token_owner, coin);

        // update the token listing
        if (token_swap.token_amount == token_amount) {
            // delete the entry in the token listing
            Table::remove(&mut token_listing.listings, token_id);
        } else {
            token_swap.token_amount = token_swap.token_amount - token_amount;
        };
    }

    public fun list_token_for_swap<CoinType>(
        token_owner: &signer,
        token_id: TokenId,
        token_amount: u64,
        min_coin_per_token: u64,
        locked_until_secs: u64
    ) acquires TokenStoreEscrow, TokenListings {
        initialize_token_store_escrow(token_owner);
        // withdraw the token and store them to the token_owner's TokenEscrow
        let token = Token::withdraw_token(token_owner, token_id, token_amount);
        deposite_token_to_escrow(token_owner, token_id, token, locked_until_secs);
        // add the exchange info TokenCoinSwap list
        initialize_token_listing<CoinType>(token_owner);
        let swap = TokenCoinSwap<CoinType>{
            token_amount,
            min_price_per_token: min_coin_per_token
        };
        let listing = &mut borrow_global_mut<TokenListings<CoinType>>(Signer::address_of(token_owner)).listings;
        //TODO: allow the users to modify existing listing of tokenId
        assert!(!Table::contains(listing, token_id), ETOKEN_ALREADY_LISTED);
        Table::add(listing, token_id, swap);
    }

    fun initialize_token_listing<CoinType>(token_owner: &signer) {
        let addr = Signer::address_of(token_owner);
        if ( !exists<TokenListings<CoinType>>(addr) ) {
            let token_listing = TokenListings<CoinType>{
                listings: Table::new<TokenId, TokenCoinSwap<CoinType>>()
            };
            move_to(token_owner, token_listing);
        }
    }

    fun initialize_token_store_escrow(token_owner: &signer) {
        let addr = Signer::address_of(token_owner);
        if ( !exists<TokenStoreEscrow>(addr) ) {
            let token_store_escrow = TokenStoreEscrow{
                token_escrows: Table::new<TokenId, TokenEscrow>()
            };
            move_to(token_owner, token_store_escrow);
        }
    }

    public fun deposite_token_to_escrow(
        token_owner: &signer,
        token_id: TokenId,
        tokens: Token,
        locked_until_secs: u64
    ) acquires TokenStoreEscrow {
        let tokens_in_escrow = &mut borrow_global_mut<TokenStoreEscrow>(
            Signer::address_of(token_owner)).token_escrows;
        if (Table::contains(tokens_in_escrow, token_id)) {
            let dst = &mut Table::borrow_mut(tokens_in_escrow, token_id).token;
            Token::merge(dst, tokens);
        } else {
            let token_escrow = TokenEscrow{
                token: tokens,
                locked_until_secs
            };
            Table::add(tokens_in_escrow, token_id, token_escrow);
        };
    }

    // private function that should be only for internal use
    fun withdraw_token_from_escrow_internal(
        token_owner_addr: address,
        token_id: TokenId,
        amount: u64
    ): Token acquires TokenStoreEscrow {
        let tokens_in_escrow = &mut borrow_global_mut<TokenStoreEscrow>(token_owner_addr).token_escrows;
        assert!(Table::contains(tokens_in_escrow, token_id), ETOKEN_NOT_IN_ESCROW);
        let token_escrow = Table::borrow_mut(tokens_in_escrow, token_id);
        assert!(Timestamp::now_seconds() > token_escrow.locked_until_secs, ETOKEN_CANNOT_MOVE_OUT_OF_ESCROW_BEFORE_LOCKUP_TIME);
        Token::split(&mut token_escrow.token, amount)
    }

    public fun withdraw_token_from_escrow(
        token_owner: &signer,
        token_id: TokenId,
        amount: u64
    ): Token acquires TokenStoreEscrow {
        withdraw_token_from_escrow_internal(Signer::address_of(token_owner), token_id, amount)
    }

    public fun cancel_token_listing<CoinType>(
        token_owner: &signer,
        token_id: TokenId,
        token_amount: u64
    ) acquires TokenListings, TokenStoreEscrow {
        let listing = &mut borrow_global_mut<TokenListings<CoinType>>(Signer::address_of(token_owner)).listings;
        // remove the listing entry
        assert!(Table::contains(listing, token_id), ETOKEN_LISTING_NOT_EXIST);
        Table::remove(listing, token_id);
        // get token out of escrow and deposite back to owner token store
        let tokens = withdraw_token_from_escrow(token_owner, token_id, token_amount);
        Token::deposit_token(token_owner, tokens);
    }

    public fun exchange_token_for_coin<CoinType>(
        token_owner: &signer,
        token_id: TokenId,
        token_amout: u64,
        coin_amount: u64
    ) {
        // TODO
    }

    public fun list_coin_for_swap<CoinType>(
        coin_owner: &signer,
        coin_amount: u64,
        token_id: TokenId,
        min_token_amount: u64
    ) {
        // TODO
    }

    public fun cancel_coin_listing<CoinType>(coin_owner: &signer, coin_amount: u64) {
        // TODO
    }

    #[test(token_owner = @0xAB, coin_owner = @0xAA)]
    public(script) fun test_exchange_coin_for_token(token_owner: signer, coin_owner: signer) acquires TokenStoreEscrow, TokenListings {
        let token_id = Token::create_collection_and_token(&token_owner, 100, 100, 100);
        Token::initialize_token_store(&coin_owner);
        Coin::create_fake_money(&coin_owner, &token_owner, 100);

        list_token_for_swap<Coin::FakeMoney>(&token_owner, token_id, 50, 1, 0);
        exchange_coin_for_token<Coin::FakeMoney>(&coin_owner, 51, Signer::address_of(&token_owner), token_id, 50);
        // coin owner only has 50 coins left and token owner's only has 50 token left
        assert!(Coin::balance<Coin::FakeMoney>(Signer::address_of(&coin_owner)) == 50, 1);
        assert!(Token::balance_of(Signer::address_of(&token_owner), token_id) == 50, 1);

        let token_listing = &borrow_global<TokenListings<Coin::FakeMoney>>(Signer::address_of(&token_owner)).listings;

        // only 1 token id for sale now
        assert!(Table::length(token_listing) == 1, 1);
        let token_coin_swap = Table::borrow(token_listing, token_id);
        // sold 50 token only 50 tokens left
        assert!(token_coin_swap.token_amount == 50, 1);
    }
}
