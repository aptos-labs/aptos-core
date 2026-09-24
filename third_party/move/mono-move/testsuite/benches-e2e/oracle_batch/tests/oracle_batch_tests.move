#[test_only]
module bench::oracle_batch_tests {
    use std::hash;
    use std::signer;
    use std::vector;
    use aptos_std::ed25519;
    use bench::orc_aggregator;
    use bench::orc_decode;
    use bench::orc_queue;
    use bench::orc_registry;
    use bench::orc_verifier;

    /// Authority key pair the vectors below were produced under, and the
    /// matching raw secp256k1 key. Both come from `StdRng::from_seed([7; 32])`,
    /// the same seed the transaction generator uses.
    const ED_PUBKEY: vector<u8> =
        x"e79a4e621583674785585866dc854fb85e2b5d208693483a4cdecd901f43d85d";
    const SECP_PUBKEY: vector<u8> =
        x"bb1370723599c0f0791048177e61339bfa429bef27d633a77f86a8da25dba195884204a41dfbbed4254fb3f579a1a4e6ce202415be0b986911aaa94459bc3597";

    /// The exact bytes the generator signs for a four-record batch: a 32-byte
    /// domain separator, the BCS length 192, then records at offsets zero
    /// through three with prices 100000000 + 7i, confidences 10 + i and
    /// timestamps 1700000000 + 60i.
    const BATCH: vector<u8> =
        x"6141460b88b6f6b32fcfec24bd6635271c75d596f929f6f65f82df1990009826c001000000000000000000000000000000000000000005f5e100000000000000000a000000006553f1000000000000000000000000000000000100000000000000000000000005f5e107000000000000000b000000006553f13c0000000000000000000000000000000200000000000000000000000005f5e10e000000000000000c000000006553f1780000000000000000000000000000000300000000000000000000000005f5e115000000000000000d000000006553f1b40000000000000000";
    const BATCH_ED_SIG: vector<u8> =
        x"d989973690b5d1ed044888df19f1b0328b91b8d1135ed640490169715f73bafd980e7a0de37e81405eac9fb6985978b2443d319cf5438d1da13eac6401863c0d";
    const BATCH_SECP_SIG: vector<u8> =
        x"3df6d879f880f459c398b46cfbd04908391bde086f92cf29b7e3280343038eea5f8a5b52a5913915f74cc9d37e23e270969a9236d22951c01f1f780d8c2ee3f8";
    /// Recovery id the signature above was produced under. The generator sends
    /// one per signature, and `test_secp256k1_recovers_the_signing_key` pins it
    /// to the only id that recovers `SECP_PUBKEY`.
    const BATCH_SECP_ID: u8 = 0;

    /// The same, for a single record: offset 0, price 100000000, confidence 10,
    /// timestamp 1700000000. Its BCS length is one byte, so the records start
    /// one byte earlier than in `BATCH`.
    const ONE: vector<u8> =
        x"6141460b88b6f6b32fcfec24bd6635271c75d596f929f6f65f82df199000982630000000000000000000000000000000000000000005f5e100000000000000000a000000006553f1000000000000000000";
    const ONE_ED_SIG: vector<u8> =
        x"974fff92788d8ab61d4c79350fea8c174135925c61dcf511c7e93c94bde4466966d9cad4065b7ad8c43b306ab9782f5e43d318f1b1f67e31ac896edea69e0d06";
    const ONE_SECP_SIG: vector<u8> =
        x"ad4a09aefdc313573c93e311f42085ed31a690fa0f82c37297bc3c5b4f1ed43c71a14dd2e09e1a20b2a7e1cde2012e677d76ec31f0fba97bbbef63cd527793fa";
    const ONE_SECP_ID: u8 = 0;

    const RECORDS_IN_BATCH: u64 = 4;
    const DEPTH: u64 = 2;
    const CHUNK: u64 = 4;

    fun authority_set(): vector<vector<u8>> {
        vector[ED_PUBKEY, SECP_PUBKEY]
    }

    fun setup(admin: &signer, feeds: u64, quorum: u64) {
        orc_queue::initialize(admin, authority_set(), quorum);
        orc_registry::create_feeds(admin, feeds);
    }

