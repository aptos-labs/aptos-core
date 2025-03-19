module 0x42::m {

    fun foo(f: |&u64| has drop) {
    }

    fun g() {
        foo(|v| {
            v == &1;
        });
    }


}
