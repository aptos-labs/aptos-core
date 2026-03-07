module 0x815::m {

    enum Direction {
        North,
        South,
        East,
        West,
    }

    fun go(d: Direction): u64 {
        match (d) {
            North => 1,
            South => 2,
        }
    }
}
