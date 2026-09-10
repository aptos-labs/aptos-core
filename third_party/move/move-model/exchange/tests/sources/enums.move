module 0x42::enums {
    enum Choice has drop {
        None,
        One(u64),
        Pair { left: u64, right: u64 },
    }

    fun inspect(choice: Choice): u64 {
        match (choice) {
            Choice::None => 0,
            Choice::One(value) => value,
            Choice::Pair { left, right } => left + right,
        }
    }

    fun make(value: u64): Choice {
        Choice::One(value)
    }
}
