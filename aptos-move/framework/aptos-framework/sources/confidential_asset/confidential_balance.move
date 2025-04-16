/// This module implements a Confidential Balance abstraction, built on top of Twisted ElGamal encryption,
/// over the Ristretto255 curve.
/// 
/// The Confidential Balance encapsulates encrypted representations of a balance, split into chunks and stored as pairs of 
/// ciphertext components `(C_i, D_i)` under basepoints `G` and `H` and an encryption key `P = dk^(-1) * H`, where `dk`
/// is the corresponding decryption key. Each pair represents an encrypted value `a_i` - the `i`-th 16-bit portion of
/// the total encrypted amount - and its associated randomness `r_i`, such that `C_i = a_i * G + r_i * H` and `D_i = r_i * P`.
///
/// The module supports two types of balances:
/// - Pending balances are represented by four ciphertext pairs `(C_i, D_i), i = 1..4`, suitable for 64-bit values.
/// - Actual balances are represented by eight ciphertext pairs `(C_i, D_i), i = 1..8`, capable of handling 128-bit values.
///
/// This implementation leverages the homomorphic properties of Twisted ElGamal encryption to allow arithmetic operations
/// directly on encrypted data.
module aptos_framework::confidential_balance {
    use std::error;
    use std::option::{Self, Option};
    use std::vector;
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar};
    use aptos_std::ristretto255_twisted_elgamal as twisted_elgamal;

    //
    // Errors
    //

    /// An internal error occurred, indicating unexpected behavior.
    const EINTERNAL_ERROR: u64 = 1;

    //
    // Contants
    //

    /// The number of chunks in a pending balance.
    const PENDING_BALANCE_CHUNKS: u64 = 4;
    /// The number of chunks in an actual balance.
    const ACTUAL_BALANCE_CHUNKS: u64 = 8;
    /// The number of bits in a single chunk.
    const CHUNK_SIZE_BITS: u64 = 16;

    //
    // Structs
    //

    /// Represents a compressed confidential balance, where each chunk is a compressed Twisted ElGamal ciphertext.
    struct CompressedConfidentialBalance has store, drop, copy {
        chunks: vector<twisted_elgamal::CompressedCiphertext>,
    }

    /// Represents a confidential balance, where each chunk is a Twisted ElGamal ciphertext.
    struct ConfidentialBalance has drop {
        chunks: vector<twisted_elgamal::Ciphertext>,
    }

    //
    // Public functions
    //

    /// Creates a new zero pending balance, where each chunk is set to zero Twisted ElGamal ciphertext.
    public fun new_pending_balance_no_randomness(): ConfidentialBalance {
        ConfidentialBalance {
            chunks: vector::range(0, PENDING_BALANCE_CHUNKS).map(|_| {
                twisted_elgamal::ciphertext_from_points(ristretto255::point_identity(), ristretto255::point_identity())
            })
        }
    }

    /// Creates a new zero actual balance, where each chunk is set to zero Twisted ElGamal ciphertext.
    public fun new_actual_balance_no_randomness(): ConfidentialBalance {
        ConfidentialBalance {
            chunks: vector::range(0, ACTUAL_BALANCE_CHUNKS).map(|_| {
                twisted_elgamal::ciphertext_from_points(ristretto255::point_identity(), ristretto255::point_identity())
            })
        }
    }

    /// Creates a new compressed zero pending balance, where each chunk is set to compressed zero Twisted ElGamal ciphertext.
    public fun new_compressed_pending_balance_no_randomness(): CompressedConfidentialBalance {
        CompressedConfidentialBalance {
            chunks: vector::range(0, PENDING_BALANCE_CHUNKS).map(|_| {
                twisted_elgamal::ciphertext_from_compressed_points(
                    ristretto255::point_identity_compressed(), ristretto255::point_identity_compressed())
            })
        }
    }

    /// Creates a new compressed zero actual balance, where each chunk is set to compressed zero Twisted ElGamal ciphertext.
    public fun new_compressed_actual_balance_no_randomness(): CompressedConfidentialBalance {
        CompressedConfidentialBalance {
            chunks: vector::range(0, ACTUAL_BALANCE_CHUNKS).map(|_| {
                twisted_elgamal::ciphertext_from_compressed_points(
                    ristretto255::point_identity_compressed(), ristretto255::point_identity_compressed())
            })
        }
    }

    /// Creates a new pending balance from a 64-bit amount with no randomness, splitting the amount into four 16-bit chunks.
    public fun new_pending_balance_u64_no_randonmess(amount: u64): ConfidentialBalance {
        ConfidentialBalance {
            chunks: split_into_chunks_u64(amount).map(|chunk| {
                twisted_elgamal::new_ciphertext_no_randomness(&chunk)
            })
        }
    }

    /// Creates a new pending balance from a serialized byte array representation.
    /// Returns `Some(ConfidentialBalance)` if deserialization succeeds, otherwise `None`.
    public fun new_pending_balance_from_bytes(bytes: vector<u8>): Option<ConfidentialBalance> {
        if (bytes.length() != 64 * PENDING_BALANCE_CHUNKS) {
            return std::option::none()
        };

        let chunks = vector::range(0, PENDING_BALANCE_CHUNKS).map(|i| {
            twisted_elgamal::new_ciphertext_from_bytes(bytes.slice(i * 64, (i + 1) * 64))
        });

        if (chunks.any(|chunk| chunk.is_none())) {
            return std::option::none()
        };

        option::some(ConfidentialBalance {
            chunks: chunks.map(|chunk| chunk.extract())
        })
    }

    /// Creates a new actual balance from a serialized byte array representation.
    /// Returns `Some(ConfidentialBalance)` if deserialization succeeds, otherwise `None`.
    public fun new_actual_balance_from_bytes(bytes: vector<u8>): Option<ConfidentialBalance> {
        if (bytes.length() != 64 * ACTUAL_BALANCE_CHUNKS) {
            return std::option::none()
        };

        let chunks = vector::range(0, ACTUAL_BALANCE_CHUNKS).map(|i| {
            twisted_elgamal::new_ciphertext_from_bytes(bytes.slice(i * 64, (i + 1) * 64))
        });

        if (chunks.any(|chunk| chunk.is_none())) {
            return std::option::none()
        };

        option::some(ConfidentialBalance {
            chunks: chunks.map(|chunk| chunk.extract())
        })
    }

    /// Compresses a confidential balance into its `CompressedConfidentialBalance` representation.
    public fun compress_balance(balance: &ConfidentialBalance): CompressedConfidentialBalance {
        CompressedConfidentialBalance {
            chunks: balance.chunks.map_ref(|ciphertext| twisted_elgamal::compress_ciphertext(ciphertext))
        }
    }

    /// Decompresses a compressed confidential balance into its `ConfidentialBalance` representation.
    public fun decompress_balance(balance: &CompressedConfidentialBalance): ConfidentialBalance {
        ConfidentialBalance {
            chunks: balance.chunks.map_ref(|ciphertext| twisted_elgamal::decompress_ciphertext(ciphertext))
        }
    }

    /// Serializes a confidential balance into a byte array representation.
    public fun balance_to_bytes(balance: &ConfidentialBalance): vector<u8> {
        let bytes = vector<u8>[];

        balance.chunks.for_each_ref(|ciphertext| {
            bytes.append(twisted_elgamal::ciphertext_to_bytes(ciphertext));
        });

        bytes
    }

    /// Extracts the `C` value component (`a * H + r * G`) of each chunk in a confidential balance as a vector of `RistrettoPoint`s.
    public fun balance_to_points_c(balance: &ConfidentialBalance): vector<RistrettoPoint> {
        balance.chunks.map_ref(|chunk| {
            let (c, _) = twisted_elgamal::ciphertext_as_points(chunk);
            ristretto255::point_clone(c)
        })
    }

    /// Extracts the `D` randomness component (`r * Y`) of each chunk in a confidential balance as a vector of `RistrettoPoint`s.
    public fun balance_to_points_d(balance: &ConfidentialBalance): vector<RistrettoPoint> {
        balance.chunks.map_ref(|chunk| {
            let (_, d) = twisted_elgamal::ciphertext_as_points(chunk);
            ristretto255::point_clone(d)
        })
    }

    /// Adds two confidential balances homomorphically, mutating the first balance in place.
    /// The second balance must have fewer or equal chunks compared to the first.
    public fun add_balances_mut(lhs: &mut ConfidentialBalance, rhs: &ConfidentialBalance) {
        assert!(lhs.chunks.length() >= rhs.chunks.length(), error::internal(EINTERNAL_ERROR));

        lhs.chunks.enumerate_mut(|i, chunk| {
            if (i < rhs.chunks.length()) {
                twisted_elgamal::ciphertext_add_assign(chunk, &rhs.chunks[i])
            }
        })
    }

    /// Subtracts one confidential balance from another homomorphically, mutating the first balance in place.
    /// The second balance must have fewer or equal chunks compared to the first.
    public fun sub_balances_mut(lhs: &mut ConfidentialBalance, rhs: &ConfidentialBalance) {
        assert!(lhs.chunks.length() >= rhs.chunks.length(), error::internal(EINTERNAL_ERROR));

        lhs.chunks.enumerate_mut(|i, chunk| {
            if (i < rhs.chunks.length()) {
                twisted_elgamal::ciphertext_add_assign(chunk, &rhs.chunks[i])
            }
        })
    }

    /// Checks if two confidential balances are equivalent, including both value and randomness components.
    public fun balance_equals(lhs: &ConfidentialBalance, rhs: &ConfidentialBalance): bool {
        assert!(lhs.chunks.length() == rhs.chunks.length(), error::internal(EINTERNAL_ERROR));

        let ok = true;

        lhs.chunks.zip_ref(&rhs.chunks, |l, r| {
            ok = ok && twisted_elgamal::ciphertext_equals(l, r);
        });

        ok
    }

    /// Checks if the corresponding value components (`C`) of two confidential balances are equivalent.
    public fun balance_c_equals(lhs: &ConfidentialBalance, rhs: &ConfidentialBalance): bool {
        assert!(lhs.chunks.length() == rhs.chunks.length(), error::internal(EINTERNAL_ERROR));

        let ok = true;

        lhs.chunks.zip_ref(&rhs.chunks, |l, r| {
            let (lc, _) = twisted_elgamal::ciphertext_as_points(l);
            let (rc, _) = twisted_elgamal::ciphertext_as_points(r);

            ok = ok && ristretto255::point_equals(lc, rc);
        });

        ok
    }

    /// Checks if a confidential balance is equivalent to zero, where all chunks are the identity element.
    public fun is_zero_balance(balance: &ConfidentialBalance): bool {
        balance.chunks.all(|chunk| {
            twisted_elgamal::ciphertext_equals(
                chunk,
                &twisted_elgamal::ciphertext_from_points(ristretto255::point_identity(), ristretto255::point_identity())
            )
        })
    }

    /// Splits a 64-bit integer amount into four 16-bit chunks, represented as `Scalar` values.
    public fun split_into_chunks_u64(amount: u64): vector<Scalar> {
        vector::range(0, PENDING_BALANCE_CHUNKS).map(|i| {
            ristretto255::new_scalar_from_u64(amount >> (i * CHUNK_SIZE_BITS as u8) & 0xffff)
        })
    }

    /// Splits a 128-bit integer amount into eight 16-bit chunks, represented as `Scalar` values.
    public fun split_into_chunks_u128(amount: u128): vector<Scalar> {
        vector::range(0, ACTUAL_BALANCE_CHUNKS).map(|i| {
            ristretto255::new_scalar_from_u128(amount >> (i * CHUNK_SIZE_BITS as u8) & 0xffff)
        })
    }

    //
    // View functions
    //

    #[view]
    /// Returns the number of chunks in a pending balance.
    public fun get_pending_balance_chunks(): u64 {
        PENDING_BALANCE_CHUNKS
    }

    #[view]
    /// Returns the number of chunks in an actual balance.
    public fun get_actual_balance_chunks(): u64 {
        ACTUAL_BALANCE_CHUNKS
    }

    #[view]
    /// Returns the number of bits in a single chunk.
    public fun get_chunk_size_bits(): u64 {
        CHUNK_SIZE_BITS
    }

    //
    // Test-only
    //

    #[test_only]
    /// A helper struct for generating randomness for confidential balances in test environments.
    /// Each `r` element represents a random scalar used for Twisted ElGamal encryption.
    /// Can be used to generate both actual and pending balances.
    struct ConfidentialBalanceRandomness has drop {
        r: vector<Scalar>,
    }

    #[test_only]
    /// Generates a `ConfidentialBalanceRandomness` instance containing four random scalars.
    /// This is useful for creating randomness for actual balances during testing.
    public fun generate_balance_randomness(): ConfidentialBalanceRandomness {
        ConfidentialBalanceRandomness {
            r: vector::range(0, ACTUAL_BALANCE_CHUNKS).map(|_| ristretto255::random_scalar())
        }
    }

    #[test_only]
    /// Returns a reference to the vector of random scalars within the provided `ConfidentialBalanceRandomness`.
    public fun balance_randomness_as_scalars(randomness: &ConfidentialBalanceRandomness): &vector<Scalar> {
        &randomness.r
    }

    #[test_only]
    /// Creates a new actual balance from a 128-bit amount using the provided randomness and encryption key.
    /// Splits the amount into eight 16-bit chunks and encrypts each chunk with the corresponding random scalar.
    public fun new_actual_balance_from_u128(
        amount: u128,
        randomness: &ConfidentialBalanceRandomness,
        ek: &twisted_elgamal::CompressedPubkey): ConfidentialBalance
    {
        let amount_chunks = split_into_chunks_u128(amount);

        ConfidentialBalance {
            chunks: vector::range(0, ACTUAL_BALANCE_CHUNKS).map(|i| {
                twisted_elgamal::new_ciphertext_with_basepoint(&amount_chunks[i], &randomness.r[i], ek)
            })
        }
    }

    #[test_only]
    /// Creates a new pending balance from a 64-bit amount using the provided randomness and encryption key.
    /// Splits the amount into four 16-bit chunks and encrypts each chunk with the corresponding random scalar.
    public fun new_pending_balance_from_u64(
        amount: u64,
        randomness: &ConfidentialBalanceRandomness,
        ek: &twisted_elgamal::CompressedPubkey): ConfidentialBalance
    {
        let amount_chunks = split_into_chunks_u64(amount);

        ConfidentialBalance {
            chunks: vector::range(0, PENDING_BALANCE_CHUNKS).map(|i| {
                twisted_elgamal::new_ciphertext_with_basepoint(&amount_chunks[i], &randomness.r[i], ek)
            })
        }
    }

    #[test_only]
    /// Verifies that an actual balance encrypts the specified 128-bit amount using the provided decryption key.
    /// Checks that the decryption of each chunk matches the corresponding 16-bit chunk of the provided amount.
    /// Use carefully, as it may return `false` if the balance is not normalized (i.e. has overflowed chunks).
    public fun verify_actual_balance(balance: &ConfidentialBalance, dk: &Scalar, amount: u128): bool {
        assert!(balance.chunks.length() == ACTUAL_BALANCE_CHUNKS, error::internal(EINTERNAL_ERROR));

        let amount_chunks = split_into_chunks_u128(amount);
        let ok = true;

        balance.chunks.zip_ref(&amount_chunks, |balance, amount| {
            let (balance_c, balance_d) = twisted_elgamal::ciphertext_as_points(balance);
            let point_amount = ristretto255::point_sub(balance_c, &ristretto255::point_mul(balance_d, dk));

            ok = ok && ristretto255::point_equals(&point_amount, &ristretto255::basepoint_mul(amount));
        });

        ok
    }

    #[test_only]
    /// Verifies that a pending balance encrypts the specified 64-bit amount using the provided decryption key.
    /// Checks that the decryption of each chunk matches the corresponding 16-bit chunk of the provided amount.
    /// Use carefully, as it may return `false` if the balance is not normalized (i.e. has overflowed chunks).
    public fun verify_pending_balance(balance: &ConfidentialBalance, dk: &Scalar, amount: u64): bool {
        assert!(balance.chunks.length() == PENDING_BALANCE_CHUNKS, error::internal(EINTERNAL_ERROR));

        let amount_chunks = split_into_chunks_u64(amount);
        let ok = true;

        balance.chunks.zip_ref(&amount_chunks, |balance, amount| {
            let (balance_c, balance_d) = twisted_elgamal::ciphertext_as_points(balance);
            let point_amount = ristretto255::point_sub(balance_c, &ristretto255::point_mul(balance_d, dk));

            ok = ok && ristretto255::point_equals(&point_amount, &ristretto255::basepoint_mul(amount));
        });

        ok
    }
}
