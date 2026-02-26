/// This module provides the shared `ConfidentialBalanceRandomness` type used by the Confidential Balance modules
/// (`confidential_pending_balance` and `confidential_available_balance`) for test-only randomness generation.
///
/// The actual balance types and their specialized chunk-splitting / randomness functions live in:
/// - `confidential_pending_balance`: PendingBalance (4 chunks, 64-bit values)
/// - `confidential_available_balance`: AvailableBalance (8 chunks, 128-bit values, with auditor A component)
module aptos_experimental::confidential_balance {
    #[test_only]
    use aptos_std::ristretto255::Scalar;

    //
    // Constants
    //

    /// The number of bits $b$ in a single chunk.
    const CHUNK_SIZE_BITS: u64 = 16;

    //
    // Public functions
    //

    /// Returns the number of bits per chunk.
    public fun get_chunk_size_bits(): u64 {
        CHUNK_SIZE_BITS
    }

    //
    // Test-only
    //

    #[test_only]
    /// A helper struct for generating randomness for confidential balances in test environments.
    struct ConfidentialBalanceRandomness has drop {
        r: vector<Scalar>
    }

    #[test_only]
    /// Creates a `ConfidentialBalanceRandomness` from a vector of random scalars.
    public fun new_randomness(r: vector<Scalar>): ConfidentialBalanceRandomness {
        ConfidentialBalanceRandomness { r }
    }

    #[test_only]
    /// Returns a reference to the vector of random scalars.
    public fun scalars(self: &ConfidentialBalanceRandomness): &vector<Scalar> {
        &self.r
    }
}
