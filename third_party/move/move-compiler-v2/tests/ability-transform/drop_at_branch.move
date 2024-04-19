module 0x42::explicate_drop {
	fun drop_at_branch(x: bool): u8 {
		if (x) {
			1
		} else {
			0
		}
	}
}
