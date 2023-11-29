//# publish
module 0x42::Mutual {

    inline fun odd(x: u64): bool {
        if (x == 0) {
         false
        } else {
         even(x - 1)
        }
    }

    fun even(x: u64): bool {
        if (x == 0) {
         return true
        } else {
         return odd(x - 1)
        }
    }

    public fun recursion_check() {
        assert!(odd(5), 0);
        assert!(even(4), 0);
    }


}

//# run 0x42::Mutual::recursion_check
