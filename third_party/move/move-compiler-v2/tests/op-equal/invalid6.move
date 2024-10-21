module 0x42::test {
    fun inc_old(x: &u256) {
        *x = *x + 1;
    }

	fun inc_new(x: &u256) {
         *x += 1;
    }
}
