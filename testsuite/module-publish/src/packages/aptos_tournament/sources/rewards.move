module tournament::rewards {

    use std::signer;
    use std::string::String;
    use std::vector::{Self};
    use aptos_std::smart_vector::{Self, SmartVector};
    use aptos_framework::aptos_account;
    use aptos_framework::coin::{Coin, Self};
    use aptos_token::token::{Self, Token};
    use tournament::tournament_manager;
    use tournament::token_manager::{Self};
    use aptos_framework::object;
    use tournament::object_refs;

    /// The user has no reward to claim
    const ENO_REWARD_FOR_USER: u64 = 1;
    /// Only the tournament signer can initialize a new rewards store
    const ENOT_TOURNAMENT_SIGNER: u64 = 2;
    /// Either the tournament or the reward pool does not exist
    const ENO_SUCH_REWARD_POOL: u64 = 3;
    /// The lengths of the vectors are not the same
    const EMISMATCHED_LENGTHS: u64 = 4;
    /// This is not an active player token address
    const ENO_SUCH_PLAYER: u64 = 5;
    /// The tournament is not over yet
    const ETOURNAMENT_NOT_ENDED: u64 = 6;
    /// The user does not own the token
    const ENOT_TOKEN_OWNER: u64 = 7;
    /// The reward has already been claimed
    const EREWARD_ALREADY_CLAIMED: u64 = 8;

    struct CoinRewardPool<phantom CoinType> has key {
        coins: Coin<CoinType>,
        coin_reward_amount: u64,
    }

    struct TokenV1RewardPool has key {
        tokens: SmartVector<Token>,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TokenV1RewardClaimed has key, drop {
        receiver_address: address,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct CoinRewardClaimed<phantom CoinType> has key, drop {
        amount: u64,
    }

    public fun is_coin_reward_claimed<CoinType>(token_address: address): bool {
        exists<CoinRewardClaimed<CoinType>>(token_address)
    }

    public fun is_token_v1_reward_claimed(token_address: address): bool {
        exists<TokenV1RewardClaimed>(token_address)
    }

    fun mark_coin_reward_claimed<CoinType>(token_address: address, amount: u64) {
        move_to(&object_refs::get_signer(token_address), CoinRewardClaimed<CoinType> {
            amount,
        });
    }

    fun mark_token_v1_reward_claimed(token_address: address, receiver_address: address) {
        move_to(&object_refs::get_signer(token_address), TokenV1RewardClaimed {
            receiver_address,
        });
    }

    public fun initialize_reward_pool<CoinType>(
        tournament_signer: &signer,
        coin_reward_amount: u64
    ) {
        intitialize_coin_reward_pool<CoinType>(tournament_signer, coin_reward_amount);
        intitialize_token_v1_reward_pool(tournament_signer);
    }

    public fun intitialize_coin_reward_pool<CoinType>(
        tournament_signer: &signer,
        coin_reward_amount: u64
    ) {
        let tournament_address = signer::address_of(tournament_signer);
        if (!reward_pool_exists<CoinType>(tournament_address)) {
            let rewards = CoinRewardPool<CoinType> {
                coins: coin::zero<CoinType>(),
                coin_reward_amount
            };
            move_to(tournament_signer, rewards);
        };
    }

    public fun intitialize_token_v1_reward_pool(tournament_signer: &signer) {
        let tournament_address = signer::address_of(tournament_signer);
        if (!token_v1_reward_pool_exists(tournament_address)) {
            let rewards = TokenV1RewardPool {
                tokens: smart_vector::new(),
            };
            move_to(tournament_signer, rewards);
        };
    }


    public fun reward_pool_exists<CoinType>(tournament_address: address): bool {
        exists<CoinRewardPool<CoinType>>(tournament_address)
    }

    public fun token_v1_reward_pool_exists(tournament_address: address): bool {
        exists<TokenV1RewardPool>(tournament_address)
    }

    public entry fun deposit_coin_rewards<CoinType>(
        funds_signer: &signer,
        tournament_address: address,
        amount: u64,
    ) acquires CoinRewardPool {
        assert!(reward_pool_exists<CoinType>(tournament_address), ENO_SUCH_REWARD_POOL);

        let coinPool = borrow_global_mut<CoinRewardPool<CoinType>>(tournament_address);
        let coin = coin::withdraw<CoinType>(funds_signer, amount);
        coin::merge(&mut coinPool.coins, coin);
    }

    public entry fun deposit_token_v1_rewards(
        funds_signer: &signer,
        tournament_address: address,
        // The address of the creator, eg: 0xcafe
        creators: vector<address>,
        // The names of collections; this is unique under the same account, eg: "Aptos Animal Collection"
        collections: vector<String>,
        // The names of the tokens; this is the same as the name field of TokenData
        token_names: vector<String>,
        // The property versions of the tokens
        property_versions: vector<u64>,
    ) acquires TokenV1RewardPool {
        assert!(token_v1_reward_pool_exists(tournament_address), ENO_SUCH_REWARD_POOL);

        let len = vector::length(&creators);
        assert!(
            len == vector::length(&collections)
                && len == vector::length(&token_names)
                && len == vector::length(&property_versions),
            EMISMATCHED_LENGTHS
        );

        let tokens_pool = borrow_global_mut<TokenV1RewardPool>(tournament_address);
        while (len > 0) {
            let creator = vector::pop_back(&mut creators);
            let collection = vector::pop_back(&mut collections);
            let token_name = vector::pop_back(&mut token_names);
            let property_version = vector::pop_back(&mut property_versions);

            let token_id = token::create_token_id_raw(creator, collection, token_name, property_version);
            let token = token::withdraw_token(funds_signer, token_id, 1);

            smart_vector::push_back(&mut tokens_pool.tokens, token);

            len = len - 1;
        };
    }

    public entry fun withdraw_coin_rewards<CoinType>(
        tournament_signer: &signer,
        target_address: address,
    ) acquires CoinRewardPool {
        let tournament_address = signer::address_of(tournament_signer);
        assert!(reward_pool_exists<CoinType>(tournament_address), ENO_SUCH_REWARD_POOL);

        let CoinRewardPool {
            coins,
            coin_reward_amount: _,
        } = move_from<CoinRewardPool<CoinType>>(tournament_address);

        aptos_account::deposit_coins(target_address, coins);
    }

    public entry fun withdraw_token_v1_rewards(
        tournament_signer: &signer,
        target_address: address,
    ) acquires TokenV1RewardPool {
        let tournament_address = signer::address_of(tournament_signer);
        assert!(token_v1_reward_pool_exists(tournament_address), ENO_SUCH_REWARD_POOL);

        let rewards = move_from<TokenV1RewardPool>(tournament_address);

        let len = smart_vector::length(&rewards.tokens);
        while (len > 0) {
            let token = smart_vector::pop_back(&mut rewards.tokens);
            token::direct_deposit_with_opt_in(target_address, token);
            len = len - 1;
        };

        let TokenV1RewardPool {
            tokens,
        } = rewards;
        smart_vector::destroy_empty(tokens);
    }

    #[deprecated]
    /// Do not use this! Use separate initialize_and_fund_coin_reward_pool or initialize_and_fund_token_pool
    public entry fun claim_reward<CoinType>(
        _user: &signer,
        _receiver_address: address,
        _token_address: address,
    ) {
        abort 0
    }

    public fun assert_player_can_claim(
        user: &signer,
        token_address: address,
    ): (address) {
        assert!(token_manager::has_player_token(token_address), ENO_SUCH_PLAYER);
        let token_object = object::address_to_object<object::ObjectCore>(token_address);
        assert!(object::owns(token_object, signer::address_of(user)), ENOT_TOKEN_OWNER);

        let tournament_address = token_manager::get_tournament_address(token_address);
        assert!(tournament_manager::tournament_has_ended(tournament_address), ETOURNAMENT_NOT_ENDED);

        (tournament_address)
    }

    public entry fun claim_coin_reward<CoinType>(
        user: &signer,
        receiver_address: address,
        token_address: address,
    ) acquires CoinRewardPool {
        let tournament_address = assert_player_can_claim(user, token_address);

        assert!(reward_pool_exists<CoinType>(tournament_address), ENO_SUCH_REWARD_POOL);
        assert!(!is_coin_reward_claimed<CoinType>(token_address), EREWARD_ALREADY_CLAIMED);

        let coin_pool = borrow_global_mut<CoinRewardPool<CoinType>>(tournament_address);
        let reward = coin::extract(&mut coin_pool.coins, coin_pool.coin_reward_amount);
        aptos_account::deposit_coins(receiver_address, reward);

        mark_coin_reward_claimed<CoinType>(token_address, coin_pool.coin_reward_amount);
    }

    public entry fun claim_token_v1_reward(
        user: &signer,
        receiver_address: address,
        token_address: address,
    ) acquires TokenV1RewardPool {
        let tournament_address = assert_player_can_claim(user, token_address);

        assert!(token_v1_reward_pool_exists(tournament_address), ENO_SUCH_REWARD_POOL);
        assert!(!is_token_v1_reward_claimed(token_address), EREWARD_ALREADY_CLAIMED);

        let token_pool = borrow_global_mut<TokenV1RewardPool>(tournament_address);
        let reward = smart_vector::pop_back(&mut token_pool.tokens);
        token::direct_deposit_with_opt_in(receiver_address, reward);

        mark_token_v1_reward_claimed(token_address, receiver_address);
    }
}
