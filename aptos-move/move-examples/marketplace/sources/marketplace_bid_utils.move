/// An marketplace library providing basic function for buy and bid
/// To see how to use the library, please check the two example contract in the same folder
module marketplace::marketplace_bid_utils {

    use aptos_framework::account;
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::timestamp;
    use aptos_std::guid::{Self, ID};
    use aptos_std::table::{Self, Table};
    use aptos_token::token::{Self, TokenId};
    use marketplace::marketplace_listing_utils::{Self as listing_util, Listing, create_listing_id_raw};
    use std::signer;
    use std::error;
    use std::string::String;
    use aptos_token::property_map::{Self, PropertyMap};

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

    /// The bidder has already bid for the same listing
    const EBID_ID_EXISTS: u64 = 9;

    /// Buy from non-instant sale listing
    const EBUY_NON_INSTANT_SALE_LISTING: u64 = 10;

    /// Cannot buy from expired listing
    const EBUY_FROM_EXPIRED_LISTING: u64 = 11;

    /// Cannot buy from a listing that hasn't started
    const EBUY_FROM_NOT_STARTED_LISTING: u64 = 12;

    /// hold the bid info and coin at user account
    struct Bid<phantom CoinType> has store {
        id: BidId,
        coin: Coin<CoinType>,
        offer_price: u64,
        expiration_sec: u64,
        config: PropertyMap,
    }

    /// This is the BidId used dedup the bid from the same signer for a listing
    struct BidId has copy, drop, store {
        bidder: address,
        listing_id: ID,
    }

    /// store all the bids by the user
    struct BidRecords<phantom CoinType> has key {
        records: Table<BidId, Bid<CoinType>>,
        bid_event: EventHandle<BidEvent<CoinType>>,
        withdraw_bid_event: EventHandle<WithdrawBidEvent<CoinType>>,
        order_executed_event: EventHandle<OrderExecutedEvent<CoinType>>,
        increase_bid_event: EventHandle<IncreaseBidEvent<CoinType>>,
    }

    struct BidEvent<phantom CoinType> has copy, drop, store {
        offer_price: u64,
        bid_id: BidId,
        expiration_sec: u64,
    }

    struct IncreaseBidEvent<phantom CoinType> has copy, drop, store {
        new_price: u64,
        bid_id: BidId,
    }

    struct WithdrawBidEvent<phantom CoinType> has copy, drop, store {
        bid_id: BidId,
    }

    struct OrderExecutedEvent<phantom CoinType> has copy, drop, store {
        buyer: address,
        lister_address: address,
        listing_creation_number: u64,
        executed_price: u64,
        market_place_address: address,
    }

    //
    // entry functions
    //

    /// Allow buyer to directly buy from a listing directly listed under an account without paying any fee
    public entry fun buy_from_owner_with_fee<CoinType>(
        buyer: &signer,
        lister_address: address,
        listing_creation_number: u64,
        market_fee_address: address,
        fee_numerator: u64,
        fee_denominator: u64,
    ) acquires BidRecords {
        let entry = listing_util::remove_listing<CoinType>(lister_address, listing_creation_number);
        buy_from_listing_with_fee<CoinType>(buyer, entry, market_fee_address, fee_numerator, fee_denominator);
    }

