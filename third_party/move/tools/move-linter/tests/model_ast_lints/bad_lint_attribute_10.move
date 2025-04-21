module 0xc0ffee::m {
    public fun test() {
        #[lint::skip(while_true)]
        while (true) {
            // do nothing
        }
    }

}
