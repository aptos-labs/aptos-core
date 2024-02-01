module lottery::lottery_common {
    use std::vector;
    use aptos_std::smart_vector;
    use aptos_std::smart_vector::SmartVector;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin;
    use aptos_framework::coin::Coin;

    friend lottery::lottery_insecure;
    friend lottery::lottery_secure;

    /// The minimum price of a lottery ticket, in APT.
    const TICKET_PRICE: u64 = 10_000;

    /// Players can pick numbers in [0, MAX_NUMBER).
    /// Currently set to (49 choose 6), since that's a popular lottery in some countries.
    const MAX_NUMBER: u64 = 13_983_816;

    // A lottery ticket for guessing the number `guess`, bought by `addr`
    struct Ticket has store, copy, drop {
        addr: address,
        guess: u64,
    }

    public fun get_ticket_price(): u64 { TICKET_PRICE }
    public fun get_max_number(): u64 { MAX_NUMBER }

    public(friend) fun new_ticket(addr: address, guess: u64): Ticket {
        Ticket { addr, guess }
    }

    public fun get_ticket_owner(ticket: &Ticket): address { ticket.addr }
    public fun get_ticket_guess(ticket: &Ticket): u64 { ticket.guess }

    public(friend) fun find_and_pay_winners(tickets: &SmartVector<Ticket>, coins: &mut Coin<AptosCoin>, number: u64): vector<address> {
        let winners = vector[];
        smart_vector::for_each_ref(tickets, |t| {
            let ticket : &Ticket = t;
            if (ticket.guess == number)
                vector::push_back(&mut winners, ticket.addr);
        });

        if (!vector::is_empty(&winners)) {
            let prize = coin::value(coins) / vector::length(&winners);
            vector::for_each_ref(&winners, |addr| {
                let coins = coin::extract(coins, prize);
                coin::deposit(*addr, coins);
            });
        };

        winners
    }
}
