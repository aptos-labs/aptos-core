module poker::poker {

use aptos_std::ristretto255_elgamal as elgamal;
use aptos_std::ristretto255;
use std::vector;
use std::option;
use std::option::Option;
use std::signer;
use std::error;

    const MAX_PLAYERS: u64 = 7;

    /// User tries to make two games
    const EGAME_ALREADY_EXISTS_FOR_USER: u64 = 1;

    /// Game doesn't exist under address
    const EGAME_DOESNT_EXIST: u64 = 2;

    /// Deserialization failed
    const EDESERIALIZATION_FAILED: u64 = 3;

    /// Game already started
    const EGAME_ALREADY_STARTED: u64 = 4;

    /// Game already has the maximum number of players
    const EMAX_PLAYERS_REACHED: u64 = 5;

    /// The player is already in the game
    const EPLAYER_IN_GAME: u64 = 6;

    /// The deck has not yet been shuffled by all players
    const EDECK_NOT_SHUFFLED: u64 = 7;

    /// The given player is not in the game
    const EPLAYER_DOESNT_EXIST: u64 = 8;

    /// The given player has already shuffled the deck
    const EPLAYER_ALREADY_SHUFFLED: u64 = 9;

struct Player has copy, drop, store {
    pk: elgamal::CompressedPubkey,
    owner: address,
    shuffled: bool,
}

struct Game has key {
    deck: Option<Deck>,
    players: vector<Player>,
    started: bool,
}

struct Deck has drop, store {
    cards: vector<elgamal::CompressedCiphertext>,
}

// Initializes the poker game, assigning it to the sender as its owner. An ElGamal public key `pk` must be provided to be associated with the owner as player
public entry fun init_game(creator: &signer, pk: vector<u8>) {
    assert!(!exists<Game>(signer::address_of(creator)), error::already_exists(EGAME_ALREADY_EXISTS_FOR_USER));
    let players = vector::empty<Player>();
    let pk = elgamal::new_pubkey_from_bytes(pk);
    assert!(std::option::is_some(&pk), error::invalid_argument(EDESERIALIZATION_FAILED));
    let player = Player { pk: std::option::extract(&mut pk), owner: signer::address_of(creator), shuffled: false };
    vector::push_back(&mut players, player);
    let game = Game { deck: option::none(), players, started: false };
    move_to<Game>(creator, game);
}

// Joins an already existing game owned by `game_addr`. Takes in an ElGamal public key `pk` to be associated with the sender as player
public entry fun join_game(sender: &signer, game_addr: address, pk: vector<u8>) {
    let new_user_addr = signer::address_of(sender);
    assert!(exists<Game>(game_addr), error::not_found(EGAME_DOESNT_EXIST));

    let game = borrow_global_mut<Game>(game_addr);
    assert!(!game.started, error::invalid_argument(EGAME_ALREADY_STARTED));
    assert!(vector::length<Player>(&game.players) <= MAX_PLAYERS, error::invalid_argument(EMAX_PLAYERS_REACHED));
    // TODO: Check player not already in game
    //assert!(!vector::contains<Player>(&game.players, new_user_addr), error::invalid_argument(EPLAYER_IN_GAME));
    let pk = elgamal::new_pubkey_from_bytes(pk);
    assert!(std::option::is_some(&pk), error::invalid_argument(EDESERIALIZATION_FAILED));
    let new_player = Player { pk: std::option::extract(&mut pk), owner: new_user_addr, shuffled: false };
    vector::push_back(&mut game.players, new_player);
}

// Begins the poker game, given that all players have shuffled the deck once
public entry fun begin_game(sender: &signer, game_addr: address) {
    let game = borrow_global_mut(game_addr);
    assert!(check_deck_shuffled(game), error::invalid_argument(EDECK_NOT_SHUFFLED));
    game.started = true;
}

// Submits a shuffle proof to the game owned by address `game_addr`. This must be done once per player before the game begins
public entry fun shuffle(sender: &signer, game_addr: address, shuffle_proof: vector<u8>) {
    let game = borrow_global_mut<Game>(game_addr);
    if (option::is_none(&game.deck)) {
        game.deck = option::some(initialize_deck());
    };
    let user_addr = signer::address_of(sender);

    // Check that the sender is in the game
    let n = vector::length(&game.players);
    let i = 0;
    let found_player = false;
    let player: Player;
    while (i < n) { 
        let player = vector::borrow(&game.players, i);
        if (player.owner == user_addr) {
            let found_player = true;
            break;
        };
        i = i + 1;
    };
    assert!(found_player, error::not_found(EPLAYER_DOESNT_EXIST));
    assert!(!player.shuffled, error::invalid_argument(EPLAYER_ALREADY_SHUFFLED));

    // TODO: post shuffle proof
    player.shuffled = true;
}

// Searches a Game for a specific Player, and returns it
/*fun find_player(game: &Game, addr: address): Player {
    while (i < n) { 
        let player = vector::borrow(&game.players, i);
        if (player.owner == addr) {
            let found_player = true;
            return player;
        };
        i = i + 1;
    };
}*/

// Initializes the deck with the values 1 through 52 encrypted with no randomness
fun initialize_deck(): Deck {
    let vec = vector::empty<elgamal::CompressedCiphertext>();
    let res = Deck { cards: vec };
    let n = 52;
    let i = 0;
    while (i < n) {
       let scalar = ristretto255::new_scalar_from_u8(i);
       let ct = elgamal::new_ciphertext_no_randomness(&scalar); 
       vector::push_back<elgamal::CompressedCiphertext>(&mut vec, elgamal::compress_ciphertext(&ct));
       i = i + 1;
    };
    res
}

// Checks if all players have shuffled the deck. This means a game is ready to begin.
fun check_deck_shuffled(game: &Game): bool {
    let n = vector::length(&game.players); 
    let i = 0;
    while (i < n) {
        let player = vector::borrow(&game.players, i);
        if (player.shuffled == false) {
            return false;
        }
    };
    true
}
}
