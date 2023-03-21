/// The drand randomness beacon outputs a piece of randomness `r_i` for every round `i` such that anybody can verify
/// it against drand's public key `pk`.
///
/// Verification is possible because `r_i` is simply a BLS signature computed over `i` under the secret key `sk`
/// corresponding to `pk`.
///
/// Rounds happen once every 3 seconds. This way, given a UNIX timestamp, one can easily derive the round # `i` that
/// drand should have signed to produce randomness for that round.
///
/// The parameters of the "unchained" drand beacon, which are hardcoded in this module, were obtained from querying
/// [the drand REST API](https://api.drand.sh/dbd506d6ef76e5f386f41c651dcb808c5bcbd75471cc4eafa3f4df7ad4e4c493/info).
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
    use std::hash::sha3_256;
    use std::option::{Self, Option};
    use std::vector;
    use std::error;

    /// The `bls-unchained-on-g1` drand beacon produces an output every 3 seconds. (Or goes into catchup mode, if nodes fall behind.)
    const PERIOD_SECS : u64 = 3;

    /// The UNIX time (in seconds) at which the beacon started operating (this is the time of round #1)
    const GENESIS_TIMESTAMP : u64 = 1677685200;

    /// The drand beacon's PK, against which any beacon output for a round `i` can be verified.
    const DRAND_PUBKEY : vector<u8> = x"a0b862a7527fee3a731bcb59280ab6abd62d5c0b6ea03dc4ddf6612fdfc9d01f01c31542541771903475eb1ec6615f8d0df0b8b6dce385811d6dcf8cbefb8759e5e616a3dfd054c928940766d9a5b9db91e3b697e5d70a975181e007f87fca5e";

    /// Error code for when anyone submits an incorrect randomness in our APIs (e.g., wrong-size).
    const E_INCORRECT_RANDOMNESS: u64 = 1;

    /// Returns the drand randomness produced in the next round after 'current_time'.
    ///
    /// Specifically, checks if `drand_signed_bytes` is a valid BLS signature for the round that succeeds the
    /// round at `current_time`. If this verifies, returns the actual randomness, which is a hash function applied over
    /// the signature in `drand_signed_bytes`.
    public fun verify_and_extract_next_randomness(
        drand_signed_bytes: vector<u8>,
        current_time: u64): Option<vector<u8>>
    {
        // Determine the next drand round after `current_time`
        let _drand_round = next_round_after(current_time);

        // TODO(Security): We'll want a more type-safe API that wraps the signature bytes inside a `RandomnessProof` and maybe returns a `Randomness` struct that has helper methods like `random(dst, lower, upper)` or `random_bit(dst)`, where `dst` is a domain-separator.
        // TODO(Security): We'll want to incorporate type information about the contract as a domain-separator in the hash function

        option::some(sha3_256(drand_signed_bytes))
    }

    /// Returns the drand randomness produced in the round at 'current_time'.
    public fun verify_and_extract_randomness(
        drand_signed_bytes: vector<u8>,
        current_time: u64): Option<vector<u8>>
    {
        // Determine the next drand round after `current_time`
        let _drand_round = current_round(current_time);

        // TODO: Implement BLS signature verification as per https://drand.love/docs/specification/#cryptographic-specification
        // e.g., drand_bls_signature_verify(bytes, _drand_round, DRAND_PUBKEY)

        option::some(sha3_256(drand_signed_bytes))
    }

    /// Returns a number in [0, n) given some drand (verified) `randomness` and a domain-separator `dst`.
    public fun uniform_random_less_than_n(dst: vector<u8>, randomness: vector<u8>, n: u64): u64 {
        assert!(vector::length(&randomness) >= 8, error::invalid_argument(E_INCORRECT_RANDOMNESS));

        // Use H(randomness || dst) as our entropy
        vector::append(&mut randomness, dst);
        let entropy = sha3_256(randomness);

        // TODO(Security): To properly map the 256 uniform bits in `randomness` into a uniform number `w \in [0, n)`, computing `randomness % n` is not correct: the result will not be uniform.
        // See
        // - https://crypto.stackexchange.com/questions/104252/how-to-generate-random-numbers-within-a-range-0-n-from-random-bits
        // - https://github.com/owlstead/RNG-BC
        // ..for solutions

        // Take the last 64 bits modulo n to get a (somewhat-biased) number in [0, n)
        let last_64_bits = vector::trim(&mut entropy, 8);
        let num = 0;

        // Ugh, we have to manually deserialize this into a u64 (8 chunks of 8 bits each)
        while (!vector::is_empty(&last_64_bits)) {
            let byte = vector::pop_back(&mut last_64_bits);
            num = num << 8;
            num = num + (byte as u64);
        };

        num % n
    }

    /// Returns the next round `i` that `drand` will sign after having signed the round corresponding to the
    /// timestamp `unix_time_in_secs`.
    fun next_round_after(unix_time_in_secs: u64): u64 {
        let (next_round, _) = next_round_and_timestamp_after(unix_time_in_secs);

        next_round
    }

    /// Returns the next upcoming round and its UNIX time (after the round at time `unix_time_in_secs`)
    /// Round at time `GENESIS_TIMESTAMP` is round # 1. Round 0 is fixed.
    /// (As per https://github.com/drand/drand/blob/0678331f90c87329a001eca4031da8259f6d1d3d/chain/time.go#L52)
    fun next_round_and_timestamp_after(unix_time_in_secs: u64): (u64, u64) {
        if(unix_time_in_secs < GENESIS_TIMESTAMP) {
            return (1, GENESIS_TIMESTAMP)
        };

        let duration = unix_time_in_secs - GENESIS_TIMESTAMP;

        // As described in https://github.com/drand/drand/blob/0678331f90c87329a001eca4031da8259f6d1d3d/chain/time.go#L57:
        //  > We take the time from genesis divided by the periods in seconds.
        //  > That gives us the number of periods since genesis.
        //  > We add +1 since we want the next round.
        //  > We also add +1 because round 1 starts at genesis time.

        // TODO(Security): Make sure any loss of precision here cannot be exploited. I think we can just compute (q, r), with r < PERIOD such that duration = q * PERIOD + r. Then, the floor will be q. This avoids any precision issues.

        let next_round = (duration / PERIOD_SECS) + 1;
        let next_time = GENESIS_TIMESTAMP + next_round * PERIOD_SECS;

        (next_round + 1, next_time)
    }

    //
    // Internals
    //

    /// Calculates the round # at time `unix_time_in_secs`.
    /// (As per https://github.com/drand/drand/blob/0678331f90c87329a001eca4031da8259f6d1d3d/chain/time.go#L41)
    fun current_round(unix_time_in_secs: u64): u64 {
        let (next_round, _) = next_round_and_timestamp_after(unix_time_in_secs);

        if (next_round <= 1) {
                return next_round
        };

        next_round - 1
    }

}
