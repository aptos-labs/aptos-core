/// The drand randomness beacon outputs a piece of randomness `r_i` for every round `i` such that anybody can verify
/// it against drand's public key `pk`.
///
/// Verification is possible because `r_i` is simply a BLS signature computed over `i` under the secret key `sk`
/// corresponding to `pk`.
///
/// Rounds happen once every 3 seconds (for the "bls-unchained-on-g1" beacon). This way, given a UNIX timestamp, one can
/// easily derive the round # `i` that drand should have signed to produce randomness for that round.
///
/// The parameters of the "bls-unchained-on-g1" drand beacon, which are hardcoded in this module, were obtained from
/// querying [the drand REST API](https://api.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/info).
///
/// ```
/// {
///     "public_key": "a0b862a7527fee3a731bcb59280ab6abd62d5c0b6ea03dc4ddf6612fdfc9d01f01c31542541771903475eb1ec6615f8d0df0b8b6dce385811d6dcf8cbefb8759e5e616a3dfd054c928940766d9a5b9db91e3b697e5d70a975181e007f87fca5e",
///     "period": 3,
///     "genesis_time": 1677685200,
///     "hash": "dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493",
///     "groupHash": "a81e9d63f614ccdb144b8ff79fbd4d5a2d22055c0bfe4ee9a8092003dab1c6c0",
///     "schemeID": "bls-unchained-on-g1",
///     "metadata": {"beaconID": "fastnet"}
/// }
/// ```

module drand::drand {
    use std::hash::{sha3_256, sha2_256};
    use std::option::{Self, Option, extract};
    use std::vector;
    use std::error;
    use velor_std::crypto_algebra::{eq, pairing, one, deserialize, hash_to, from_u64, serialize};
    use velor_std::bls12381_algebra::{G1, G2, Gt, FormatG2Compr, FormatG1Compr, HashG1XmdSha256SswuRo, Fr, FormatFrMsb};

    /// The `bls-unchained-on-g1` drand beacon produces an output every 3 seconds.
    /// (Or goes into catchup mode, if nodes fall behind.)
    const PERIOD_SECS : u64 = 3;

    /// The UNIX time (in seconds) at which the beacon started operating (this is the time of round #1)
    const GENESIS_TIMESTAMP : u64 = 1677685200;

    /// The drand beacon's PK, against which any beacon output for a round `i` can be verified.
    const DRAND_PUBKEY : vector<u8> = x"a0b862a7527fee3a731bcb59280ab6abd62d5c0b6ea03dc4ddf6612fdfc9d01f01c31542541771903475eb1ec6615f8d0df0b8b6dce385811d6dcf8cbefb8759e5e616a3dfd054c928940766d9a5b9db91e3b697e5d70a975181e007f87fca5e";

    /// The domain-separation tag (DST) used in drand's BLS signatures
    const DRAND_DST: vector<u8> = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";

    /// Error code for when anyone submits an incorrect randomness in our APIs (e.g., wrong-size).
    const E_INCORRECT_RANDOMNESS: u64 = 1;

    /// A randomness object, created from a successfully-verified piece of drand randomness.
    /// This object can be converted into a uniform random integer.
    struct Randomness has drop {
        bytes: vector<u8>
    }

    /// Checks if the randomness in `signature` verifies for the specified `round`.
    /// If it verifies, returns the actual randomness, which is a hash function applied over `signature`.
    public fun verify_and_extract_randomness(
        signature: vector<u8>,
        round: u64): Option<Randomness>
    {
        let pk = extract(&mut deserialize<G2, FormatG2Compr>(&DRAND_PUBKEY));
        let sig = extract(&mut deserialize<G1, FormatG1Compr>(&signature));
        let msg_hash = hash_to<G1, HashG1XmdSha256SswuRo>(&DRAND_DST, &round_number_to_bytes(round));
        assert!(eq(&pairing<G1, G2, Gt>(&msg_hash, &pk), &pairing<G1, G2, Gt>(&sig, &one<G2>())), 1);
        option::some(Randomness {
            bytes: sha3_256(signature)
        })
    }

    /// Returns a uniform number in $[0, max)$ given some drand (verified) `randomness`.
    /// (Technically, there is a small, computationally-indistinguishable bias in the number.)
    /// Note: This is a one-shot API that consumes the `randomness`.
    public fun random_number(randomness: Randomness, max: u64): u64 {
        assert!(vector::length(&randomness.bytes) >= 8, error::invalid_argument(E_INCORRECT_RANDOMNESS));

        let entropy = sha3_256(randomness.bytes);

        // We can convert the 256 uniform bits in `randomness` into a uniform 64-bit number `w \in [0, max)` by
        // taking the last 128 bits in `randomness` modulo `max`.
        let num : u256 = 0;
        let max_256 = (max as u256);

        // Ugh, we have to manually deserialize this into a u128
        while (!vector::is_empty(&entropy)) {
            let byte = vector::pop_back(&mut entropy);
            num = num << 8;
            num = num + (byte as u256);
        };

        ((num % max_256) as u64)
    }

    /// Returns the next round `i` that `drand` will sign after having signed the round corresponding to the
    /// timestamp `unix_time_in_secs`.
    public fun next_round_after(unix_time_in_secs: u64): u64 {
        let (next_round, _) = next_round_and_timestamp_after(unix_time_in_secs);

        next_round
    }

    /// Returns the next round and its UNIX time (after the round at time `unix_time_in_secs`).
    /// (Round at time `GENESIS_TIMESTAMP` is round # 1. Round 0 is fixed.)
    public fun next_round_and_timestamp_after(unix_time_in_secs: u64): (u64, u64) {
        if(unix_time_in_secs < GENESIS_TIMESTAMP) {
            return (1, GENESIS_TIMESTAMP)
        };

        let duration = unix_time_in_secs - GENESIS_TIMESTAMP;

        // As described in https://github.com/drand/drand/blob/0678331f90c87329a001eca4031da8259f6d1d3d/chain/time.go#L57:
        //  > We take the time from genesis divided by the periods in seconds.
        //  > That gives us the number of periods since genesis.
        //  > We add +1 since we want the next round.
        //  > We also add +1 because round 1 starts at genesis time.

        let next_round = (duration / PERIOD_SECS) + 1;
        let next_time = GENESIS_TIMESTAMP + next_round * PERIOD_SECS;

        (next_round + 1, next_time)
    }

    //
    // Internals
    //

    /// drand signatures are not over the round # directly, but over the SHA2-256 of the 8 bytes (little-endian)
    /// representation of the round #.
    ///
    /// See:
    ///  - https://drand.love/docs/specification/#beacon-signature
    ///  - https://github.com/drand/drand/blob/v1.2.1/chain/store.go#L39-L44
    fun round_number_to_bytes(round: u64): vector<u8> {
        let buf = serialize<Fr, FormatFrMsb>(&from_u64<Fr>(round));
        sha2_256(std::vector::trim(&mut buf, 24))
    }
}
