module lottery::lottery {
    use aptos_framework::account;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::resource_account;
    use aptos_framework::timestamp;

    // NOTE: If deployed, this would be aptos_std (or aptos_framework).
    use aptos_std_extra::randomness;

    use std::error;
    use std::signer;
    use std::vector;

    // We need this friend declaration so our tests can call `init_module`.
    friend lottery::lottery_test;

    /// Error code for when a user tries to initate the drawing but no users
    /// bought any tickets.
    const E_NO_TICKETS: u64 = 2;

    /// Error code for when a user tries to initiate the drawing too early
    /// (enough time must've elapsed since the lottery started for users to
    /// have time to register).
    const E_LOTTERY_DRAW_IS_TOO_EARLY: u64 = 3;

    /// The minimum time between when a lottery is 'started' and when it's
    /// closed & the randomized drawing can happen.
    /// Currently set to (10 mins * 60 secs / min) seconds.
    const MINIMUM_LOTTERY_DURATION_SECS : u64 = 10 * 60;

    /// The minimum price of a lottery ticket, in APT.
    const TICKET_PRICE: u64 = 10_000;

    /// The address from which the developers created the resource account.
    /// TODO: This needs to be updated before deploying. See the [resource account flow here](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/resource_account.move).
    const DEVELOPER_ADDRESS: address = @0xcafe;

    /// A lottery: a list of users who bought tickets and the time at which
    /// it was started.
    ///
    /// The winning user will be randomly picked from this list.
    struct Lottery has key {
        // A list of users who bought lottery tickets (repeats allowed).
        tickets: vector<address>,

        // Blockchain time when the lottery started. Prevents closing it too "early."
        started_at: u64,
    }

    /// Stores the signer capability for the resource account.
    struct Credentials has key {
        // Signer capability for the resource account storing the coins that can be won
        signer_cap: account::SignerCapability,
    }

    /// Initializes a so-called "resource" account which will maintain the list
    /// of lottery tickets bought by users.
    ///
    /// WARNING: For the `lottery` module to be secure, it must be deployed at
    /// the same address as the created resource account. See an example flow
    /// [here](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/sources/resource_account.move).
    public(friend) fun init_module(resource_account: &signer) {
        let signer_cap = resource_account::retrieve_resource_account_cap(
            resource_account, DEVELOPER_ADDRESS
        );

        // Initialize an AptosCoin coin store there, which is where the lottery
        // bounty will be kept.
        coin::register<AptosCoin>(resource_account);

        // Store the signer cap for the resource account in the resource account
        // itself.
        move_to(
            resource_account,
            Credentials { signer_cap }
        );
    }

    /// The minimum time the lottery must be open for before anyone can call
    /// `decide_winners`
    public fun get_minimum_lottery_duration_in_secs(): u64 {
        MINIMUM_LOTTERY_DURATION_SECS
    }

    /// The price of buying a lottery ticket.
    public fun get_ticket_price(): u64 { TICKET_PRICE }

    /// Allows anyone to (re)start the lottery.
    public entry fun start_lottery() acquires Credentials {
        let info = borrow_global<Credentials>(@lottery);
        let resource_account = account::create_signer_with_capability(&info.signer_cap);

        let lottery = Lottery {
            tickets: vector::empty<address>(),
            started_at: timestamp::now_seconds(),
        };

        // Create the Lottery resource, effectively 'starting' the lottery.
        // NOTE: Will fail if a previous lottery has already started & hasn't ended yet.
        move_to(&resource_account, lottery);

        //debug::print(&string::utf8(b"Started a lottery at time: "));
        //debug::print(&lottery.started_at);
    }

    /// Called by any user to purchase a ticket in the lottery.
    public entry fun buy_a_ticket(user: &signer) acquires Lottery {
        let lottery = borrow_global_mut<Lottery>(@lottery);

        // Charge the price of a lottery ticket from the user's balance, and
        // accumulate it into the lottery's bounty.
        coin::transfer<AptosCoin>(user, @lottery, TICKET_PRICE);

        // ...and issue a ticket for that user
        vector::push_back(&mut lottery.tickets, signer::address_of(user))
    }

    /// Securely wraps around `decide_winners_internal` so it can only be called
    /// as a top-level call from a TXN, preventing **test-and-abort** attacks (see
    /// [AIP-41](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-41.md)).
    entry fun decide_winners() acquires Lottery, Credentials {
        decide_winners_internal();
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
    public(friend) fun decide_winners_internal(): address acquires Lottery, Credentials {
        let lottery = borrow_global_mut<Lottery>(@lottery);

        // Make sure the lottery is not being closed too early...
        assert!(
            timestamp::now_seconds() >= lottery.started_at + MINIMUM_LOTTERY_DURATION_SECS,
            error::invalid_state(E_LOTTERY_DRAW_IS_TOO_EARLY)
        );

        // ...and that more than one person bought tickets.
        if (vector::length(&lottery.tickets) < 2) {
            abort(error::invalid_state(E_NO_TICKETS))
        };

        // Pick a random winner by permuting the vector [0, 1, 2, ..., n-1], and
        // where n = |lottery.tickets|
        let rng = randomness::rng();
        let winner_idx = randomness::u64_range(
            &mut rng,
            0,
            vector::length(&lottery.tickets)
        );
        let winner = *vector::borrow(&lottery.tickets, winner_idx);

        // Pay the winner
        let signer = get_signer();
        let balance = coin::balance<AptosCoin>(signer::address_of(&signer));

        coin::transfer<AptosCoin>(
            &signer,
            winner,
            balance
        );

        winner
    }

    /// Returns a signer for the resource account.
    fun get_signer(): signer acquires Credentials {
        let info = borrow_global<Credentials>(@lottery);

        account::create_signer_with_capability(&info.signer_cap)
    }
}
