module 0xcafe::emitter {
    // Minimal event type so the script can call `0x1::event::emit`.
    #[event]
    struct Marker has store, drop { value: u64 }

    public fun make(): Marker {
        Marker { value: 0 }
    }
}
