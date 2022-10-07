/// An marketplace library providing basic function for listing NFTs
/// To see how to use the library, please check the two example contract in the same folder
module marketplace::marketplace_listing_utils {
    use std::error;
    use std::signer;
    use std::string::String;
    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_std::table::{Self, Table};
    use aptos_std::guid::{Self, ID};
    use aptos_token::token::{Self, TokenId, WithdrawCapability};
    use aptos_token::property_map::{Self, PropertyMap};

    friend marketplace::marketplace_bid_utils;


    //
    // Errors
    //

    /// Not enough token to list
    const EOWNER_NOT_HAVING_ENOUGH_TOKEN: u64 = 1;

    /// Listing doesn't exist
    const ELISTING_NOT_EXIST:u64 = 2;

    /// Withdraw time should be longer than listing time
    const EWITHDRAW_EXPIRE_TIME_SHORT_THAN_LISTING_TIME: u64 = 3;

    /// Start time should be less than expire time
    const ESTART_TIME_LARGER_THAN_EXPIRE_TIME: u64 = 4;

    /// Listing zero token
    const ELISTING_ZERO_TOKEN: u64 = 5;


    /// immutable struct for recording listing info.
    struct Listing<phantom CoinType> has drop, store {
        id: ID,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        instant_sale: bool, // true for marketplace and false for auction
        start_sec: u64, // timestamp in secs for the listing starting time
        expiration_sec: u64, // timestamp in secs for the listing expiration time
        withdraw_cap: WithdrawCapability,
        config: PropertyMap,
    }

    struct ListingEvent has copy, drop, store {
        id: ID,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        instant_sale: bool,
        start_sec: u64,
        expiration_sec: u64,
        withdraw_sec: u64,
        market_address: address,
        config: PropertyMap,
    }

    struct CancelListingEvent has copy, drop, store {
        id: ID,
        market_address: address,
    }

    /// store listings on the owner's account
    struct ListingRecords<phantom CoinType> has key {
        records: Table<ID, Listing<CoinType>>,
        listing_event: EventHandle<ListingEvent>,
        cancel_listing_event: EventHandle<CancelListingEvent>,
    }

