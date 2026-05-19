module 0x42::generic_pack_unpack {
    struct Box<T> has drop {
        value: T,
    }

    enum Either<L, R> has drop {
        Left { l: L },
        Right { r: R },
    }

    fun unpack_box(b: Box<u64>): u64 {
        let Box<u64> { value } = b;
        value
    }

    fun pack_left(v: u64): Either<u64, bool> {
        Either::Left<u64, bool> { l: v }
    }

    fun unpack_left(e: Either<u64, bool>): u64 {
        match (e) {
            Either::Left { l } => l,
            Either::Right { r: _ } => 0,
        }
    }
}
