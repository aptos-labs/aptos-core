module 0x42::enums {
    enum Action has copy, drop, store {
        Idle,
        Transfer { amount: u64 },
        Split { left: u64, right: u64 },
    }

    fun total(action: Action): u64 {
        match (action) {
            Action::Idle => 0,
            Action::Transfer { amount } => amount,
            Action::Split { left, right } => left + right,
        }
    }
    spec total {
        ensures result == match (action) {
            Action::Idle => 0,
            Action::Transfer { amount } => amount,
            Action::Split { left, right } => left + right,
        };
    }

    fun classify(action: Action): u64 {
        match (action) {
            Action::Idle => 0,
            Action::Transfer { .. } => 1,
            Action::Split { .. } => 2,
        }
    }

    fun is_transfer(action: Action): bool {
        action is Action::Transfer
    }

    fun guarded(value: u64): u64 {
        match (value) {
            0 => 10,
            1..4 if value != 2 => 20,
            4..=6 => 30,
            _ => 40,
        }
    }

    fun make(flag: bool, amount: u64): Action {
        if (flag) Action::Transfer { amount } else Action::Idle
    }
}