    fun append_be(out: &mut vector<u8>, value: u128, width: u64) {
        let i = width;
        while (i > 0) {
            i = i - 1;
            vector::push_back(out, (((value >> ((i * 8) as u8)) & 255) as u8));
        }
    }

    fun make_record(feed: u64, price: u128, conf: u64, ts: u64): vector<u8> {
        let out = vector::empty<u8>();
        append_be(&mut out, (feed as u128), 8);
        append_be(&mut out, price, 16);
        append_be(&mut out, (conf as u128), 8);
        append_be(&mut out, (ts as u128), 8);
        append_be(&mut out, 0, 8);
        out
    }

    /// The header layout the signer produces. Only its shape matters to the
    /// decoder, so the domain separator here is zeros.
    fun signed_message(payload: vector<u8>): vector<u8> {
        let out = vector::empty<u8>();
        while (vector::length(&out) < 32) {
            vector::push_back(&mut out, 0);
        };
        let len = vector::length(&payload);
        while (len >= 128) {
            vector::push_back(&mut out, (((len % 128) + 128) as u8));
            len = len / 128;
        };
        vector::push_back(&mut out, (len as u8));
        vector::append(&mut out, payload);
        out
    }

    fun padded(bytes: vector<u8>, len: u64): vector<u8> {
        while (vector::length(&bytes) < len) {
            vector::push_back(&mut bytes, 0);
        };
        bytes
    }

    #[test]
    fun test_ed25519_verifies_the_signed_message() {
        // The wire format is the whole workload: a signature that silently
        // fails burns the same scalar multiplication and measures nothing.
        let pubkey = ed25519::new_unvalidated_public_key_from_bytes(ED_PUBKEY);
        let signature = ed25519::new_signature_from_bytes(BATCH_ED_SIG);
        assert!(
            ed25519::signature_verify_strict(&signature, &pubkey, BATCH),
            0,
        );
        assert!(
            orc_verifier::verify_ed25519(ED_PUBKEY, BATCH_ED_SIG, BATCH),
            0,
        );
        assert!(orc_verifier::verify_ed25519(ED_PUBKEY, ONE_ED_SIG, ONE), 0);
        // Bound to its own bytes, so it cannot be replayed onto the other
        // batch.
        assert!(!orc_verifier::verify_ed25519(ED_PUBKEY, ONE_ED_SIG, BATCH), 0);
    }

