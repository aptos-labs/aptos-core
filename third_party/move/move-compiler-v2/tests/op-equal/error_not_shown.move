module 0x42::test {
    struct Coin(u256) has drop;

    fun inc_old(x: &u256) {
        *x = *x + 1;
    }

    fun coin_inc_new_1(self: &Coin) {
        let p = &mut self.0;
        *p += 1;
    }
}
