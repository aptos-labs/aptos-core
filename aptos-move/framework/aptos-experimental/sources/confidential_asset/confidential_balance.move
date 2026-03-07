/// Balance types for the confidential asset protocol.
///
/// `CompressedBalance<T>` and `Balance<T>` are parameterized by phantom markers `Pending` and `Available`.
/// P_i = a_i*G + r_i*H, R_i = r_i*EK. R_aud is empty for Pending balances.
module aptos_experimental::confidential_balance {
    use std::error;
    use std::vector;
    use aptos_std::ristretto255::{RistrettoPoint, Scalar, CompressedRistretto,
        new_scalar_from_u128, scalar_one, point_identity, point_identity_compressed};
    use aptos_experimental::sigma_protocol_utils::deserialize_compressed_points;

    friend aptos_experimental::confidential_amount;
    friend aptos_experimental::confidential_asset;
    friend aptos_experimental::confidential_range_proofs;
    friend aptos_experimental::sigma_protocol_transfer;

    //
    // Constants
    //

    /// The number of bits $b$ in a single chunk.
    const CHUNK_SIZE_BITS: u64 = 16;
    /// All chunks are < than this value
    const CHUNK_UPPER_BOUND: u64 = 65_536;
    /// The number of chunks $n$ in a pending balance.
    const PENDING_BALANCE_CHUNKS: u64 = 4;
    /// The number of chunks $\ell$ in an available balance.
    const AVAILABLE_BALANCE_CHUNKS: u64 = 8;

    /// Expected the P or R components to have the wrong number of chunks.
    const E_WRONG_NUM_CHUNKS: u64 = 1;
    /// Expected the auditor R-component to be either empty or have the correct number of chunks.
    const E_WRONG_NUM_CHUNKS_FOR_AUDITOR: u64 = 2;

    //
    // Phantom markers
    //

    struct Pending has drop {}
    struct Available has drop {}

    //
    // Generic balance types (enum for future upgradeability)
    //

    enum CompressedBalance<phantom T> has store, drop, copy {
        V1 {
            P: vector<CompressedRistretto>,
            R: vector<CompressedRistretto>,
            R_aud: vector<CompressedRistretto>,
        }
    }

    enum Balance<phantom T> has drop {
        V1 {
            P: vector<RistrettoPoint>,
            R: vector<RistrettoPoint>,
            R_aud: vector<RistrettoPoint>,
        }
    }

    // === Generic accessors ===

    public fun get_P<T>(self: &Balance<T>): &vector<RistrettoPoint> { &self.P }
    public fun get_R<T>(self: &Balance<T>): &vector<RistrettoPoint> { &self.R }
    public fun get_R_aud<T>(self: &Balance<T>): &vector<RistrettoPoint> { &self.R_aud }
    public fun get_compressed_P<T>(self: &CompressedBalance<T>): &vector<CompressedRistretto> { &self.P }
    public fun get_compressed_R<T>(self: &CompressedBalance<T>): &vector<CompressedRistretto> { &self.R }
    public fun get_compressed_R_aud<T>(self: &CompressedBalance<T>): &vector<CompressedRistretto> { &self.R_aud }

    // === Generic compress/decompress ===

    public fun compress<T>(self: &Balance<T>): CompressedBalance<T> {
        CompressedBalance::V1 {
            P: self.P.map_ref(|p| p.point_compress()),
            R: self.R.map_ref(|r| r.point_compress()),
            R_aud: self.R_aud.map_ref(|r| r.point_compress()),
        }
    }

    public fun decompress<T>(self: &CompressedBalance<T>): Balance<T> {
        Balance::V1 {
            P: self.P.map_ref(|p| p.point_decompress()),
            R: self.R.map_ref(|r| r.point_decompress()),
            R_aud: self.R_aud.map_ref(|r| r.point_decompress()),
        }
    }

    // === Generic operations ===

    public fun is_zero<T>(self: &CompressedBalance<T>): bool {
        self.P.all(|p| p.is_identity()) &&
        self.R.all(|r| r.is_identity())
    }

    /// Sets the R component of a compressed balance (friend-gated, no validation).
    public(friend) fun set_R<T>(self: &mut CompressedBalance<T>, new_R: vector<CompressedRistretto>) {
        self.R = new_R;
    }

