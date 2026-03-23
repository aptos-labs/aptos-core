/// A builder for `Statement<P>` that eliminates manual parallel-vector construction.
///
/// Instead of manually maintaining two parallel vectors (`points` and `compressed_points`) that must
/// stay in sync, callers add points via builder methods that handle both vectors internally.
///
/// ## CRITICAL: Builder order must match index constants
///
/// Points must be added in exactly the order the index constants define:
/// - `IDX_H = 0` → first `add_point` call adds H
/// - `IDX_EK = 1` → second `add_point` call adds ek
/// - etc.
///
/// The `assert_*_statement_is_well_formed()` check catches size mismatches but NOT ordering mistakes.
/// The builder does NOT change the index layout.
module aptos_framework::sigma_protocol_statement_builder {
    friend aptos_framework::sigma_protocol_registration;
    friend aptos_framework::sigma_protocol_withdraw;
    friend aptos_framework::sigma_protocol_transfer;
    friend aptos_framework::sigma_protocol_key_rotation;
    #[test_only]
    friend aptos_framework::sigma_protocol_pedeq_example;
    #[test_only]
    friend aptos_framework::sigma_protocol_schnorr_example;
    #[test_only]
    friend aptos_framework::sigma_protocol_proof_tests;

    use aptos_std::ristretto255::{RistrettoPoint, Scalar, CompressedRistretto};
    use aptos_framework::sigma_protocol_statement::{Self, Statement};

    struct StatementBuilder<phantom P> has drop {
        points: vector<RistrettoPoint>,
        compressed_points: vector<CompressedRistretto>,
        scalars: vector<Scalar>,
    }

    public(friend) fun new_builder<P>(): StatementBuilder<P> {
        StatementBuilder {
            points: vector[],
            compressed_points: vector[],
            scalars: vector[],
        }
    }

    /// Add a compressed point; decompresses internally. Returns the index.
    public(friend) fun add_point<P>(self: &mut StatementBuilder<P>, p: CompressedRistretto): u64 {
        let idx = self.points.length();
        self.points.push_back(p.point_decompress());
        self.compressed_points.push_back(p);
        idx
    }

    /// Add a vector of compressed points; decompresses all internally. Returns the starting index.
    public(friend) fun add_points<P>(self: &mut StatementBuilder<P>, v: &vector<CompressedRistretto>): u64 {
        let start = self.points.length();
        v.for_each_ref(|p| {
            let p_val = *p;
            self.points.push_back(p_val.point_decompress());
            self.compressed_points.push_back(p_val);
        });
        start
    }

    /// Like `add_points`, but also returns clones of the decompressed points.
    /// Useful when the caller needs the decompressed points for other purposes (e.g., range proofs).
    public(friend) fun add_points_cloned<P>(self: &mut StatementBuilder<P>, v: &vector<CompressedRistretto>): (u64, vector<RistrettoPoint>) {
        let start = self.points.length();
        let cloned = vector[];
        v.for_each_ref(|p| {
            let p_val = *p;
            let decompressed = p_val.point_decompress();
            cloned.push_back(decompressed.point_clone());
            self.points.push_back(decompressed);
            self.compressed_points.push_back(p_val);
        });
        (start, cloned)
    }

    public(friend) fun add_scalar<P>(self: &mut StatementBuilder<P>, s: Scalar): u64 {
        let idx = self.scalars.length();
        self.scalars.push_back(s);
        idx
    }

    public(friend) fun build<P>(self: StatementBuilder<P>): Statement<P> {
        let StatementBuilder { points, compressed_points, scalars } = self;
        sigma_protocol_statement::new_statement(points, compressed_points, scalars)
    }
}
