//# publish
module 0x42::m {
    use std::vector;
    const KEYS: vector<vector<vector<u8>>> = vector[vector[]];
    const VALUES: vector<vector<u64>> = vector[vector[]];

    public entry fun init() {
        let _x: vector<u64> = vector::map<vector<u8>, u64>(*vector::borrow(&KEYS, 0), |key| { let t: vector<u8> = key; (vector::length<u8>(&t) + 2) });
        let _y: vector<u64> = vector::map<u64, u64>(*vector::borrow(&VALUES, 0), |v| { (v + 3u64) });
    }
}

//# run 0x42::m::init