    /// Element-wise P and R addition. R_aud is NOT touched.
    public(friend) fun add_mut_base<T>(self: &mut Balance<T>, rhs_P: &vector<RistrettoPoint>, rhs_R: &vector<RistrettoPoint>) {
        vector::range(0, rhs_P.length()).for_each(|i| {
            self.P[i].point_add_assign(&rhs_P[i]);
            self.R[i].point_add_assign(&rhs_R[i]);
        });
    }

    // === Generic constructors (friend-gated) ===

    public(friend) fun new_balance<T>(
        p: vector<RistrettoPoint>, r: vector<RistrettoPoint>, r_aud: vector<RistrettoPoint>,
        expected_chunks: u64,
    ): Balance<T> {
        assert_correct_num_chunks(&p, &r, &r_aud, expected_chunks);
        Balance::V1 { P: p, R: r, R_aud: r_aud }
    }

    public(friend) fun new_compressed_balance<T>(
        p: vector<CompressedRistretto>, r: vector<CompressedRistretto>, r_aud: vector<CompressedRistretto>,
        expected_chunks: u64,
    ): CompressedBalance<T> {
        assert_correct_num_chunks(&p, &r, &r_aud, expected_chunks);
        CompressedBalance::V1 { P: p, R: r, R_aud: r_aud }
    }

    fun new_zero_compressed<T>(num_chunks: u64): CompressedBalance<T> {
        let identity = point_identity_compressed();
        new_compressed_balance<T>(
            vector::range(0, num_chunks).map(|_| identity),
            vector::range(0, num_chunks).map(|_| identity),
            vector[],
            num_chunks
        )
    }

    // ========================================= //
    //          Pending balance functions         //
    // ========================================= //

    public(friend) fun new_pending_from_p_and_r(p: vector<RistrettoPoint>, r: vector<RistrettoPoint>): Balance<Pending> {
        new_balance(p, r, vector[], PENDING_BALANCE_CHUNKS)
    }

    public(friend) fun new_compressed_pending_from_p_and_r(
        p: vector<CompressedRistretto>, r: vector<CompressedRistretto>
    ): CompressedBalance<Pending> {
        new_compressed_balance(p, r, vector[], PENDING_BALANCE_CHUNKS)
    }

    public fun new_zero_pending_compressed(): CompressedBalance<Pending> {
        new_zero_compressed(PENDING_BALANCE_CHUNKS)
    }

    /// Creates a pending balance from a 64-bit amount with no randomness (R = identity).
    public fun new_pending_u64_no_randomness(amount: u64): Balance<Pending> {
        let identity = point_identity();
        new_pending_from_p_and_r(
            split_pending_into_chunks((amount as u128)).map(|chunk| chunk.basepoint_mul()),
            vector::range(0, PENDING_BALANCE_CHUNKS).map(|_| identity.point_clone()),
        )
    }

    /// Splits an integer amount into `PENDING_BALANCE_CHUNKS` 16-bit chunks.
    public fun split_pending_into_chunks(amount: u128): vector<Scalar> {
        split_into_chunks(amount, PENDING_BALANCE_CHUNKS)
    }

    /// Adds a pending balance to a compressed pending balance in place.
    public fun add_assign_pending(balance: &mut CompressedBalance<Pending>, rhs: &Balance<Pending>) {
        let decompressed = balance.decompress();
        decompressed.add_mut_base(rhs.get_P(), rhs.get_R());
        *balance = decompressed.compress();
    }

    #[view]
    public fun get_num_pending_chunks(): u64 { PENDING_BALANCE_CHUNKS }

    // ========================================= //
    //        Available balance functions         //
    // ========================================= //

    public(friend) fun new_available_from_p_r_r_aud(
        p: vector<RistrettoPoint>, r: vector<RistrettoPoint>, r_aud: vector<RistrettoPoint>
    ): Balance<Available> {
        new_balance(p, r, r_aud, AVAILABLE_BALANCE_CHUNKS)
    }

    public(friend) fun new_compressed_available_from_p_r_r_aud(
        p: vector<CompressedRistretto>, r: vector<CompressedRistretto>, r_aud: vector<CompressedRistretto>
    ): CompressedBalance<Available> {
        new_compressed_balance(p, r, r_aud, AVAILABLE_BALANCE_CHUNKS)
    }

