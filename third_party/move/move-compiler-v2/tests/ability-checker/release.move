module 0x42::test {
	struct Impotent {}

	fun test() {
		let x = Impotent {};
		let y = &x;
	}
}
