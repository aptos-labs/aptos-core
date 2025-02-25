//# init --addresses alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

//# publish --private-key alice
module alice::foo {

    // --- All these should be okay ---

    public entry fun no_arg() {}

    entry fun one_signer_arg_1(_s: &signer) {}

    entry fun one_signer_arg_2(_s: signer) {}

    entry fun multiple_signers(_s1: &signer, _s2: &signer) {}

    // --- All these should be warnings ---

    entry fun one_signer_later(_x: u64, _s: &signer) {}

    entry fun multiple_signers_later(_x: u64, _y: u64, _s1: &signer, _z: u64, _s2: &signer) {}
}

//# publish --private-key alice
module alice::bar {
    // ---- Should be an error ----
    entry fun return_something(): u64 {
        42
    }
}

//# publish --private-key alice
module alice::baz {
    use std::bit_vector::BitVector;

    // ---- Should be an error ----
    public entry fun invalid_txn_param(_bv: BitVector) {}
}
