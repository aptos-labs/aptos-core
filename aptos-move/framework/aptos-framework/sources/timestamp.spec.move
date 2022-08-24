spec aptos_framework::timestamp {
    spec fun spec_now_microseconds(): u64 {
        global<CurrentTimeMicroseconds>(@aptos_framework).microseconds
    }

    spec fun spec_now_seconds(): u64 {
        spec_now_microseconds() / MICRO_CONVERSION_FACTOR
    }

    spec module {
        use aptos_framework::chain_status;
        invariant chain_status::is_operating() ==> exists<CurrentTimeMicroseconds>(@aptos_framework);
    }
}
