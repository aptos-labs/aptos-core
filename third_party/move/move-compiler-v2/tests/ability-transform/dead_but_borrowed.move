module 0x42::explicate_drop {
	fun test0(): u8 {
		let x = 42;
		let y = &x;
		*y
	}
}
