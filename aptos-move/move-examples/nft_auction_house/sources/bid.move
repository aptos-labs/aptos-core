/// bid library for listing token for sale and bid for tokens
/// An example can be found under aptos-move/move_examples/nft_auction_house
module auction_house::bid {

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::timestamp;
    use aptos_std::guid::{Self, ID};
    use aptos_std::table::{Self, Table};
    use aptos_token::token::{Self, TokenId};
    use auction_house::listing::{Self, Listing};
    use std::signer;
    use std::error;


    //
    // Errors
    //

    /// No sufficient fund to bid
    const ENO_SUFFICIENT_FUND: u64 = 1;

    /// Token ID doesn't match
    const ETOKEN_ID_NOT_MATCH: u64 = 2;

    /// Listing expired
    const ELISTING_EXPIRED: u64 = 3;

    /// Listing hasn't started yet
    const ELISTING_NOT_STARTED: u64 = 4;

    /// Token amount doesn't match
    const ETOKEN_AMOUNT_NOT_MATCH: u64 = 5;

    /// Bid doesn't exist
    const EBID_NOT_EXIST: u64 = 6;

    /// Cannot withdraw fund before bid expiration time
    const ECANNOT_DRAW_FUND_BEFORE_EXPIRATION_TIME: u64 = 7;

    /// Listing Id doesn't match
    const ELISTING_ID_NOT_MATCH: u64 = 8;

    /// hold the bid info and coin at user account
    struct Bid<phantom CoinType> has store {
        bidder: address,
        coin: Coin<CoinType>,
        offer_price: u64,
        listing_id: ID, // ensure the bid is only executed for the chosen listing_id
        expiration_sec: u64,
    }

    /// store all the bids by the user
    struct BidRecords<phantom CoinType> has key {
        records: Table<ID, Bid<CoinType>>,
        bid_event: EventHandle<BidEvent<CoinType>>,
        withdraw_bid_event: EventHandle<WithdrawBidEvent<CoinType>>,
    }

    struct BidEvent<phantom CoinType> has copy, drop, store {
        offer_price: u64,
        listing_id: ID,
        bid_id: ID,
        expiration_sec: u64,
    }

    struct WithdrawBidEvent<phantom CoinType> has copy, drop, store {
        bid_id: ID,
    }

    public fun initialize_bid_records<CoinType>(bidder: &signer)  {
        let owner_addr = signer::address_of(bidder);

        if (!exists<BidRecords<CoinType>>(owner_addr)) {
            move_to(
                bidder,
                BidRecords<CoinType> {
                    records: table::new(),
                    bid_event: account::new_event_handle<BidEvent<CoinType>>(bidder),
                    withdraw_bid_event: account::new_event_handle<WithdrawBidEvent<CoinType>>(bidder),
                }
            );
        };
    }

    public fun assert_bid_for_a_listing<CoinType>(
        token_id: TokenId,
        offer_price: u64,
        token_amount: u64,
        entry: &Listing<CoinType>,
        bid_time: u64,
    ) {
        // validate token_id match
        assert!(token_id == listing::get_listing_token_id(entry), error::invalid_argument(ETOKEN_ID_NOT_MATCH));
        // validate offerred amount and price
        let listed_amount =  listing::get_listing_token_amount(entry);
        let min_total = listing::get_listing_min_price(entry) * listed_amount;
        let total_coin_amount = offer_price * token_amount;
        assert!( total_coin_amount >= min_total, ENO_SUFFICIENT_FUND);
        assert!(token_amount == listed_amount, ETOKEN_AMOUNT_NOT_MATCH);
        assert!(bid_time >= listing::get_listing_start(entry), error::invalid_argument(ELISTING_NOT_STARTED));
        assert!(bid_time <= listing::get_listing_expiration(entry), error::invalid_argument(ELISTING_EXPIRED));
    }

