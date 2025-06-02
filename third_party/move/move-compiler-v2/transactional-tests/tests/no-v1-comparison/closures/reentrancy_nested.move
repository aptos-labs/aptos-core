//# publish
module 0x42::worker1 {
    struct R(u64) has key;

    public fun work(cont: ||) acquires R {
        R[@0x42].0 += 1;
        cont()
    }

    public fun set(x: u64) acquires R {
        R[@0x42].0 = x
    }

    public fun init(s: &signer) {
        move_to(s, R(0))
    }
}

//# publish
module 0x42::worker2 {
    use 0x42::worker1;
    struct R(u64) has key;

    public fun work(cont: ||) acquires R {
        R[@0x42].0 += 1;
        worker1::work(cont)
    }

    public fun set(x: u64) acquires R {
        R[@0x42].0 = x
    }

    public fun init(s: &signer) {
        move_to(s, R(0))
    }
}

//# publish
module 0x42::tests {
    use 0x42::worker1;
    use 0x42::worker2;
    struct R(u64) has key;

    fun init(s: &signer) {
        worker1::init(s);
        worker2::init(s);
        move_to(s, R(0));
    }

    fun direct_failure(): bool acquires R {
        worker2::work(|| R[@042].0 += 1);
        false
    }

    fun worker1_failure(): bool {
        worker2::work(|| worker1::set(10));
        false
    }

    fun worker2_failure(): bool {
        worker2::work(|| worker2::set(10));
        false
    }

    fun worker2_ok(): bool {
        worker1::work(|| worker2::set(10));
        true
    }
}

//# run 0x42::tests::init --signers 0x42

// task 4
//# run 0x42::tests::direct_failure

// task 5
//# run 0x42::tests::worker1_failure

// task 6
//# run 0x42::tests::worker2_failure

// task 7
//# run 0x42::tests::worker2_ok
