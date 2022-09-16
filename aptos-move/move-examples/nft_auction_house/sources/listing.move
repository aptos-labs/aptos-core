module auction_house::listing {
    use std::error;
    use std::signer;
    use std::string::String;
    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_std::table::{Self, Table};
    use aptos_std::guid::{Self, ID};
    use aptos_token::token::{Self, TokenId, WithdrawCapability};


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

    /// immutable struct for recording listing info.
    struct Listing<phantom CoinType> has drop, store {
        id: ID,
        owner: address,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        instant_sale: bool, // true for marketplace and false for auction
        start_sec: u64, // timestamp in secs for the listing starting time
        expiration_sec: u64, // timestamp in secs for the listing expiration time
        withdraw_cap: WithdrawCapability,
    }

    /// return a listing struct, marketplace owner can use this function to create a listing and store it in its inventory
    public fun create_list<CoinType>(
        owner: &signer,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        instant_sale: bool,
        start_sec: u64,
        listing_expiration_sec: u64,
        withdraw_expiration_sec: u64
    ): Listing<CoinType> {
        let owner_addr = signer::address_of(owner);
        assert!(listing_expiration_sec > start_sec, error::invalid_argument(ESTART_TIME_LARGER_THAN_EXPIRE_TIME));
        assert!(token::balance_of(owner_addr, token_id) >= amount, error::invalid_argument(EOWNER_NOT_HAVING_ENOUGH_TOKEN));
        assert!(withdraw_expiration_sec > listing_expiration_sec, error::invalid_argument(EWITHDRAW_EXPIRE_TIME_SHORT_THAN_LISTING_TIME));
        Listing<CoinType> {
            id: create_listing_id(owner),
            owner: owner_addr,
            token_id,
            amount,
            min_price,
            instant_sale,
            start_sec,
            expiration_sec: listing_expiration_sec,
            withdraw_cap: token::create_withdraw_capability(owner, token_id, amount, withdraw_expiration_sec),
        }
    }

    struct ListingEvent has copy, drop, store {
        id: ID,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        instant_sale: bool,
        start_sec: u64,
        expiration_sec: u64,
        market_address: address,
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
        let record = create_list<CoinType>(
            owner,
            token_id,
            amount,
            min_price,
            instant_sale,
            start_sec,
            expiration_sec,
            withdraw_expiration_sec,
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
                market_address: owner_addr,
            },
        );
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
    public entry fun cancel_direct_list<CoinType>(
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

    public fun destroy_listing<CoinType>(entry: Listing<CoinType>): (
        ID,
        address,
        TokenId,
        u64,
        u64,
        bool,
        u64,
        u64,
        WithdrawCapability
    ){
        let Listing {
            id,
            owner,
            token_id,
            amount,
            min_price,
            instant_sale,
            start_sec,
            expiration_sec,
            withdraw_cap,
        } = entry;
        (id, owner, token_id, amount, min_price, instant_sale, start_sec, expiration_sec, withdraw_cap)
    }

    /// internal function for assigned a global unique id for a listing
    fun create_listing_id(owner: &signer): ID {
        let gid = account::create_guid(owner);
        guid::id(&gid)
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
        market_address: address,
    ): ListingEvent {
        ListingEvent {
            id, token_id, amount, min_price, instant_sale, start_sec, expiration_sec, market_address
        }
    }


    #[test(owner = @0xAF)]
    public fun test_cancel_listing(owner: signer)acquires ListingRecords {
        use aptos_framework::coin;

        account::create_account_for_test(signer::address_of(&owner));
        let token_id = token::create_collection_and_token(&owner, 2, 2, 2);
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
        cancel_direct_list<coin::FakeMoney>(&owner, guid::id_creation_num(&listing_id));
    }

}
