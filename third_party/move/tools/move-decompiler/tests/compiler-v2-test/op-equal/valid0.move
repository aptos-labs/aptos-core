module 0x42::test {
	struct Coin(u256) has drop, key;
	fun coin_inc_old_1(self: &mut Coin) {
		self.0 = self.0 + 1;
	}
}
