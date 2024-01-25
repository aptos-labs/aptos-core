module lottery::lottery {
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    // NOTE: If deployed, this would be aptos_std (or aptos_framework).
    use aptos_std_extra::randomness;
    use aptos_std::smart_vector;
    use aptos_std::smart_vector::SmartVector;
    use aptos_framework::coin::Coin;
    use std::signer;

    // We need this friend declaration so our tests can call `init_module`.
    friend lottery::lottery_test;

    /// Error code for when a user tries to initate the drawing but no users
    /// bought any tickets.
    const E_NO_TICKETS: u64 = 2;

    /// Error code for when the somebody tries to draw an already-closed lottery
    const E_LOTTERY_HAS_CLOSED: u64 = 3;

    /// The minimum price of a lottery ticket, in APT.
    const TICKET_PRICE: u64 = 10_000;

    /// A lottery: a list of users who bought tickets and the time at which
    /// it was started.
    ///
    /// The winning user will be randomly picked from this list.
    struct Lottery has key {
        // A list of users who bought lottery tickets (repeats allowed).
        tickets: SmartVector<address>,
        coins: Coin<AptosCoin>,
        is_closed: bool,
    }

    /// Initializes the `Lottery` resource, which will maintain the list of lottery tickets bought by users.
    fun init_module(deployer: &signer) {
        init_module_for_testing(deployer)
    }

    public(friend) fun init_module_for_testing(deployer: &signer) {
        move_to(
            deployer,
            Lottery {
                tickets: smart_vector::empty(),
                coins: coin::zero(),
                is_closed: false,
            }
        );
    }

    /// The price of buying a lottery ticket.
    public fun get_ticket_price(): u64 { TICKET_PRICE }

    /// Any user can call this to purchase a ticket in the lottery.
    public entry fun buy_a_ticket(user: &signer) acquires Lottery {
        let lottery = borrow_global_mut<Lottery>(@lottery);

        // Charge the price of a lottery ticket from the user's balance, and
        // accumulate it into the lottery's bounty.
        let coins = coin::withdraw<AptosCoin>(user, TICKET_PRICE);
        coin::merge(&mut lottery.coins, coins);

        // Issue a ticket for that user
        smart_vector::push_back(&mut lottery.tickets, signer::address_of(user))
    }

    /// Securely wraps around `decide_winners_internal` so it can only be called
    /// as a top-level call from a TXN, preventing **test-and-abort** attacks (see
    /// [AIP-41](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-41.md)).
    entry fun randomly_pick_winner() acquires Lottery {
        randomly_pick_winner_internal();
    }

    /// Insecurely wraps around `decide_winners_internal` allowing this function to
    /// be called from a Move script or another module, leaving it vulnerable to
    /// **test-and-abort** attacks (see [AIP-41](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-41.md)).
    ///
    /// Commented out for security.
    //public fun decide_winners_insecure() acquires Lottery, Credentials {
    //    decide_winners_internal();
    //}

    /// Allows anyone to close the lottery (if enough time has elapsed & more than
    /// 1 user bought tickets) and to draw a random winner.
    public(friend) fun randomly_pick_winner_internal(): address acquires Lottery {
        let lottery = borrow_global_mut<Lottery>(@lottery);
        assert!(!lottery.is_closed, E_LOTTERY_HAS_CLOSED);
        assert!(!smart_vector::is_empty(&lottery.tickets), E_NO_TICKETS);

        // Pick a random winner in [0, |lottery.tickets|)
        let winner_idx = randomness::u64_range(0, smart_vector::length(&lottery.tickets));
        let winner = *smart_vector::borrow(&lottery.tickets, winner_idx);

        // Pay the winner
        let coins = coin::extract_all(&mut lottery.coins);
        coin::deposit<AptosCoin>(winner, coins);
        lottery.is_closed = true;

        winner
    }
}
