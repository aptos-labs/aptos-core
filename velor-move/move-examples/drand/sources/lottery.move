/// An example of a decentralized lottery that picks its winner based on randomness generated in the future
/// by the drand randomnes beacon.
///
/// **WARNING #1:** This example has not been audited and should thus NOT be relied upon as an authoritative guide on
/// using `drand` randomness safely in Move.
///
/// WARNING #2: This code makes a STRONG assumption that the Velor clock and the drand clock are synchronized.
/// In practice, the Velor clock could be lagging behind. As an example, even though the current time is Friday, July
/// 14th, 2023, 7:34PM, from the perspective of the blockchain validators, the time could be Thursday, July 13th, 2023.
/// (Exaggerating the difference, to make the point clearer.) Therefore, a drand round for noon at Friday would be
/// incorrectly treated as a valid future drand round, even though that round has passed. It is therefore important that
/// contracts account for any drift between the Velor clock and the drand clock. In this example, this can be done by
/// increasing the MINIMUM_LOTTERY_DURATION_SECS to account for this drift.

module drand::lottery {
    use std::signer;
    use velor_framework::account;
    use std::vector;
    use std::option::{Self, Option};
    use velor_framework::coin;
    use std::error;
    use velor_framework::timestamp;
    use velor_framework::velor_coin::VelorCoin;
    use drand::drand;
    //use velor_std::debug;

    /// Error code code when someone tries to start a very "short" lottery where users might not have enough time
    /// to buy tickets.
    const E_LOTTERY_IS_NOT_LONG_ENOUGH: u64 = 0;
    /// Error code for when someone tries to modify the time when the lottery is drawn.
    /// Once set, this time cannot be modified (for simplicity).
    const E_LOTTERY_ALREADY_STARTED: u64 = 1;
    /// Error code for when a user tries to purchase a ticket after the lottery has closed. This would not be secure
    /// since such users might know the public randomness, which is revealed soon after the lottery has closed.
    const E_LOTTERY_HAS_CLOSED: u64 = 2;
    /// Error code for when a user tries to initiating the drawing too early (enough time must've elapsed since the
    /// lottery started for users to have time to register).
    const E_LOTTERY_DRAW_IS_TOO_EARLY: u64 = 3;
    /// Error code for when anyone submits an incorrect randomness for the randomized draw phase of the lottery.
    const E_INCORRECT_RANDOMNESS: u64 = 4;

    /// The minimum time between when a lottery is 'started' and when it's closed & the randomized drawing can happen.
    /// Currently set to (10 mins * 60 secs / min) seconds.
    const MINIMUM_LOTTERY_DURATION_SECS: u64 = 10 * 60;

    /// The minimum price of a lottery ticket.
    const TICKET_PRICE: u64 = 10000;

    /// A lottery: a list of users who bought tickets and a time past which the randomized drawing can happen.
    ///
    /// The winning user will be randomly picked (via drand public randomness) from this list.
    struct Lottery has key {
        // A list of which users bought lottery tickets
        tickets: vector<address>,

        // The time when the lottery ends (and thus when the drawing happens).
        // Specifically, the drawing will happen during the drand round at time `draw_at`.
        // `None` if the lottery is in the 'not started' state.
        draw_at: Option<u64>,

        // Signer for the resource accounts storing the coins that can be won
        signer_cap: account::SignerCapability,

        winner: Option<address>,
    }

    // Declare the testing module as a friend, so it can call `init_module` below for testing.
    friend drand::lottery_test;

    /// Initializes a so-called "resource" account which will maintain the list of lottery tickets bought by users.
    fun init_module(deployer: &signer) {
        // Create the resource account. This will allow this module to later obtain a `signer` for this account and
        // update the list of purchased lottery tickets.
        let (_resource, signer_cap) = account::create_resource_account(deployer, vector::empty());

        // Acquire a signer for the resource account that stores the coin bounty
        let rsrc_acc_signer = account::create_signer_with_capability(&signer_cap);

        // Initialize an VelorCoin coin store there, which is where the lottery bounty will be kept
        coin::register<VelorCoin>(&rsrc_acc_signer);

        // Initialiaze the loterry as 'not started'
        move_to(deployer,
            Lottery {
                tickets: vector::empty<address>(),
                draw_at: option::none(),
                signer_cap,
                winner: option::none(),
            }
        )
    }

    public fun get_ticket_price(): u64 { TICKET_PRICE }
    public fun get_minimum_lottery_duration_in_secs(): u64 { MINIMUM_LOTTERY_DURATION_SECS }

    public fun get_lottery_winner(): Option<address> acquires Lottery {
        let lottery = borrow_global_mut<Lottery>(@drand);
        lottery.winner
    }

