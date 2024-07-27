//# publish
module 0x42::m {
    use std::bcs;
    use std::string::{Self};
    use std::vector;

    const KEYS: vector<vector<u8>> = vector[];
    const VALUES: vector<u64> = vector[];

    public entry fun init(
    ) {
        let _ = vector::map(KEYS, |key|{ string::utf8(key)});
        let _ = vector::map(VALUES, |v|{ bcs::to_bytes<u64>(&v)});
    }
}

//# run 0x42::m::init
