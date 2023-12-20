module tournament::rps_utils {
    use std::hash;
    use std::signer;
    use std::string;
    use std::vector;
    use std::table::{Self, Table};
    use std::option::{Self, Option};
    use std::string_utils::{to_string};
    use aptos_framework::object::Object;
    use aptos_token_objects::token::Token;

    use tournament::admin;
    use tournament::aptos_tournament;
    use tournament::rock_paper_scissor::{Self, MyAddress, RockPaperScissorsGame};
    use tournament::tournament_manager;

    const ETOURNAMENT_DOES_NOT_EXIST_1: u64 = 1;
    const ETOURNAMENT_DOES_NOT_EXIST_2: u64 = 2;
    const EPLAYER_DOES_NOT_EXIST: u64 = 3;
    const EMAPPING_ALREADY_EXISTS: u64 = 4;
    const EMAPPING_DOESNT_EXIST: u64 = 5;
    const EPLAYER_MISSING_PLAY_TOKEN: u64 = 6;

    struct TournamentConfig has key {
        tournament_address: address,
        // round_address: Option<address>,
    }

    // Stores a map of player address to the game address.
    // Admin of the tournament stores this resource.
    struct PlayerToGameMapping has key {
        mapping: Table<address, MyAddress>
    }

    struct PlayerConfig has key {
        // Configuration of the player for each tournament.
        player_tokens: Table<address, Object<Token>>,
    }

    // struct MissingPlayerConfig has key {
    //     // Configuration of the player for each tournament.
    //     missing: vector<address>,
    // }

    public entry fun setup_tournament(
        admin: &signer
    ) {
        // token_manager::init_module_for_test(admin);
        // aptos_tournament::init_module_for_test(admin);
        admin::set_admin_signer(admin, signer::address_of(admin));
        let tournament_address = aptos_tournament::create_new_tournament_returning(admin);
        let admin2 = admin::get_admin_signer_as_admin(admin);
        tournament_manager::set_tournament_joinable(&admin2, tournament_address);

        move_to(admin, TournamentConfig {
            tournament_address,
            // round_address: option::none(),
        });
        move_to(admin, PlayerToGameMapping {
            mapping: table::new(),
        });
    }

    public entry fun setup_player(
        user: &signer,
        admin_address: address
    ) acquires TournamentConfig, PlayerConfig {
        let user_address = signer::address_of(user);
        // TODO: Does this create a new resource account for each tournament? Should we just use one resource account for all tournaments?
        let player_name = string::sub_string(&to_string<address>(&user_address), 0, 15);
        assert!(exists<TournamentConfig>(admin_address), ETOURNAMENT_DOES_NOT_EXIST_1);
        let tournament_address = borrow_global<TournamentConfig>(admin_address).tournament_address;
        let player_token = tournament_manager::join_tournament_with_return(
            user,
            tournament_address,
            player_name
        );
        if (!exists<PlayerConfig>(user_address)) {
            move_to(user, PlayerConfig {
                player_tokens: table::new(),
            })
        };
        assert!(exists<PlayerConfig>(user_address), EPLAYER_DOES_NOT_EXIST);

        // TODO: Can a resource be inserted and modified in the same transaction?
        let player_config = borrow_global_mut<PlayerConfig>(user_address);
        table::upsert(&mut player_config.player_tokens, tournament_address, player_token);
    }

    // public entry fun start_new_round_check(_fee_payer: &signer, admin: &signer, player_addresses: vector<address>) acquires MissingPlayerConfig {
    //     let admin_address = signer::address_of(admin);
    //     if (!exists<MissingPlayerConfig>(admin_address)) {
    //         move_to<MissingPlayerConfig>(admin, MissingPlayerConfig {
    //             missing: vector::empty(),
    //         });
    //     };

    //     let missing_players = borrow_global_mut<MissingPlayerConfig>(admin_address);

    //     let _ = vector::map_ref(&player_addresses, |player_address| {
    //         assert!(exists<PlayerConfig>(*player_address), EPLAYER_DOES_NOT_EXIST);
    //         1
    //     });
    // }

    public entry fun start_new_round(_fee_payer: &signer, admin: &signer) acquires TournamentConfig {
        let admin_address = signer::address_of(admin);

        assert!(exists<TournamentConfig>(admin_address), ETOURNAMENT_DOES_NOT_EXIST_2);
        let tournament_address = borrow_global<TournamentConfig>(admin_address).tournament_address;
        aptos_tournament::start_new_round<RockPaperScissorsGame>(admin, tournament_address);

        // let round_address = tournament_manager::get_round_address(tournament_address);
        // let tournament_config = borrow_global_mut<TournamentConfig>(admin_address);
        // tournament_config.round_address = option::some(round_address);
    }

    public entry fun move_players_to_round(_fee_payer: &signer, admin: &signer, player_addresses: vector<address>) acquires PlayerConfig, TournamentConfig, PlayerToGameMapping {
        let admin_address = signer::address_of(admin);
        assert!(exists<TournamentConfig>(admin_address), ETOURNAMENT_DOES_NOT_EXIST_2);
        let tournament_address = borrow_global<TournamentConfig>(admin_address).tournament_address;

        let player_tokens: vector<Object<Token>> = vector::map_ref(&player_addresses, |player_address| {
            assert!(exists<PlayerConfig>(*player_address), EPLAYER_DOES_NOT_EXIST);
            let player_config = borrow_global_mut<PlayerConfig>(*player_address);

            assert!(table::contains(&player_config.player_tokens, tournament_address), EPLAYER_MISSING_PLAY_TOKEN);
            table::remove(&mut player_config.player_tokens, tournament_address)
        });
        // TODO: Should this be done every round, or only once in the beginning?
        let game_addresses = aptos_tournament::add_players_to_game_returning(
            admin,
            tournament_address,
            player_tokens
        );

        let player_to_game_mapping = &mut borrow_global_mut<PlayerToGameMapping>(admin_address).mapping;

        rock_paper_scissor::update_player_to_game_mapping(&game_addresses, player_to_game_mapping);
    }

    fun player_commit(player: &signer, game_address: address, action: vector<u8>, hash_addition: vector<u8>) {
        let combo = copy action;
        vector::append(&mut combo, hash_addition);
        rock_paper_scissor::commit_action(player, game_address, hash::sha3_256(combo));
    }

    public entry fun game_play(
        player: &signer,
        admin_address: address,
    ) acquires PlayerToGameMapping {
        let player_address = signer::address_of(player);
        assert!(exists<PlayerToGameMapping>(admin_address), EMAPPING_DOESNT_EXIST);
        let player_to_game_mapping = borrow_global<PlayerToGameMapping>(admin_address);
        let game_address = rock_paper_scissor::get_address(*table::borrow(&player_to_game_mapping.mapping, player_address));
        let action = b"Rock";
        let hash_addition = b"random uuid";
        player_commit(player, game_address, action, hash_addition);
    }
}
