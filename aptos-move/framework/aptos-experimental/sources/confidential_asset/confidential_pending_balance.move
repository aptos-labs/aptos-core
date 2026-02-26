/// This module implements a Confidential Pending Balance abstraction, built on top of Twisted ElGamal encryption,
/// over the Ristretto255 curve.
///
/// A pending balance stores encrypted representations of incoming transfers, split into chunks and stored as pairs of
/// ciphertext components `(P_i, R_i)` under basepoints `G` and `H` and an encryption key `EK = dk^(-1) * H`, where `dk`
/// is the corresponding decryption key. Each pair represents an encrypted value `a_i` - the `i`-th 16-bit portion of
/// the total encrypted amount - and its associated randomness `r_i`, such that `P_i = a_i * G + r_i * H` and `R_i = r_i * EK`.
///
/// Pending balances are represented by four ciphertext pairs `(P_i, R_i), i = 1..4`, suitable for 64-bit values.
module aptos_experimental::confidential_pending_balance {
    use std::error;
    use std::vector;
    use aptos_std::ristretto255::{Self, RistrettoPoint, CompressedRistretto};
    use aptos_experimental::confidential_balance;

    friend aptos_experimental::confidential_asset;
    friend aptos_experimental::confidential_proof;

    //
    // Errors
    //

    /// An internal error occurred, indicating unexpected behavior.
    const EINTERNAL_ERROR: u64 = 1;

    //
    // Constants
    //

    /// The number of chunks $n$ in a pending balance.
    const PENDING_BALANCE_CHUNKS: u64 = 4;

    //
    // Structs
    //

    /// Represents a compressed pending balance.
    /// - `P[i]` is the value component: `chunk_i * G + r_i * H`
    /// - `R[i]` is the EK component: `r_i * EK`
    struct CompressedPendingBalance has store, drop, copy {
        P: vector<CompressedRistretto>,
        R: vector<CompressedRistretto>,
    }

    /// Represents an uncompressed pending balance.
    /// - `P[i]` is the value component: `chunk_i * G + r_i * H`
    /// - `R[i]` is the EK component: `r_i * EK`
    struct PendingBalance has drop {
        P: vector<RistrettoPoint>,
        R: vector<RistrettoPoint>,
    }

    //
    // Accessor functions
    //

    /// Returns a reference to the P components (value components) of a pending balance.
    public fun get_P(self: &PendingBalance): &vector<RistrettoPoint> {
        &self.P
    }

    /// Returns a reference to the R components (EK components) of a pending balance.
    public fun get_R(self: &PendingBalance): &vector<RistrettoPoint> {
        &self.R
    }

    /// Returns a reference to the P components (value components) of a compressed pending balance.
    public fun get_compressed_P(self: &CompressedPendingBalance): &vector<CompressedRistretto> {
        &self.P
    }

    /// Returns a reference to the R components (EK components) of a compressed pending balance.
    public fun get_compressed_R(self: &CompressedPendingBalance): &vector<CompressedRistretto> {
        &self.R
    }

    //
    // Friend functions
    //

    /// Creates a PendingBalance from separate P and R component vectors.
    public(friend) fun new_from_p_and_r(p: vector<RistrettoPoint>, r: vector<RistrettoPoint>): PendingBalance {
        PendingBalance { P: p, R: r }
    }

    /// Creates a CompressedPendingBalance from separate compressed P and R component vectors.
    public(friend) fun new_compressed_from_p_and_r(
        p: vector<CompressedRistretto>,
        r: vector<CompressedRistretto>
    ): CompressedPendingBalance {
        CompressedPendingBalance { P: p, R: r }
    }

    /// Destructures a PendingBalance into its P and R component vectors.
    public(friend) fun into_p_and_r(self: PendingBalance): (vector<RistrettoPoint>, vector<RistrettoPoint>) {
        let PendingBalance { P: p, R: r } = self;
        (p, r)
    }

    //
    // Public functions
    //

    /// Splits an integer amount into `PENDING_BALANCE_CHUNKS` 16-bit chunks, represented as `Scalar` values.
    public fun split_into_chunks(amount: u128): vector<ristretto255::Scalar> {
        let chunk_size_bits = confidential_balance::get_chunk_size_bits();
        vector::range(0, PENDING_BALANCE_CHUNKS).map(|i| {
            ristretto255::new_scalar_from_u128(amount >> (i * chunk_size_bits as u8) & 0xffff)
        })
    }

    /// Creates a new compressed zero pending balance.
    public fun new_zero_compressed(): CompressedPendingBalance {
        let identity = ristretto255::point_identity_compressed();
        CompressedPendingBalance {
            P: vector::range(0, PENDING_BALANCE_CHUNKS).map(|_| identity),
            R: vector::range(0, PENDING_BALANCE_CHUNKS).map(|_| identity),
        }
    }

    /// Creates a new pending balance from a 64-bit amount with no randomness (R components are identity).
    /// Splits the amount into four 16-bit chunks.
    public fun new_u64_no_randomness(amount: u64): PendingBalance {
        let identity = ristretto255::point_identity();
        PendingBalance {
            P: split_into_chunks((amount as u128)).map(|chunk| chunk.basepoint_mul()),
            R: vector::range(0, PENDING_BALANCE_CHUNKS).map(|_| identity.point_clone()),
        }
    }

