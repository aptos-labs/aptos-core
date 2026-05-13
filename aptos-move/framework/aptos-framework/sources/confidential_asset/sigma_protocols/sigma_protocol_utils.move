module aptos_framework::sigma_protocol_utils {
    friend aptos_framework::sigma_protocol_proof;
    friend aptos_framework::sigma_protocol;
    friend aptos_framework::sigma_protocol_registration;
    friend aptos_framework::sigma_protocol_withdraw;
    friend aptos_framework::sigma_protocol_transfer;
    friend aptos_framework::sigma_protocol_key_rotation;
    friend aptos_framework::confidential_asset;
    friend aptos_framework::confidential_balance;
    friend aptos_framework::confidential_amount;
    #[test_only]
    friend aptos_framework::confidential_asset_tests;
    #[test_only]
    friend aptos_framework::sigma_protocol_pedeq_example;
    #[test_only]
    friend aptos_framework::sigma_protocol_schnorr_example;
    #[test_only]
    friend aptos_framework::confidential_crypto_test_utils;
    #[test_only]
    friend aptos_framework::sigma_protocol_proof_tests;

    use aptos_std::ristretto255::{RistrettoPoint, Scalar, CompressedRistretto,
        new_point_and_compressed_from_bytes, new_compressed_point_from_bytes,
        new_scalar_from_bytes
    };

    // === Shared error codes for sigma protocol proof modules ===

    use std::error;

    public(friend) fun e_wrong_num_points(): u64 { error::invalid_argument(1) }
    public(friend) fun e_wrong_num_scalars(): u64 { error::invalid_argument(2) }
    public(friend) fun e_wrong_witness_len(): u64 { error::invalid_argument(3) }
    public(friend) fun e_wrong_output_len(): u64 { error::invalid_argument(4) }

    /// Clones a vector of Ristretto255 points
    // TODO(Perf): Annoying limitation of our Ristretto255 module. (Should we "fix" it as per `crypto_algebra`?)
    public(friend) fun points_clone(a: &vector<RistrettoPoint>): vector<RistrettoPoint> {
        a.map_ref(|p| p.point_clone())
    }

    /// Deserializes a vector of point bytes to a vector of RistrettoPoints and a vector of their compressed counterparts.
    public(friend) fun deserialize_points(points_bytes: vector<vector<u8>>): (vector<RistrettoPoint>, vector<CompressedRistretto>) {
        let points = vector[];
        let compressed_points = vector[];
        points_bytes.for_each(|point_bytes| {
            let (point, compressed_point) = new_point_and_compressed_from_bytes(point_bytes);
            points.push_back(point);
            compressed_points.push_back(compressed_point);
        });

        (points, compressed_points)
    }

    /// Deserializes a vector of point bytes to a vector of CompressedRistretto's (without decompressing to RistrettoPoint).
    public(friend) fun deserialize_compressed_points(points_bytes: vector<vector<u8>>): vector<CompressedRistretto> {
        points_bytes.map(|bytes| new_compressed_point_from_bytes(bytes).extract())
    }

    public(friend) fun deserialize_scalars(scalars_bytes: vector<vector<u8>>): vector<Scalar> {
        scalars_bytes.map(|scalar_bytes| new_scalar_from_bytes(scalar_bytes).extract())
    }

    /// Negates a vector of scalars `a`, returns a new vector `c` where `c[i] = -a[i]`.
    public(friend) fun neg_scalars(a: &vector<Scalar>): vector<Scalar> {
        a.map_ref(|s| s.scalar_neg())
    }

}
