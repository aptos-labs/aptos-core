module 0x77::m {

    fun f(s: |&u64,bool|(bool,&u64)): (bool,&u64) {
        s(&1, false)
    }
}
