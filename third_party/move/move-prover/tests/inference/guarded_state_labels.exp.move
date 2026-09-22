// inference-reject-mutation: *borrow_global_mut<Config>(addr) = config; => *borrow_global_mut<Config>(addr) = Config { value: 0 };
module 0x42::guarded_state_labels {
    use std::signer;
    struct Pending has key { value: u64 }
    struct Config has key, drop { value: u64 }

    fun pending(addr: address): bool { exists<Pending>(addr) }
    spec pending {
        pragma opaque;
        aborts_if false;
        ensures result == exists<Pending>(addr);
        ensures [inferred] result == exists<Pending>(addr);
        aborts_if [inferred] false;
    }

    fun extract(addr: address): Config acquires Pending {
        let Pending { value } = move_from<Pending>(addr);
        Config { value }
    }
    spec extract {
        pragma inference = none;
        aborts_if !exists<Pending>(addr);
        modifies Pending[addr];
        ensures result.value == old(global<Pending>(addr)).value;
        ensures !exists<Pending>(addr);
    }

    // The guard defines the read call's state and consumes extract's later
    // state. This is a forward chain, not a cyclic state definition.
    fun install(account: &signer) acquires Pending, Config {
        let addr = signer::address_of(account);
        if (pending(addr)) {
            let config = extract(addr);
            if (exists<Config>(addr)) {
                *borrow_global_mut<Config>(addr) = config;
            } else {
                move_to(account, config);
            }
        }
    }
    spec install(account: &signer) {
        use 0x1::signer;
        pragma opaque = true;
        modifies Config[signer::address_of(account)];
        ensures [inferred = sathard] (..S1 |~ result_of<pending>(signer::address_of(account))) && (S2 |~ exists<Config>(signer::address_of(account))) ==> {
            let a = signer::address_of(account);
            let b = S1..S2 |~ result_of<extract>(signer::address_of(account));
            S2.. |~ update<Config>(a, b)
        };
        ensures [inferred = sathard] (..S1 |~ result_of<pending>(signer::address_of(account))) && (S2 |~ !exists<Config>(signer::address_of(account))) ==> {
            let a = signer::address_of(account);
            let b = S1..S2 |~ result_of<extract>(signer::address_of(account));
            S2.. |~ publish<Config>(a, b)
        };
        aborts_if [inferred] (..S1 |~ result_of<pending>(signer::address_of(account))) && (S1 |~ aborts_of<extract>(signer::address_of(account)));
    }

}
/*
Verification: Succeeded.
Mutation: Rejected by postcondition.
*/
