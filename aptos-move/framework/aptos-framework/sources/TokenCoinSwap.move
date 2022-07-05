/// A module for
/// 1. hold tokens escrow to prevent token been transferred
/// 2. list token for swapping with a targeted CoinType.
/// 3. execute the swapping
module AptosFramework::TokenCoinSwap {
    use Std::Event::{Self, EventHandle};
    use Std::Signer;
    use AptosFramework::Table::{Self, Table};
    use AptosFramework::TokenV1::{Self, Token, TokenId, deposit_token, withdraw_token, merge, split};
    use AptosFramework::Coin;
    use AptosFramework::Timestamp;
    use AptosFramework::TypeInfo::{Self, TypeInfo};

    const ETOKEN_ALREADY_LISTED: u64 = 1;
    const ETOKEN_LISTING_NOT_EXIST: u64 = 2;
    const ETOKEN_NOT_IN_ESCROW: u64 = 3;
    const ETOKEN_CANNOT_MOVE_OUT_OF_ESCROW_BEFORE_LOCKUP_TIME: u64 = 4;
    const ETOKEN_MIN_PRICE_NOT_MATCH: u64 = 5;
    const ETOKEN_AMOUNT_NOT_MATCH: u64 = 6;
    const ENOT_ENOUGH_COIN: u64 = 7;

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

    /// Coin owner withdraw coin to swap with tokens listed for swapping at the token owner's address.
    public(script) fun exchange_coin_for_token<CoinType>(
        coin_owner: &signer,
        coin_amount: u64,
        token_owner: address,
        token_id: TokenId,
        token_amount: u64,
    ) acquires TokenListings, TokenStoreEscrow {
        // valide listing existing and coin owner has sufficient balance
        let coin_address = Signer::address_of(coin_owner);
        let token_listing = borrow_global_mut<TokenListings<CoinType>>(token_owner);
        assert!(Table::contains(&token_listing.listings, token_id), ETOKEN_LISTING_NOT_EXIST);
        assert!(Coin::balance<CoinType>(coin_address) >= coin_amount, ENOT_ENOUGH_COIN);
        // validate min price and amount
        let token_swap = Table::borrow_mut(&mut token_listing.listings, token_id);
        assert!(token_swap.min_price_per_token * token_amount <= coin_amount, ETOKEN_MIN_PRICE_NOT_MATCH);
        assert!(token_swap.token_amount >= token_amount, ETOKEN_AMOUNT_NOT_MATCH);

        // withdraw from token escrow of tokens
        let tokens = withdraw_token_from_escrow_internal(token_owner, token_id, token_amount);

        // deposit tokens to the coin_owner
        deposit_token(coin_owner, tokens);

        // handle the royalty
        let royalty = TokenV1::get_royalty(token_id);

        let total_cost = token_swap.min_price_per_token * token_amount;
        let royalty_fee = total_cost * TokenV1::get_royalty_nominator(&royalty) / TokenV1::get_royalty_denominator(&royalty);
        let remaining = total_cost - royalty_fee;

        //deposite to the original creators
        let royalty_payee = TokenV1::get_royalty_payee(&royalty);
        let coin = Coin::withdraw<CoinType>(coin_owner, royalty_fee);
        Coin::deposit(royalty_payee, coin);

        // deposit coin to the token_owner
        let coin = Coin::withdraw<CoinType>(coin_owner, remaining);
        Coin::deposit(token_owner, coin);

        // update the token listing
        if (token_swap.token_amount == token_amount) {
            // delete the entry in the token listing
            Table::remove(&mut token_listing.listings, token_id);
        } else {
            token_swap.token_amount = token_swap.token_amount - token_amount;
        };

        Event::emit_event<TokenSwapEvent>(
            &mut token_listing.swap_events,
            TokenSwapEvent {
                token_id,
                token_buyer: coin_address,
                token_amount,
                coin_amount: total_cost,
                coin_type_info: TypeInfo::type_of<CoinType>(),
            },
        );
    }

    /// Token owner lists their token for swapping
    public(script) fun list_token_for_swap<CoinType>(
        token_owner: &signer,
        token_id: TokenId,
        token_amount: u64,
        min_coin_per_token: u64,
        locked_until_secs: u64
    ) acquires TokenStoreEscrow, TokenListings {
        initialize_token_store_escrow(token_owner);
        // withdraw the token and store them to the token_owner's TokenEscrow
        let token = withdraw_token(token_owner, token_id, token_amount);
        deposit_token_to_escrow(token_owner, token_id, token, locked_until_secs);
        // add the exchange info TokenCoinSwap list
        initialize_token_listing<CoinType>(token_owner);
        let swap = TokenCoinSwap<CoinType>{
            token_amount,
            min_price_per_token: min_coin_per_token
        };
        let listing = &mut borrow_global_mut<TokenListings<CoinType>>(Signer::address_of(token_owner)).listings;
        assert!(!Table::contains(listing, token_id), ETOKEN_ALREADY_LISTED);
        Table::add(listing, token_id, swap);

        let event_handle = &mut borrow_global_mut<TokenListings<CoinType>>(Signer::address_of(token_owner)).listing_events;
        Event::emit_event<TokenListingEvent>(
            event_handle,
            TokenListingEvent {
                token_id,
                amount: token_amount,
                min_price: min_coin_per_token,
                locked_until_secs,
                coin_type_info: TypeInfo::type_of<CoinType>(),
            },
        );
    }

