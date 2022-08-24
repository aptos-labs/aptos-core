module aptos_token::marketplace_utils {
    use std::signer;
    use std::string::String;
    use aptos_std::table::{Self, Table};
    use aptos_token::token::{Self, TokenId};
    use std::guid::{Self, ID};
    use aptos_framework::coin;
    use aptos_framework::timestamp;

    const EOWNER_NOT_HAVING_ENOUGH_TOKEN: u64 = 1;
    const ELISTING_NOT_EXIST:u64 = 2;
    const EINVALID_BUY_NOT_INSTANT_SALE: u64 = 3;
    const ELISTING_RECORDS_NOT_EXIST: u64 = 4;
    const EBUYER_NOT_HAVING_ENOUGH_COINS: u64 = 5;
    const EEXPIRED_LISTING: u64 = 6;

    /// immutable struct for recording listing info.
    struct Listing<phantom CoinType> has copy, drop, store {
        id: ID,
        owner: address,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        instant_sale: bool, // true for marketplace and false for auction
        expiration_sec: u64, // timestamp in secs for the listing expiration date
    }

    /// return a listing struct, marketplace owner can use this function to create a listing and store it in its inventory
    public fun create_list<CoinType>(
        owner: &signer,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        instant_sale: bool,
        expiration_sec: u64
    ): Listing<CoinType> {
        let owner_addr = signer::address_of(owner);
        assert!(token::balance_of(owner_addr, token_id) >= amount, EOWNER_NOT_HAVING_ENOUGH_TOKEN);

        Listing<CoinType> {
            id: create_listing_id(owner),
            owner: owner_addr,
            token_id,
            amount,
            min_price,
            instant_sale,
            expiration_sec,
        }
    }

    /// store listings on the owner's account
    struct ListingRecords<phantom CoinType> has key {
        records: Table<ID, Listing<CoinType>>
    }

    public fun initialize_listing_records<CoinType>(owner: &signer){
        let owner_addr = signer::address_of(owner);

        if (!exists<ListingRecords<CoinType>>(owner_addr)) {
            move_to(
                owner,
                ListingRecords<CoinType> {
                    records: table::new(),
                }
            );
        };
    }

    public fun create_list_under_user_account<CoinType>(
        owner: &signer,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        instant_sale: bool,
        expiration_sec: u64
    ): ID acquires ListingRecords {
        let owner_addr = signer::address_of(owner);
        let record = create_list<CoinType>(owner, token_id, amount, min_price, instant_sale, expiration_sec);
        initialize_listing_records<CoinType>(owner);
        let records = borrow_global_mut<ListingRecords<CoinType>>(owner_addr);

        let id = create_listing_id(owner);
        // add a new record to the listing
        table::add(&mut records.records, id, record);
        id
    }

    /// creator uses this function to directly list token for sale under their own accounts
    public entry fun direct_listing<CoinType>(
        owner: &signer,
        creator: address,
        collection_name: String,
        token_name: String,
        property_version: u64,
        amount: u64,
        min_price: u64,
        instant_sale: bool, // indicate if this listing is for sale or for auction
        expiration_sec: u64
    ) acquires ListingRecords {
        let token_id = token::create_token_id_raw(creator, collection_name, token_name, property_version);
        create_list_under_user_account<CoinType>(owner, token_id, amount, min_price, instant_sale, expiration_sec);
    }

    /// remove a listing for the direct listing records
    public entry fun cancel_direct_list<CoinType>(
        owner: &signer,
        listing_id_creation_number: u64
    ) acquires ListingRecords {
        let listing_id = guid::create_id(signer::address_of(owner), listing_id_creation_number);
        let owner_addr = signer::address_of(owner);
        let records = &mut borrow_global_mut<ListingRecords<CoinType>>(owner_addr).records;
        assert!(table::contains(records, listing_id), ELISTING_NOT_EXIST);
        table::remove(records, listing_id);
    }

    /// internal function for assigned a global unique id for a listing
    fun create_listing_id(owner: &signer): ID {
        let gid = guid::create(owner);
        guid::id(&gid)
    }

    public fun buy_internal<CoinType>(coin_owner: &signer, listing: Listing<CoinType>): u64 {
        assert!(listing.instant_sale, EINVALID_BUY_NOT_INSTANT_SALE);
        let coin_owner_address = signer::address_of(coin_owner);

        let total_amount = listing.min_price * listing.amount;
        assert!(timestamp::now_seconds() <= listing.expiration_sec, EEXPIRED_LISTING);
        assert!(coin::balance<CoinType>(coin_owner_address) >= total_amount, EBUYER_NOT_HAVING_ENOUGH_COINS);

        let token = token::withdraw_with_event_internal(listing.owner, listing.token_id, listing.amount);
        coin::transfer<CoinType>(coin_owner, listing.owner, total_amount);
        token::direct_deposit(signer::address_of(coin_owner), token);
        total_amount
    }

    public entry fun buy<CoinType>(coin_owner: &signer, token_owner_address: address, id_creation_number: u64) acquires ListingRecords {
        assert!(exists<ListingRecords<CoinType>>(token_owner_address), ELISTING_RECORDS_NOT_EXIST);
        let token_owner_records = &mut borrow_global_mut<ListingRecords<CoinType>>(token_owner_address).records;
        let listing_id = guid::create_id(token_owner_address, id_creation_number);
        assert!(table::contains(token_owner_records, listing_id), ELISTING_NOT_EXIST);
        let listing = table::borrow(token_owner_records, listing_id);
        buy_internal(coin_owner, *listing);
    }

    public fun get_listing_id<CoinType>(list: &Listing<CoinType>): ID {
        list.id
    }

    public fun get_listing_id_tuple<CoinType>(list: &Listing<CoinType>): (u64, address) {
        let id = list.id;
        (guid::id_creation_num(&id), guid::id_creator_address(&id))
    }

    public fun get_listing_owner<CoinType>(list: &Listing<CoinType>): address {
        list.owner
    }

    public fun get_listing_token_id<CoinType>(list: &Listing<CoinType>): TokenId {
        list.token_id
    }

    public fun get_listing_expiration<CoinType>(list: &Listing<CoinType>): u64 {
        list.expiration_sec
    }

    public fun get_listing_min_price<CoinType>(list: &Listing<CoinType>): u64 {
        list.min_price
    }

    public fun get_listing_token_amount<CoinType>(list: &Listing<CoinType>): u64 {
        list.amount
    }

    public fun get_listing_instant_sale<CoinType>(list: &Listing<CoinType>): bool {
        list.instant_sale
    }

    #[test(owner = @0xAF)]
    public fun test_cancel_listing(owner: signer) acquires ListingRecords {
        use aptos_framework::coin;

        let token_id = token::create_collection_and_token(&owner, 2, 2, 2);
        let listing_id = create_list_under_user_account<coin::FakeMoney>(
            &owner,
            token_id,
            1,
            1,
            false,
            10000
        );
        cancel_direct_list<coin::FakeMoney>(&owner, guid::id_creation_num(&listing_id));
    }

    #[test_only]
    public fun set_up_buy_test(owner: signer, buyer: signer, aptos_framework: signer, min_price: u64, instant_sale: bool, valid_listing_id: bool, expiration_sec: u64): (u64, TokenId) acquires ListingRecords {
        timestamp::set_time_has_started_for_testing(&aptos_framework);
        timestamp::update_global_time_for_test(11000000);

        let buyer_addr = signer::address_of(&buyer);
        coin::create_fake_money(&aptos_framework, &buyer, 100);
        coin::transfer<coin::FakeMoney>(&aptos_framework, buyer_addr, 100);
        coin::register_for_test<coin::FakeMoney>(&owner);
        token::initialize_token_store(&buyer);

        let token_id = token::create_collection_and_token(&owner, 2, 2, 2);
        let listing_id = create_list_under_user_account<coin::FakeMoney>(
            &owner,
            token_id,
            1,
             min_price,
            instant_sale,
            expiration_sec,
        );
        let id = guid::id_creation_num(&listing_id);

        if (valid_listing_id) {
            buy<coin::FakeMoney>(&buyer, signer::address_of(&owner), id);
        } else {
            buy<coin::FakeMoney>(&buyer, signer::address_of(&owner), 1);
        };

        (id, token_id)
    }

    #[test(owner = @0xAF, buyer = @0xAB, aptos_framework = @aptos_framework)]
    public fun test_successful_buy(owner: signer, buyer: signer, aptos_framework: signer) acquires ListingRecords {
        let buyer_addr = signer::address_of(&buyer);
        let (_, token_id) = set_up_buy_test(owner, buyer, aptos_framework, 1, true, true, 100);

        assert!(coin::balance<coin::FakeMoney>(buyer_addr) == 99, 0);
        assert!(token::balance_of(buyer_addr, token_id) == 1, 1);
    }

    #[test(owner = @0xAF, buyer = @0xAB, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 2)]
    public fun test_failed_buy_invalid_listing_id(owner: signer, buyer: signer, aptos_framework: signer) acquires ListingRecords {
        set_up_buy_test(owner, buyer, aptos_framework, 1, true, false, 100);
    }

    #[test(owner = @0xAF, buyer = @0xAB, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 3)]
    public fun test_failed_buy_not_instant_sale(owner: signer, buyer: signer, aptos_framework: signer) acquires ListingRecords {
        set_up_buy_test(owner, buyer, aptos_framework, 1, false, true, 100);
    }

    #[test(owner = @0xAF, buyer = @0xAB, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 5)]
    public fun test_failed_buy_insufficient_balance(owner: signer, buyer: signer, aptos_framework: signer) acquires ListingRecords {
       set_up_buy_test(owner, buyer, aptos_framework, 1000, true, true, 100);
    }

    #[test(owner = @0xAF, buyer = @0xAB, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 6)]
    public fun test_failed_buy_expired_listing(owner: signer, buyer: signer, aptos_framework: signer) acquires ListingRecords {
        set_up_buy_test(owner, buyer, aptos_framework, 1, true, true, 1);
    }
}
