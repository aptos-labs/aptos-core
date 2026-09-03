module repro::repro {
    enum State has store, drop { Open, Closed }
    enum Config has store { V1 { x: u64 }, V2 { state: State, x: u64 } }

    fun is_closed(self: &Config): bool {
        match (self) {
            Config::V1 { .. } => false,
            Config::V2 { state, .. } => state is State::Closed
        }
    }

    fun uses_it(c: &Config): u64 {
        if (c.is_closed()) { 0 } else { 1 }
    }
    spec uses_it {
        // Referencing the helper from a spec is what triggers the bad Boogie.
        ensures result == if (is_closed(c)) { 0 } else { 1 };
    }
}
