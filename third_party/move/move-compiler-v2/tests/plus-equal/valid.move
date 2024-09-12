module 0x42::test {
	use std::vector;

	struct Coin(u256) has drop;

	struct Wrapper<T>(T);

	fun add1_old(x: u256): u256 {
		x = x + 1;
		x
	}

	fun add1_new(x: u256): u256 {
		x += 1;
		x
	}

	fun inc_new(x: &mut u256) {
		*x += 1;
	}

	fun inc_old(x: &mut u256) {
		*x = *x + 1;
	}

	fun coin_inc_new_1(self: &mut Coin) {
		self.0 += 1;
	}

	fun coin_inc_new_2(self: &mut Coin) {
		let p = &mut self.0;
		*p = *p + 1;
	}

	fun coin_inc_old_1(self: &mut Coin) {
		self.0 = self.0 + 1;
	}

	fun coin_inc_old_2(self: &mut Coin) {
		let p = &mut self.0;
		*p = *p + 1;
	}

	fun inc_wrapped_coin_new(x: &mut Wrapper<Coin>) {
		x.0.0 += 1;
	}

	fun inc_wrapped_coin_old(x: &mut Wrapper<Coin>) {
		x.0.0 = x.0.0 + 1;
	}

	fun inc_vec_new(x: &mut vector<u256>, index: u64) {
		x[index] += 1;
	}

	fun inc_vec_old(x: vector<u256>, index: u64) {
		x[index] = x[index] + 1;
	}

	fun inc_vec_coin_new(x: vector<Coin>, index: u64) {
		x[index].0 += 1;
	}

	fun inc_vec_coin_old(x: vector<Coin>, index: u64) {
		x[index].0 = x[index].0 + 1;
	}
}
