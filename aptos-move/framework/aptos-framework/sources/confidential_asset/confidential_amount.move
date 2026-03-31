/// A transfer amount encrypted under multiple keys, sharing P (commitment) components.
///
/// P = v*G + r*H encodes the amount; each R_* = r*ek_* allows decryption under that key.
/// This bundles the sender, recipient, effective-auditor, and voluntary-auditor R components
/// together with their shared P components.
module aptos_framework::confidential_amount {
    use std::error;
    use aptos_std::ristretto255::{RistrettoPoint, CompressedRistretto};
    use aptos_framework::confidential_balance;
    use aptos_framework::sigma_protocol_utils::deserialize_compressed_points;

    friend aptos_framework::confidential_asset;
    friend aptos_framework::sigma_protocol_transfer;
    #[test_only]
    friend aptos_framework::confidential_asset_tests;
    #[test_only]
    friend aptos_framework::confidential_crypto_test_utils;
    #[test_only]
    friend aptos_framework::sigma_protocol_proof_tests;

    /// Expected the effective auditor R-component to be either empty or have n chunks.
    const E_WRONG_NUM_CHUNKS_FOR_EFFECTIVE_AUDITOR: u64 = 1;
    /// Expected either all voluntary auditors' R-components to be empty or all to have n chunks.
    const E_WRONG_NUM_CHUNKS_FOR_VOLUN_AUDITORS: u64 = 2;
    /// Expected the P, R_sender, or R_recip components to have exactly n chunks.
    const E_WRONG_NUM_CHUNKS: u64 = 3;

    /// Uncompressed transfer amount encrypted under multiple keys.
    struct Amount has drop {
        P: vector<RistrettoPoint>,
        R_sender: vector<RistrettoPoint>,
        R_recip: vector<RistrettoPoint>,
        R_eff_aud: vector<RistrettoPoint>,
        R_volun_auds: vector<vector<RistrettoPoint>>,
    }

    /// Compressed transfer amount encrypted under multiple keys.
    struct CompressedAmount has drop, store, copy {
        compressed_P: vector<CompressedRistretto>,
        compressed_R_sender: vector<CompressedRistretto>,
        compressed_R_recip: vector<CompressedRistretto>,
        compressed_R_eff_aud: vector<CompressedRistretto>,
        compressed_R_volun_auds: vector<vector<CompressedRistretto>>,
    }

    // === Constructors ===

    fun assert_correct_num_chunks<T>(
        p: &vector<T>, r_sender: &vector<T>, r_recip: &vector<T>,
        r_eff_aud: &vector<T>, r_volun_auds: &vector<vector<T>>
    ) {
        let n = confidential_balance::get_num_pending_chunks();
        assert!(p.length() == n, error::invalid_argument(E_WRONG_NUM_CHUNKS));
        assert!(r_sender.length() == n, error::invalid_argument(E_WRONG_NUM_CHUNKS));
        assert!(r_recip.length() == n, error::invalid_argument(E_WRONG_NUM_CHUNKS));
        assert!(r_eff_aud.is_empty() || r_eff_aud.length() == n, error::invalid_argument(
            E_WRONG_NUM_CHUNKS_FOR_EFFECTIVE_AUDITOR
        ));
        assert!(
            r_volun_auds.all(|r| r.length() == n),
            error::invalid_argument(E_WRONG_NUM_CHUNKS_FOR_VOLUN_AUDITORS)
        );
    }

    public(friend) fun new(
        _P: vector<RistrettoPoint>,
        _R_sender: vector<RistrettoPoint>,
        _R_recip: vector<RistrettoPoint>,
        _R_eff_aud: vector<RistrettoPoint>,
        _R_volun_auds: vector<vector<RistrettoPoint>>,
    ): Amount {
        assert_correct_num_chunks(&_P, &_R_sender, &_R_recip, &_R_eff_aud, &_R_volun_auds);
        Amount { P: _P, R_sender: _R_sender, R_recip: _R_recip, R_eff_aud: _R_eff_aud, R_volun_auds: _R_volun_auds }
    }

    public(friend) fun new_compressed(
        compressed_P: vector<CompressedRistretto>,
        compressed_R_sender: vector<CompressedRistretto>,
        compressed_R_recip: vector<CompressedRistretto>,
        compressed_R_eff_aud: vector<CompressedRistretto>,
        compressed_R_volun_auds: vector<vector<CompressedRistretto>>,
    ): CompressedAmount {
        assert_correct_num_chunks(&compressed_P, &compressed_R_sender, &compressed_R_recip, &compressed_R_eff_aud, &compressed_R_volun_auds);

        CompressedAmount {
            compressed_P,
            compressed_R_sender,
            compressed_R_recip,
            compressed_R_eff_aud,
            compressed_R_volun_auds,
        }
    }

    public(friend) fun new_compressed_from_bytes(
        amount_P_bytes: vector<vector<u8>>,
        amount_R_sender_bytes: vector<vector<u8>>,
        amount_R_recip_bytes: vector<vector<u8>>,
        amount_R_eff_aud_bytes: vector<vector<u8>>,
        amount_R_volun_auds_bytes: vector<vector<vector<u8>>>,
    ): CompressedAmount {
        new_compressed(
            deserialize_compressed_points(amount_P_bytes),
            deserialize_compressed_points(amount_R_sender_bytes),
            deserialize_compressed_points(amount_R_recip_bytes),
            deserialize_compressed_points(amount_R_eff_aud_bytes),
            amount_R_volun_auds_bytes.map(|bytes| deserialize_compressed_points(bytes)),
        )
    }

    // === Accessors (CompressedAmount) ===

    public(friend) fun get_compressed_P(self: &CompressedAmount): &vector<CompressedRistretto> { &self.compressed_P }
    public(friend) fun get_compressed_R_sender(self: &CompressedAmount): &vector<CompressedRistretto> { &self.compressed_R_sender }
    public(friend) fun get_compressed_R_recip(self: &CompressedAmount): &vector<CompressedRistretto> { &self.compressed_R_recip }
    public(friend) fun get_compressed_R_eff_aud(self: &CompressedAmount): &vector<CompressedRistretto> { &self.compressed_R_eff_aud }
    public(friend) fun get_compressed_R_volun_auds(self: &CompressedAmount): &vector<vector<CompressedRistretto>> { &self.compressed_R_volun_auds }

    public(friend) fun num_volun_auditors_compressed(self: &CompressedAmount): u64 {
        self.compressed_R_volun_auds.length()
    }

    public(friend) fun has_effective_auditor_compressed(self: &CompressedAmount): bool {
        !self.compressed_R_eff_aud.is_empty()
    }

    // === Compress ===

    public(friend) fun compress(self: &Amount): CompressedAmount {
        CompressedAmount {
            compressed_P: self.P.map_ref(|p| p.point_compress()),
            compressed_R_sender: self.R_sender.map_ref(|r| r.point_compress()),
            compressed_R_recip: self.R_recip.map_ref(|r| r.point_compress()),
            compressed_R_eff_aud: self.R_eff_aud.map_ref(|r| r.point_compress()),
            compressed_R_volun_auds: self.R_volun_auds.map_ref(|rs| {
                rs.map_ref(|r: &RistrettoPoint| r.point_compress())
            }),
        }
    }

}
