module tournament::rps_unit_tests {
    use aptos_framework::account;
    use std::hash;
    use std::signer;
    use std::string;
    use std::vector;
    use std::option::{Self, Option};
    use std::string_utils::{to_string};
    use aptos_framework::object::{Self, Object};
    use aptos_token_objects::token::Token;

    use tournament::admin;
    use tournament::aptos_tournament;
    use tournament::rock_paper_scissor::{Self, RockPaperScissorsGame};
    use tournament::token_manager;
    use tournament::tournament_manager;

    struct TournamentConfig has key {
        // signer_cap: account::SignerCapability,
        tournament_address: address,
        game_addresses: vector<address>,
        round_address: Option<address>,
    }

    struct PlayerConfig has key {
        player_token: Object<Token>,
        signer_cap: account::SignerCapability,
    }

    fun get_signer(account_address: address): signer acquires PlayerConfig {
        account::create_signer_with_capability(&borrow_global<PlayerConfig>(account_address).signer_cap)
    }

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
        let (resource_signer, signer_cap) = account::create_resource_account(user, vector::empty());
        let player_name = to_string<address>(&signer::address_of(user));
        let tournament_address = borrow_global<TournamentConfig>(admin_address).tournament_address;
        let player_token = tournament_manager::join_tournament_with_return(
            &resource_signer,
            tournament_address,
            player_name
        );
        move_to(user, PlayerConfig {
            player_token,
            signer_cap
        });
    }

    public entry fun start_new_round(admin: &signer, player_addresses: vector<address>) acquires PlayerConfig, TournamentConfig {
        let admin_address = signer::address_of(admin);
        let tournament_address = borrow_global<TournamentConfig>(admin_address).tournament_address;
        aptos_tournament::start_new_round<RockPaperScissorsGame>(admin, tournament_address);

        let player_tokens: vector<Object<Token>> = vector::map_ref(&player_addresses, |player_address| borrow_global<PlayerConfig>(*player_address).player_token);
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

    fun full_play(
        admin: &signer,
        game_index: u64,
        player1_address: address,
        player2_address: address,
        action1: vector<u8>,
        action2: vector<u8>,
        // 0: no one goes. 1: first goes. 2: second goes. 3: all go
        move_players: u8,
    ): (vector<address>, vector<address>, address) acquires PlayerConfig, TournamentConfig {
        let player1 = get_signer(player1_address);
        let player2 = get_signer(player2_address);
        let admin_address = signer::address_of(admin);
        let game_address = *vector::borrow(&borrow_global<TournamentConfig>(admin_address).game_addresses, game_index);
        let hash_addition1 = b"random uuid 1";
        let hash_addition2 = b"random uuid 2";

        player_commit(&player1, game_address, action1, hash_addition1);
        player_commit(&player2, game_address, action2, hash_addition2);
        if (move_players == 1 || move_players == 3) {
            let (is_game_over, _winners, _losers) = rock_paper_scissor::verify_action_returning(
                &player1,
                game_address,
                action1,
                hash_addition1
            );
            assert!(!is_game_over, 0);
        };

        let winners = vector[];
        let losers = vector[];
        if (move_players == 2 || move_players == 3) {
            let (_is_game_over, winnersi, losersi) = rock_paper_scissor::verify_action_returning(
                &player2,
                game_address,
                action2,
                hash_addition2
            );
            winners = winnersi;
            losers = losersi;
        };

        (winners, losers, game_address)
    }
}