    #[test]
    fun test_secp256k1_recovers_the_signing_key() {
        // Exactly one of the four ids recovers the signer, and it is the one
        // the generator sends. Any other recovers a key the authority set does
        // not hold, which is what the quorum has to reject.
        let digest = hash::sha3_256(BATCH);
        let matches = 0;
        let rid = 0;
        while (rid < 4) {
            let raw = orc_verifier::recover_raw(digest, rid, BATCH_SECP_SIG);
            if (raw == SECP_PUBKEY) {
                matches = matches + 1;
                assert!(rid == BATCH_SECP_ID, 0);
                assert!(
                    orc_verifier::recover_secp256k1(digest, rid, BATCH_SECP_SIG)
                        == hash::sha3_256(SECP_PUBKEY),
                    0,
                );
            };
            rid = rid + 1;
        };
        assert!(matches == 1, 0);

        let digest = hash::sha3_256(ONE);
        let matches = 0;
        let rid = 0;
        while (rid < 4) {
            if (orc_verifier::recover_raw(digest, rid, ONE_SECP_SIG)
                == SECP_PUBKEY) {
                matches = matches + 1;
                assert!(rid == ONE_SECP_ID, 0);
            };
            rid = rid + 1;
        };
        assert!(matches == 1, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce, bob = @0xb0b)]
    fun test_harness_onboard_then_mix(
        admin: &signer, alice: &signer, bob: &signer
    ) {
        // Publisher-signed setup, in the order the generator issues it.
        orc_queue::initialize(admin, authority_set(), 1);
        orc_registry::create_feeds(admin, CHUNK);
        orc_registry::create_feeds(admin, CHUNK);
        assert!(orc_registry::num_feeds() == 2 * CHUNK, 0);
        assert!(orc_queue::num_ed_pubkeys() == 1, 0);
        assert!(orc_queue::num_secp_addrs() == 1, 0);

        // One onboarding transaction per account, each on its own window.
        orc_aggregator::bench_onboard(alice, 0);
        orc_aggregator::bench_onboard(bob, RECORDS_IN_BATCH);

        // One call of every mix branch, at the arguments the generator builds.
        orc_aggregator::bench_update_ed25519(
            alice, 0, BATCH, vector[BATCH_ED_SIG], vector[0], DEPTH);
        orc_aggregator::bench_update_secp256k1(
            alice, 0, BATCH, vector[BATCH_SECP_SIG], vector[BATCH_SECP_ID], DEPTH);
        orc_aggregator::bench_verify_only(
            alice, BATCH, vector[BATCH_ED_SIG], vector[0]);
        orc_aggregator::bench_write_only(alice, 0, BATCH, DEPTH);
        orc_aggregator::bench_quorum(
            alice,
            0,
            BATCH,
            vector[BATCH_ED_SIG],
            vector[0],
            vector[BATCH_SECP_SIG],
            vector[BATCH_SECP_ID],
            DEPTH,
        );
        orc_aggregator::bench_read_aggregate(alice, 0, RECORDS_IN_BATCH);

        // Four of the six branches write, and each writes every feed of the
        // window once.
        let i = 0;
        while (i < RECORDS_IN_BATCH) {
            let (price, aggregate, conf, ts, updates) = orc_registry::read(i);
            assert!(updates == 4, 0);
            assert!(price == 100000000 + (i as u128) * 7, 0);
            assert!(aggregate == price, 0);
            assert!(conf == 10 + i, 0);
            assert!(ts == 1700000000 + i * 60, 0);
            i = i + 1;
        };

        // The window the generator gave bob is untouched.
        let (_price, _aggregate, _conf, _ts, updates) =
            orc_registry::read(RECORDS_IN_BATCH);
        assert!(updates == 0, 0);

        // Both schemes verified on both the update and the quorum branch.
        let (batches, verified, rejected, _authorized) =
            orc_aggregator::subscriber_state(signer::address_of(alice));
        assert!(batches == 4, 0);
        assert!(verified == 4, 0);
        assert!(rejected == 0, 0);
        assert!(orc_aggregator::read_aggregate(0, RECORDS_IN_BATCH) != 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_update_ed25519_verifies_real_signature(
        admin: &signer, alice: &signer
    ) {
        setup(admin, CHUNK, 1);
        orc_aggregator::bench_onboard(alice, 0);
        orc_aggregator::bench_update_ed25519(
            alice, 0, ONE, vector[ONE_ED_SIG], vector[0], DEPTH);
        let (_batches, verified, rejected, _authorized) =
            orc_aggregator::subscriber_state(signer::address_of(alice));
        assert!(verified == 1, 0);
        assert!(rejected == 0, 0);
        let (price, _aggregate, conf, ts, updates) = orc_registry::read(0);
        assert!(updates == 1, 0);
        assert!(price == 100000000, 0);
        assert!(conf == 10, 0);
        assert!(ts == 1700000000, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_update_ed25519_counts_a_bad_signature(
        admin: &signer, alice: &signer
    ) {
        setup(admin, CHUNK, 1);
        orc_aggregator::bench_onboard(alice, 0);
        // The right signature over the wrong message.
        orc_aggregator::bench_update_ed25519(
            alice, 0, BATCH, vector[ONE_ED_SIG], vector[0], DEPTH);
        let (batches, verified, rejected, _authorized) =
            orc_aggregator::subscriber_state(signer::address_of(alice));
        assert!(batches == 1, 0);
        assert!(verified == 0, 0);
        assert!(rejected == 1, 0);
        // A rejected signature does not stop the batch from landing.
        let (_price, _aggregate, _conf, _ts, updates) = orc_registry::read(0);
        assert!(updates == 1, 0);
    }

    #[test]
    fun test_verifier_tolerates_short_pubkey() {
        let short = x"0102030405060708090a";
        assert!(vector::length(&short) == 10, 0);
        assert!(!orc_verifier::verify_ed25519(short, BATCH_ED_SIG, BATCH), 0);
        assert!(vector::length(&orc_verifier::clamp(short, 32)) == 32, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_update_tolerates_oversized_signature(
        admin: &signer, alice: &signer
    ) {
        setup(admin, CHUNK, 1);
        orc_aggregator::bench_onboard(alice, 0);
        let long = padded(BATCH_ED_SIG, 100);
        assert!(vector::length(&long) == 100, 0);
        assert!(vector::length(&orc_verifier::clamp(long, 64)) == 64, 0);
        orc_aggregator::bench_update_ed25519(
            alice, 0, BATCH, vector[long], vector[0], DEPTH);
        // Truncating to 64 bytes puts back the signature the padding grew
        // from, so this one verifies instead of being counted as a failure.
        let (batches, verified, rejected, _authorized) =
            orc_aggregator::subscriber_state(signer::address_of(alice));
        assert!(batches == 1, 0);
        assert!(verified == 1, 0);
        assert!(rejected == 0, 0);
        let (_price, _aggregate, _conf, _ts, updates) = orc_registry::read(0);
        assert!(updates == 1, 0);

        // A signature that is oversized on its own is counted, never fatal.
        let garbage = padded(x"ff", 100);
        orc_aggregator::bench_update_ed25519(
            alice, 0, BATCH, vector[garbage], vector[0], DEPTH);
        let (batches, _verified, rejected, _authorized) =
            orc_aggregator::subscriber_state(signer::address_of(alice));
        assert!(batches == 2, 0);
        assert!(rejected == 1, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_update_tolerates_short_record(admin: &signer, alice: &signer) {
        setup(admin, CHUNK, 1);
        orc_aggregator::bench_onboard(alice, 0);

        // A header with three payload bytes behind it: every field past the
        // third byte reads as zero.
        let stub = signed_message(x"010203");
        assert!(orc_decode::n_records(&stub) == 1, 0);
        orc_aggregator::bench_update_ed25519(
            alice, 0, stub, vector[BATCH_ED_SIG], vector[0], DEPTH);
        let (price, aggregate, conf, ts, updates) = orc_registry::read(0);
        assert!(updates == 1, 0);
        assert!(price == 0, 0);
        assert!(aggregate == 0, 0);
        assert!(conf == 0, 0);
        assert!(ts == 0, 0);

        // Too short to even hold a header: nothing to write, still no abort.
        assert!(orc_decode::n_records(&x"010203") == 0, 0);
        orc_aggregator::bench_update_ed25519(
            alice, 0, x"010203", vector[BATCH_ED_SIG], vector[0], DEPTH);
        let (_price, _aggregate, _conf, _ts, updates) = orc_registry::read(0);
        assert!(updates == 1, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_update_tolerates_feed_past_num_feeds(
        admin: &signer, alice: &signer
    ) {
        setup(admin, 2, 1);
        orc_aggregator::bench_onboard(alice, 1000);
        assert!(!orc_registry::contains(1000), 0);
        orc_aggregator::bench_update_ed25519(
            alice, 1000, BATCH, vector[BATCH_ED_SIG], vector[0], DEPTH);
        assert!(orc_registry::contains(1000), 0);
        assert!(orc_registry::contains(1003), 0);
        // The registry grows to cover the highest feed it was asked to write.
        assert!(orc_registry::num_feeds() == 1004, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_quorum_tolerates_unmet_quorum(admin: &signer, alice: &signer) {
        setup(admin, CHUNK, 5);
        orc_aggregator::bench_onboard(alice, 0);
        orc_aggregator::bench_quorum(
            alice,
            0,
            BATCH,
            vector[BATCH_ED_SIG],
            vector[0],
            vector::empty<vector<u8>>(),
            vector::empty<u8>(),
            DEPTH,
        );
        let (batches, verified, rejected, _authorized) =
            orc_aggregator::subscriber_state(signer::address_of(alice));
        assert!(verified == 1, 0);
        assert!(rejected == 0, 0);
        // One verified signature is short of the quorum, so nothing published.
        assert!(batches == 0, 0);
        let (_price, _aggregate, _conf, _ts, updates) = orc_registry::read(0);
        assert!(updates == 0, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_quorum_counts_only_authorized_signatures(
        admin: &signer, alice: &signer
    ) {
        setup(admin, CHUNK, 2);
        orc_aggregator::bench_onboard(alice, 0);
        // The right signature under the wrong recovery id recovers a key the
        // authority set does not hold, so it cannot make up a quorum.
        orc_aggregator::bench_quorum(
            alice,
            0,
            BATCH,
            vector[BATCH_ED_SIG],
            vector[0],
            vector[BATCH_SECP_SIG],
            vector[BATCH_SECP_ID + 1],
            DEPTH,
        );
        let (batches, verified, rejected, authorized) =
            orc_aggregator::subscriber_state(signer::address_of(alice));
        assert!(verified == 1, 0);
        assert!(rejected == 1, 0);
        assert!(authorized == 0, 0);
        assert!(batches == 0, 0);
        let (_price, _aggregate, _conf, _ts, updates) = orc_registry::read(0);
        assert!(updates == 0, 0);

        // Under the id it was signed with, the same pair meets the quorum.
        orc_aggregator::bench_quorum(
            alice,
            0,
            BATCH,
            vector[BATCH_ED_SIG],
            vector[0],
            vector[BATCH_SECP_SIG],
            vector[BATCH_SECP_ID],
            DEPTH,
        );
        let (batches, _verified, _rejected, authorized) =
            orc_aggregator::subscriber_state(signer::address_of(alice));
        assert!(batches == 1, 0);
        assert!(authorized == 1, 0);
        let (_price, _aggregate, _conf, _ts, updates) = orc_registry::read(0);
        assert!(updates == 1, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_update_secp256k1_counts_an_unauthorized_recovery(
        admin: &signer, alice: &signer
    ) {
        setup(admin, CHUNK, 1);
        orc_aggregator::bench_onboard(alice, 0);
        orc_aggregator::bench_update_secp256k1(
            alice, 0, BATCH, vector[BATCH_SECP_SIG], vector[BATCH_SECP_ID + 1], DEPTH);
        let (batches, verified, rejected, authorized) =
            orc_aggregator::subscriber_state(signer::address_of(alice));
        assert!(batches == 1, 0);
        assert!(verified == 0, 0);
        assert!(rejected == 1, 0);
        assert!(authorized == 0, 0);
        // The batch still lands: only the count changes, never the write path.
        let (_price, _aggregate, _conf, _ts, updates) = orc_registry::read(0);
        assert!(updates == 1, 0);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_read_aggregate_tolerates_missing_feeds(
        admin: &signer, alice: &signer
    ) {
        setup(admin, 1, 1);
        orc_aggregator::bench_onboard(alice, 0);
        assert!(orc_aggregator::read_aggregate(500, 16) == 0, 0);
        orc_aggregator::bench_read_aggregate(alice, 500, 16);
    }

    #[test(admin = @bench, alice = @0xa11ce)]
    fun test_aggregate_is_median_of_history(admin: &signer, alice: &signer) {
        setup(admin, 1, 1);
        orc_aggregator::bench_onboard(alice, 0);
        let prices = vector[50u128, 10, 30];
        let i = 0;
        while (i < vector::length(&prices)) {
            let record =
                signed_message(make_record(0, *vector::borrow(&prices, i), 1, 100 + i));
            orc_aggregator::bench_write_only(alice, 0, record, 1);
            i = i + 1;
        };
        let (price, aggregate, _conf, _ts, updates) = orc_registry::read(0);
        assert!(updates == 3, 0);
        assert!(price == 30, 0);
        // History is [50, 10, 30] and its median is 30.
        assert!(aggregate == 30, 0);
    }

    #[test(admin = @bench)]
    fun test_create_feeds_chunks_are_additive(admin: &signer) {
        orc_registry::create_feeds(admin, CHUNK);
        assert!(orc_registry::num_feeds() == CHUNK, 0);
        orc_registry::create_feeds(admin, CHUNK);
        assert!(orc_registry::num_feeds() == 2 * CHUNK, 0);
        assert!(orc_registry::contains(2 * CHUNK - 1), 0);
        assert!(!orc_registry::contains(2 * CHUNK), 0);
    }

    #[test]
    fun test_decode_skips_the_signed_message_header() {
        // A two-byte BCS length for 192 payload bytes, one byte for 48.
        assert!(orc_decode::payload_offset(&BATCH) == 34, 0);
        assert!(orc_decode::payload_offset(&ONE) == 33, 0);
        assert!(orc_decode::n_records(&BATCH) == RECORDS_IN_BATCH, 0);
        assert!(orc_decode::n_records(&ONE) == 1, 0);
        assert!(
            orc_decode::payload_offset(&signed_message(make_record(0, 0, 0, 0)))
                == 33,
            0,
        );
    }

    #[test]
    fun test_decode_reads_big_endian_fields() {
        let (feed, price, conf, ts) = orc_decode::decode_record(&ONE, 0, 1);
        assert!(feed == 0, 0);
        assert!(price == 100000000, 0);
        assert!(conf == 10, 0);
        assert!(ts == 1700000000, 0);
    }

    #[test]
    fun test_decode_indexes_records() {
        assert!(orc_decode::record_len() == 48, 0);
        let (feed, price, conf, ts) = orc_decode::decode_record(&BATCH, 2, 1);
        assert!(feed == 2, 0);
        assert!(price == 100000014, 0);
        assert!(conf == 12, 0);
        assert!(ts == 1700000120, 0);
    }

    #[test]
    fun test_decode_matches_hand_built_record() {
        let record = make_record(7, 123456789012345, 99, 1234567890);
        assert!(vector::length(&record) == 48, 0);
        assert!(orc_decode::byte_at(&record, 7) == 7, 0);
        assert!(orc_decode::u64_at(&record, 0) == 7, 0);
        let message = signed_message(record);
        assert!(vector::length(&message) == 81, 0);
        let (feed, price, conf, ts) = orc_decode::decode_record(&message, 0, 3);
        assert!(feed == 7, 0);
        assert!(price == 123456789012345, 0);
        assert!(conf == 99, 0);
        assert!(ts == 1234567890, 0);
    }

    #[test]
    fun test_decode_depth_repeats_the_same_decode() {
        let (feed_one, price_one, conf_one, ts_one) =
            orc_decode::decode_record(&BATCH, 1, 1);
        let (feed_many, price_many, conf_many, ts_many) =
            orc_decode::decode_record(&BATCH, 1, 8);
        assert!(feed_one == feed_many, 0);
        assert!(price_one == price_many, 0);
        assert!(conf_one == conf_many, 0);
        assert!(ts_one == ts_many, 0);
        // A zero depth decodes nothing rather than aborting.
        let (feed_none, price_none, _conf, _ts) =
            orc_decode::decode_record(&BATCH, 1, 0);
        assert!(feed_none == 0, 0);
        assert!(price_none == 0, 0);
    }

    #[test]
    fun test_decode_zero_fills_past_the_end() {
        let stub = x"0102030405060708090a";
        assert!(orc_decode::byte_at(&stub, 9) == 10, 0);
        assert!(orc_decode::byte_at(&stub, 10) == 0, 0);
        assert!(orc_decode::byte_at(&stub, 10000) == 0, 0);
        assert!(orc_decode::u64_at(&stub, 0) == 72623859790382856, 0);
        assert!(orc_decode::u64_at(&stub, 64) == 0, 0);
        // A header that runs off the end leaves an offset past the blob.
        assert!(orc_decode::payload_offset(&stub) == 33, 0);
        assert!(orc_decode::n_records(&stub) == 0, 0);
        assert!(orc_decode::n_records(&vector::empty<u8>()) == 0, 0);
        let (feed, price, conf, ts) = orc_decode::decode_record(&stub, 0, 2);
        assert!(feed == 0, 0);
        assert!(price == 0, 0);
        assert!(conf == 0, 0);
        assert!(ts == 0, 0);
    }

    #[test]
    fun test_median_of_k() {
        assert!(orc_aggregator::median(vector::empty<u128>()) == 0, 0);
        assert!(orc_aggregator::median(vector[5u128]) == 5, 0);
        assert!(orc_aggregator::median(vector[9u128, 1, 5]) == 5, 0);
        assert!(orc_aggregator::median(vector[1u128, 5, 9]) == 5, 0);
        assert!(orc_aggregator::median(vector[4u128, 1, 3, 2]) == 3, 0);
        assert!(orc_aggregator::median(vector[7u128, 7, 7, 1, 9]) == 7, 0);
    }
}
