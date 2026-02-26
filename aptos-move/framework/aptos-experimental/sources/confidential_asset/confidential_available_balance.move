/// This module implements a Confidential Available Balance abstraction, built on top of Twisted ElGamal encryption,
/// over the Ristretto255 curve.
///
/// An available balance stores the user's spendable balance, split into chunks and stored as triples of
/// ciphertext components `(P_i, R_i, R_aud_i)` under basepoints `G` and `H` and an encryption key `EK = dk^(-1) * H`,
/// where `dk` is the corresponding decryption key. Each triple represents an encrypted value `a_i` - the `i`-th 16-bit
/// portion of the total encrypted amount - and its associated randomness `r_i`, such that:
///   `P_i = a_i * G + r_i * H`
///   `R_i = r_i * EK`
///   `R_aud_i = r_i * EK_auditor` (if an auditor is set; empty otherwise)
///
/// The R_aud component allows an auditor to decrypt the available balance. After rollover, R_aud becomes stale
/// (since pending balances have no R_aud); it's refreshed by withdraw/transfer/normalize (which produce
/// fresh AvailableBalance with new R_aud).
///
/// Available balances are represented by eight ciphertext pairs/triples, supporting 128-bit values.
module aptos_experimental::confidential_available_balance {
    use std::vector;
    use aptos_std::ristretto255::{Self, RistrettoPoint, CompressedRistretto};
    use aptos_experimental::confidential_balance;
    use aptos_experimental::confidential_pending_balance::CompressedPendingBalance;

    friend aptos_experimental::confidential_asset;
    friend aptos_experimental::confidential_proof;

    //
    // Constants
    //

    /// The number of chunks $\ell$ in an available balance.
    const AVAILABLE_BALANCE_CHUNKS: u64 = 8;

    //
    // Structs
    //

    /// Represents a compressed available balance.
    /// - `P[i]` is the value component: `chunk_i * G + r_i * H`
    /// - `R[i]` is the EK component: `r_i * EK`
    /// - `R_aud[i]` is the auditor component: `r_i * EK_auditor` (empty vector if no auditor)
    struct CompressedAvailableBalance has store, drop, copy {
        P: vector<CompressedRistretto>,
        R: vector<CompressedRistretto>,
        R_aud: vector<CompressedRistretto>,
    }

    /// Represents an uncompressed available balance.
    /// - `P[i]` is the value component: `chunk_i * G + r_i * H`
    /// - `R[i]` is the EK component: `r_i * EK`
    /// - `R_aud[i]` is the auditor component: `r_i * EK_auditor` (empty vector if no auditor)
    struct AvailableBalance has drop {
        P: vector<RistrettoPoint>,
        R: vector<RistrettoPoint>,
        R_aud: vector<RistrettoPoint>,
    }

    //
    // Accessor functions
    //

    /// Returns a reference to the P components (value components) of an available balance.
    public fun get_P(self: &AvailableBalance): &vector<RistrettoPoint> {
        &self.P
    }

    /// Returns a reference to the R components (EK components) of an available balance.
    public fun get_R(self: &AvailableBalance): &vector<RistrettoPoint> {
        &self.R
    }

    /// Returns a reference to the R_aud components (auditor components) of an available balance.
    public fun get_R_aud(self: &AvailableBalance): &vector<RistrettoPoint> {
        &self.R_aud
    }

    /// Returns a reference to the P components (value components) of a compressed available balance.
    public fun get_compressed_P(self: &CompressedAvailableBalance): &vector<CompressedRistretto> {
        &self.P
    }

    /// Returns a reference to the R components (EK components) of a compressed available balance.
    public fun get_compressed_R(self: &CompressedAvailableBalance): &vector<CompressedRistretto> {
        &self.R
    }

    /// Returns a reference to the R_aud components (auditor components) of a compressed available balance.
    public fun get_compressed_R_aud(self: &CompressedAvailableBalance): &vector<CompressedRistretto> {
        &self.R_aud
    }

