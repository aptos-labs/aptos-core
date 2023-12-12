//#publish --print-bytecode
module 0xcafe::vectors {
    use std::vector;

    // multi-break
    public entry fun guess_flips_break2(flips: vector<u8>) : u64 {
        let i = 0;
        let flipsref5 = &flips;
        while (i < vector::length(flipsref5)) {
            if (*vector::borrow(flipsref5, i) != 0) {
                break
            };
            i = i + 1;
            if (*vector::borrow(flipsref5, i) == 5) {
                break
            };
        };
        let _v = copy flips; // this is ok
        // this used to fail with an UNKNOWN_INVARIANT_VIOLATION_ERROR (code 2000)
        let _v2 =  flips;
	let x = flipsref5;
	vector::length(x)
    }

}