    public fun new_zero_available_compressed(): CompressedBalance<Available> {
        new_zero_compressed(AVAILABLE_BALANCE_CHUNKS)
    }

    /// Deserializes raw byte vectors into a CompressedBalance<Available> (without decompressing).
    public(friend) fun new_compressed_available_from_bytes(
        p_bytes: vector<vector<u8>>,
        r_bytes: vector<vector<u8>>,
        r_aud_bytes: vector<vector<u8>>,
    ): CompressedBalance<Available> {
        new_compressed_available_from_p_r_r_aud(
            deserialize_compressed_points(p_bytes),
            deserialize_compressed_points(r_bytes),
            deserialize_compressed_points(r_aud_bytes),
        )
    }

    /// Splits an integer amount into `AVAILABLE_BALANCE_CHUNKS` 16-bit chunks.
    public fun split_available_into_chunks(amount: u128): vector<Scalar> {
        split_into_chunks(amount, AVAILABLE_BALANCE_CHUNKS)
    }

    /// Sets only the R component (EK component) of a compressed available balance.
    public fun set_available_R(balance: &mut CompressedBalance<Available>, new_R: vector<CompressedRistretto>) {
        assert!(new_R.length() == AVAILABLE_BALANCE_CHUNKS, error::invalid_argument(E_WRONG_NUM_CHUNKS));
        balance.R = new_R;
    }

    /// Adds a pending balance to an available balance in place. R_aud is NOT touched (stale after rollover).
    public fun add_assign_available_excluding_auditor(balance: &mut CompressedBalance<Available>, rhs: &CompressedBalance<Pending>) {
        let lhs_P = balance.P.map_ref(|p| p.point_decompress());
        let lhs_R = balance.R.map_ref(|r| r.point_decompress());
        let rhs_P = rhs.P.map_ref(|p| p.point_decompress());
        let rhs_R = rhs.R.map_ref(|r| r.point_decompress());

        vector::range(0, rhs_P.length()).for_each(|i| {
            lhs_P[i].point_add_assign(&rhs_P[i]);
            lhs_R[i].point_add_assign(&rhs_R[i]);
        });

        balance.P = lhs_P.map_ref(|p| p.point_compress());
        balance.R = lhs_R.map_ref(|r| r.point_compress());
    }

    #[view]
    public fun get_num_available_chunks(): u64 { AVAILABLE_BALANCE_CHUNKS }

    // ========================================= //
    //          Chunk-splitting functions         //
    // ========================================= //

    public fun get_chunk_size_bits(): u64 { CHUNK_SIZE_BITS }

    /// Every balance chunk is $<$ than this bound (i.e., $< 2^{16}$).
    public fun get_chunk_upper_bound(): u64 { CHUNK_UPPER_BOUND }

    /// Splits `amount` into `num_chunks` 16-bit chunks as `Scalar` values.
    public fun split_into_chunks(amount: u128, num_chunks: u64): vector<Scalar> {
        vector::range(0, num_chunks).map(|i| {
            new_scalar_from_u128(amount >> (i * CHUNK_SIZE_BITS as u8) & 0xffff)
        })
    }

    /// Returns [B^0, B^1, ..., B^{count-1}] where B = 2^chunk_size_bits.
    public fun get_b_powers(count: u64): vector<Scalar> {
        let b = new_scalar_from_u128((CHUNK_UPPER_BOUND as u128));
        let powers = vector[scalar_one()];
        let prev = scalar_one();
        for (i in 1..count) {
            prev = prev.scalar_mul(&b);
            powers.push_back(prev);
        };
        powers
    }

    //
    // Helpers
    //

    fun assert_correct_num_chunks<T>(p: &vector<T>, r: &vector<T>, r_aud: &vector<T>, expected_chunks: u64) {
        assert!(p.length() == expected_chunks, error::invalid_argument(E_WRONG_NUM_CHUNKS));
        assert!(r.length() == expected_chunks, error::invalid_argument(E_WRONG_NUM_CHUNKS));
        assert!(r_aud.is_empty() || r_aud.length() == expected_chunks, error::invalid_argument(
            E_WRONG_NUM_CHUNKS_FOR_AUDITOR
        ));
    }

    // ========================================= //
    //              Test-only                     //
    // ========================================= //

    #[test_only]
    struct ConfidentialBalanceRandomness has drop {
        r: vector<Scalar>
    }

