module 0x42::m {

    struct S has drop { x: u64 }

    fun receiver(self: S, _y: u64) {
    }

    spec receiver(self: S, _y: u64) {
        requires true;
    }
}
