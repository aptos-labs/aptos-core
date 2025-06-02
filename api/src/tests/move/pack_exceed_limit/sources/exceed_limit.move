module addr::exceed_limit {
    use std::option::Option;
    use aptos_framework::account::Account;
    use std::signer;
    use std::string::{Self};
    use std::simple_map::{SimpleMap, Self};
    use std::vector;

    use aptos_token::token;
    use aptos_token::token::TokenId;

    // This struct is too large and exceeds the maximum allowed number of type nodes.
    enum MyStructEnum has key, store {
        V1 { x: MyStruct },
        V2 { x: MyStruct, y: u64 },
        V3 { x: MyStruct, y: u64, z: u64 },
    }

    struct MyStruct has key, store {
        token_owner: SimpleMap<TokenId, Account>,
        owners: vector<address>,
        game_servers: vector<address>,
        minimal_fee_rate: u32,
        epoch_minimal_interval: u64,
        max_bug_per_session: u64,
        token_types: SimpleMap<TokenId, u8>,
        token_health: SimpleMap<TokenId, u128>,
        percent_for_track: SimpleMap<TokenId, u32>,
        price_main_coin_usd: u256,
        one_main_coin: u256,
        game_sessions: SimpleMap<TokenId, GameSession>,
        game_bids: SimpleMap<TokenId, TrackGameBid>,
        previous_owners: SimpleMap<TokenId, address>,
        bug_owners: SimpleMap<TokenId, TokenInfo>,
        tracks_owners: SimpleMap<TokenId, TokenInfo>,
        latest_epoch_update: u64,
        counter: u256,
    }

    struct TokenInfo has key, store {
        owner_id: Option<address>,
        token_type: Option<u8>,
        active_session: Option<TokenId>,
        collected_fee: u256,
    }

    struct GameSessionBug has key, store {
        bug_owner_id: Option<address>,
        bug_token_id: TokenId,
        last_track_time_result: u64,
    }

    struct GameBid has key, store {
        amount: u256,
        bug: TokenId,
        timestamp: u64,
        bidder: Option<address>,
    }

    struct TrackGameBid has key, store {
        game_bids: vector<GameBid>,
    }

    struct EpochPayment has key, store {
        track_token_id: TokenId,
        bug_token_id: Option<TokenId>,
        receiver_type: u8,
        amount: u256,
        receiver_id: Option<address>,
    }

    struct GameSession has key, store {
        init_time: u128,
        track_token_id: TokenId,
        bug: vector<GameSessionBug>,
        latest_update_time: u64,
        latest_track_time_result: u64,
        attempts: u8,
        game_bids_sum: u256,
        game_fees_sum: u256,
        current_winner_bug: Option<GameSessionBug>,
        epoch_payment: vector<EpochPayment>,
        max_bug_per_session: u64,
    }


    fun init_module(source_account: &signer) {
        let list = vector::empty<address>();
        let account_addr = signer::address_of(source_account);
        vector::push_back(&mut list, account_addr);
        let x = MyStruct {
            token_owner: simple_map::create(),
            owners: list,
            game_servers: list,
            minimal_fee_rate: 1,
            epoch_minimal_interval: 1,
            max_bug_per_session: 1000,
            token_types: simple_map::create(),
            token_health: simple_map::create(),
            percent_for_track: simple_map::create(),
            price_main_coin_usd: 1,
            one_main_coin: 1,
            game_sessions: simple_map::create(),
            game_bids: simple_map::create(),
            previous_owners: simple_map::create(),
            bug_owners: simple_map::create(),
            tracks_owners: simple_map::create(),
            latest_epoch_update: 0,
            counter: 0,
        };
        move_to(
            source_account,
            MyStructEnum::V3 { x, y: 1, z: 2 },
        );
    }

    fun mint_nft(source_account: &signer) {
        let collection_name = string::utf8(b"Collection name");
        let description = string::utf8(b"Description");
        let collection_uri = string::utf8(b"Collection uri");
        let maximum_supply = 0;
        let mutate_setting = vector<bool>[ false, false, false ];
        token::create_collection(
            source_account,
            collection_name,
            description,
            collection_uri,
            maximum_supply,
            mutate_setting
        );
    }
}
