module tournament::matchmaker {
    use std::option::{Self, Option};
    use std::signer;
    use std::vector;
    use aptos_std::table_with_length::{Self, TableWithLength};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::transaction_context;

    use aptos_token_objects::token::Token;

    use tournament::object_refs;
    use tournament::room;

    friend tournament::round;

    #[test_only]
    friend tournament::main_unit_test;


    /// Attempted to matchmake for a tournament that the signer does not own
    const ENOT_MATCHMAKER_OWNER: u64 = 0;

    /// Joining is not allowed
    const EJOINING_NOT_ALLOWED: u64 = 1;

    // How many buckets we want to use to sort players into, for parallelism
    const NUM_BUCKETS: u8 = 10;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct MatchMaker<phantom GameType> has key {
        min_players_per_room: u64,
        max_players_per_room: u64,
        joining_allowed: bool,
        // Only one of the two below will be Some
        unlimited_room_address: Option<address>,
        user_buckets: Option<TableWithLength<u8, vector<Object<Token>>>>
    }

    public(friend) fun create_unlimited_matchmaker<GameType>(
        owner: &signer,
    ): (signer, Option<signer>) {
        // Create the unlimited room
        let room_signer = room::create_room<GameType>(owner, false);

        let matchmaker = MatchMaker<GameType> {
            min_players_per_room: 0,
            max_players_per_room: 0,
            joining_allowed: true,
            unlimited_room_address: option::some(signer::address_of(&room_signer)),
            user_buckets: option::none(),
        };

        // Create the matchmaker
        let matchmaker_signer = create_matchmaker_inner(
            owner,
            matchmaker,
        );

        (matchmaker_signer, option::some(room_signer))
    }

    public(friend) fun create_limited_matchmaker<GameType>(
        owner: &signer,
        min_players_per_room: u64,
        max_players_per_room: u64,
    ): (signer, Option<signer>) {
        let user_buckets = table_with_length::new();

        let i: u8 = 0;
        while (i < NUM_BUCKETS) {
            table_with_length::add(&mut user_buckets, i, vector::empty<Object<Token>>());
            i = i + 1;
        };

        let matchmaker = MatchMaker<GameType> {
            min_players_per_room,
            max_players_per_room,
            joining_allowed: true,
            unlimited_room_address: option::none(),
            user_buckets: option::some(user_buckets),
        };

        let matchmaker_signer = create_matchmaker_inner(
            owner,
            matchmaker,
        );

        (matchmaker_signer, option::none())
    }

    fun create_matchmaker_inner<GameType>(
        owner: &signer,
        matchmaker: MatchMaker<GameType>
    ): signer {
        let tournament_addr = signer::address_of(owner);
        let constructor_ref = object::create_object(tournament_addr);
        let (mm_obj_signer, _mm_obj_addr) = object_refs::create_refs<MatchMaker<GameType>>(&constructor_ref);

        move_to(&mm_obj_signer, matchmaker);
        mm_obj_signer
    }

    public(friend) fun destroy_matchmaker<GameType>(
        matchmaker_address: address,
    ) acquires MatchMaker {
        let matchmaker = object::address_to_object<MatchMaker<GameType>>(matchmaker_address);

        let matchmaker_addr = object::object_address(&matchmaker);
        let MatchMaker<GameType> {
            min_players_per_room: _,
            max_players_per_room: _,
            joining_allowed: _,
            unlimited_room_address: _,
            user_buckets,
        } = move_from<MatchMaker<GameType>>(matchmaker_addr);


        // If we allocated user buckets, de-allocate them now
        if (option::is_some(&user_buckets)) {
            let user_buckets = option::extract(&mut user_buckets);
            let i = 0;
            while (i < NUM_BUCKETS) {
                table_with_length::remove(&mut user_buckets, i);
                i = i + 1;
            };
            table_with_length::destroy_empty(user_buckets);
        };
        option::destroy_none(user_buckets);


        object_refs::destroy_object(matchmaker_addr)
    }

    public(friend) fun allow_joining<GameType>(
        matchmaker_address: address,
    ) acquires MatchMaker {
        let matchmaker = borrow_global_mut<MatchMaker<GameType>>(matchmaker_address);
        matchmaker.joining_allowed = true;
    }

