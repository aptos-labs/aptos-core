//# publish
module 0x66::zz {

    public fun eval(_x: u64) {
        let a: |u64|u64 has drop = |x| x + 1;
        let b = vector[a];

        zzz(&b);
    }

    fun zzz(_a: &vector<|u64|u64>) {}
}
