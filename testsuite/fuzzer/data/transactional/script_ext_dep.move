//# init --addresses poc=0xcafe

//# publish
module poc::a1 {

    public entry fun noop_generic<T>() {

        // Do nothing.

    }
}

//# publish
module poc::a2 {

    struct A2 has key, store, copy, drop {}
}

//# publish
module poc::a3 {
    use std::signer;

    struct Counter has key { value: u64 }

    fun init_module(account: &signer) {
        move_to(account, Counter { value: 0 });
    }

    public entry fun increment_counter() acquires Counter {
        let cnt = &mut borrow_global_mut<Counter>(@poc).value;
        *cnt = *cnt + 1;
    }
}

//# run --signers poc --exec-group 1 -- poc::a3::increment_counter

//# run --signers 0x1111 --exec-group 1
script {
    fun main() {
        poc::a1::noop_generic<poc::a2::A2>();
        poc::a3::increment_counter();
    }
}

//# run --signers 0x2222 --exec-group 1
script {
    fun main() {
        poc::a1::noop_generic<poc::a2::A2>();
        poc::a3::increment_counter();
    }
}

//# run --signers 0x3333 --exec-group 1
script {
    fun main() {
        poc::a1::noop_generic<poc::a2::A2>();
        poc::a3::increment_counter();
    }
}