    public(friend) fun disallow_joining<GameType>(
        matchmaker_address: address,
    ) acquires MatchMaker {
        let matchmaker = borrow_global_mut<MatchMaker<GameType>>(matchmaker_address);
        matchmaker.joining_allowed = false;
    }

    // Uses the current transaction hash to get the bucket number
    public(friend) fun get_bucket_num(): u8 {
        let hash = transaction_context::get_transaction_hash();
        let last_u8 = vector::pop_back(&mut hash);
        last_u8 % NUM_BUCKETS
    }

    public(friend) fun add_players<GameType>(
        owner: &signer,
        matchmaker_address: address,
        players: vector<Object<Token>>,
    ): Option<vector<signer>> acquires MatchMaker {
        let matchmaker = object::address_to_object<MatchMaker<GameType>>(matchmaker_address);

        let matchmaker_addr = object::object_address(&matchmaker);
        let matchmaker = borrow_global_mut<MatchMaker<GameType>>(matchmaker_addr);

        assert!(matchmaker.joining_allowed, EJOINING_NOT_ALLOWED);

        let room_signers = option::none<vector<signer>>();

        let is_limited = option::is_some(&matchmaker.user_buckets);
        if (is_limited) {
            let table = option::borrow_mut(&mut matchmaker.user_buckets);
            let bucket_num = get_bucket_num();
            let bucket = table_with_length::borrow_mut(table, bucket_num);
            vector::append(bucket, players);
            if (vector::length(bucket) >= matchmaker.max_players_per_room) {
                // Time to create some new rooms!
                room_signers = option::some(
                    create_rooms_for_players<GameType>(owner, bucket, matchmaker.max_players_per_room)
                );
            }
        } else {
            let room_addr = option::borrow(&matchmaker.unlimited_room_address);
            room::add_players<GameType>(*room_addr, players);
        };

        room_signers
    }

    inline fun create_rooms_for_players<GameType>(
        owner: &signer,
        bucket: &mut vector<Object<Token>>,
        players_per_room: u64,
    ): vector<signer> {
        let room_signers = vector::empty<signer>();
        while (vector::length(bucket) >= players_per_room) {
            let room_signer = room::create_room<GameType>(owner, true);
            let room_address = signer::address_of(&room_signer);
            vector::push_back(&mut room_signers, room_signer);
            let length = vector::length(bucket);
            let to_add = vector::trim_reverse(bucket, length - players_per_room);
            room::add_players<GameType>(room_address, to_add);
        };
        room_signers
    }

    public(friend) fun finish_matchmaking<GameType>(
        owner: &signer,
        matchmaker_address: address,
    ): (Option<vector<signer>>, vector<Object<Token>>) acquires MatchMaker {
        let matchmaker = borrow_global_mut<MatchMaker<GameType>>(matchmaker_address);
        matchmaker.joining_allowed = false;
        // Iterate through all buckets, if we need to, and kick them into rooms
        // Any players returned get a bye!
        if (option::is_some(&matchmaker.user_buckets)) {
            let user_buckets = option::borrow_mut(&mut matchmaker.user_buckets);
            let i = 0;
            let room_signers = vector::empty<signer>();
            let all_players = vector::empty<Object<Token>>();
            let table_length = table_with_length::length(user_buckets);
            while (i < table_length) {
                let bucket = table_with_length::borrow_mut(user_buckets, (i as u8));
                let len = vector::length(bucket);
                while (len > 0) {
                    vector::push_back(&mut all_players, vector::pop_back(bucket));
                    len = len - 1;
                };
                i = i + 1;
            };

            // Create full rooms
            let signers = create_rooms_for_players<GameType>(owner, &mut all_players, matchmaker.max_players_per_room);
            vector::reverse_append(&mut room_signers, signers);

            // See if there are any partial rooms left to create
            let signers = create_rooms_for_players<GameType>(owner, &mut all_players, matchmaker.min_players_per_room);
            vector::reverse_append(&mut room_signers, signers);

            return (option::some(room_signers), all_players)
        };

        (option::none(), vector::empty<Object<Token>>())
    }

}