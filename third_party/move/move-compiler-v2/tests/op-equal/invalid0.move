module 0x42::test {
	struct Coin(u256) has drop;

	struct Wrapper<T>(T) has drop;


	fun coin_inc_new_1(self: &Coin) {
		self.0 += 1;
	}

	fun inc_vec_new(x: &vector<u256>, index: u64) {
		x[index] += 1;
	}

	fun inc_vec_wrapped_coin_new(x: &vector<Wrapper<Coin>>, index: u64) {
		x[index].0.0 += 1;
	}
}
