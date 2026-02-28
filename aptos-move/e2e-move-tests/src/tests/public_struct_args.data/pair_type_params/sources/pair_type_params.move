/// Test module for Pair<T, U> with two non-phantom type parameters.
///
/// Exercises independent validation of each type argument at construction time:
/// - both valid  → succeeds
/// - first valid, second private  → fails when constructing the second field
/// - first private, second valid  → fails when constructing the first field
module 0xcafe::pair_type_params {

    /// Private struct: no public pack function will be generated.
    struct PrivateData has copy, drop {
        value: u64,
    }

    /// Public copy struct: valid as a transaction argument.
    public struct PublicPoint has copy, drop {
        x: u64,
        y: u64,
    }

    /// Public copy struct with two non-phantom type parameters.
    /// Both T and U are stored as fields, so both are validated at construction time.
    public struct Pair<T, U> has copy, drop {
        first: T,
        second: U,
    }

    /// Both type args are valid public copy structs → succeeds.
    public entry fun test_pair_both_valid(
        _sender: &signer,
        _pair: Pair<PublicPoint, PublicPoint>,
    ) {}

    /// First is valid, second is private → fails at construction (no pack function for PrivateData).
    public entry fun test_pair_second_invalid(
        _sender: &signer,
        _pair: Pair<PublicPoint, PrivateData>,
    ) {}

    /// First is private, second is valid → fails at construction (no pack function for PrivateData).
    public entry fun test_pair_first_invalid(
        _sender: &signer,
        _pair: Pair<PrivateData, PublicPoint>,
    ) {}
}