    /// Bidder can withdraw the bid after the bid expires to get the coin back and store them in the coinstore
    public entry fun withdraw_coin_from_bid<CoinType>(
        bidder: &signer,
        lister_addr: address,
        listing_creation_number: u64,
    ) acquires BidRecords {
        let bidder_address = signer::address_of(bidder);
        let listing_id = create_listing_id_raw(lister_addr, listing_creation_number);
        let bid_id = create_bid_id(bidder_address, listing_id);

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

    //
    // public functions
    //

    /// Buy from listings. This can be called by marketplace contracts with their own fee config and stored Listing
    public fun buy_from_listing_with_fee<CoinType>(
        buyer: &signer,
        entry: Listing<CoinType>,
        market_fund_address: address,
        fee_numerator: u64,
        fee_denominator: u64,
    ) acquires BidRecords {
        // assert the listing is active
        let (
            id,
            token_id,
            listed_amount,
            min_price,
            instant_sale,
            start_sec,
            expiration_sec,
            withdraw_cap,
            _,
        ) = listing_util::destroy_listing(entry);
        let now = timestamp::now_seconds();
        assert!(now > start_sec, error::invalid_argument(EBUY_FROM_NOT_STARTED_LISTING));
        assert!(now < expiration_sec, error::invalid_argument(EBUY_FROM_EXPIRED_LISTING));

        // listing is instant sale
        assert!(instant_sale, error::invalid_argument(EBUY_NON_INSTANT_SALE_LISTING));

        // assert the buyer has sufficient balance
        let buyer_addr = signer::address_of(buyer);
        let required_balance = min_price * listed_amount;
        // check bidder has sufficient balance
        assert!(coin::balance<CoinType>(buyer_addr) >= required_balance, error::invalid_argument(ENO_SUFFICIENT_FUND));
        initialize_bid_records<CoinType>(buyer);

        // swap the coin and token
        let token = token::withdraw_with_capability(
            withdraw_cap
        );
        token::deposit_token(buyer, token);

        let coins = coin::withdraw<CoinType>(buyer, required_balance);

        // deduct royalty fee from the transactions
        let royalty = token::get_royalty(token_id);
        let royalty_payee = token::get_royalty_payee(&royalty);
        let royalty_coin = deduct_fee<CoinType>(
            &mut coins,
            token::get_royalty_numerator(&royalty),
            token::get_royalty_denominator(&royalty)
        );
        coin::deposit(royalty_payee, royalty_coin);

        // deduct marketplace fee
        let market_fee = deduct_fee<CoinType>(&mut coins, fee_numerator, fee_denominator);
        coin::deposit(market_fund_address, market_fee);

        // give the remaining to the seller
        let token_owner = guid::id_creator_address(&id);
        coin::deposit(token_owner, coins);

        emit_order_executed_event<CoinType>(
            buyer_addr,
            token_owner,
            guid::id_creation_num(&id),
            min_price,
            market_fund_address,
        );
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
                    increase_bid_event: account::new_event_handle<IncreaseBidEvent<CoinType>>(bidder),
                    order_executed_event: account::new_event_handle<OrderExecutedEvent<CoinType>>(bidder),
                }
            );
        };
    }

    /// withdraw the coin and store them in bid struct and return a global unique bid id
    public fun bid<CoinType>(
        bidder: &signer,
        token_id: TokenId,
        token_amount:u64,
        offer_price: u64,
        entry: &Listing<CoinType>,
        expiration_sec: u64,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,

    ): BidId acquires BidRecords {
        initialize_bid_records<CoinType>(bidder);
        let bidder_address = signer::address_of(bidder);
        // check the bid is legit for the listing
        let total_coin_amount = offer_price * token_amount; // the total coin offerred by the bidder
        // check bidder has sufficient balance
        assert!(coin::balance<CoinType>(bidder_address) >= total_coin_amount, error::invalid_argument(ENO_SUFFICIENT_FUND));
        assert_bid_parameters(token_id, total_coin_amount, token_amount, entry, timestamp::now_seconds());


        // assert the bid_id not exist in the bid records
        initialize_bid_records<CoinType>(bidder);
        let bid_records = borrow_global_mut<BidRecords<CoinType>>(bidder_address);
        let bid_id = create_bid_id(bidder_address, listing_util::get_listing_id(entry));
        assert!(!table::contains(&bid_records.records,  bid_id), error::already_exists(EBID_ID_EXISTS));

        // withdraw the coin and store them in escrow to ensure the fund is avaliable until expiration_sec
        let coin = coin::withdraw<CoinType>(bidder, total_coin_amount);

        let bid = Bid<CoinType> {
            id: bid_id,
            coin,
            offer_price,
            expiration_sec,
            config: property_map::new(keys, values, types),
        };

        table::add(&mut bid_records.records, bid_id, bid);
        event::emit_event<BidEvent<CoinType>>(
            &mut bid_records.bid_event,
            BidEvent<CoinType> {
                offer_price,
                bid_id,
                expiration_sec,
            },
        );
        // opt-in direct transfer to receive token without signer
        token::opt_in_direct_transfer(bidder, true);

        bid_id
    }

    /// Allow the bid to increase the coin for an existing bid
    public fun increase_bid<CoinType>(
        bidder: &signer,
        bid_id: BidId,
        price_delta: u64,
        entry: &Listing<CoinType>,
    ) acquires BidRecords {
        let bidder_address = signer::address_of(bidder);

        let bid_records = borrow_global_mut<BidRecords<CoinType>>(bidder_address);
        assert!(table::contains(&bid_records.records,  bid_id), error::not_found(EBID_NOT_EXIST));

        let listing_id = listing_util::get_listing_id(entry);
        assert!(bid_id.listing_id == listing_id, error::invalid_argument(ELISTING_ID_NOT_MATCH));

        // check the bid is legit for the listing
        let token_amount =  listing_util::get_listing_token_amount(entry);
        let added_amount = price_delta * token_amount;
        // check bidder has sufficient balance
        assert!(coin::balance<CoinType>(bidder_address) >= added_amount, error::invalid_argument(ENO_SUFFICIENT_FUND));

        // add coin to the bid and update its info
        let added_coin = coin::withdraw<CoinType>(bidder, added_amount);
        let bid = table::borrow_mut(&mut bid_records.records, bid_id);
        bid.offer_price = bid.offer_price + price_delta;
        coin::merge(&mut bid.coin, added_coin);

        event::emit_event<IncreaseBidEvent<CoinType>>(
            &mut bid_records.increase_bid_event,
            IncreaseBidEvent<CoinType> {
                new_price: bid.offer_price,
                bid_id,
            },
        );
    }


    /// execute a bid to a listing, no signer required to perform this function
    /// pay fee to 3rd party based on a percentage
    /// deduct royalty and send to the payee account
    /// only the listing owner can execute the bid
    public fun execute_listing_bid<CoinType>(
        bid_id: BidId,
        entry: Listing<CoinType>,
        market_fund_address: address,
        fee_numerator: u64,
        fee_denominator: u64,
    ) acquires BidRecords {
        let bid_records = &mut borrow_global_mut<BidRecords<CoinType>>(bid_id.bidder).records;
        assert!(table::contains(bid_records, bid_id), error::not_found(EBID_NOT_EXIST));
        let bid = table::borrow(bid_records, bid_id);
        let (
            id,
            token_id,
            listed_amount,
            min_price,
            _,
            _,
            expiration_sec,
            withdraw_cap,
            _,
        ) = listing_util::destroy_listing(entry);
        let coin_owner = bid.id.bidder;
        // validate offerred amount and price
        let min_total = min_price * listed_amount;
        assert!(coin::value(&bid.coin) >= min_total, error::invalid_argument(ENO_SUFFICIENT_FUND));
        // validate expiration time
        let now = timestamp::now_seconds();
        assert!(now >= expiration_sec, error::invalid_argument(ELISTING_EXPIRED));
        //listing_id matches
        assert!(id == bid.id.listing_id, error::invalid_argument(ELISTING_ID_NOT_MATCH));

        // transfer coin and token
        let token = token::withdraw_with_capability(
            withdraw_cap
        );

        token::direct_deposit_with_opt_in(coin_owner, token);

        let bid_mut = table::remove(bid_records, bid_id);
        let offer_price = bid_mut.offer_price;
        let coins = clear_bid(bid_mut);

        // deduct royalty fee from the transactions
        let royalty = token::get_royalty(token_id);
        let royalty_payee = token::get_royalty_payee(&royalty);
        let royalty_coin = deduct_fee<CoinType>(
            &mut coins,
            token::get_royalty_numerator(&royalty),
            token::get_royalty_denominator(&royalty)
        );
        coin::deposit(royalty_payee, royalty_coin);

        // deduct marketplace fee
        let market_fee = deduct_fee<CoinType>(&mut coins, fee_numerator, fee_denominator);
        coin::deposit(market_fund_address, market_fee);

        // give the remaining to the seller
        let token_owner = guid::id_creator_address(&id);
        coin::deposit(token_owner, coins);

        emit_order_executed_event<CoinType>(
            coin_owner,
            token_owner,
            guid::id_creation_num(&id),
            offer_price,
            market_fund_address,
        );
    }

    /// validate if bid is legit for a listing.
    public fun assert_bid_parameters<CoinType>(
        token_id: TokenId,
        offer_price: u64,
        token_amount: u64,
        entry: &Listing<CoinType>,
        bid_time: u64,
    ) {
        // validate token_id match
        assert!(token_id == listing_util::get_listing_token_id(entry), error::invalid_argument(ETOKEN_ID_NOT_MATCH));
        // validate offerred amount and price
        let listed_amount =  listing_util::get_listing_token_amount(entry);
        let min_total = listing_util::get_listing_min_price(entry) * listed_amount;
        let total_coin_amount = offer_price * token_amount;
        assert!(total_coin_amount >= min_total, ENO_SUFFICIENT_FUND);
        assert!(token_amount == listed_amount, ETOKEN_AMOUNT_NOT_MATCH);
        assert!(bid_time >= listing_util::get_listing_start(entry), error::invalid_argument(ELISTING_NOT_STARTED));
        assert!(bid_time <= listing_util::get_listing_expiration(entry), error::invalid_argument(ELISTING_EXPIRED));
    }

    public fun get_bid_info<CoinType>(
       bid_id: BidId
    ): (u64, u64) acquires BidRecords {
        let bid_records = &mut borrow_global_mut<BidRecords<CoinType>>(bid_id.bidder).records;
        assert!(table::contains(bid_records, bid_id), error::not_found(EBID_NOT_EXIST));

        let bid = table::borrow(bid_records, bid_id);
        (bid.offer_price, bid.expiration_sec)
    }

    /// internal function for assigned a global unique id for a listing
    public fun create_bid_id(bidder: address, listing_id: ID): BidId {
        BidId {
            bidder,
            listing_id,
        }
    }

    /// get bidder address from BidId
    public fun get_bid_id_address(bid_id: &BidId): address {
        bid_id.bidder
    }

    /// get bidder listing id from BidId
    public fun get_bid_id_listing_id(bid_id: &BidId): ID {
        bid_id.listing_id
    }

    //
    // Private or friend functions
    //

    /// destruct the bid struct and extract coins
    fun clear_bid<CoinType>(bid: Bid<CoinType>): Coin<CoinType> {
        let Bid {
            id: _,
            coin,
            offer_price: _,
            expiration_sec: _,
            config: _
        } = bid;
        coin
    }

    fun emit_order_executed_event<CoinType>(
        buyer: address,
        lister_address: address,
        listing_creation_number: u64,
        executed_price: u64,
        market_place_address: address,
    ) acquires BidRecords {
        let records = borrow_global_mut<BidRecords<CoinType>>(buyer);
        event::emit_event<OrderExecutedEvent<CoinType>>(
            &mut records.order_executed_event,
            OrderExecutedEvent<CoinType> {
                buyer,
                lister_address,
                listing_creation_number,
                executed_price,
                market_place_address,
            },
        );
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

    #[test_only]
    public fun test_aution_setup(
        owner: &signer,
        bidder_a: &signer,
        aptos_framework: &signer,
        use_wrong_coin_amount: bool,
        use_wrong_token_amount: bool,
    ): (BidId, Listing<coin::FakeMoney>) acquires BidRecords {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test(11000000);

        account::create_account_for_test(signer::address_of(owner));
        account::create_account_for_test(signer::address_of(bidder_a));
        account::create_account_for_test(signer::address_of(aptos_framework));


        // owner creats a listing
        let token_id = token:: create_collection_and_token(
            owner,
            2,
            2,
            2,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        let entry = listing_util::create_listing<coin::FakeMoney>(
            owner,
            token_id,
            1,
            2,
            false,
            0,
            100,
            200,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
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
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
        );
        (bid_1, entry)
    }

    #[test(owner = @0xFE, bidder_a = @0xBC, aptos_framework = @aptos_framework)]
    public fun test_successful(
        owner: signer,
        bidder_a: signer,
        aptos_framework: signer
    ) acquires BidRecords {
        let (bid_id, entry) = test_aution_setup(
            &owner,
            &bidder_a,
            &aptos_framework,
            false,
            false,
        );
        let lister = listing_util::get_listing_creator(&entry);
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
        let (bid_id, entry) = test_aution_setup(
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
        let (bid_id, entry) = test_aution_setup(
            &owner,
            &bidder_a,
            &aptos_framework,
            false,
            true,
        );
        timestamp::update_global_time_for_test(100000000);
        execute_listing_bid(bid_id, entry, @aptos_framework, 0, 1);
    }

    #[test(owner = @0xAF, bidder_a = @0xBB, aptos_framework = @aptos_framework)]
    public fun test_increase_bid(
        owner: signer,
        bidder_a: signer,
        aptos_framework: signer
    ) acquires BidRecords {
        let (bid_id, entry) = test_aution_setup(
            &owner,
            &bidder_a,
            &aptos_framework,
            false,
            false,
        );
        increase_bid(&bidder_a, bid_id, 10, &entry);

        assert!(coin::balance<coin::FakeMoney>(signer::address_of(&bidder_a)) == 80, 1);
    }

    #[test_only]
    public fun test_instant_sale_setup(
        owner: &signer,
        buyer: &signer,
        aptos_framework: &signer,
        start_sec: u64,
        end_sec: u64,
    ): (Listing<coin::FakeMoney>, TokenId) {
        timestamp::set_time_has_started_for_testing(aptos_framework);

        account::create_account_for_test(signer::address_of(owner));
        account::create_account_for_test(signer::address_of(buyer));
        account::create_account_for_test(signer::address_of(aptos_framework));


        // owner creats a listing
        let token_id = token::create_collection_and_token(
            owner,
            2,
            2,
            2,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        let entry = listing_util::create_listing<coin::FakeMoney>(
            owner,
            token_id,
            1,
            100,
            true,
            start_sec,
            end_sec,
            end_sec + 1, // token transfer happens immedidately after buying
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
        );

        coin::create_fake_money(aptos_framework, buyer, 100);
        coin::transfer<coin::FakeMoney>(aptos_framework, signer::address_of(buyer), 100);
        //assert!(signer::address_of(&owner) == @0x1, 1);
        token::initialize_token_store(buyer);
        coin::register<coin::FakeMoney>(owner);
        (entry, token_id)
    }

    #[test(owner = @0xAF, buyer = @0xBB, framework = @aptos_framework, market = @0x33)]
    fun test_buy_successful(
        owner: &signer,
        buyer: &signer,
        framework: &signer,
        market: &signer,
    ) acquires BidRecords {
        account::create_account_for_test(signer::address_of(market));
        coin::register<coin::FakeMoney>(market);

        let (entry, token_id) = test_instant_sale_setup(owner, buyer, framework, 1, 10);
        timestamp::update_global_time_for_test(2000000);

        let owner_addr = signer::address_of(owner);
        let buyer_addr = signer::address_of(buyer);
        buy_from_listing_with_fee(
            buyer,
            entry,
            signer::address_of(market),
            1,
            100,
        );

        // assert the token and coin are transferred as expected
        assert!(token::balance_of(owner_addr, token_id) == 1, 1);
        assert!(token::balance_of(buyer_addr, token_id) == 1, 1);
        assert!(coin::balance<coin::FakeMoney>(buyer_addr) == 0, 1);
        // 1 % is paid as market fee
        assert!(coin::balance<coin::FakeMoney>(owner_addr) == 99, 1);
    }

    #[test(owner = @0x12, buyer = @0x34, framework = @aptos_framework)]
    #[expected_failure(abort_code = 65538)]
    fun test_buy_before_start(
        owner: &signer,
        buyer: &signer,
        framework: &signer,
    ) acquires BidRecords {
        let (entry, _) = test_instant_sale_setup(owner, buyer, framework, 1, 10);
        timestamp::update_global_time_for_test(0);

        buy_from_listing_with_fee(
            buyer,
            entry,
            signer::address_of(framework),
            1,
            100,
        );
    }

    #[test(owner = @0x12, buyer = @0x34, framework = @aptos_framework)]
    #[expected_failure(abort_code = 65547)]
    fun test_buy_after_expire(
        owner: &signer,
        buyer: &signer,
        framework: &signer,
    ) acquires BidRecords {
        let (entry, _) = test_instant_sale_setup(owner, buyer, framework, 1, 10);
        timestamp::update_global_time_for_test(30000000);

        buy_from_listing_with_fee(
            buyer,
            entry,
            signer::address_of(framework),
            1,
            100,
        );
    }

    #[test(owner = @0xAF, bidder_a = @0xBB, framework = @aptos_framework, buyer = @0xee)]
    #[expected_failure(abort_code = 65546)]
    fun test_buy_from_auction_listing(
        owner: &signer,
        bidder_a: &signer,
        framework: &signer,
        buyer: &signer,
    ) acquires BidRecords {
        let (_, entry) = test_aution_setup(
            owner,
            bidder_a,
            framework,
            false,
            false,
        );
        buy_from_listing_with_fee(
            buyer,
            entry,
            signer::address_of(framework),
            1,
            100,
        );
    }
}
