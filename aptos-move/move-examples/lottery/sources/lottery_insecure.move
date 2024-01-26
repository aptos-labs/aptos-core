module lottery::lottery_insecure {
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    // NOTE: If deployed, this would be aptos_std (or aptos_framework).
    use aptos_std_extra::randomness;
    use aptos_std::smart_vector;
    use aptos_std::smart_vector::SmartVector;
    use aptos_framework::coin::Coin;
    use std::signer;
    use std::vector;

    // We need this friend declaration so our tests can call `init_module`.
    friend lottery::lottery_test;

    /// Error code for when a user tries to initate the drawing but no users
    /// bought any tickets.
    const E_NO_TICKETS: u64 = 1;

    /// Error code for when the somebody tries to draw an already-closed lottery
    const E_LOTTERY_HAS_CLOSED: u64 = 2;

    /// The minimum price of a lottery ticket, in APT.
    const TICKET_PRICE: u64 = 10_000;

    /// Players can pick numbers in [0, MAX_NUMBER).
    /// Currently set to (49 choose 6), since that's a popular lottery in some countries.
    const MAX_NUMBER: u32 = 13_983_816;

    // A lottery ticket for the number `guess` owned by `addr`
    struct Ticket has store, copy, drop {
        addr: address,
        guess: u32,
    }

    /// A lottery: a list of users who bought tickets.
    struct Lottery has key {
        tickets: SmartVector<Ticket>,
        coins: Coin<AptosCoin>,
        is_closed: bool,
    }

    /// Initializes the `Lottery` resource, which will maintain the list of lottery tickets bought by users.
    fun init_module(deployer: &signer) {
        move_to(
            deployer,
            Lottery {
                tickets: smart_vector::empty(),
                coins: coin::zero(),
                is_closed: false,
            }
        );
    }

    #[test_only]
    public(friend) fun init_module_for_testing(deployer: &signer) {
        init_module(deployer)
    }

    public fun get_jackpot(): u64 acquires Lottery {
        let lottery = borrow_global_mut<Lottery>(@lottery);
        coin::value(&lottery.coins)
    }

    public fun get_ticket_price(): u64 { TICKET_PRICE }

    fun new_ticket(addr: address, guess: u32): Ticket {
        Ticket { addr, guess }
    }

    public fun get_ticket_owner(ticket: &Ticket): address { ticket.addr }

    /// Any user can call this to purchase a ticket in the lottery and guess that the outcome will be `guess`.
    public entry fun buy_a_ticket(user: &signer, guess: u32) acquires Lottery {
        let lottery = borrow_global_mut<Lottery>(@lottery);

        // Charge the price of a lottery ticket from the user's balance, and
        // accumulate it into the lottery's bounty.
        let coins = coin::withdraw<AptosCoin>(user, TICKET_PRICE);
        coin::merge(&mut lottery.coins, coins);

        // Issue a ticket for that user
        smart_vector::push_back(&mut lottery.tickets, new_ticket(signer::address_of(user), guess))
    }

    /// Securely wraps around `decide_winners_internal` so it can only be called
    /// as a top-level call from a TXN, preventing **test-and-abort** attacks (see
    /// [AIP-41](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-41.md)).
    entry fun randomly_pick_winner() acquires Lottery {
        let v = randomly_pick_winner_internal();
        smart_vector::destroy(v);
    }

    /// Allows anyone to close the lottery (if enough time has elapsed & more than
    /// 1 user bought tickets) and to draw a random winner.
    public(friend) fun randomly_pick_winner_internal(admin: signer): vector<address> acquires Lottery {
        let lottery = borrow_global_mut<Lottery>(@lottery);
        // TODO: check admin
        // check that TXN has gas set to max gas.
        // this will likely be the default behavior for randapp transactions anyway since TXN simulation
        // cannot predict the gas cost.
        assert!(!lottery.is_closed, E_LOTTERY_HAS_CLOSED);
        assert!(!smart_vector::is_empty(&lottery.tickets), E_NO_TICKETS);

        // Pick the winning lottery number
        let winning_number = randomness::u32_range(0, MAX_NUMBER);

        // See if anyone won and pay them
        let winners = find_winners(&lottery.tickets, winning_number);
        let prize = coin::value(&lottery.coins) / vector::length(&winners);
        vector::for_each_ref(&winners, |addr| {
            let coins = coin::extract(&mut lottery.coins, prize);
            coin::deposit(*addr, coins);
        });

        // If nobody won, the coins can be moved out of the closed `Lottery` resource
        lottery.is_closed = true;
        winners
    }

    fun find_winners(tickets: &SmartVector<Ticket>, number: u32): vector<address> {
        let matches = vector[];
        smart_vector::for_each_ref(tickets, |t| {
            let ticket : &Ticket = t;
            if (ticket.guess == number)
                vector::push_back(&mut matches, ticket.addr);
        });

        matches
    }
}
