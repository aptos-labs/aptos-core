module drand::lottery {
    use std::signer;
    use aptos_framework::account;
    use std::vector;
    use std::option::{Self, Option};
    use aptos_framework::coin;
    use std::error;
    use aptos_framework::timestamp;
    use aptos_framework::aptos_coin::AptosCoin;
    use drand::drand;
    use aptos_std::type_info;
    use std::string::{Self};
    //use aptos_std::debug;

    /// Error code code when someone tries to start a very "short" lottery where users might not have enough time to buy tickets.
    const E_LOTTERY_IS_NOT_LONG_ENOUGH: u64 = 0;
    /// Error code for when someone tries to modify the time when the lottery is drawn. Once set, this time cannot be modified (for simplicity).
    const E_LOTTERY_ALREADY_STARTED: u64 = 1;
    /// Error code for when a user tries to purchase a ticket after the lottery has closed. This would not be secure since such users might know the public randomness, which is revealed soon after the lottery has closed.
    const E_LOTTERY_HAS_CLOSED: u64 = 2;
    /// Error code for when a user tries to initiating the drawing too early (enough time must've elapsed since the lottery started for users to have time to register).
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
        // Specifically, the drawing will happen during the next drand round after time `end_time`.
        // `None` if the lottery is in the 'not started' state.
        end_time: Option<u64>,

        // Signer for the resource accounts storing the coins that can be won
        signer_cap: account::SignerCapability,
    }

    // Declare the testing module as a friend, so it can call `init_module` below for testing.
    friend drand::lottery_test;

    /// Initializes a so-called "resource" account which will maintain the list of lottery tickets bought by users.
    public(friend) fun init_module(deployer: &signer) {
        // Create the resource account. This will allow this module to later obtain a `signer` for this account and
        // update the list of purchased lottery tickets.
        let (_resource, signer_cap) = account::create_resource_account(deployer, vector::empty());

        // Acquire a signer for the resource account that stores the coin bounty
        let rsrc_acc_signer = account::create_signer_with_capability(&signer_cap);

        // Initialize an AptosCoin coin store there, which is where the lottery bounty will be kept
        coin::register<AptosCoin>(&rsrc_acc_signer);

        // Initialiaze the loterry as 'not started'
        move_to(deployer,
            Lottery {
                tickets: vector::empty<address>(),
                end_time: option::none(),
                signer_cap,
            }
        )
    }

    public fun get_ticket_price(): u64 { TICKET_PRICE }
    public fun get_minimum_lottery_duration_in_secs(): u64 { MINIMUM_LOTTERY_DURATION_SECS }

    /// Allows anyone to start & configure the lottery so that drawing happens at time `end_time_secs` (and thus users
    /// have plenty of time to buy tickets), where `end_time_secs` is a UNIX timestamp in seconds.
    ///
    /// NOTE: A real application can access control this.
    public entry fun start_lottery(end_time_secs: u64) acquires Lottery {
        // Make sure the lottery stays open long enough for people to buy tickets.
        assert!(end_time_secs >= timestamp::now_seconds() + MINIMUM_LOTTERY_DURATION_SECS, error::out_of_range(E_LOTTERY_IS_NOT_LONG_ENOUGH));

        // Update the Lottery resource with the (future) lottery drawing time, effectively 'starting' the lottery.
        let lottery = borrow_global_mut<Lottery>(@drand);
        assert!(option::is_none(&lottery.end_time), error::permission_denied(E_LOTTERY_ALREADY_STARTED));
        lottery.end_time = option::some(end_time_secs);

        //debug::print(&string::utf8(b"Started a lottery that will draw at time: "));
        //debug::print(&end_time_secs);
    }

    /// Called by any user to purchase a ticket in the lottery.
    public entry fun buy_a_ticket(user: &signer) acquires Lottery {
        // Get the Lottery resource
        let lottery = borrow_global_mut<Lottery>(@drand);

        // Make sure the lottery has been 'started' but has NOT been 'drawn' yet
        let end_time = *option::borrow(&lottery.end_time);
        assert!(timestamp::now_seconds() < end_time, error::out_of_range(E_LOTTERY_HAS_CLOSED));

        // Get the address of the resource account that stores the coin bounty
        let (_, rsrc_acc_addr) = get_rsrc_acc(lottery);

        // Charge the price of a lottery ticket from the user's balance, and accumulate it into the lottery's bounty
        coin::transfer<AptosCoin>(user, rsrc_acc_addr, TICKET_PRICE);

        // ...and issue a ticket for that user
        vector::push_back(&mut lottery.tickets, signer::address_of(user))
    }

    /// Allows anyone to close the lottery (if enough time has elapsed) and to decide the winner, by uploading
    /// the correct _drand-signed bytes_ associated with the committed draw time in `Lottery::draw_post`.
    /// These bytes will then be verified and used to extract randomness.
    public entry fun close_lottery(drand_signed_bytes: vector<u8>): Option<address> acquires Lottery {
        // Get the Lottery resource
        let lottery = borrow_global_mut<Lottery>(@drand);

        // Make sure the lottery has been 'started' and enough time has elapsed before the drawing can start
        let end_time = *option::borrow(&lottery.end_time);
        assert!(timestamp::now_seconds() >= end_time, error::out_of_range(E_LOTTERY_DRAW_IS_TOO_EARLY));

        // It could be that no one signed up...
        if(vector::is_empty(&lottery.tickets)) {
            // It's time to draw, but nobody signed up => nobody won.
            // Close the lottery (even if the randomness might be incorrect).
            option::extract(&mut lottery.end_time);
            return option::none<address>()
        };

        // Verify the randomness for the next drand round after `end_time` and pick a winner
        let randomness = drand::verify_and_extract_next_randomness(drand_signed_bytes, end_time);
        assert!(option::is_some(&randomness), error::permission_denied(E_INCORRECT_RANDOMNESS));

        // Use the bytes to pick a number at random from 0 to `|lottery.tickets| - 1` and select the winner
        let randomness = option::extract(&mut randomness);

        let dst = type_info::type_name<Lottery>();
        let dst = string::bytes(&dst);
        let winner_idx = drand::uniform_random_less_than_n(*dst, randomness, vector::length(&lottery.tickets));

        // Pay the winner
        let (rsrc_acc_signer, rsrc_acc_addr) = get_rsrc_acc(lottery);
        let balance = coin::balance<AptosCoin>(rsrc_acc_addr);
        let winner_addr = *vector::borrow(&lottery.tickets, winner_idx);

        coin::transfer<AptosCoin>(
            &rsrc_acc_signer,
            winner_addr,
            balance);

        // Close the lottery
        option::extract(&mut lottery.end_time);
        lottery.tickets = vector::empty<address>();
        option::some(winner_addr)
    }

    //
    // Internal functions
    //

    fun get_rsrc_acc(lottery: &Lottery): (signer, address) {
        let rsrc_acc_signer = account::create_signer_with_capability(&lottery.signer_cap);
        let rsrc_acc_addr = signer::address_of(&rsrc_acc_signer);

        (rsrc_acc_signer, rsrc_acc_addr)
    }
}
