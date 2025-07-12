module account::test {

    struct FunctionStore has key {
        f: |&mut u64|&u64 has copy+drop+store,
    }

    public fun freeze_ref(x: &mut u64): &u64 {
        x
    }

    fun init_module(account: &signer) {
        let f: |&mut u64|&u64 has copy+drop+store = |s| freeze_ref(s);
        move_to(account, FunctionStore { f });
    }
}
