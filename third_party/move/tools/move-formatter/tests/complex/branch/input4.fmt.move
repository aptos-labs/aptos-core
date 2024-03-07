module test {
    fun f(x: u64): u64 {
        if (x == 1) {
            if (x == 11) { 11 }
            else if (x == 12) {
                ();
                12
            }
            else if (x == 13) {
                ();
                13
            }
            else {
                ();
                14
            }
        }
        else if (x == 2) { 3 }
        else if (x == 3) { 4 }
        else { 5 }
    }
}