    /// Initalize the token listing for a token owner
    fun initialize_token_listing<CoinType>(token_owner: &signer) {
        let addr = Signer::address_of(token_owner);
        if ( !exists<TokenListings<CoinType>>(addr) ) {
            let token_listing = TokenListings<CoinType>{
                listings: Table::new<TokenId, TokenCoinSwap<CoinType>>(),
                listing_events: Event::new_event_handle<TokenListingEvent>(token_owner),
                swap_events: Event::new_event_handle<TokenSwapEvent>(token_owner),

            };
            move_to(token_owner, token_listing);
        }
    }

    /// Intialize the token escrow
    fun initialize_token_store_escrow(token_owner: &signer) {
        let addr = Signer::address_of(token_owner);
        if ( !exists<TokenStoreEscrow>(addr) ) {
            let token_store_escrow = TokenStoreEscrow{
                token_escrows: Table::new<TokenId, TokenEscrow>()
            };
            move_to(token_owner, token_store_escrow);
        }
    }

    /// Put the token into escrow that cannot be transferred or withdrawed by the owner.
    public fun deposit_token_to_escrow(
        token_owner: &signer,
        token_id: TokenId,
        tokens: Token,
        locked_until_secs: u64
    ) acquires TokenStoreEscrow {
        let tokens_in_escrow = &mut borrow_global_mut<TokenStoreEscrow>(
            Signer::address_of(token_owner)).token_escrows;
        if (Table::contains(tokens_in_escrow, token_id)) {
            let dst = &mut Table::borrow_mut(tokens_in_escrow, token_id).token;
            merge(dst, tokens);
        } else {
            let token_escrow = TokenEscrow{
                token: tokens,
                locked_until_secs
            };
            Table::add(tokens_in_escrow, token_id, token_escrow);
        };
    }

    /// Private function for withdraw tokens from an escrow stored in token owner address
    fun withdraw_token_from_escrow_internal(
        token_owner_addr: address,
        token_id: TokenId,
        amount: u64
    ): Token acquires TokenStoreEscrow {
        let tokens_in_escrow = &mut borrow_global_mut<TokenStoreEscrow>(token_owner_addr).token_escrows;
        assert!(Table::contains(tokens_in_escrow, token_id), ETOKEN_NOT_IN_ESCROW);
        let token_escrow = Table::borrow_mut(tokens_in_escrow, token_id);
        assert!(Timestamp::now_seconds() > token_escrow.locked_until_secs, ETOKEN_CANNOT_MOVE_OUT_OF_ESCROW_BEFORE_LOCKUP_TIME);
        split(&mut token_escrow.token, amount)
    }

    /// Withdraw tokens from the token escrow. It needs a signer to authorize
    public fun withdraw_token_from_escrow(
        token_owner: &signer,
        token_id: TokenId,
        amount: u64
    ): Token acquires TokenStoreEscrow {
        withdraw_token_from_escrow_internal(Signer::address_of(token_owner), token_id, amount)
    }

    /// Cancel token listing for a fixed amount
    public fun cancel_token_listing<CoinType>(
        token_owner: &signer,
        token_id: TokenId,
        token_amount: u64
    ) acquires TokenListings, TokenStoreEscrow {
        let listing = &mut borrow_global_mut<TokenListings<CoinType>>(Signer::address_of(token_owner)).listings;
        // remove the listing entry
        assert!(Table::contains(listing, token_id), ETOKEN_LISTING_NOT_EXIST);
        Table::remove(listing, token_id);
        // get token out of escrow and deposit back to owner token store
        let tokens = withdraw_token_from_escrow(token_owner, token_id, token_amount);
        deposit_token(token_owner, tokens);
    }

    #[test(token_owner = @0xAB, coin_owner = @0x1, core_resources = @CoreResources)]
    public(script) fun test_exchange_coin_for_token(token_owner: signer, coin_owner: signer, core_resources: signer) acquires TokenStoreEscrow, TokenListings {
        Timestamp::set_time_has_started_for_testing(&core_resources);
        Timestamp::update_global_time_for_test(10000000);
        let token_id = TokenV1::create_collection_and_token(&token_owner, 100, 100, 100);
        TokenV1::initialize_token_store(&coin_owner);
        Coin::create_fake_money(&coin_owner, &token_owner, 100);

        list_token_for_swap<Coin::FakeMoney>(&token_owner, token_id, 100, 1, 0);
        exchange_coin_for_token<Coin::FakeMoney>(&coin_owner, 51, Signer::address_of(&token_owner), token_id, 50);
        // coin owner only has 50 coins left
        assert!(Coin::balance<Coin::FakeMoney>(Signer::address_of(&coin_owner)) == 50, 1);
        // all tokens in token escrow or transferred. Token owner has 0 token in token_store
        assert!(TokenV1::balance_of(Signer::address_of(&token_owner), token_id) == 0, 1);

        let token_listing = &borrow_global<TokenListings<Coin::FakeMoney>>(Signer::address_of(&token_owner)).listings;

        // completely sold, no listing left
        assert!(Table::length(token_listing) == 1, 1);
        let token_coin_swap = Table::borrow(token_listing, token_id);
        // sold 50 token only 50 tokens left
        assert!(token_coin_swap.token_amount == 50, token_coin_swap.token_amount);
    }
}
