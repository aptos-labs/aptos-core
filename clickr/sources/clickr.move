module clickr::clickr {
    use std::signer;
    use std::string;
    use aptos_std::math128;
    use aptos_std::table;
    use aptos_std::table::Table;
    use aptos_framework::aggregator_v2::Aggregator;
    use aptos_framework::aggregator_v2;
    use aptos_framework::aptos_account;
    use aptos_framework::coin;
    use aptos_framework::coin::MintCapability;
    use aptos_framework::event;
    use aptos_framework::timestamp;

    const ONE_CLICKR: u128 = 100_000_000;
    const NOT_ADMIN: u64 = 0;
    const INACTIVE_GAME: u64 = 1;

    struct ClickrCoin {}

    struct Config has key {
        total_plays: Table<u64, u128>,
        mint_cap: MintCapability<ClickrCoin>,
        current_game_id: u64,
        is_paused: bool,
        end_time: u64,
    }

    struct CurrentGame has key {
        total_plays: Aggregator<u128>,
    }

    struct Player has key {
        rounds: u128,
        game_played: u64,
    }

    #[event]
    struct GameEnd {
        game_id: u64,
        total_plays: u64,
    }

    #[view]
    public fun is_registered(user: address): bool {
        return exists<Player>(user)
    }

    #[view]
    public fun curr_total_plays(): u128 acquires CurrentGame {
        let game = borrow_global<CurrentGame>(@clickr);
        aggregator_v2::read(&game.total_plays)
    }

    #[view]
    public fun end_time(): u64 acquires Config {
        let config = borrow_global<Config>(@clickr);
        config.end_time
    }

    #[view]
    public fun current_plays(user: address): u128 acquires Player {
        if (!is_registered(user)) {
            return 0
        };
        let player = borrow_global<Player>(user);
        player.rounds
    }

    fun init_module(clickr_signer: &signer) {
        let (burn_cap, freeze_cap, mint_cap) = coin::initialize<ClickrCoin>(
            clickr_signer,
            string::utf8(b"CLICKR"),
            string::utf8(b"CLICKR"),
            8, // decimals
            false, // monitor_supply
        );
        coin::destroy_burn_cap(burn_cap);
        coin::destroy_freeze_cap(freeze_cap);

        move_to(clickr_signer, Config {
            total_plays: table::new(),
            mint_cap,
            current_game_id: 0,
            is_paused: false,
            end_time: 0,
        });
    }

    entry fun start_game(admin: &signer, end_time: u64) acquires Config, CurrentGame {
        assert!(signer::address_of(admin) == @clickr, NOT_ADMIN);

        let config = borrow_global_mut<Config>(@clickr);
        if (exists<CurrentGame>(@clickr)) {
            let CurrentGame { total_plays } = move_from<CurrentGame>(@clickr);
            let game_id = config.current_game_id;
            let total_plays = aggregator_v2::read(&total_plays);
            table::add(&mut config.total_plays, game_id, total_plays);
            //event::emit(GameEnd { game_id, total_plays });
        };

        config.current_game_id = config.current_game_id + 1;
        config.is_paused = false;
        config.end_time = end_time;
        move_to(admin, CurrentGame { total_plays: aggregator_v2::create_unbounded_aggregator() });
    }

    entry fun set_pause(admin: &signer, paused: bool) acquires Config {
        assert!(signer::address_of(admin) == @clickr, NOT_ADMIN);
        let config = borrow_global_mut<Config>(@clickr);
        config.is_paused = paused;
    }

    entry fun register(user: &signer) {
        move_to(user, Player { rounds: 0, game_played: 0 });
    }

    entry fun play(user: &signer) acquires Config, CurrentGame, Player {
        if (!is_registered(signer::address_of(user))) {
            register(user);
        };

        let config = borrow_global<Config>(@clickr);
        assert!(config.end_time > timestamp::now_seconds() && !config.is_paused, INACTIVE_GAME);
        claim(user);
        let current_game = borrow_global_mut<CurrentGame>(@clickr);
        aggregator_v2::try_add(&mut current_game.total_plays, 1);
        let player = borrow_global_mut<Player>(signer::address_of(user));
        player.rounds = player.rounds + 1;
    }

    entry fun claim(user: &signer) acquires Config, Player {
        let player = borrow_global_mut<Player>(signer::address_of(user));
        let config = borrow_global<Config>(@clickr);
        if (player.game_played == 0) {
            player.game_played = config.current_game_id;
        };

        if (config.current_game_id > player.game_played) {
            let rewards_amt = math128::mul_div(
                player.rounds,
                ONE_CLICKR,
                *table::borrow(&config.total_plays, player.game_played),
            );
            player.game_played = config.current_game_id;
            let rewards = coin::mint((rewards_amt as u64), &config.mint_cap);
            aptos_account::deposit_coins(signer::address_of(user), rewards);
        };
    }
}