    /// Sets the R components (EK components) of a compressed available balance.
    public fun set_compressed_R(self: &mut CompressedAvailableBalance, new_R: vector<CompressedRistretto>) {
        self.R = new_R;
    }

    //
    // Friend functions
    //

    /// Creates an AvailableBalance from separate P, R, and R_aud component vectors.
    public(friend) fun new_from_p_r_r_aud(
        p: vector<RistrettoPoint>,
        r: vector<RistrettoPoint>,
        r_aud: vector<RistrettoPoint>
    ): AvailableBalance {
        AvailableBalance { P: p, R: r, R_aud: r_aud }
    }

    /// Creates a CompressedAvailableBalance from separate compressed P, R, and R_aud component vectors.
    public(friend) fun new_compressed_from_p_r_r_aud(
        p: vector<CompressedRistretto>,
        r: vector<CompressedRistretto>,
        r_aud: vector<CompressedRistretto>
    ): CompressedAvailableBalance {
        CompressedAvailableBalance { P: p, R: r, R_aud: r_aud }
    }

    //
    // Public functions
    //

    /// Creates a new compressed zero available balance (R_aud = empty, since no auditor for zero balance).
    public fun new_zero_compressed(): CompressedAvailableBalance {
        let identity = ristretto255::point_identity_compressed();
        CompressedAvailableBalance {
            P: vector::range(0, AVAILABLE_BALANCE_CHUNKS).map(|_| identity),
            R: vector::range(0, AVAILABLE_BALANCE_CHUNKS).map(|_| identity),
            R_aud: vector[],
        }
    }

    /// Creates a new available balance from separate P, R, and R_aud byte vectors.
    /// Each element in `p_bytes`, `r_bytes`, and `r_aud_bytes` is a 32-byte compressed Ristretto point.
    /// `r_aud_bytes` may be empty (no auditor) or must have the same length as `p_bytes`/`r_bytes`.
    /// Aborts if any point fails to deserialize or if vector lengths are inconsistent.
    public fun new_from_byte_vectors(
        p_bytes: vector<vector<u8>>,
        r_bytes: vector<vector<u8>>,
        r_aud_bytes: vector<vector<u8>>,
    ): AvailableBalance {
        assert!(p_bytes.length() == r_bytes.length());
        assert!(r_aud_bytes.length() == 0 || r_aud_bytes.length() == p_bytes.length());

        AvailableBalance {
            P: p_bytes.map(|bytes| ristretto255::new_point_from_bytes(bytes).extract()),
            R: r_bytes.map(|bytes| ristretto255::new_point_from_bytes(bytes).extract()),
            R_aud: r_aud_bytes.map(|bytes| ristretto255::new_point_from_bytes(bytes).extract()),
        }
    }

    /// Compresses an available balance into its `CompressedAvailableBalance` representation.
    public fun compress(self: &AvailableBalance): CompressedAvailableBalance {
        CompressedAvailableBalance {
            P: self.P.map_ref(|p| p.point_compress()),
            R: self.R.map_ref(|r| r.point_compress()),
            R_aud: self.R_aud.map_ref(|r_aud| r_aud.point_compress()),
        }
    }

    /// Decompresses a compressed available balance into its `AvailableBalance` representation.
    public fun decompress(self: &CompressedAvailableBalance): AvailableBalance {
        AvailableBalance {
            P: self.P.map_ref(|p| p.point_decompress()),
            R: self.R.map_ref(|r| r.point_decompress()),
            R_aud: self.R_aud.map_ref(|r_aud| r_aud.point_decompress()),
        }
    }

    /// Adds a compressed pending balance to this compressed available balance in place.
    /// Decompresses both, adds the pending balance's P and R components, and recompresses.
    /// The R_aud components remain unchanged (stale after rollover; refreshed by normalize/withdraw/transfer).
    public fun add_assign(self: &mut CompressedAvailableBalance, rhs: &CompressedPendingBalance) {
        let decompressed_self = self.decompress();
        let decompressed_rhs = rhs.decompress();

        let rhs_P = decompressed_rhs.get_P();
        let rhs_R = decompressed_rhs.get_R();
        let rhs_len = rhs_P.length();

        let i = 0;
        while (i < rhs_len) {
            decompressed_self.P[i].point_add_assign(&rhs_P[i]);
            decompressed_self.R[i].point_add_assign(&rhs_R[i]);
            i = i + 1;
        };
        // Note: R_aud components are NOT modified. They become stale after rollover.
        *self = decompressed_self.compress();
    }

