module 0x42::test {
	struct Coin(u256) has drop;

	struct Wrapper<T>(T) has drop;

	fun inc_new(x: &u256) {
		*x += 1;
	}

	fun inc_old(x: &u256) {
		*x = *x + 1;
	}

	fun coin_inc_new_1(self: &Coin) {
		self.0 += 1;
	}

	fun coin_inc_old_1(self: &Coin) {
		self.0 = self.0 + 1;
	}

	fun inc_wrapped_coin_new(x: &Wrapper<Coin>) {
		x.0.0 += 1;
	}

	fun inc_vec_new(x: &vector<u256>, index: u64) {
		x[index] += 1;
	}

	fun inc_vec_wrapped_coin_new(x: &vector<Wrapper<Coin>>, index: u64) {
		x[index].0.0 += 1;
	}
}
