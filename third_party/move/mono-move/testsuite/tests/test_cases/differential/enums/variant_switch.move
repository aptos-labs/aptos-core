// RUN: publish
module 0x42::enums_variant_switch {
    enum State has drop {
        Idle,
        Running { progress: u64 },
        Done { result: u64 },
    }

    fun switch_and_read(start: u64): u64 {
        let s = State::Idle;
        s = State::Running { progress: start };
        s = State::Done { result: start + 1 };
        match (s) {
            State::Idle => 0,
            State::Running { progress } => progress,
            State::Done { result } => result,
        }
    }
}

// RUN: execute 0x42::enums_variant_switch::switch_and_read --args 41
// CHECK: results: 42
