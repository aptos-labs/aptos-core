module tournament::rps_unit_tests {
    use aptos_framework::account;
    use std::hash;
    use std::signer;
    use std::string;
    use std::vector;
    use std::table::{Self, Table};
    use std::option::{Self, Option};
    use std::string_utils::{to_string};
    use aptos_framework::object::{Self, Object};
    use aptos_token_objects::token::Token;

    use tournament::admin;
    use tournament::aptos_tournament;
    use tournament::rock_paper_scissor::{Self, RockPaperScissor, RockPaperScissorsGame};
    use tournament::token_manager;
    use tournament::tournament_manager;

    struct TournamentConfig has key {
        // signer_cap: account::SignerCapability,
        tournament_address: address,
        game_addresses: vector<address>,
        round_address: Option<address>,
    }

    struct PlayerConfig has key {
        // Configuration of the player for each tournament.
        player_configs: Table<address, Object<Token>>
    }

    // fun get_signer(account_address: address): signer acquires PlayerConfig {
    //     account::create_signer_with_capability(&borrow_global<PlayerConfig>(account_address).signer_cap)
    // }

    public entry fun setup_tournament(
        admin: &signer
    ) {
        // Create a resource account owned by admin.
        // TODO: Not sure if the signer capability is required
        // let (resource_signer, signer_cap) = account::create_resource_account(admin, vector::empty());

        token_manager::init_module_for_test(admin);
        aptos_tournament::init_module_for_test(admin);
        admin::set_admin_signer(admin, signer::address_of(admin));
        let tournament_address = aptos_tournament::create_new_tournament_returning(admin);
        let admin2 = admin::get_admin_signer_as_admin(admin);
        tournament_manager::set_tournament_joinable(&admin2, tournament_address);
        
        move_to(admin, TournamentConfig {
            // signer_cap, 
            tournament_address,
            game_addresses: vector[],
            round_address: option::none(),
        });
    }

    public entry fun setup_player(
        user: &signer,
        admin_address: address
    ) acquires TournamentConfig {
        let user_address = signer::address_of(user);
        // TODO: Does this create a new resource account for each tournament? Should we just use one resource account for all tournaments?
        let player_name = to_string<address>(&user_address);
        let tournament_address = borrow_global<TournamentConfig>(admin_address).tournament_address;
        let player_token = tournament_manager::join_tournament_with_return(
            user,
            tournament_address,
            player_name
        );
        if (exists<PlayerConfig>(user_address)) {
            move_to(user, PlayerConfig {
                player_configs: table::new()
            })
        };
        // TODO: Can a resource be inserted and modified in the same transaction?
        let player_config = borrow_global_mut<PlayerConfig>(user_address);
        table::upsert(&mut player_config.player_configs, tournament_address, player_token);
    }

    public entry fun start_new_round(admin: &signer, player_addresses: vector<address>) acquires PlayerConfig, TournamentConfig {
        let admin_address = signer::address_of(admin);
        let tournament_address = borrow_global<TournamentConfig>(admin_address).tournament_address;
        aptos_tournament::start_new_round<RockPaperScissorsGame>(admin, tournament_address);

        let player_tokens: vector<Object<Token>> = vector::map_ref(&player_addresses, |player_address| {
            let player_config = borrow_global_mut<PlayerConfig>(*player_address);
            table::remove(&mut player_config.player_configs, admin_address)
        });
        // TODO: Should this be done every round, or only once in the beginning?
        let game_addresses = aptos_tournament::add_players_to_game_returning(
            admin,
            tournament_address,
            player_tokens
        );
        let round_address = tournament_manager::get_round_address(tournament_address);
        let tournament_config = borrow_global_mut<TournamentConfig>(admin_address);
        // TODO: Is this correct syntax?
        tournament_config.round_address = option::some(round_address);
        tournament_config.game_addresses = game_addresses;
    }

    fun player_commit(player: &signer, game_address: address, action: vector<u8>, hash_addition: vector<u8>) {
        let combo = copy action;
        vector::append(&mut combo, hash_addition);
        rock_paper_scissor::commit_action(player, game_address, hash::sha3_256(combo));
    }

    fun game_play(
        player: signer,
        admin_address: address,
        action: vector<u8>,
        game_index: u64,
    ) {
        let game_address = *vector::borrow(&borrow_global<TournamentConfig>(admin_address).game_addresses, game_index);
        let RockPaperScissor {player1, player2} =  borrow_global<RockPaperScissor>(game_address);
        let hash_addition = b"random uuid";
        player_commit(&player, game_address, action, hash_addition);

    }

    // fun full_play(
    //     admin: address,
    //     game_index: u64,
    //     // TODO: We need to get player1 and player 2 details from game_index instead of supplying them as input here.
    //     player1: address,
    //     player2: address,
    //     action1: vector<u8>,
    //     action2: vector<u8>,
    //     // 0: no one goes. 1: first goes. 2: second goes. 3: all go
    //     move_players: u8,
    // ): (vector<address>, vector<address>, address) acquires PlayerConfig, TournamentConfig {
    //     let game_address = *vector::borrow(&borrow_global<TournamentConfig>(admin).game_addresses, game_index);
    //     // let RockPaperScissor {player1, player2} =  borrow_global<RockPaperScissor>(game_address);

    //     let player1_signer = get_signer(player1);
    //     let player2_signer = get_signer(player2);

    //     let hash_addition1 = b"random uuid 1";
    //     let hash_addition2 = b"random uuid 2";

    //     player_commit(&player1_signer, game_address, action1, hash_addition1);
    //     player_commit(&player2_signer, game_address, action2, hash_addition2);
    //     if (move_players == 1 || move_players == 3) {
    //         let (is_game_over, _winners, _losers) = rock_paper_scissor::verify_action_returning(
    //             &player1_signer,
    //             game_address,
    //             action1,
    //             hash_addition1
    //         );
    //         assert!(!is_game_over, 0);
    //     };

    //     let winners = vector[];
    //     let losers = vector[];
    //     if (move_players == 2 || move_players == 3) {
    //         let (_is_game_over, winnersi, losersi) = rock_paper_scissor::verify_action_returning(
    //             &player2_signer,
    //             game_address,
    //             action2,
    //             hash_addition2
    //         );
    //         winners = winnersi;
    //         losers = losersi;
    //     };

    //     (winners, losers, game_address)
    // }
}