    /// Splits an integer amount into `AVAILABLE_BALANCE_CHUNKS` 16-bit chunks, represented as `Scalar` values.
    public fun split_into_chunks(amount: u128): vector<ristretto255::Scalar> {
        let chunk_size_bits = confidential_balance::get_chunk_size_bits();
        vector::range(0, AVAILABLE_BALANCE_CHUNKS).map(|i| {
            ristretto255::new_scalar_from_u128(amount >> (i * chunk_size_bits as u8) & 0xffff)
        })
    }

    //
    // View functions
    //

    #[view]
    /// Returns the number of chunks in an available balance.
    public fun get_num_chunks(): u64 {
        AVAILABLE_BALANCE_CHUNKS
    }

    //
    // Test-only
    //

    #[test_only]
    use std::option::Option;
    #[test_only]
    use aptos_std::ristretto255::Scalar;
    #[test_only]
    use aptos_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    #[test_only]
    /// Generates a `ConfidentialBalanceRandomness` with `AVAILABLE_BALANCE_CHUNKS` random scalars.
    public fun generate_balance_randomness(): confidential_balance::ConfidentialBalanceRandomness {
        confidential_balance::new_randomness(
            vector::range(0, AVAILABLE_BALANCE_CHUNKS).map(|_| ristretto255::random_scalar())
        )
    }

    #[test_only]
    /// Creates a new available balance from an amount using the provided randomness and encryption key.
    /// If `auditor_ek` is `Some`, computes R_aud_i = r_i * EK_auditor for each chunk.
    /// If `auditor_ek` is `None`, R_aud is empty.
    public fun new_from_amount(
        amount: u128,
        randomness: &confidential_balance::ConfidentialBalanceRandomness,
        ek: &CompressedRistretto,
        auditor_ek: &Option<CompressedRistretto>
    ): AvailableBalance {
        let amount_chunks = split_into_chunks(amount);
        let r = randomness.scalars();
        let ek_point = ek.point_decompress();
        let basepoint_H = twisted_elgamal::get_encryption_key_basepoint();

        let r_aud_components = if (auditor_ek.is_some()) {
            let auditor_ek_point = auditor_ek.borrow().point_decompress();
            vector::range(0, AVAILABLE_BALANCE_CHUNKS).map(|i| {
                // R_aud_i = r_i * EK_auditor
                auditor_ek_point.point_mul(&r[i])
            })
        } else {
            vector[]
        };

        AvailableBalance {
            P: vector::range(0, AVAILABLE_BALANCE_CHUNKS).map(|i| {
                // P_i = amount_i * G + r_i * H
                ristretto255::double_scalar_mul(
                    &amount_chunks[i], &ristretto255::basepoint(),
                    &r[i], &basepoint_H
                )
            }),
            R: vector::range(0, AVAILABLE_BALANCE_CHUNKS).map(|i| {
                // R_i = r_i * EK
                ek_point.point_mul(&r[i])
            }),
            R_aud: r_aud_components,
        }
    }

    #[test_only]
    /// Verifies that an available balance encrypts the specified amount (checks P,R only).
    public fun check_decrypts_to(self: &AvailableBalance, dk: &Scalar, amount: u128): bool {
        let amount_chunks = split_into_chunks(amount);
        let i = 0;

        while (i < AVAILABLE_BALANCE_CHUNKS) {
            // Decrypt: m*G = P - dk*R
            let point_amount = self.P[i].point_sub(&self.R[i].point_mul(dk));
            if (!point_amount.point_equals(&amount_chunks[i].basepoint_mul())) return false;
            i = i + 1;
        };

        true
    }
}
