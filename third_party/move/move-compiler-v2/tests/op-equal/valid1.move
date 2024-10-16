module 0x42::test {
	struct Coin(u256) has drop, key;

	struct Wrapper<T>(T) has drop, key;

	fun sub1(x: &mut u256) {
		*x -= 1;
	}

	fun coin_double(self: &mut Coin) {
		self.0 *= 2;
	}

	fun coin_mod_2(self: &mut Coin) {
		self.0 %= 2;
	}

	fun half_wrapped_coin_new(x: &mut Wrapper<Coin>) {
		x.0.0 /= 2;
	}

	fun bitor_vec_new(x: &mut vector<u256>, index: u64) {
		x[index] |= 42;
	}

	fun bitand_vec_coin_new(x: vector<Coin>, index: u64) {
		x[index].0 &= 42;
	}

	fun xor_vec_wrapped_coin_new(x: vector<Wrapper<Coin>>, index: u64) {
		x[index].0.0 ^= 1;
	}

	fun shl_vec_wrapped_coin_old(x: vector<Wrapper<Coin>>, index: u64) {
		x[index].0.0 <<= 1;
	}

	fun shr_coin_at(addr: address) acquires Coin {
		let coin = &mut Coin[addr];
		coin.0 >>= 1;
	}
}
