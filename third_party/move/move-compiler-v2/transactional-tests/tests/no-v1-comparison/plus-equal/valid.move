//# publish
module 0x42::test {
	struct Coin(u256) has drop;

	struct Wrapper<T>(T) has drop;

	fun add1_old(x: u256): u256 {
		x = x + 1;
		x
	}

	fun add1_new(x: u256): u256 {
		x += 1;
		x
	}

	fun test1() {
		assert!(add1_old(42) == add1_new(42));
	}

	fun inc_new(x: &mut u256) {
		*x += 1;
	}

	fun inc_old(x: &mut u256) {
		*x = *x + 1;
	}

	fun test2() {
		let x = 42;
		let y = x;
		inc_new(&mut x);
		inc_old(&mut y);
		assert!(x == y);
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

	fun test3() {
		let x = Coin(42);
		let y = Coin(42);
		let z = Coin(42);
		let w = Coin(42);
		coin_inc_new_1(&mut x);
		coin_inc_new_2(&mut y);
		coin_inc_old_1(&mut z);
		coin_inc_old_2(&mut w);
		assert!(&x == &y);
		assert!(&x == &z);
		assert!(&x == &w);
	}

	fun inc_wrapped_coin_new(x: &mut Wrapper<Coin>) {
		x.0.0 += 1;
	}

	fun inc_wrapped_coin_old(x: &mut Wrapper<Coin>) {
		x.0.0 = x.0.0 + 1;
	}

	fun test4() {
		let x = Wrapper(Coin(42));
		let y = Wrapper(Coin(42));
		inc_wrapped_coin_new(&mut x);
		inc_wrapped_coin_old(&mut y);
		assert!(x == y);
	}

	fun inc_vec_new(x: &mut vector<u256>, index: u64) {
		x[index] += 1;
	}

	fun inc_vec_old(x: &mut vector<u256>, index: u64) {
		x[index] = x[index] + 1;
	}

	fun test5() {
		let x = vector<u256>[42];
		let y = vector<u256>[42];
		inc_vec_new(&mut x, 0);
		inc_vec_old(&mut y, 0);
		assert!(x == y);
	}

	fun inc_vec_coin_new(x: vector<Coin>, index: u64): vector<Coin> {
		x[index].0 += 1;
		x
	}

	fun inc_vec_coin_old(x: vector<Coin>, index: u64): vector<Coin> {
		x[index].0 = x[index].0 + 1;
		x
	}

	fun test6() {
		let x = vector<Coin>[Coin(42)];
		let y = vector<Coin>[Coin(42)];
		let x = inc_vec_coin_new(x, 0);
		let y = inc_vec_coin_old(y, 0);
		assert!(x == y);
	}

	fun inc_vec_wrapped_coin_new(x: vector<Wrapper<Coin>>, index: u64): vector<Wrapper<Coin>> {
		x[index].0.0 += 1;
		x
	}

	fun inc_vec_wrapped_coin_old(x: vector<Wrapper<Coin>>, index: u64): vector<Wrapper<Coin>> {
		x[index].0.0 = x[index].0.0 + 1;
		x
	}

	fun test7() {
		let x = vector<Wrapper<Coin>>[Wrapper(Coin(42))];
		let y = vector<Wrapper<Coin>>[Wrapper(Coin(42))];
		let x = inc_vec_wrapped_coin_new(x, 0);
		let y = inc_vec_wrapped_coin_old(y, 0);
		assert!(x == y);
	}

	fun x_plusplus(x: &mut u64): u64 {
		let res = *x;
		*x += 1;
		res
	}

	fun test8(): vector<u256> {
		let x = 0;
		let y = vector<u256>[0, 1];
		y[x_plusplus(&mut x)] += 1;
		y
	}

}

//# run --verbose -- 0x42::test::test1

//# run --verbose -- 0x42::test::test2

//# run --verbose -- 0x42::test::test3

//# run --verbose -- 0x42::test::test4

//# run --verbose -- 0x42::test::test5

//# run --verbose -- 0x42::test::test6

//# run --verbose -- 0x42::test::test7

//# run --verbose -- 0x42::test::test8
