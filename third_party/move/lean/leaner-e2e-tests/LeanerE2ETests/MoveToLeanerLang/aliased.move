/// A module declared under a named address, with a receiver-style function.
module aptos_framework::counter {
    struct Counter has key, drop {
        value: u64,
    }

    /// Reads the counter's value (receiver style: `c.value()`).
    public fun value(self: &Counter): u64 {
        self.value
    }

    public fun bump(self: &mut Counter) {
        self.value = self.value + 1;
    }

    public fun fresh(): Counter {
        Counter { value: 0 }
    }

    public fun bumped_twice(): u64 {
        let c = fresh();
        c.bump();
        c.bump();
        c.value()
    }
    spec bumped_twice {
        ensures result == 2;
    }
}
