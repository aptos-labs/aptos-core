module 0x42::m {

    fun foo(f: |&u64|) {
    }

    fun g() {
        foo(|v| {
            v == &1;
        });
    }


}
