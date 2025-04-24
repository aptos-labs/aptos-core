module poc::aggregate_signatures_internal {
    use aptos_std::bls12381::{Self, Signature};
    use std::vector;

    public entry fun main(_owner: &signer) {
        let sig_bytes1 = x"a55ac2d64b4c1d141b15d876d3e54ad1eea07ee488e8287cce7cdf3eec551458ab5795ab196f8c112590346f7bc7c97e0053cd5be0f9bd74b93a87cd44458e98d125d6d5c6950ea5e62666beb34422ead79121f8cb0815dae41a986688d03eaf";
        let sig_bytes2 = x"90a639a44491191c46379a843266c293de3a46197714ead2ad3886233dd5c2b608b6437fa32fbf9d218b20f1cbfa7970182663beb9c148e2e9412b148e16abf283ffa51b8a536c0e55d61b2e5c849edc49f636c0ef07cb99f125cbcf602e22bb";

        let sig1 = bls12381::signature_from_bytes(sig_bytes1);
        let sig2 = bls12381::signature_from_bytes(sig_bytes2);

        let sigs = vector::empty<Signature>();
        vector::push_back(&mut sigs, sig1);
        vector::push_back(&mut sigs, sig2);

        let agg_sig_opt = bls12381::aggregate_signatures(sigs);
        let _ = agg_sig_opt;
    }

    #[test(owner = @0x123)]
    fun a(owner: &signer) {
        main(owner);
    }
}
