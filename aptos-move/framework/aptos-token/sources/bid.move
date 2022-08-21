module aptos_token::bid {
    use aptos_token::listing::{Self, Listing};
    use aptos_token::token::{Self, TokenId};
    use std::guid::{Self, ID};
    use std::signer;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};

    const ENO_SUFFICIENT_FUND: u64 = 1;
    const ETOKEN_ID_NOT_MATCH: u64 = 2;
    const ELISTING_EXPIRED: u64 = 3;
    const ETOKEN_AMOUNT_NOT_MATCH: u64 = 4;
    const EBID_NOT_EXIST: u64 = 5;
    const ECANNOT_DRAW_FUND_BEFORE_EXPIRATION_TIME: u64 = 6;
    const ELISTING_ID_NOT_MATCH: u64 = 7;

    /// hold the bid info and coin at user account
    /// ensure the coin cannot be withdrawn or transferred during the auction
    struct Bid<phantom CoinType> has store {
        bidder: address,
        coin: Coin<CoinType>,
        token_id: TokenId,
        token_amount: u64,
        lock_until_sec: u64, // coin can be extracted from the Bid only after lock time expires
        listing_id: ID, // ensure the bid is only executed for the chosen listing_id
    }

    /// store all the bids by the user
    struct BidRecords<phantom CoinType> has key {
        records: Table<ID, Bid<CoinType>>,
    }

    public fun initialize_bid_records<CoinType>(bidder: &signer)  {
        let owner_addr = signer::address_of(bidder);

        if (!exists<BidRecords<CoinType>>(owner_addr)) {
            move_to(
                bidder,
                BidRecords<CoinType> {
                    records: table::new(),
                }
            );
        };
    }

    /// withdraw the coin and store them in bid struct and return a global unique bid id
    public fun bid<CoinType>(
        bidder: &signer,
        token_id: TokenId,
        token_amount:u64,
        withdraw_amount: u64,
        lock_until_sec: u64,
        listing_id: ID,
    ): ID acquires BidRecords {
        let bidder_address = signer::address_of(bidder);

        assert!(coin::balance<CoinType>(bidder_address) >= withdraw_amount, ENO_SUFFICIENT_FUND);
        // withdraw the coin and store them in escrow to ensure the fund is avaliable until expiration_sec
        let coin = coin::withdraw<CoinType>(bidder, withdraw_amount);

        let bid = Bid<CoinType> {
            bidder: bidder_address,
            coin,
            token_id,
            token_amount,
            lock_until_sec,
            listing_id,
        };
        let bid_id = create_bid_id(bidder);
        initialize_bid_records<CoinType>(bidder);
        let bid_records = &mut borrow_global_mut<BidRecords<CoinType>>(bidder_address).records;
        table::add(bid_records, bid_id, bid);
        bid_id
    }

    /// bidder can cancel a bid and get fund back
    public entry fun cancel_bid<CoinType>(
        bidder: &signer,
        bid_id_creation_number: u64,
    ) acquires BidRecords {
        let bidder_address = signer::address_of(bidder);
        let bid_id = guid::create_id(bidder_address, bid_id_creation_number);

        let bid_records = &mut borrow_global_mut<BidRecords<CoinType>>(bidder_address).records;
        assert!(table::contains(bid_records, bid_id), EBID_NOT_EXIST);

        let bid = table::remove(bid_records, bid_id);
        assert!(timestamp::now_seconds() >= bid.lock_until_sec, ECANNOT_DRAW_FUND_BEFORE_EXPIRATION_TIME);
        coin::deposit(bidder_address, clear_bid(bid));
    }

    /// execute a bid to a listing, no signer required to perform this function
    public fun execute_listing_bid<CoinType>(bidder: address, bid_id: ID, entry: Listing<CoinType>) acquires BidRecords {
        let bid_records = &mut borrow_global_mut<BidRecords<CoinType>>(bidder).records;
        assert!(table::contains(bid_records, bid_id), EBID_NOT_EXIST);
        let bid = table::borrow(bid_records, bid_id);

        let coin_owner = bid.bidder;
        let token_owner = listing::get_listing_owner(&entry);
        // validate token_id match
        assert!(bid.token_id == listing::get_listing_token_id(&entry), ETOKEN_ID_NOT_MATCH);
        // validate offerred amount and price
        let listed_amount =  listing::get_listing_token_amount(&entry);
        let min_total = listing::get_listing_min_price(&entry) * listed_amount;
        assert!(coin::value(&bid.coin) >= min_total, ENO_SUFFICIENT_FUND);
        assert!(bid.token_amount == listed_amount, ETOKEN_AMOUNT_NOT_MATCH);
        // validate expiration time
        let now = timestamp::now_seconds();
        // only when lock time expires, funds can be extracted for executions
        assert!(bid.lock_until_sec <= now, ECANNOT_DRAW_FUND_BEFORE_EXPIRATION_TIME);
        // listing should expire after auction ends
        assert!(listing::get_listing_expiration(&entry) >= now, ELISTING_EXPIRED);
        //listing_id matches
        assert!(listing::get_listing_id(&entry) == bid.listing_id, ELISTING_ID_NOT_MATCH);

        // transfer coin and token
        let token = token::withdraw_with_event_internal(token_owner, bid.token_id, listed_amount);
        token::direct_deposit(coin_owner, token);

        let bid_mut = table::remove(bid_records, bid_id);
        coin::deposit(token_owner, clear_bid(bid_mut));
    }

    /// destruct the bid struct and extract coins
    fun clear_bid<CoinType>(bid: Bid<CoinType>): Coin<CoinType> {
        let Bid {
            bidder: _,
            coin,
            token_id: _,
            token_amount: _,
            lock_until_sec: _,
            listing_id: _,
        } = bid;
        coin
    }

    /// internal function for assigned a global unique id for a listing
    fun create_bid_id(owner: &signer): ID {
        let gid = guid::create(owner);
        guid::id(&gid)
    }

    #[test_only]
    public fun test_setup(
        owner: &signer,
        bidder_a: &signer,
        aptos_framework: &signer,
        use_wrong_listing_id: bool,
        use_wrong_coin_amount: bool,
        use_wrong_lock_time: bool,
        use_wrong_token_amount: bool,
    ): (ID, Listing<coin::FakeMoney>) acquires BidRecords {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test(11000000);

        // owner creats a listing
        let token_id = token::create_collection_and_token(owner, 2, 2, 2);
        let entry = listing::create_list<coin::FakeMoney>(
            owner,
            token_id,
            1,
            2,
            false,
            100
        );
        let list_id = listing::get_listing_id(&entry);
        // setup the bider

        coin::create_fake_money(aptos_framework, bidder_a, 100);
        coin::transfer<coin::FakeMoney>(aptos_framework, signer::address_of(bidder_a), 100);
        //assert!(signer::address_of(&owner) == @0x1, 1);

        token::initialize_token_store(bidder_a);
        coin::register_for_test<coin::FakeMoney>(owner);
        let token_amount =  if (use_wrong_token_amount) { 10 } else {1};
        let offered_coin = if (use_wrong_coin_amount) {1} else {10};
        let bid_lock_time = if (use_wrong_lock_time) {12} else {10};
        let bid_listing_id = if (use_wrong_listing_id) {guid::create_id(signer::address_of(aptos_framework), 1)} else {list_id};
        let bid_1 = bid<coin::FakeMoney>(
            bidder_a,
            token_id,
            token_amount,
            offered_coin,
            bid_lock_time,
            bid_listing_id);
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
            false,
            false
        );
        execute_listing_bid(signer::address_of(&bidder_a), bid_id, entry);
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
            false,
            true,
            false,
            false
        );
        execute_listing_bid(signer::address_of(&bidder_a), bid_id, entry);
    }

    #[test(owner = @0xAF, bidder_a = @0xBB, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 7)]
    public fun test_wrong_listing_id(
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
            false,
            false
        );
        execute_listing_bid(signer::address_of(&bidder_a), bid_id, entry);
    }
    #[test(owner = @0xAF, bidder_a = @0xBB, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 6)]
    public fun test_wrong_lock_time(
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
            true,
            false
        );
        execute_listing_bid(signer::address_of(&bidder_a), bid_id, entry);
    }
    #[test(owner = @0xAF, bidder_a = @0xBB, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 4)]
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
            false,
            false,
            true
        );
        execute_listing_bid(signer::address_of(&bidder_a), bid_id, entry);
    }
}
