//#publish --print-bytecode
module 0xcafe::vectors {
    use std::vector;

    // multi-break
    public entry fun guess_flips_break2(flips: vector<u8>) : u64 {
        let flipsref5 = &flips;
        let _v = copy flips; // this is ok
        // the following stresses live var analysis.
        let _v2 = flips;
	let x = flipsref5;
	vector::length(x)
    }
}