    #[test_only]
    public fun new_randomness(r: vector<Scalar>): ConfidentialBalanceRandomness {
        ConfidentialBalanceRandomness { r }
    }

    #[test_only]
    public fun scalars(self: &ConfidentialBalanceRandomness): &vector<Scalar> {
        &self.r
    }

    #[test_only]
    use aptos_std::ristretto255::{Self, random_scalar, double_scalar_mul, multi_scalar_mul};
    #[test_only]
    use aptos_experimental::ristretto255_twisted_elgamal as twisted_elgamal;
    #[test_only]
    use std::option::Option;

    #[test_only]
    public fun generate_randomness(num_chunks: u64): ConfidentialBalanceRandomness {
        new_randomness(vector::range(0, num_chunks).map(|_| random_scalar()))
    }

    #[test_only]
    public fun generate_pending_randomness(): ConfidentialBalanceRandomness {
        generate_randomness(PENDING_BALANCE_CHUNKS)
    }

    #[test_only]
    public fun generate_available_randomness(): ConfidentialBalanceRandomness {
        generate_randomness(AVAILABLE_BALANCE_CHUNKS)
    }

    #[test_only]
    /// Shared encryption logic: computes (P, R) where P_i = amount_i*G + r_i*H, R_i = r_i*EK.
    public fun encrypt_amount(
        amount_chunks: &vector<Scalar>,
        randomness: &ConfidentialBalanceRandomness,
        ek: &CompressedRistretto,
        num_chunks: u64,
    ): (vector<RistrettoPoint>, vector<RistrettoPoint>) {
        let r = randomness.scalars();
        let ek_point = ek.point_decompress();
        let basepoint_H = twisted_elgamal::get_encryption_key_basepoint();

        let p = vector::range(0, num_chunks).map(|i| {
            double_scalar_mul(
                &amount_chunks[i], &ristretto255::basepoint(),
                &r[i], &basepoint_H
            )
        });
        let r_out = vector::range(0, num_chunks).map(|i| {
            ek_point.point_mul(&r[i])
        });

        (p, r_out)
    }

    #[test_only]
    /// Creates a new pending balance from an amount using the provided randomness and encryption key.
    public fun new_pending_from_amount(
        amount: u128,
        randomness: &ConfidentialBalanceRandomness,
        ek: &CompressedRistretto
    ): Balance<Pending> {
        let amount_chunks = split_pending_into_chunks(amount);
        let (p, r) = encrypt_amount(&amount_chunks, randomness, ek, PENDING_BALANCE_CHUNKS);
        new_pending_from_p_and_r(p, r)
    }

    #[test_only]
    /// If `auditor_ek` is `Some`, computes R_aud_i = r_i * EK_auditor; otherwise R_aud is empty.
    public fun new_available_from_amount(
        amount: u128,
        randomness: &ConfidentialBalanceRandomness,
        ek: &CompressedRistretto,
        auditor_ek: &Option<CompressedRistretto>
    ): Balance<Available> {
        let amount_chunks = split_available_into_chunks(amount);
        let (p, r) = encrypt_amount(&amount_chunks, randomness, ek, AVAILABLE_BALANCE_CHUNKS);

        let r_aud_components = if (auditor_ek.is_some()) {
            let auditor_ek_point = auditor_ek.borrow().point_decompress();
            let r_scalars = randomness.scalars();
            vector::range(0, AVAILABLE_BALANCE_CHUNKS).map(|i| {
                auditor_ek_point.point_mul(&r_scalars[i])
            })
        } else {
            vector[]
        };

        new_available_from_p_r_r_aud(p, r, r_aud_components)
    }

    #[test_only]
    /// Verifies that a balance encrypts `amount` using DK on the given R component.
    public fun check_decrypts_to<T>(
        self: &Balance<T>, decrypt_R: &vector<RistrettoPoint>,
        dk: &Scalar, amount: u128,
    ): bool {
        let num_chunks = self.P.length();
        let b_powers = get_b_powers(num_chunks);

        let decrypted_chunks: vector<RistrettoPoint> = vector::range(0, num_chunks).map(|i| {
            self.P[i].point_sub(&decrypt_R[i].point_mul(dk))
        });

        let combined = multi_scalar_mul(&decrypted_chunks, &b_powers);
        combined.point_equals(&new_scalar_from_u128(amount).basepoint_mul())
    }
}
