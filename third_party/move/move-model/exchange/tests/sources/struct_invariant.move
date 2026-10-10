module 0x42::struct_invariant {
    struct R {
        x: u64,
    }

    fun make(): R { R { x: 1 } }

    spec R {
        invariant x > 0;
    }
}
