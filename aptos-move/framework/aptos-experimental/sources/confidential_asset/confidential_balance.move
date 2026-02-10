/// This module implements a Confidential Balance abstraction, built on top of Twisted ElGamal encryption,
/// over the Ristretto255 curve.
///
/// The Confidential Balance encapsulates encrypted representations of a balance, split into chunks and stored as pairs of
/// ciphertext components `(C_i, D_i)` under basepoints `G` and `H` and an encryption key `EK = dk^(-1) * H`, where `dk`
/// is the corresponding decryption key. Each pair represents an encrypted value `a_i` - the `i`-th 16-bit portion of
/// the total encrypted amount - and its associated randomness `r_i`, such that `C_i = a_i * G + r_i * H` and `D_i = r_i * EK`.
///
/// The module supports two types of balances:
/// - Pending balances are represented by four ciphertext pairs `(C_i, D_i), i = 1..4`, suitable for 64-bit values.
/// - Available balances are represented by eight ciphertext pairs `(C_i, D_i), i = 1..8`, capable of handling 128-bit values.
///
/// This implementation leverages the homomorphic properties of Twisted ElGamal encryption to allow arithmetic operations
/// directly on encrypted data.
module aptos_experimental::confidential_balance {
    use std::error;
    use std::option::{Self, Option};
    use std::vector;
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar, CompressedRistretto};

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

    /// The number of chunks $\ell$ in an available balance.
    const AVAILABLE_BALANCE_CHUNKS: u64 = 8;

    /// The number of bits $b$ in a single chunk.
    const CHUNK_SIZE_BITS: u64 = 16;

    //
    // Structs
    //

    /// Represents a compressed confidential balance.
    /// - `C[i]` is the value component: `chunk_i * G + r_i * H`
    /// - `D[i]` is the EK component: `r_i * EK`
    struct CompressedConfidentialBalance has store, drop, copy {
        C: vector<CompressedRistretto>,
        D: vector<CompressedRistretto>,
    }

    /// Represents an uncompressed confidential balance.
    /// - `C[i]` is the value component: `chunk_i * G + r_i * H`
    /// - `D[i]` is the EK component: `r_i * EK`
    struct ConfidentialBalance has drop {
        C: vector<RistrettoPoint>,
        D: vector<RistrettoPoint>,
    }

    //
    // Accessor functions
    //

    /// Returns a reference to the C components (value components) of a confidential balance.
    public fun get_C(self: &ConfidentialBalance): &vector<RistrettoPoint> {
        &self.C
    }

    /// Returns a reference to the D components (EK components) of a confidential balance.
    public fun get_D(self: &ConfidentialBalance): &vector<RistrettoPoint> {
        &self.D
    }

    /// Returns a reference to the C components (value components) of a compressed confidential balance.
    public fun get_compressed_C(self: &CompressedConfidentialBalance): &vector<CompressedRistretto> {
        &self.C
    }

    /// Returns a reference to the D components (EK components) of a compressed confidential balance.
    public fun get_compressed_D(self: &CompressedConfidentialBalance): &vector<CompressedRistretto> {
        &self.D
    }

    /// Sets the D components (EK components) of a compressed confidential balance.
    public fun set_compressed_D(self: &mut CompressedConfidentialBalance, new_D: vector<CompressedRistretto>) {
        self.D = new_D;
    }

    //
    // Public functions
    //

    /// Creates a new compressed zero balance with the specified number of chunks.
    public fun new_compressed_zero_balance(num_chunks: u64): CompressedConfidentialBalance {
        let identity = ristretto255::point_identity_compressed();
        CompressedConfidentialBalance {
            C: vector::range(0, num_chunks).map(|_| identity),
            D: vector::range(0, num_chunks).map(|_| identity),
        }
    }

    /// Creates a new pending balance from a 64-bit amount with no randomness (D components are identity).
    /// Splits the amount into four 16-bit chunks.
    public fun new_pending_balance_u64_no_randomness(amount: u64): ConfidentialBalance {
        let identity = ristretto255::point_identity();
        ConfidentialBalance {
            C: split_into_chunks((amount as u128), PENDING_BALANCE_CHUNKS).map(|chunk| ristretto255::basepoint_mul(&chunk)),
            D: vector::range(0, PENDING_BALANCE_CHUNKS).map(|_| ristretto255::point_clone(&identity)),
        }
    }

    /// Creates a new balance from a serialized byte array representation.
    /// Format: [C_0 (32 bytes), D_0 (32 bytes), C_1, D_1, ...] - interleaved for SDK compatibility.
    /// Returns `Some(ConfidentialBalance)` if deserialization succeeds, otherwise `None`.
    public fun new_balance_from_bytes(bytes: vector<u8>, num_chunks: u64): Option<ConfidentialBalance> {
        if (bytes.length() != 64 * num_chunks) {
            return option::none()
        };

        let c_vec = vector[];
        let d_vec = vector[];

        let i = 0;
        while (i < num_chunks) {
            let c_opt = ristretto255::new_point_from_bytes(bytes.slice(i * 64, i * 64 + 32));
            let d_opt = ristretto255::new_point_from_bytes(bytes.slice(i * 64 + 32, i * 64 + 64));

            if (c_opt.is_none() || d_opt.is_none()) {
                return option::none()
            };

            c_vec.push_back(c_opt.extract());
            d_vec.push_back(d_opt.extract());
            i = i + 1;
        };

        option::some(ConfidentialBalance { C: c_vec, D: d_vec })
    }

    /// Compresses a confidential balance into its `CompressedConfidentialBalance` representation.
    public fun compress(balance: &ConfidentialBalance): CompressedConfidentialBalance {
        CompressedConfidentialBalance {
            C: balance.C.map_ref(|c| ristretto255::point_compress(c)),
            D: balance.D.map_ref(|d| ristretto255::point_compress(d)),
        }
    }

    /// Decompresses a compressed confidential balance into its `ConfidentialBalance` representation.
    public fun decompress(balance: &CompressedConfidentialBalance): ConfidentialBalance {
        ConfidentialBalance {
            C: balance.C.map_ref(|c| ristretto255::point_decompress(c)),
            D: balance.D.map_ref(|d| ristretto255::point_decompress(d)),
        }
    }

    /// Serializes a confidential balance into a byte array representation.
    /// Format: [C_0 (32 bytes), D_0 (32 bytes), C_1, D_1, ...] - interleaved for SDK compatibility.
    public fun balance_to_bytes(balance: &ConfidentialBalance): vector<u8> {
        let bytes = vector[];
        let i = 0;
        let len = balance.C.length();

        while (i < len) {
            bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(&balance.C[i])));
            bytes.append(ristretto255::point_to_bytes(&ristretto255::point_compress(&balance.D[i])));
            i = i + 1;
        };

        bytes
    }

    /// Adds two confidential balances homomorphically, mutating the first balance in place.
    /// The second balance must have fewer or equal chunks compared to the first.
    public fun add_balances_mut(lhs: &mut ConfidentialBalance, rhs: &ConfidentialBalance) {
        assert!(lhs.C.length() >= rhs.C.length(), error::internal(EINTERNAL_ERROR));

        let i = 0;
        let rhs_len = rhs.C.length();
        while (i < rhs_len) {
            ristretto255::point_add_assign(&mut lhs.C[i], &rhs.C[i]);
            ristretto255::point_add_assign(&mut lhs.D[i], &rhs.D[i]);
            i = i + 1;
        };
    }

    /// Checks if the corresponding value components (`C`) of two confidential balances are equivalent.
    public fun balance_c_equals(lhs: &ConfidentialBalance, rhs: &ConfidentialBalance): bool {
        assert!(lhs.C.length() == rhs.C.length(), error::internal(EINTERNAL_ERROR));

        let ok = true;
        let i = 0;
        let len = lhs.C.length();

        while (i < len) {
            ok = ok && ristretto255::point_equals(&lhs.C[i], &rhs.C[i]);
            i = i + 1;
        };

        ok
    }

    /// Checks if a confidential balance is equivalent to zero (all C and D are identity).
    public fun is_zero_balance(balance: &CompressedConfidentialBalance): bool {
        balance.C.all(|c| c.is_identity()) &&
        balance.D.all(|d| d.is_identity())
    }

    /// Splits an integer amount into `num_chunks` 16-bit chunks, represented as `Scalar` values.
    public fun split_into_chunks(amount: u128, num_chunks: u64): vector<Scalar> {
        vector::range(0, num_chunks).map(|i| {
            ristretto255::new_scalar_from_u128(amount >> (i * CHUNK_SIZE_BITS as u8) & 0xffff)
        })
    }

    //
    // View functions
    //

    #[view]
    /// Returns the number of chunks in a pending balance.
    public fun get_num_pending_chunks(): u64 {
        PENDING_BALANCE_CHUNKS
    }

    #[view]
    /// Returns the number of chunks in an available balance.
    public fun get_num_available_chunks(): u64 {
        AVAILABLE_BALANCE_CHUNKS
    }

    //
    // Test-only
    //

    #[test_only]
    use aptos_experimental::ristretto255_twisted_elgamal as twisted_elgamal;

    #[test_only]
    /// A helper struct for generating randomness for confidential balances in test environments.
    struct ConfidentialBalanceRandomness has drop {
        r: vector<Scalar>
    }

    #[test_only]
    /// Generates a `ConfidentialBalanceRandomness` instance containing random scalars for available balance.
    public fun generate_balance_randomness(): ConfidentialBalanceRandomness {
        ConfidentialBalanceRandomness {
            r: vector::range(0, AVAILABLE_BALANCE_CHUNKS).map(|_| ristretto255::random_scalar())
        }
    }

    #[test_only]
    /// Returns a reference to the vector of random scalars.
    public fun balance_randomness_as_scalars(randomness: &ConfidentialBalanceRandomness): &vector<Scalar> {
        &randomness.r
    }

    #[test_only]
    /// Creates a new balance from an amount using the provided randomness and encryption key.
    public fun new_balance_from_amount(
        amount: u128,
        num_chunks: u64,
        randomness: &ConfidentialBalanceRandomness,
        ek: &CompressedRistretto
    ): ConfidentialBalance {
        let amount_chunks = split_into_chunks(amount, num_chunks);
        let ek_point = ristretto255::point_decompress(ek);
        let basepoint_H = twisted_elgamal::get_encryption_key_basepoint();

        ConfidentialBalance {
            C: vector::range(0, num_chunks).map(|i| {
                // C_i = amount_i * G + r_i * H
                ristretto255::double_scalar_mul(
                    &amount_chunks[i], &ristretto255::basepoint(),
                    &randomness.r[i], &basepoint_H
                )
            }),
            D: vector::range(0, num_chunks).map(|i| {
                // D_i = r_i * EK
                ristretto255::point_mul(&ek_point, &randomness.r[i])
            }),
        }
    }

    #[test_only]
    /// Verifies that a balance encrypts the specified amount.
    /// Infers the number of chunks from the balance itself.
    public fun check_decrypts_to(balance: &ConfidentialBalance, dk: &Scalar, amount: u128): bool {
        let num_chunks = balance.C.length();
        let amount_chunks = split_into_chunks(amount, num_chunks);
        let ok = true;
        let i = 0;

        while (i < num_chunks) {
            // Decrypt: m*G = C - dk*D
            let point_amount = ristretto255::point_sub(&balance.C[i], &ristretto255::point_mul(&balance.D[i], dk));
            ok = ok && ristretto255::point_equals(&point_amount, &ristretto255::basepoint_mul(&amount_chunks[i]));
            i = i + 1;
        };

        ok
    }
}
