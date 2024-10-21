module 0x42::test {
	fun test() {
		let x = 42;
		let p = &mut x;
		x += 1;
		*p += 1;
		x;
	}
}
