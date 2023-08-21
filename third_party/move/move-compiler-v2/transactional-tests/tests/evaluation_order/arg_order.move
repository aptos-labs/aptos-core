//# publish
module 0x42::test {
    public fun two_args(x: u64, b: bool): u64 {
        if (b) {
            x
        } else {
            0
        }
    }
}

//# run
script {
    use 0x42::test::two_args;
    fun mymain() {
        assert!(two_args(42, true) == 42, 1);
    }
}