    /// Allows anyone to start & configure the lottery so that drawing happens at time `draw_at` (and thus users
    /// have plenty of time to buy tickets), where `draw_at` is a UNIX timestamp in seconds.
    ///
    /// NOTE: A real application can access control this.
    public entry fun start_lottery(end_time_secs: u64) acquires Lottery {
        // Make sure the lottery stays open long enough for people to buy tickets.
        assert!(end_time_secs >= timestamp::now_seconds() + MINIMUM_LOTTERY_DURATION_SECS, error::out_of_range(E_LOTTERY_IS_NOT_LONG_ENOUGH));

        // Update the Lottery resource with the (future) lottery drawing time, effectively 'starting' the lottery.
        let lottery = borrow_global_mut<Lottery>(@drand);
        assert!(option::is_none(&lottery.draw_at), error::permission_denied(E_LOTTERY_ALREADY_STARTED));
        lottery.draw_at = option::some(end_time_secs);

        //debug::print(&string::utf8(b"Started a lottery that will draw at time: "));
        //debug::print(&draw_at_in_secs);
    }

    /// Called by any user to purchase a ticket in the lottery.
    public entry fun buy_a_ticket(user: &signer) acquires Lottery {
        // Get the Lottery resource
        let lottery = borrow_global_mut<Lottery>(@drand);

        // Make sure the lottery has been 'started' but has NOT been 'drawn' yet
        let draw_at = *option::borrow(&lottery.draw_at);
        assert!(timestamp::now_seconds() < draw_at, error::out_of_range(E_LOTTERY_HAS_CLOSED));

        // Get the address of the resource account that stores the coin bounty
        let (_, rsrc_acc_addr) = get_rsrc_acc(lottery);

        // Charge the price of a lottery ticket from the user's balance, and accumulate it into the lottery's bounty
        coin::transfer<VelorCoin>(user, rsrc_acc_addr, TICKET_PRICE);

        // ...and issue a ticket for that user
        vector::push_back(&mut lottery.tickets, signer::address_of(user))
    }

    /// Allows anyone to close the lottery (if enough time has elapsed) and to decide the winner, by uploading
    /// the correct _drand-signed bytes_ associated with the committed draw time in `Lottery::draw_at`.
    /// These bytes will then be verified and used to extract randomness.
    public entry fun close_lottery(drand_signed_bytes: vector<u8>) acquires Lottery {
        // Get the Lottery resource
        let lottery = borrow_global_mut<Lottery>(@drand);

        // Make sure the lottery has been 'started' and enough time has elapsed before the drawing can start
        let draw_at = *option::borrow(&lottery.draw_at);
        assert!(timestamp::now_seconds() >= draw_at, error::out_of_range(E_LOTTERY_DRAW_IS_TOO_EARLY));

        // It could be that no one signed up...
        if(vector::is_empty(&lottery.tickets)) {
            // It's time to draw, but nobody signed up => nobody won.
            // Close the lottery (even if the randomness might be incorrect).
            option::extract(&mut lottery.draw_at);
            return
        };

        // Determine the next drand round after `draw_at`
        let drand_round = drand::next_round_after(draw_at);

        // Verify the randomness for this round and pick a winner
        let randomness = drand::verify_and_extract_randomness(
            drand_signed_bytes,
            drand_round
        );
        assert!(option::is_some(&randomness), error::permission_denied(E_INCORRECT_RANDOMNESS));

        // Use the bytes to pick a number at random from 0 to `|lottery.tickets| - 1` and select the winner
        let winner_idx = drand::random_number(
            option::extract(&mut randomness),
            vector::length(&lottery.tickets)
        );

        // Pay the winner
        let (rsrc_acc_signer, rsrc_acc_addr) = get_rsrc_acc(lottery);
        let balance = coin::balance<VelorCoin>(rsrc_acc_addr);
        let winner_addr = *vector::borrow(&lottery.tickets, winner_idx);

        coin::transfer<VelorCoin>(
            &rsrc_acc_signer,
            winner_addr,
            balance);

        // Close the lottery
        option::extract(&mut lottery.draw_at);
        lottery.tickets = vector::empty<address>();
        lottery.winner = option::some(winner_addr);
    }

    //
    // Internal functions
    //

    fun get_rsrc_acc(lottery: &Lottery): (signer, address) {
        let rsrc_acc_signer = account::create_signer_with_capability(&lottery.signer_cap);
        let rsrc_acc_addr = signer::address_of(&rsrc_acc_signer);

        (rsrc_acc_signer, rsrc_acc_addr)
    }

    //
    // Test functions
    //

    #[test_only]
    public fun init_module_for_testing(developer: &signer) {
        account::create_account_for_test(signer::address_of(developer));
        init_module(developer)
    }
}
