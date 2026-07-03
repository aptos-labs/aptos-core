module 0xcafe::emitter {
    // A trivial event type so a script can call `0x1::event::emit` (which the VM rejects for
    // scripts). `store + drop` is all `event::emit` requires of its type argument.
    #[event]
    struct Marker has store, drop { value: u64 }

    public fun make(): Marker {
        Marker { value: 0 }
    }
}