    /// Creates a new pending balance from separate P and R byte vectors.
    /// Each element in `p_bytes` and `r_bytes` is a 32-byte compressed Ristretto point.
    /// Aborts if any point fails to deserialize or if vector lengths are inconsistent.
    public fun new_from_byte_vectors(
        p_bytes: vector<vector<u8>>,
        r_bytes: vector<vector<u8>>,
    ): PendingBalance {
        assert!(p_bytes.length() == r_bytes.length());

        PendingBalance {
            P: p_bytes.map(|bytes| ristretto255::new_point_from_bytes(bytes).extract()),
            R: r_bytes.map(|bytes| ristretto255::new_point_from_bytes(bytes).extract()),
        }
    }

    /// Compresses a pending balance into its `CompressedPendingBalance` representation.
    public fun compress(self: &PendingBalance): CompressedPendingBalance {
        CompressedPendingBalance {
            P: self.P.map_ref(|p| p.point_compress()),
            R: self.R.map_ref(|r| r.point_compress()),
        }
    }

    /// Adds a pending balance to this compressed pending balance in place.
    /// Decompresses, adds, and recompresses internally.
    public fun add_assign(self: &mut CompressedPendingBalance, rhs: &PendingBalance) {
        let decompressed = self.decompress();
        decompressed.add_mut(rhs);
        *self = decompressed.compress();
    }

    /// Decompresses a compressed pending balance into its `PendingBalance` representation.
    public fun decompress(self: &CompressedPendingBalance): PendingBalance {
        PendingBalance {
            P: self.P.map_ref(|p| p.point_decompress()),
            R: self.R.map_ref(|r| r.point_decompress()),
        }
    }

    /// Adds two pending balances homomorphically, mutating the first balance in place.
    /// The second balance must have fewer or equal chunks compared to the first.
    public fun add_mut(self: &mut PendingBalance, rhs: &PendingBalance) {
        assert!(self.P.length() >= rhs.P.length(), error::internal(EINTERNAL_ERROR));

        let i = 0;
        let rhs_len = rhs.P.length();
        while (i < rhs_len) {
            self.P[i].point_add_assign(&rhs.P[i]);
            self.R[i].point_add_assign(&rhs.R[i]);
            i = i + 1;
        };
    }

    /// Checks if a compressed pending balance is equivalent to zero (all P and R are identity).
    public fun is_zero(self: &CompressedPendingBalance): bool {
        self.P.all(|p| p.is_identity()) &&
        self.R.all(|r| r.is_identity())
    }

    //
    // View functions
    //

    #[view]
    /// Returns the number of chunks in a pending balance.
    public fun get_num_chunks(): u64 {
        PENDING_BALANCE_CHUNKS
    }

    //
    // Test-only
    //

    #[test_only]
    use aptos_std::ristretto255::Scalar;
    #[test_only]
    use aptos_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    #[test_only]
    /// Generates a `ConfidentialBalanceRandomness` with `PENDING_BALANCE_CHUNKS` random scalars.
    public fun generate_balance_randomness(): confidential_balance::ConfidentialBalanceRandomness {
        confidential_balance::new_randomness(
            vector::range(0, PENDING_BALANCE_CHUNKS).map(|_| ristretto255::random_scalar())
        )
    }

    #[test_only]
    /// Creates a new pending balance from an amount using the provided randomness and encryption key.
    public fun new_from_amount(
        amount: u128,
        randomness: &confidential_balance::ConfidentialBalanceRandomness,
        ek: &CompressedRistretto
    ): PendingBalance {
        let amount_chunks = split_into_chunks(amount);
        let r = randomness.scalars();
        let ek_point = ek.point_decompress();
        let basepoint_H = twisted_elgamal::get_encryption_key_basepoint();

        PendingBalance {
            P: vector::range(0, PENDING_BALANCE_CHUNKS).map(|i| {
                // P_i = amount_i * G + r_i * H
                ristretto255::double_scalar_mul(
                    &amount_chunks[i], &ristretto255::basepoint(),
                    &r[i], &basepoint_H
                )
            }),
            R: vector::range(0, PENDING_BALANCE_CHUNKS).map(|i| {
                // R_i = r_i * EK
                ek_point.point_mul(&r[i])
            }),
        }
    }

    #[test_only]
    /// Verifies that a pending balance encrypts the specified amount.
    public fun check_decrypts_to(self: &PendingBalance, dk: &Scalar, amount: u128): bool {
        let amount_chunks = split_into_chunks(amount);
        let i = 0;

        while (i < PENDING_BALANCE_CHUNKS) {
            // Decrypt: m*G = P - dk*R
            let point_amount = self.P[i].point_sub(&self.R[i].point_mul(dk));
            if (!point_amount.point_equals(&amount_chunks[i].basepoint_mul())) return false;
            i = i + 1;
        };

        true
    }
}
