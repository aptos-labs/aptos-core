// RUN: publish
module 0x1::bls12381_algebra {
    struct Fr {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct FormatFrLsb {}
    struct FormatFrMsb {}
    struct FormatFq12LscLsb {}
    struct FormatG1Compr {}
    struct FormatG1Uncompr {}
    struct FormatG2Compr {}
    struct FormatG2Uncompr {}
    struct FormatGt {}
}

module 0x1::bn254_algebra {
    struct Fr {}
    struct Fq {}
    struct Fq12 {}
    struct G1 {}
    struct G2 {}
    struct Gt {}
    struct FormatFrLsb {}
    struct FormatFrMsb {}
    struct FormatFqLsb {}
    struct FormatFqMsb {}
    struct FormatFq12LscLsb {}
    struct FormatG1Compr {}
    struct FormatG1Uncompr {}
    struct FormatG2Compr {}
    struct FormatG2Uncompr {}
    struct FormatGt {}
}

module 0x1::crypto_algebra {
    use 0x1::bls12381_algebra;
    use 0x1::bn254_algebra;

    const BLS_FQ12_VAL_7_SERIALIZED: vector<u8> = x"070000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    const BLS_G1_GENERATOR_SERIALIZED_UNCOMP: vector<u8> = x"17f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb08b3f481e3aaa0f1a09e30ed741d8ae4fcf5e095d5d00af600db18cb2c04b3edd03cc744a2888ae40caa232946c5e7e1";
    const BLS_G1_GENERATOR_SERIALIZED_COMP: vector<u8> = x"97f1d3a73197d7942695638c4fa9ac0fc3688c4f9774b905a14e3a3f171bac586c55e83ff97a1aeffb3af00adb22c6bb";
    const BLS_G2_GENERATOR_SERIALIZED_UNCOMP: vector<u8> = x"13e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb80606c4a02ea734cc32acd2b02bc28b99cb3e287e85a763af267492ab572e99ab3f370d275cec1da1aaa9075ff05f79be0ce5d527727d6e118cc9cdc6da2e351aadfd9baa8cbdd3a76d429a695160d12c923ac9cc3baca289e193548608b82801";
    const BLS_G2_GENERATOR_SERIALIZED_COMP: vector<u8> = x"93e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8";
    const BLS_GT_GENERATOR_SERIALIZED: vector<u8> = x"b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f";
    const BLS_FR_VAL_7_SERIALIZED_LSB: vector<u8> = x"0700000000000000000000000000000000000000000000000000000000000000";
    const BLS_FR_VAL_7_SERIALIZED_MSB: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000007";
    const BLS_R_SERIALIZED: vector<u8> = x"01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73";
    const BN_FQ12_VAL_7_SERIALIZED: vector<u8> = x"070000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    const BN_G1_GENERATOR_SERIALIZED_UNCOMP: vector<u8> = x"01000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000";
    const BN_G1_GENERATOR_SERIALIZED_COMP: vector<u8> = x"0100000000000000000000000000000000000000000000000000000000000000";
    const BN_G2_GENERATOR_SERIALIZED_UNCOMP: vector<u8> = x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19aa7dfa6601cce64c7bd3430c69e7d1e38f40cb8d8071ab4aeb6d8cdba55ec8125b9722d1dcdaac55f38eb37033314bbc95330c69ad999eec75f05f58d0890609";
    const BN_G2_GENERATOR_SERIALIZED_COMP: vector<u8> = x"edf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19";
    const BN_GT_GENERATOR_SERIALIZED: vector<u8> = x"950e879d73631f5eb5788589eb5f7ef8d63e0a28de1ba00dfe4ca9ed3f252b264a8afb8eb4349db466ed1809ea4d7c39bdab7938821f1b0a00a295c72c2de002e01dbdfd0254134efcb1ec877395d25f937719b344adb1a58d129be2d6f2a9132b16a16e8ab030b130e69c69bd20b4c45986e6744a98314b5c1a0f50faa90b04dbaf9ef8aeeee3f50be31c210b598f4752f073987f9d35be8f6770d83f2ffc0af0d18dd9d2dbcdf943825acc12a7a9ddca45e629d962c6bd64908c3930a5541cfe2924dcc5580d5cef7a4bfdec90a91b59926f850d4a7923c01a5a5dbf0f5c094a2b9fb9d415820fa6b40c59bb9eade9c953407b0fc11da350a9d872cad6d3142974ca385854afdf5f583c04231adc5957c8914b6b20dc89660ed7c3bbe7c01d972be2d53ecdb27a1bcc16ac610db95aa7d237c8ff55a898cb88645a0e32530b23d7ebf5dafdd79b0f9c2ac4ba07ce18d3d16cf36e47916c4cae5d08d3afa813972c769e8514533e380c9443b3e1ee5c96fa3a0a73f301b626454721527bf900";
    const BN_FR_VAL_7_SERIALIZED_LSB: vector<u8> = x"0700000000000000000000000000000000000000000000000000000000000000";
    const BN_FR_VAL_7_SERIALIZED_MSB: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000007";
    const BN_FQ_VAL_7_SERIALIZED_LSB: vector<u8> = x"0700000000000000000000000000000000000000000000000000000000000000";
    const BN_FQ_VAL_7_SERIALIZED_MSB: vector<u8> = x"0000000000000000000000000000000000000000000000000000000000000007";

    native fun deserialize_internal<S, F>(bytes: &vector<u8>): (bool, u64);
    native fun eq_internal<S>(handle_1: u64, handle_2: u64): bool;
    native fun from_u64_internal<S>(value: u64): u64;
    native fun serialize_internal<S, F>(handle: u64): vector<u8>;

    // `serialize(deserialize(bytes)) == bytes` pins the format both ways: a
    // dropped byte reversal or a compressed/uncompressed mixup breaks it.
    fun roundtrip<S, F>(bytes: vector<u8>): bool {
        let (ok, handle) = deserialize_internal<S, F>(&bytes);
        ok && serialize_internal<S, F>(handle) == bytes
    }

    // A rejected encoding is `(false, 0)`, never an abort.
    fun rejected<S, F>(bytes: vector<u8>): bool {
        let (ok, handle) = deserialize_internal<S, F>(&bytes);
        !ok && handle == 0
    }

    fun shorter(bytes: vector<u8>): vector<u8> {
        bytes.pop_back();
        bytes
    }

    fun longer(bytes: vector<u8>): vector<u8> {
        bytes.push_back(0);
        bytes
    }

    fun flip_last(bytes: vector<u8>): vector<u8> {
        let last = bytes.length() - 1;
        bytes[last] = bytes[last] ^ 0xff;
        bytes
    }

    fun reversed(bytes: vector<u8>): vector<u8> {
        let out = vector[];
        let i = bytes.length();
        while (i > 0) {
            i -= 1;
            out.push_back(bytes[i]);
        };
        out
    }

    public fun bls12381_roundtrips(): bool {
        roundtrip<bls12381_algebra::Fr, bls12381_algebra::FormatFrLsb>(BLS_FR_VAL_7_SERIALIZED_LSB)
            && roundtrip<bls12381_algebra::Fr, bls12381_algebra::FormatFrMsb>(
                BLS_FR_VAL_7_SERIALIZED_MSB
            )
            && roundtrip<bls12381_algebra::Fq12, bls12381_algebra::FormatFq12LscLsb>(
                BLS_FQ12_VAL_7_SERIALIZED
            )
            && roundtrip<bls12381_algebra::G1, bls12381_algebra::FormatG1Compr>(
                BLS_G1_GENERATOR_SERIALIZED_COMP
            )
            && roundtrip<bls12381_algebra::G1, bls12381_algebra::FormatG1Uncompr>(
                BLS_G1_GENERATOR_SERIALIZED_UNCOMP
            )
            && roundtrip<bls12381_algebra::G2, bls12381_algebra::FormatG2Compr>(
                BLS_G2_GENERATOR_SERIALIZED_COMP
            )
            && roundtrip<bls12381_algebra::G2, bls12381_algebra::FormatG2Uncompr>(
                BLS_G2_GENERATOR_SERIALIZED_UNCOMP
            )
            && roundtrip<bls12381_algebra::Gt, bls12381_algebra::FormatGt>(
                BLS_GT_GENERATOR_SERIALIZED
            )
    }

    public fun bn254_roundtrips(): bool {
        roundtrip<bn254_algebra::Fr, bn254_algebra::FormatFrLsb>(BN_FR_VAL_7_SERIALIZED_LSB)
            && roundtrip<bn254_algebra::Fr, bn254_algebra::FormatFrMsb>(BN_FR_VAL_7_SERIALIZED_MSB)
            && roundtrip<bn254_algebra::Fq, bn254_algebra::FormatFqLsb>(BN_FQ_VAL_7_SERIALIZED_LSB)
            && roundtrip<bn254_algebra::Fq, bn254_algebra::FormatFqMsb>(BN_FQ_VAL_7_SERIALIZED_MSB)
            && roundtrip<bn254_algebra::Fq12, bn254_algebra::FormatFq12LscLsb>(
                BN_FQ12_VAL_7_SERIALIZED
            )
            && roundtrip<bn254_algebra::G1, bn254_algebra::FormatG1Compr>(
                BN_G1_GENERATOR_SERIALIZED_COMP
            )
            && roundtrip<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(
                BN_G1_GENERATOR_SERIALIZED_UNCOMP
            )
            && roundtrip<bn254_algebra::G2, bn254_algebra::FormatG2Compr>(
                BN_G2_GENERATOR_SERIALIZED_COMP
            )
            && roundtrip<bn254_algebra::G2, bn254_algebra::FormatG2Uncompr>(
                BN_G2_GENERATOR_SERIALIZED_UNCOMP
            )
            && roundtrip<bn254_algebra::Gt, bn254_algebra::FormatGt>(BN_GT_GENERATOR_SERIALIZED)
    }

    // The MSB encoding is the LSB encoding byte-reversed. Serializing both and
    // reversing one catches an MSB arm that forgot to reverse.
    public fun msb_is_reversed_lsb(): bool {
        let bls_seven = from_u64_internal<bls12381_algebra::Fr>(7);
        let bn_seven = from_u64_internal<bn254_algebra::Fr>(7);
        let bn_fq_seven = from_u64_internal<bn254_algebra::Fq>(7);
        serialize_internal<bls12381_algebra::Fr, bls12381_algebra::FormatFrMsb>(bls_seven)
            == reversed(
                serialize_internal<bls12381_algebra::Fr, bls12381_algebra::FormatFrLsb>(bls_seven)
            )
            && serialize_internal<bn254_algebra::Fr, bn254_algebra::FormatFrMsb>(bn_seven)
                == reversed(
                    serialize_internal<bn254_algebra::Fr, bn254_algebra::FormatFrLsb>(bn_seven)
                )
            && serialize_internal<bn254_algebra::Fq, bn254_algebra::FormatFqMsb>(bn_fq_seven)
                == reversed(
                    serialize_internal<bn254_algebra::Fq, bn254_algebra::FormatFqLsb>(bn_fq_seven)
                )
    }

    // The length check runs before decoding, so one byte either way is refused.
    public fun wrong_length_rejected(): bool {
        rejected<bls12381_algebra::Fr, bls12381_algebra::FormatFrLsb>(
            shorter(BLS_FR_VAL_7_SERIALIZED_LSB)
        )
            && rejected<bls12381_algebra::Fr, bls12381_algebra::FormatFrLsb>(
                longer(BLS_FR_VAL_7_SERIALIZED_LSB)
            )
            && rejected<bls12381_algebra::Fr, bls12381_algebra::FormatFrMsb>(
                shorter(BLS_FR_VAL_7_SERIALIZED_MSB)
            )
            && rejected<bls12381_algebra::Fq12, bls12381_algebra::FormatFq12LscLsb>(
                shorter(BLS_FQ12_VAL_7_SERIALIZED)
            )
            && rejected<bls12381_algebra::G1, bls12381_algebra::FormatG1Compr>(
                shorter(BLS_G1_GENERATOR_SERIALIZED_COMP)
            )
            && rejected<bls12381_algebra::G1, bls12381_algebra::FormatG1Uncompr>(
                longer(BLS_G1_GENERATOR_SERIALIZED_UNCOMP)
            )
            && rejected<bls12381_algebra::G2, bls12381_algebra::FormatG2Compr>(
                shorter(BLS_G2_GENERATOR_SERIALIZED_COMP)
            )
            && rejected<bls12381_algebra::G2, bls12381_algebra::FormatG2Uncompr>(
                longer(BLS_G2_GENERATOR_SERIALIZED_UNCOMP)
            )
            && rejected<bls12381_algebra::Gt, bls12381_algebra::FormatGt>(
                shorter(BLS_GT_GENERATOR_SERIALIZED)
            )
            && rejected<bn254_algebra::Fq, bn254_algebra::FormatFqMsb>(
                longer(BN_FQ_VAL_7_SERIALIZED_MSB)
            )
            && rejected<bn254_algebra::G1, bn254_algebra::FormatG1Compr>(
                longer(BN_G1_GENERATOR_SERIALIZED_COMP)
            )
            && rejected<bn254_algebra::G2, bn254_algebra::FormatG2Uncompr>(
                shorter(BN_G2_GENERATOR_SERIALIZED_UNCOMP)
            )
            && rejected<bn254_algebra::Gt, bn254_algebra::FormatGt>(
                shorter(BN_GT_GENERATOR_SERIALIZED)
            )
    }

    // A field element equal to the modulus is out of range; a tampered curve
    // point is off the curve or off the subgroup.
    public fun out_of_range_rejected(): bool {
        rejected<bls12381_algebra::Fr, bls12381_algebra::FormatFrLsb>(BLS_R_SERIALIZED)
            && rejected<bls12381_algebra::G1, bls12381_algebra::FormatG1Compr>(
                flip_last(BLS_G1_GENERATOR_SERIALIZED_COMP)
            )
            && rejected<bls12381_algebra::G1, bls12381_algebra::FormatG1Uncompr>(
                flip_last(BLS_G1_GENERATOR_SERIALIZED_UNCOMP)
            )
            && rejected<bls12381_algebra::G2, bls12381_algebra::FormatG2Uncompr>(
                flip_last(BLS_G2_GENERATOR_SERIALIZED_UNCOMP)
            )
            && rejected<bn254_algebra::G1, bn254_algebra::FormatG1Uncompr>(
                flip_last(BN_G1_GENERATOR_SERIALIZED_UNCOMP)
            )
    }

    // `Gt` decodes as `Fq12` and then checks the `r`-th root of unity
    // condition, so a valid `Fq12` element outside `Gt` is refused.
    public fun fq12_element_is_not_gt(): bool {
        roundtrip<bls12381_algebra::Fq12, bls12381_algebra::FormatFq12LscLsb>(
            BLS_FQ12_VAL_7_SERIALIZED
        )
            && rejected<bls12381_algebra::Gt, bls12381_algebra::FormatGt>(
                BLS_FQ12_VAL_7_SERIALIZED
            )
            && roundtrip<bn254_algebra::Fq12, bn254_algebra::FormatFq12LscLsb>(
                BN_FQ12_VAL_7_SERIALIZED
            )
            && rejected<bn254_algebra::Gt, bn254_algebra::FormatGt>(BN_FQ12_VAL_7_SERIALIZED)
    }

    // Deserializing the same bytes twice yields distinct handles that compare
    // equal, and the compressed and uncompressed encodings agree on the point.
    public fun formats_agree_on_the_same_point(): bool {
        let (ok_c, compressed) =
            deserialize_internal<bls12381_algebra::G1, bls12381_algebra::FormatG1Compr>(
                &BLS_G1_GENERATOR_SERIALIZED_COMP
            );
        let (ok_u, uncompressed) =
            deserialize_internal<bls12381_algebra::G1, bls12381_algebra::FormatG1Uncompr>(
                &BLS_G1_GENERATOR_SERIALIZED_UNCOMP
            );
        ok_c
            && ok_u
            && compressed != uncompressed
            && eq_internal<bls12381_algebra::G1>(compressed, uncompressed)
    }
}

// RUN: execute 0x1::crypto_algebra::bls12381_roundtrips
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::bn254_roundtrips
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::msb_is_reversed_lsb
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::wrong_length_rejected
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::out_of_range_rejected
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::fq12_element_is_not_gt
// CHECK: results: true

// RUN: execute 0x1::crypto_algebra::formats_agree_on_the_same_point
// CHECK: results: true
