// bls12381 natives on fixed aptos_std test vectors, so outputs do not depend on either VM:
// two public keys outside the prime-order subgroup fail validation beside a valid one, and
// aggregating five public keys and five signatures gives the published values. Aggregation
// with a malformed element returns empty bytes and false, and the subgroup check rejects a
// one-byte signature.

// RUN: publish
module 0x1::bls12381 {
    struct PublicKeyWithPoP has copy, drop, store {
        bytes: vector<u8>,
    }

    struct Signature has copy, drop, store {
        bytes: vector<u8>,
    }

    native fun validate_pubkey_internal(public_key: vector<u8>): bool;
    native fun signature_subgroup_check_internal(signature: vector<u8>): bool;
    native fun aggregate_pubkeys_internal(public_keys: vector<PublicKeyWithPoP>): (vector<u8>, bool);
    native fun aggregate_signatures_internal(signatures: vector<Signature>): (vector<u8>, bool);

    fun pks(): vector<PublicKeyWithPoP> {
        vector[
            PublicKeyWithPoP { bytes: x"92e201a806af246f805f460fbdc6fc90dd16a18d6accc236e85d3578671d6f6690dde22134d19596c58ce9d63252410a" },
            PublicKeyWithPoP { bytes: x"ab9df801c6f96ade1c0490c938c87d5bcc2e52ccb8768e1b5d14197c5e8bfa562783b96711b702dda411a1a9f08ebbfa" },
            PublicKeyWithPoP { bytes: x"b698c932cf7097d99c17bd6e9c9dc4eeba84278c621700a8f80ec726b1daa11e3ab55fc045b4dbadefbeef05c4182494" },
            PublicKeyWithPoP { bytes: x"934706a8b876d47a996d427e1526ce52c952d5ec0858d49cd262efb785b62b1972d06270b0a7adda1addc98433ad1843" },
            PublicKeyWithPoP { bytes: x"a4cd352daad3a0651c1998dfbaa7a748e08d248a54347544bfedd51a197e016bb6008e9b8e45a744e1a030cc3b27d2da" },
        ]
    }

    fun sigs(): vector<Signature> {
        vector[
            Signature { bytes: x"a55ac2d64b4c1d141b15d876d3e54ad1eea07ee488e8287cce7cdf3eec551458ab5795ab196f8c112590346f7bc7c97e0053cd5be0f9bd74b93a87cd44458e98d125d6d5c6950ea5e62666beb34422ead79121f8cb0815dae41a986688d03eaf" },
            Signature { bytes: x"90a639a44491191c46379a843266c293de3a46197714ead2ad3886233dd5c2b608b6437fa32fbf9d218b20f1cbfa7970182663beb9c148e2e9412b148e16abf283ffa51b8a536c0e55d61b2e5c849edc49f636c0ef07cb99f125cbcf602e22bb" },
            Signature { bytes: x"9527d81aa15863ef3a3bf96bea6d58157d5063a93a6d0eb9d8b4f4bbda3b31142ec4586cb519da2cd7600941283d1bad061b5439703fd584295b44037a969876962ae1897dcc7cadf909d06faae213c4fef8e015dfb33ec109af02ab0c3f6833" },
            Signature { bytes: x"a54d264f5cab9654b1744232c4650c42b29adf2b19bd00bbdaf4a4d792ee4dfd40a1fe1b067f298bcfd8ae4fdc8250660a2848bd4a80d96585afccec5c6cfa617033dd7913c9acfdf98a72467e8a5155d4cad589a72d6665be7cb410aebc0068" },
            Signature { bytes: x"8d22876bdf73e6ad36ed98546018f6258cd47e45904b87c071e774a6ef4b07cac323258cb920b2fe2b07cca1f2b24bcb0a3194ec76f32edb92391ed2c39e1ada8919f8ea755c5e39873d33ff3a8f4fba21b1261c1ddb9d1688c2b40b77e355d1" },
        ]
    }

    public fun validate(): (bool, bool, bool) {
        (
            validate_pubkey_internal(x"ae3cd9403b69c20a0d455fd860e977fe6ee7140a7f091f26c860f2caccd3e0a7a7365798ac10df776675b3a67db8faa0"),
            validate_pubkey_internal(x"928d4862a40439a67fd76a9c7560e2ff159e770dcf688ff7b2dd165792541c88ee76c82eb77dd6e9e72c89cbf1a56a68"),
            validate_pubkey_internal(x"b3e4921277221e01ed71284be5e3045292b26c7f465a6fcdba53ee47edd39ec5160da3b229a73c75671024dcb36de091"),
        )
    }

    public fun aggregate_five_pks(): (vector<u8>, bool) {
        aggregate_pubkeys_internal(pks())
    }

    public fun aggregate_pks_malformed(): (vector<u8>, bool) {
        let keys = pks();
        keys.insert(1, PublicKeyWithPoP { bytes: x"00" });
        aggregate_pubkeys_internal(keys)
    }

    public fun aggregate_five_sigs(): (vector<u8>, bool) {
        aggregate_signatures_internal(sigs())
    }

    public fun aggregate_sigs_malformed(): (vector<u8>, bool) {
        let all = sigs();
        all.push_back(Signature { bytes: x"00" });
        aggregate_signatures_internal(all)
    }

    public fun subgroup_wrong_length(): bool {
        signature_subgroup_check_internal(x"00")
    }
}

// RUN: execute 0x1::bls12381::validate
// CHECK: results: false, false, true

// RUN: execute 0x1::bls12381::aggregate_five_pks
// CHECK: results: 0xb53df1cfee2168f59e5792e710bf22928dc0553e6531dae5c7656c0a66fc12cb82fbb04863938c953dc901a5a79cc0f3, true

// RUN: execute 0x1::bls12381::aggregate_pks_malformed
// CHECK: results: 0x, false

// RUN: execute 0x1::bls12381::aggregate_five_sigs
// CHECK: results: 0x8284e4e3983f29cb45020c3e2d89066df2eae533a01cb6ca2c4d466b5e02dd22467f59640aa120db2b9cc49e931415c3097e3d54ff977fd9067b5bc6cfa1c885d9d8821aef20c028999a1d97e783ae049d8fa3d0bbac36ce4ca8e10e551d3461, true

// RUN: execute 0x1::bls12381::aggregate_sigs_malformed
// CHECK: results: 0x, false

// RUN: execute 0x1::bls12381::subgroup_wrong_length
// CHECK: results: false