    /// withdraw the coin and store them in bid struct and return a global unique bid id
    public fun bid<CoinType>(
        bidder: &signer,
        token_id: TokenId,
        token_amount:u64,
        offer_price: u64,
        entry: &Listing<CoinType>,
        expiration_sec: u64,
    ): ID acquires BidRecords {
        initialize_bid_records<CoinType>(bidder);
        let bidder_address = signer::address_of(bidder);
        // check the bid is legit for the listing
        let total_coin_amount = offer_price * token_amount; // the total coin offerred by the bidder
        assert_bid_for_a_listing(token_id, total_coin_amount, token_amount, entry, timestamp::now_seconds());
        // check bidder has sufficient balance
        assert!(coin::balance<CoinType>(bidder_address) >= total_coin_amount, error::invalid_argument(ENO_SUFFICIENT_FUND));
        // withdraw the coin and store them in escrow to ensure the fund is avaliable until expiration_sec
        let coin = coin::withdraw<CoinType>(bidder, total_coin_amount);

        let bid = Bid<CoinType> {
            bidder: bidder_address,
            coin,
            offer_price,
            listing_id: listing::get_listing_id(entry),
            expiration_sec,
        };
        let bid_id = create_bid_id(bidder);
        initialize_bid_records<CoinType>(bidder);
        let bid_records = borrow_global_mut<BidRecords<CoinType>>(bidder_address);
        table::add(&mut bid_records.records, bid_id, bid);
        event::emit_event<BidEvent<CoinType>>(
            &mut bid_records.bid_event,
            BidEvent<CoinType> {
                offer_price,
                listing_id: listing::get_listing_id(entry),
                bid_id,
                expiration_sec,
            },
        );

        // opt-in direct transfer to receive token without signer
        token::opt_in_direct_transfer(bidder, true);

        bid_id
    }

    /// bidder can withdraw fund from any bid
    public entry fun release_coin_from_bid<CoinType>(
        bidder: &signer,
        bid_id_creation_number: u64,
    ) acquires BidRecords {
        let bidder_address = signer::address_of(bidder);
        let bid_id = guid::create_id(bidder_address, bid_id_creation_number);

        let bid_records = borrow_global_mut<BidRecords<CoinType>>(bidder_address);
        assert!(table::contains(&bid_records.records, bid_id), error::not_found(EBID_NOT_EXIST));

        let bid = table::remove(&mut bid_records.records, bid_id);
        assert!(timestamp::now_seconds() > bid.expiration_sec, error::permission_denied(ECANNOT_DRAW_FUND_BEFORE_EXPIRATION_TIME));

        coin::deposit(bidder_address, clear_bid(bid));
        event::emit_event<WithdrawBidEvent<CoinType>>(
            &mut bid_records.withdraw_bid_event,
            WithdrawBidEvent<CoinType> {
                bid_id
            },
        );
    }

    /// validate if bid still exists
    public fun is_bid_valid<CoinType>(bid_id: ID): bool acquires BidRecords {
        let bidder_address = guid::id_creator_address(&bid_id);

        let bid_records = borrow_global_mut<BidRecords<CoinType>>(bidder_address);
        table::contains(&bid_records.records, bid_id)

    }

    fun deduct_fee<CoinType>(
        total_coin: &mut Coin<CoinType>,
        fee_numerator: u64,
        fee_denominator: u64
    ): Coin<CoinType> {
        let value = coin::value(total_coin);
        let fee = if (fee_denominator == 0) {
            0
        } else {
            value * fee_numerator/ fee_denominator
        };
        coin::extract(total_coin, fee)
    }

    /// execute a bid to a listing, no signer required to perform this function
    /// pay fee to 3 party based on a percentage
    /// deduct royalty and send to the payee account
    public fun execute_listing_bid<CoinType>(
        bid_id: ID,
        entry: Listing<CoinType>,
        market_fund_address: address,
        fee_numerator: u64,
        fee_denominator: u64,
    ) acquires BidRecords {
        let bid_records = &mut borrow_global_mut<BidRecords<CoinType>>(guid::id_creator_address(&bid_id)).records;
        assert!(table::contains(bid_records, bid_id), error::not_found(EBID_NOT_EXIST));
        let bid = table::borrow(bid_records, bid_id);
        let (
            id,
            token_owner,
            token_id,
            listed_amount,
            min_price,
            _,
            _,
            expiration_sec,
            withdraw_cap,
        ) = listing::destroy_listing(entry);
        let coin_owner = bid.bidder;
        // validate offerred amount and price
        let min_total = min_price * listed_amount;
        assert!(coin::value(&bid.coin) >= min_total, error::invalid_argument(ENO_SUFFICIENT_FUND));
        // validate expiration time
        let now = timestamp::now_seconds();
        assert!(now >= expiration_sec, error::invalid_argument(ELISTING_EXPIRED));
        //listing_id matches
        assert!(id == bid.listing_id, error::invalid_argument(ELISTING_ID_NOT_MATCH));

        // transfer coin and token
        let token = token::withdraw_with_capability(
            withdraw_cap
        );

        token::direct_deposit_with_opt_in(coin_owner, token);

        let bid_mut = table::remove(bid_records, bid_id);
        let coins = clear_bid(bid_mut);

        // deduct royalty fee from the transactions
        let royalty = token::get_royalty(token_id);
        let royalty_payee = token::get_royalty_payee(&royalty);
        let royalty_coin = deduct_fee(
            &mut coins,
            token::get_royalty_numerator(&royalty),
            token::get_royalty_denominator(&royalty)
        );
        coin::deposit(royalty_payee, royalty_coin);

        // deduct marketplace fee
        let market_fee = deduct_fee(&mut coins, fee_numerator, fee_denominator);
        coin::deposit(market_fund_address, market_fee);

        // give the remaining to the seller
        coin::deposit(token_owner, coins);
    }

