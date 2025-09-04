module velor_experimental::helpers {
    use std::vector;
    use std::error;

    use velor_std::ristretto255_elgamal as elgamal;
    use velor_std::ristretto255;

    /// Tried cutting out more elements than are in the vector via `cut_vector`.
    const EVECTOR_CUT_TOO_LARGE: u64 = 1;

    /// Given a vector `vec`, removes the last `cut_len` elements of `vec` and returns them in order. (This function
    /// exists because we did not like the interface of `std::vector::trim`.)
    public fun cut_vector<T>(vec: &mut vector<T>, cut_len: u64): vector<T> {
        let len = vector::length(vec);
        let res = vector::empty();
        assert!(len >= cut_len, error::out_of_range(EVECTOR_CUT_TOO_LARGE));
        while (cut_len > 0) {
            res.push_back(vector::pop_back(vec));
            cut_len -= 1;
        };
        res.reverse();
        res
    }

    /// Returns an encryption of zero, without any randomness (i.e., $r=0$), under any ElGamal PK.
    public fun get_veiled_balance_zero_ciphertext(): elgamal::CompressedCiphertext {
        elgamal::ciphertext_from_compressed_points(
            ristretto255::point_identity_compressed(),
            ristretto255::point_identity_compressed()
        )
    }

    /// Returns an encryption of `amount`, without any randomness (i.e., $r=0$), under any ElGamal PK.
    /// WARNING: This is not a proper ciphertext: the value `amount` can be easily bruteforced.
    public fun public_amount_to_veiled_balance(amount: u32): elgamal::Ciphertext {
        let scalar = ristretto255::new_scalar_from_u32(amount);

        elgamal::new_ciphertext_no_randomness(&scalar)
    }

    #[test_only]
    /// Returns a random ElGamal keypair
    public fun generate_elgamal_keypair(): (ristretto255::Scalar, elgamal::CompressedPubkey) {
        let sk = ristretto255::random_scalar();
        let pk = elgamal::pubkey_from_secret_key(&sk);
        (sk, pk)
    }
}
