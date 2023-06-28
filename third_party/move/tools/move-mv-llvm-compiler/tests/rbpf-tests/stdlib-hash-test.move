
// This is move-stdlib/sources/hash.move until we build move-stdlib.


/// Module which defines SHA hashes for byte vectors.
///
/// The functions in this module are natively declared both in the Move runtime
/// as in the Move prover's prelude.
//module std::hash {
module 0x10::hash {
    native public fun sha2_256(data: vector<u8>): vector<u8>;
    native public fun sha3_256(data: vector<u8>): vector<u8>;
}


module 0x10::hash_tests {
    use 0x10::hash;

    public fun sha2_256_expected_hash() {
        let input = x"616263";
        let expected_output = x"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(hash::sha2_256(input) == expected_output, 0);
    }

    public fun sha3_256_expected_hash() {
        let input = x"616263";
        let expected_output = x"3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532";
        assert!(hash::sha3_256(input) == expected_output, 0);
    }
}

script {
    use 0x10::hash_tests as HT;

    fun main() {
        HT::sha2_256_expected_hash();
        HT::sha3_256_expected_hash();
    }
}
