// Move bytecode v6
module cafe.test_add {


entry public calibrate_add_1() {
B0:
	0: LdU64(1)
	1: Pop
	2: Ret
}
entry public calibrate_add_2() {
L0:	loc0: u64
B0:
	0: LdU64(0)
	1: StLoc[0](loc0: u64)
	2: CopyLoc[0](loc0: u64)
	3: MoveLoc[0](loc0: u64)
	4: Add
	5: LdU64(1)
	6: Add
	7: Pop
	8: Ret
}
}