    //
    // entry functions
    //

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
        start_sec: u64,
        expiration_sec: u64,
        withdraw_expiration_sec: u64,
    ) acquires ListingRecords {
        let token_id = token::create_token_id_raw(creator, collection_name, token_name, property_version);
        create_list_under_user_account<CoinType>(
            owner,
            token_id,
            amount,
            min_price,
            instant_sale,
            start_sec,
            expiration_sec,
            withdraw_expiration_sec,
        );
    }

    /// remove a listing for the direct listing records
    public entry fun cancel_direct_listing<CoinType>(
        owner: &signer,
        listing_id_creation_number: u64
    ) acquires ListingRecords {
        let listing_id = guid::create_id(signer::address_of(owner), listing_id_creation_number);
        let owner_addr = signer::address_of(owner);
        let records = borrow_global_mut<ListingRecords<CoinType>>(owner_addr);
        assert!(table::contains(&records.records, listing_id), error::not_found(ELISTING_NOT_EXIST));
        table::remove(&mut records.records, listing_id);

        event::emit_event<CancelListingEvent>(
            &mut records.cancel_listing_event,
            CancelListingEvent {
                id: listing_id,
                market_address: signer::address_of(owner),
            },
        );
    }

    //
    // public functions
    //

    /// Return a listing struct, marketplace owner can use this function to create a listing and store it in its inventory
    public fun create_listing<CoinType>(
        owner: &signer,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        instant_sale: bool,
        start_sec: u64,
        listing_expiration_sec: u64,
        withdraw_expiration_sec: u64, // The end time when the listed token can be withdrawn.
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ): Listing<CoinType> {
        let owner_addr = signer::address_of(owner);
        assert!(listing_expiration_sec > start_sec, error::invalid_argument(ESTART_TIME_LARGER_THAN_EXPIRE_TIME));
        assert!(token::balance_of(owner_addr, token_id) >= amount, error::invalid_argument(EOWNER_NOT_HAVING_ENOUGH_TOKEN));
        assert!(withdraw_expiration_sec > listing_expiration_sec, error::invalid_argument(EWITHDRAW_EXPIRE_TIME_SHORT_THAN_LISTING_TIME));
        assert!(amount > 0, error::invalid_argument(ELISTING_ZERO_TOKEN));
        Listing<CoinType> {
            id: create_listing_id(owner),
            token_id,
            amount,
            min_price,
            instant_sale,
            start_sec,
            expiration_sec: listing_expiration_sec,
            withdraw_cap: token::create_withdraw_capability(owner, token_id, amount, withdraw_expiration_sec),
            config: property_map::new(keys, values, types),
        }
    }

    public fun initialize_listing_records<CoinType>(owner: &signer){
        let owner_addr = signer::address_of(owner);

        if (!exists<ListingRecords<CoinType>>(owner_addr)) {
            move_to(
                owner,
                ListingRecords<CoinType> {
                    records: table::new(),
                    listing_event: account::new_event_handle<ListingEvent>(owner),
                    cancel_listing_event: account::new_event_handle<CancelListingEvent>(owner),
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
        start_sec: u64,
        expiration_sec: u64,
        withdraw_expiration_sec: u64,
    ): ID acquires ListingRecords {
        let owner_addr = signer::address_of(owner);
        let record = create_listing<CoinType>(
            owner,
            token_id,
            amount,
            min_price,
            instant_sale,
            start_sec,
            expiration_sec,
            withdraw_expiration_sec,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
        );
        initialize_listing_records<CoinType>(owner);
        let records = borrow_global_mut<ListingRecords<CoinType>>(owner_addr);

        let id = create_listing_id(owner);
        // add a new record to the listing
        table::add(&mut records.records, id, record);
        event::emit_event<ListingEvent>(
            &mut records.listing_event,
            ListingEvent {
                id,
                token_id,
                amount,
                min_price,
                instant_sale,
                start_sec,
                expiration_sec,
                withdraw_sec: withdraw_expiration_sec,
                market_address: owner_addr,
                config: property_map::empty(),
            },
        );
        id
    }

    public fun destroy_listing<CoinType>(entry: Listing<CoinType>): (
        ID,
        TokenId,
        u64,
        u64,
        bool,
        u64,
        u64,
        WithdrawCapability,
        PropertyMap,
    ){
        let Listing {
            id,
            token_id,
            amount,
            min_price,
            instant_sale,
            start_sec,
            expiration_sec,
            withdraw_cap,
            config,
        } = entry;
        (id, token_id, amount, min_price, instant_sale, start_sec, expiration_sec, withdraw_cap, config)
    }

    /// util function for constructing the listing id from raw fields
    public fun create_listing_id_raw(lister: address, listing_creation_number: u64): ID {
        guid::create_id(lister, listing_creation_number)
    }

    public fun get_listing_id<CoinType>(list: &Listing<CoinType>): ID {
        list.id
    }

    public fun get_listing_id_tuple<CoinType>(list: &Listing<CoinType>): (u64, address) {
        let id = list.id;
        (guid::id_creation_num(&id), guid::id_creator_address(&id))
    }

    public fun get_listing_creator<CoinType>(list: &Listing<CoinType>): address {
        guid::id_creator_address(&list.id)
    }

    public fun get_listing_token_id<CoinType>(list: &Listing<CoinType>): TokenId {
        list.token_id
    }

    public fun get_listing_expiration<CoinType>(list: &Listing<CoinType>): u64 {
        list.expiration_sec
    }

    public fun get_listing_start<CoinType>(list: &Listing<CoinType>): u64 {
        list.start_sec
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

    public fun create_listing_event(
        id: ID,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        instant_sale: bool,
        start_sec: u64,
        expiration_sec: u64,
        withdraw_sec: u64,
        market_address: address,
        config: PropertyMap
    ): ListingEvent {
        ListingEvent {
            id, token_id, amount, min_price, instant_sale, start_sec, expiration_sec, withdraw_sec, market_address, config
        }
    }

    /// Get the read-only listing reference from listing stored on user account.
    public fun get_listing_info<CoinType>(
        lister_address: address,
        listing_creation_number: u64
    ): (TokenId, u64, u64, bool, u64, u64) acquires ListingRecords {
        let listing_id = guid::create_id(lister_address, listing_creation_number);
        let records = borrow_global_mut<ListingRecords<CoinType>>(lister_address);
        assert!(table::contains(&records.records, listing_id), error::not_found(ELISTING_NOT_EXIST));
        let listing = table::borrow(&records.records, listing_id);
        (
            listing.token_id,
            listing.amount,
            listing.min_price,
            listing.instant_sale,
            listing.start_sec,
            listing.expiration_sec,
        )
    }

    //
    // Private or friend functions
    //

    /// internal function for creating a new unique id for a listing
    fun create_listing_id(owner: &signer): ID {
        let gid = account::create_guid(owner);
        guid::id(&gid)
    }

    /// Get the listing struct which contains withdraw_capability
    /// This function should stay friend to prevent Listing be exposed to un-trusted module
    public(friend) fun remove_listing<CoinType>(lister_address: address, listing_creation_number: u64): Listing<CoinType> acquires ListingRecords {
        let listing_id = guid::create_id(lister_address, listing_creation_number);
        let records = borrow_global_mut<ListingRecords<CoinType>>(lister_address);
        assert!(table::contains(&records.records, listing_id), error::not_found(ELISTING_NOT_EXIST));
        table::remove(&mut records.records, listing_id)
    }


    #[test(owner = @0xAF)]
    public fun test_cancel_listing(owner: signer)acquires ListingRecords {
        use aptos_framework::coin;

        account::create_account_for_test(signer::address_of(&owner));
        let token_id = token::create_collection_and_token(
            &owner,
            1,
            2,
            1,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        let listing_id = create_list_under_user_account<coin::FakeMoney>(
            &owner,
            token_id,
            1,
            1,
            false,
            0,
            10000,
            10001,
        );
        cancel_direct_listing<coin::FakeMoney>(&owner, guid::id_creation_num(&listing_id));
    }

}
