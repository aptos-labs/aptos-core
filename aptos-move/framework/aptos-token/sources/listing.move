module aptos_token::listing {
    use std::signer;
    use std::string::String;
    use aptos_std::table::{Self, Table};
    use aptos_token::token::{Self, TokenId};
    use aptos_std::guid::{Self, ID};
    use aptos_framework::account;

    const EOWNER_NOT_HAVING_ENOUGH_TOKEN: u64 = 1;
    const ELISTING_NOT_EXIST:u64 = 2;

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
    }

    /// return a listing struct, marketplace owner can use this function to create a listing and store it in its inventory
    public fun create_list<CoinType>(
        owner: &signer,
        token_id: TokenId,
        amount: u64,
        min_price: u64,
        instant_sale: bool,
        start_sec: u64,
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
            start_sec,
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
        start_sec: u64,
        expiration_sec: u64
    ): ID acquires ListingRecords {
        let owner_addr = signer::address_of(owner);
        let record = create_list<CoinType>(
            owner,
            token_id,
            amount,
            min_price,
            instant_sale,
            start_sec,
            expiration_sec
        );
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
        start_sec: u64,
        expiration_sec: u64
    ) acquires ListingRecords {
        let token_id = token::create_token_id_raw(creator, collection_name, token_name, property_version);
        create_list_under_user_account<CoinType>(owner, token_id, amount, min_price, instant_sale, start_sec, expiration_sec);
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

    #[test(owner = @0xAF)]
    public fun test_cancel_listing(owner: signer)acquires ListingRecords {
        use aptos_framework::coin;

        let token_id = token::create_collection_and_token(&owner, 2, 2, 2);
        let listing_id = create_list_under_user_account<coin::FakeMoney>(
            &owner,
            token_id,
            1,
            1,
            false,
            0,
            10000
        );
        cancel_direct_list<coin::FakeMoney>(&owner, guid::id_creation_num(&listing_id));
    }

}
