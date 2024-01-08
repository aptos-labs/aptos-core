module 0x42::test {
	struct Impotent {}

	fun test(): Impotent {
		let x = Impotent {};
		if (false) {
			return x;
		} else {

		};
		return Impotent {}
	}
}
