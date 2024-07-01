module 0x42::test {
	struct S {
		x: u8,
		y: bool,
	}

	struct T {
		z: S,
	}

	fun unused_assign_in_pattern() {
		let x;
		let y;
		let s = S { x: 42, y: true };
		S { x, y } = s;
	}

	fun unused_decl_in_pattern() {
		let s = S { x: 42, y: true };
		let S { x, y } = s;
	}

	fun unused_assign_in_nested_pattern() {
		let x;
		let y;
		let z: S;
		let t = T { z: S { x: 42, y: true } };
		T { z: S { x, y } } = t;
	}

	fun unused_decl_in_nested_pattern() {
		let t = T { z: S { x: 42, y: true } };
		let T { z: S { x, y } } = t;
	}

	fun unused_in_pattern_ok() {
		let s = S { x: 42, y: true };
		let S { x: _x, y: _y }= s;
	}
}