    /// destruct the bid struct and extract coins
    fun clear_bid<CoinType>(bid: Bid<CoinType>): Coin<CoinType> {
        let Bid {
            bidder: _,
            coin,
            offer_price: _,
            listing_id: _,
            expiration_sec: _,
        } = bid;
        coin
    }

    public fun get_bid_info<CoinType>(
        bidder_address: address,
        bid_id_creation_number: u64
    ): (u64, ID, u64) acquires BidRecords {
        let bid_id = guid::create_id(bidder_address, bid_id_creation_number);

        let bid_records = &mut borrow_global_mut<BidRecords<CoinType>>(bidder_address).records;
        assert!(table::contains(bid_records, bid_id), error::not_found(EBID_NOT_EXIST));

        let bid = table::borrow(bid_records, bid_id);
        ( bid.offer_price, bid.listing_id, bid.expiration_sec)
    }

    /// internal function for assigned a global unique id for a listing
    fun create_bid_id(owner: &signer): ID {
        let gid = account::create_guid(owner);
        guid::id(&gid)
    }

    #[test_only]
    public fun test_setup(
        owner: &signer,
        bidder_a: &signer,
        aptos_framework: &signer,
        use_wrong_coin_amount: bool,
        use_wrong_token_amount: bool,
    ): (ID, Listing<coin::FakeMoney>) acquires BidRecords {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test(11000000);

        account::create_account_for_test(signer::address_of(owner));
        account::create_account_for_test(signer::address_of(bidder_a));
        account::create_account_for_test(signer::address_of(aptos_framework));


        // owner creats a listing
        let token_id = token::create_collection_and_token(owner, 2, 2, 2);
        let entry = listing::create_list<coin::FakeMoney>(
            owner,
            token_id,
            1,
            2,
            false,
            0,
            100,
            200,
        );

        coin::create_fake_money(aptos_framework, bidder_a, 100);
        coin::transfer<coin::FakeMoney>(aptos_framework, signer::address_of(bidder_a), 100);
        //assert!(signer::address_of(&owner) == @0x1, 1);

        token::initialize_token_store(bidder_a);
        coin::register<coin::FakeMoney>(owner);
        let token_amount =  if (use_wrong_token_amount) { 10 } else {1};
        let offered_price = if (use_wrong_coin_amount) {1} else {10};
        let bid_1 = bid<coin::FakeMoney>(
            bidder_a,
            token_id,
            token_amount,
            offered_price,
            &entry,
            100000001,
        );
        (bid_1, entry)
    }

    #[test(owner = @0xAF, bidder_a = @0xBB, aptos_framework = @aptos_framework)]
    public fun test_successful(
        owner: signer,
        bidder_a: signer,
        aptos_framework: signer
    ) acquires BidRecords {
        let (bid_id, entry) = test_setup(
            &owner,
            &bidder_a,
            &aptos_framework,
            false,
            false,
        );
        let lister = listing::get_listing_owner(&entry);
        timestamp::update_global_time_for_test(100000000);
        execute_listing_bid(bid_id, entry,@aptos_framework, 10, 100);

        // listing owner get paid with a deduction of market fee
        // 1 * 10 - (1 * 10) * (10 / 100)
        assert!(coin::balance<coin::FakeMoney>(lister) == 9, 1);
    }

    #[test(owner = @0xAF, bidder_a = @0xBB, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 1)]
    public fun test_wrong_coin_amount(
        owner: signer,
        bidder_a: signer,
        aptos_framework: signer
    ) acquires BidRecords {
        let (bid_id, entry) = test_setup(
            &owner,
            &bidder_a,
            &aptos_framework,
            true,
            false,
        );
        timestamp::update_global_time_for_test(100000000);
        execute_listing_bid(bid_id,  entry, @aptos_framework, 0, 1);
    }

    #[test(owner = @0xAF, bidder_a = @0xBB, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 5)]
    public fun test_wrong_token_amount(
        owner: signer,
        bidder_a: signer,
        aptos_framework: signer
    ) acquires BidRecords {
        let (bid_id, entry) = test_setup(
            &owner,
            &bidder_a,
            &aptos_framework,
            false,
            true,
        );
        timestamp::update_global_time_for_test(100000000);
        execute_listing_bid(bid_id, entry, @aptos_framework, 0, 1);
    }
}
