module 0x42::m {

    fun f(): u64 { 1 }

    spec fun f(): u64 { 1 }

    spec module {
        invariant f() > 0;
    }
}
